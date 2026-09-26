#[cfg(not(all(target_os = "linux", target_arch = "aarch64", feature = "jack-runtime")))]
fn main() {
    eprintln!("lvb-arm-plugin-standalone requires Linux AArch64 and --features jack-runtime");
    std::process::exit(64);
}

#[cfg(all(target_os = "linux", target_arch = "aarch64", feature = "jack-runtime"))]
fn main() {
    if let Err(error) = appliance::entry() {
        eprintln!("PI_STANDALONE_REFUSAL {error}");
        std::process::exit(1);
    }
}

#[cfg(all(target_os = "linux", target_arch = "aarch64", feature = "jack-runtime"))]
mod appliance {
    use ap2_backend::rpi0::{Identity, Instance, Message, STATE_CAPACITY};
    use lvb_arm_pigments_standalone::{
        config::{hex, Config},
        contract, master, preparation,
        retirement::RetirementStatus,
        startup,
        supervisor::{read_journal, Cohort},
    };
    use lvb_arm_standalone::{
        audio::ParameterUpdate,
        jack::{Client, Control},
        midi::ControllerPolicy,
        protocol::ArchitectureHandshake,
    };
    use std::{
        env, fs,
        io::{self, BufRead, Read, Write},
        os::unix::fs::{OpenOptionsExt, PermissionsExt},
        path::{Path, PathBuf},
        sync::{atomic::Ordering, mpsc},
        thread,
        time::{Duration, Instant},
    };

    pub fn entry() -> io::Result<()> {
        let arguments = env::args_os().collect::<Vec<_>>();
        if !matches!(arguments.len(), 2 | 4 | 5) {
            return Err(invalid(
                "usage: lvb-arm-plugin-standalone /absolute/config [--state /absolute/state [--prewarm]]",
            ));
        }
        let state = if arguments.len() >= 4 {
            if arguments[2] != "--state" {
                return Err(invalid("expected --state"));
            }
            Some(absolute_path(&arguments[3].to_string_lossy())?)
        } else {
            None
        };
        let prewarm = arguments.len() == 5;
        if prewarm && arguments[4] != "--prewarm" {
            return Err(invalid("expected --prewarm after --state"));
        }
        let config = Config::load(Path::new(&arguments[1]))?;
        preparation::selected_plan(&config.binding, prewarm, state.is_some())?;
        let session = random_session()?;
        run(config, session, state, prewarm)
    }

    fn run(
        config: Config,
        session: [u8; 16],
        state: Option<PathBuf>,
        prewarm: bool,
    ) -> io::Result<()> {
        let identity = Identity {
            class: config.binding.class,
            module: config.plugin.sha256,
        };
        let prepared_bytes = if prewarm {
            Some(preparation::read_private_state(
                state
                    .as_ref()
                    .ok_or_else(|| invalid("prepared startup requires state"))?,
                identity,
            ).map_err(|error| {
                eprintln!("PI_PREPARATION_RECEIPT status=refused stage=state_input state_sha256=unavailable");
                error
            })?)
        } else {
            None
        };
        let session_hex = hex(&session);
        let directory = config
            .prefix()
            .join("drive_c/bridge/sessions")
            .join(&session_hex);
        fs::create_dir(&directory)?;
        fs::set_permissions(&directory, fs::Permissions::from_mode(0o700))?;
        let retirement = RetirementStatus::create(&directory, &session)?;
        let architecture = ArchitectureHandshake::native(page_size()?);
        atomic_new(&directory.join("rpi0.arch.native"), &architecture.encode())?;
        let bridge_directory = directory.clone();
        let delay = config.bridge_frames;
        let bridge =
            thread::spawn(move || Instance::open_bound(bridge_directory, session, identity, delay));
        let ready = directory.join(format!("{session_hex}.ready"));
        let gate = directory.join(format!("{session_hex}.gate"));
        let expected = contract::handshake(&config, &session_hex)?;
        let host_arguments = contract::host_arguments(&config, &session_hex)?;
        let cohort = Cohort::launch(&config, &session_hex, &host_arguments)?;
        wait_exact(&ready, expected.as_bytes(), Duration::from_secs(180))?;
        let peer_bytes = wait_bounded(
            &directory.join("rpi0.arch.windows"),
            64,
            Duration::from_secs(30),
        )?;
        let peer = ArchitectureHandshake::decode(&peer_bytes)
            .ok_or_else(|| invalid("Windows architecture handshake extent"))?;
        if !architecture.accepts(&peer) {
            return Err(invalid("Windows architecture handshake differs"));
        }
        config.verify_files()?;
        atomic_new(&gate, expected.as_bytes())?;
        let instance = bridge
            .join()
            .map_err(|_| invalid("backend open thread panicked"))??;
        startup::await_initialization(&cohort, &session_hex, Duration::from_secs(120))?;

        let control = Control::new();
        let mut jack = Client::open_with_io(
            &config.jack_client,
            &instance,
            &control,
            ControllerPolicy::TrackedNoteOffs,
            config.binding.stereo_input,
            config.binding.midi_input,
        )?;
        let traits = instance.setup(jack.buffer_size(), 48_000.0, &config.binding.buses)?;
        if let Some(path) = &state {
            if prewarm {
                let bytes = prepared_bytes
                    .as_ref()
                    .ok_or_else(|| invalid("prepared startup state absent"))?;
                match preparation::execute(&instance, &config.binding, identity, bytes) {
                    Ok(receipt) => println!("PI_PREPARATION_RECEIPT status=complete {receipt}"),
                    Err(failure) => {
                        eprintln!("PI_PREPARATION_RECEIPT status=refused {}", failure.receipt);
                        drop(jack);
                        let close = instance.close();
                        let ready = retirement.await_ready(Duration::from_secs(30));
                        let retired = cohort.retire();
                        if close.is_ok() && ready.is_ok() && retired.is_ok() {
                            let _ = cleanup_session(&directory);
                        }
                        return Err(failure.error);
                    }
                }
            } else {
                let bytes = read_state(path)?;
                let mut readback = vec![0; STATE_CAPACITY];
                let count = instance.restore_state(&bytes, &mut readback)?;
                println!(
                    "PI_STATE_RESTORED input_bytes={} returned_bytes={count}",
                    bytes.len()
                );
            }
        }
        instance.activate(256)?;
        instance.start()?;
        jack.activate()?;
        let ports = jack.ports();
        println!("PI_READY midi={} left={} right={} jack_frames={} quantum=256 reserve={} vendor_frames={} total_frames={} tail_frames={} protocol=12 map=1",
            ports[0], ports[1], ports[2], jack.buffer_size(), config.bridge_frames,
            traits.vendor_frames, traits.total_frames, traits.tail_frames);
        let result = commands(
            &instance, &control, &cohort, &mut jack, &config, prewarm, identity,
        );

        let jack_frames = jack.buffer_size();
        let deactivate_jack = jack.deactivate();
        drop(jack);
        print_metrics(&instance, &control, jack_frames);
        let stop = instance.stop();
        let off = instance.deactivate();
        let close = instance.close();
        let retirement_result = retirement.await_ready(Duration::from_secs(30));
        let unit = cohort.identity().unit.clone();
        let retire = cohort.retire();
        drop(retirement);
        let cleanup = if close.is_ok() && retirement_result.is_ok() && retire.is_ok() {
            let summary = format!("{{\"event\":\"pi_callback_summary\",\"callbacks\":{},\"missing_frames\":{},\"expired_frames\":{},\"gaps\":{},\"delivered_frames\":{},\"jack_frames\":{},\"bridge_frames\":{}}}\n",
                control.metrics.callbacks.load(Ordering::Acquire),
                control.metrics.missing_frames.load(Ordering::Acquire),
                control.metrics.expired_frames.load(Ordering::Acquire),
                control.metrics.gaps.load(Ordering::Acquire),
                control.metrics.delivered_frames.load(Ordering::Acquire),
                jack_frames, config.bridge_frames);
            preserve_evidence(
                &directory.join("rpi0.performance.jsonl"),
                &config
                    .evidence_directory
                    .join(format!("{session_hex}-native.jsonl")),
                summary.as_bytes(),
            )
            .and_then(|_| read_journal(&unit))
            .and_then(|bytes| {
                atomic_new(
                    &config
                        .evidence_directory
                        .join(format!("{session_hex}-windows.log")),
                    &bytes,
                )
            })
            .and_then(|_| cleanup_session(&directory))
        } else {
            Err(invalid(
                "session retained after incomplete close or retirement",
            ))
        };
        result?;
        deactivate_jack?;
        stop?;
        off?;
        close?;
        retirement_result?;
        retire?;
        cleanup?;
        println!("PI_CLEAN_SHUTDOWN session={session_hex}");
        Ok(())
    }

    fn commands(
        instance: &Instance,
        control: &Control,
        cohort: &Cohort,
        jack: &mut Client,
        config: &Config,
        prewarm: bool,
        identity: Identity,
    ) -> io::Result<()> {
        let generation = instance.gui_generation()?;
        instance.gui_capabilities(generation, 7)?;
        request_parameter_snapshot(instance, generation)?;
        let (sender, receiver) = mpsc::channel();
        thread::spawn(move || {
            for line in io::stdin().lock().lines() {
                if sender
                    .send(line.unwrap_or_else(|_| "quit".to_owned()))
                    .is_err()
                {
                    break;
                }
            }
        });
        loop {
            drain_gui(instance, generation, control, &config.binding)?;
            match receiver.recv_timeout(Duration::from_millis(5)) {
                Ok(line) => {
                    let line = line.trim();
                    if line == "quit" {
                        return Ok(());
                    }
                    if line == "status" {
                        print_metrics(instance, control, jack.buffer_size());
                        println!("PI_COHORT_PROCESSES {}", cohort.verify()?.len());
                    } else if let Some(path) = line.strip_prefix("save ") {
                        let path = absolute_path(path)?;
                        let mut bytes = vec![0; STATE_CAPACITY];
                        let count = instance.capture_state(&mut bytes)?;
                        atomic_new(&path, &bytes[..count])?;
                        println!("PI_STATE_SAVED bytes={count}");
                    } else if let Some(path) = line.strip_prefix("restore prepared ") {
                        if preparation::selected_plan(&config.binding, prewarm, true)?.is_none() {
                            return Err(invalid(
                                "prepared restore requires startup --prewarm authorization",
                            ));
                        }
                        let bytes = preparation::read_private_state(&absolute_path(path)?, identity)
                            .map_err(|error| {
                                eprintln!("PI_PREPARATION_RECEIPT status=refused stage=state_input state_sha256=unavailable");
                                error
                            })?;
                        match preparation::restore_prepared(
                            instance,
                            jack,
                            &config.binding,
                            identity,
                            &bytes,
                        ) {
                            Ok(receipt) => {
                                println!("PI_PREPARATION_RECEIPT status=complete {receipt}")
                            }
                            Err(failure) => {
                                eprintln!(
                                    "PI_PREPARATION_RECEIPT status=refused {}",
                                    failure.receipt
                                );
                                return Err(failure.error);
                            }
                        }
                        request_parameter_snapshot(instance, generation)?;
                    } else if let Some(path) = line.strip_prefix("restore ") {
                        let bytes = read_state(&absolute_path(path)?)?;
                        let mut readback = vec![0; STATE_CAPACITY];
                        jack.pause_processing()?;
                        instance.stop()?;
                        instance.deactivate()?;
                        let count = instance.restore_state(&bytes, &mut readback)?;
                        instance.activate(256)?;
                        instance.start()?;
                        jack.resume_processing();
                        request_parameter_snapshot(instance, generation)?;
                        println!(
                            "PI_STATE_RESTORED input_bytes={} returned_bytes={count}",
                            bytes.len()
                        );
                    } else if let Some(arguments) = line.strip_prefix("parameter ") {
                        let words = arguments.split_whitespace().collect::<Vec<_>>();
                        if words.len() != 2 {
                            return Err(invalid("parameter requires id and normalized value"));
                        }
                        let id = words[0]
                            .parse::<u32>()
                            .map_err(|_| invalid("parameter id"))?;
                        config.binding.control(id)?;
                        let value = master::normalized(words[1])?;
                        control
                            .parameters
                            .push(ParameterUpdate { id, value })
                            .map_err(|_| invalid("parameter queue full"))?;
                        instance.gui_command(
                            generation,
                            &mut Message {
                                kind: 3,
                                id,
                                value,
                                ..Message::default()
                            },
                        )?;
                        request_parameter_snapshot(instance, generation)?;
                        println!("PI_PARAMETER_QUEUED id={id} normalized={value:.17}");
                    } else if !line.is_empty() {
                        return Err(invalid("unknown command"));
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => return Ok(()),
            }
        }
    }

    fn request_parameter_snapshot(instance: &Instance, generation: u64) -> io::Result<()> {
        instance.gui_command(
            generation,
            &mut Message {
                kind: 4,
                flags: 1 << 2,
                ..Message::default()
            },
        )
    }

    fn drain_gui(
        instance: &Instance,
        generation: u64,
        control: &Control,
        binding: &lvb_arm_pigments_standalone::binding::Binding,
    ) -> io::Result<()> {
        for _ in 0..128 {
            let mut message = Message::default();
            instance.gui_take(generation, &mut message)?;
            if message.kind == 0 {
                break;
            }
            if message.kind == 110 && binding.controls.iter().any(|p| p.id == message.id) {
                let readback = binding.readback(&message)?;
                if let Some(value) = readback.value {
                    println!(
                        "PI_PARAMETER_READBACK id={} normalized={value:.17} revision={}",
                        message.id, message.revision
                    );
                } else {
                    println!(
                        "PI_PARAMETER_UNAVAILABLE id={} revision={}",
                        message.id, message.revision
                    );
                }
            }
            if message.kind == 102 {
                control
                    .parameters
                    .push(ParameterUpdate {
                        id: message.id,
                        value: message.value,
                    })
                    .map_err(|_| invalid("GUI parameter queue full"))?;
            }
        }
        Ok(())
    }

    fn print_metrics(instance: &Instance, control: &Control, jack_frames: u32) {
        let m = &control.metrics;
        if let Ok(s) = instance.stats() {
            println!("PI_METRICS callbacks={} processed={} processed_frames={} delivered_frames={} missing_frames={} expired_frames={} gaps={} xruns={} callback_ns_max={} output_nonzero_l={} output_nonzero_r={} fault={} request_high={} result_high={} jack_frames={jack_frames}",
                m.callbacks.load(Ordering::Acquire), s.processed,
                instance.processed_frames().unwrap_or(0), m.delivered_frames.load(Ordering::Acquire),
                m.missing_frames.load(Ordering::Acquire), m.expired_frames.load(Ordering::Acquire),
                m.gaps.load(Ordering::Acquire), m.xruns.load(Ordering::Acquire),
                m.callback_ns_max.load(Ordering::Acquire),
                m.output_nonzero[0].load(Ordering::Acquire), m.output_nonzero[1].load(Ordering::Acquire),
                s.fault, s.request_high, s.result_high);
        }
    }

    fn wait_exact(path: &Path, expected: &[u8], duration: Duration) -> io::Result<()> {
        let end = Instant::now() + duration;
        loop {
            match fs::read(path) {
                Ok(bytes) if bytes == expected => return Ok(()),
                Ok(_) => return Err(invalid("Windows readiness binding differs")),
                Err(error) if error.kind() == io::ErrorKind::NotFound && Instant::now() < end => {
                    thread::sleep(Duration::from_millis(20))
                }
                Err(error) if error.kind() == io::ErrorKind::NotFound => {
                    return Err(invalid("Windows readiness deadline"))
                }
                Err(error) => return Err(error),
            }
        }
    }
    fn wait_bounded(path: &Path, exact: usize, duration: Duration) -> io::Result<Vec<u8>> {
        let end = Instant::now() + duration;
        loop {
            match fs::read(path) {
                Ok(bytes) if bytes.len() == exact => return Ok(bytes),
                Ok(_) => return Err(invalid("bounded file extent differs")),
                Err(error) if error.kind() == io::ErrorKind::NotFound && Instant::now() < end => {
                    thread::sleep(Duration::from_millis(20))
                }
                Err(error) if error.kind() == io::ErrorKind::NotFound => {
                    return Err(invalid("Windows architecture handshake deadline"))
                }
                Err(error) => return Err(error),
            }
        }
    }
    fn atomic_new(path: &Path, bytes: &[u8]) -> io::Result<()> {
        if !path.is_absolute() || bytes.len() > 16 * 1024 * 1024 {
            return Err(invalid("bounded absolute output required"));
        }
        let temporary = path.with_extension("pi-tmp");
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&temporary)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        fs::rename(temporary, path)
    }
    fn absolute_path(value: &str) -> io::Result<PathBuf> {
        let path = PathBuf::from(value);
        if !path.is_absolute() || path.file_name().is_none() {
            return Err(invalid("absolute file path required"));
        }
        Ok(path)
    }
    fn read_state(path: &Path) -> io::Result<Vec<u8>> {
        let metadata = fs::symlink_metadata(path)?;
        if !metadata.is_file()
            || metadata.file_type().is_symlink()
            || metadata.len() > STATE_CAPACITY as u64
        {
            return Err(invalid("state extent or file type"));
        }
        fs::read(path)
    }
    fn random_session() -> io::Result<[u8; 16]> {
        let mut value = [0; 16];
        fs::File::open("/dev/urandom")?.read_exact(&mut value)?;
        Ok(value)
    }
    fn page_size() -> io::Result<u32> {
        let value = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
        if !(4096..=65536).contains(&value) || !(value as u64).is_power_of_two() {
            return Err(invalid("unsupported native page size"));
        }
        Ok(value as u32)
    }
    fn cleanup_session(directory: &Path) -> io::Result<()> {
        for entry in fs::read_dir(directory)? {
            let entry = entry?;
            if !entry.metadata()?.is_file() {
                return Err(invalid("non-file remained in session directory"));
            }
            fs::remove_file(entry.path())?;
        }
        fs::remove_dir(directory)
    }
    fn preserve_evidence(source: &Path, destination: &Path, suffix: &[u8]) -> io::Result<()> {
        let mut bytes = fs::read(source)?;
        if bytes.is_empty() || bytes.len() + suffix.len() > 2 * 1024 * 1024 {
            return Err(invalid("performance evidence extent"));
        }
        bytes.extend_from_slice(suffix);
        atomic_new(destination, &bytes)
    }
    fn invalid(message: impl Into<String>) -> io::Error {
        io::Error::other(message.into())
    }
}
