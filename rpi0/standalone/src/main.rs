#[cfg(not(all(target_os = "linux", target_arch = "aarch64", feature = "jack-runtime")))]
fn main() {
    eprintln!("lvb-arm-standalone requires Linux AArch64 and --features jack-runtime");
    std::process::exit(64);
}

#[cfg(all(target_os = "linux", target_arch = "aarch64", feature = "jack-runtime"))]
fn main() {
    if let Err(error) = appliance::entry() {
        eprintln!("RPI0 refusal: {error}");
        std::process::exit(1);
    }
}

#[cfg(all(target_os = "linux", target_arch = "aarch64", feature = "jack-runtime"))]
mod appliance {
    use ap2_backend::rpi0::{Identity as BridgeIdentity, Instance, Message, STATE_CAPACITY};
    use lvb_arm_standalone::{
        audio::ParameterUpdate,
        config::{hex, Config},
        jack::{Client, Control},
        protocol::{instrument_bus_contract, ArchitectureHandshake},
        supervisor::{read_journal, run_owned_probe, Cohort},
    };
    use std::{
        env,
        ffi::OsString,
        fs,
        io::{self, BufRead, Write},
        os::unix::fs::{OpenOptionsExt, PermissionsExt},
        path::{Path, PathBuf},
        sync::{atomic::Ordering, mpsc},
        thread,
        time::{Duration, Instant},
    };

    const PROCESSOR_CLASS: [u8; 16] = [
        0x52, 0x50, 0x49, 0x30, 0x41, 0x52, 0x4d, 0x41, 0x50, 0x50, 0x53, 0x59, 0x4e, 0x54, 0x00,
        0x01,
    ];

    pub fn entry() -> io::Result<()> {
        let args = env::args_os().collect::<Vec<_>>();
        if args.len() != 3 || args[2].is_empty() {
            return Err(invalid(
                "usage: lvb-arm-standalone <preflight|run> /absolute/config",
            ));
        }
        let action = args[1].to_str().ok_or_else(|| invalid("action encoding"))?;
        let config = Config::load(Path::new(&args[2]))?;
        let session = random_session()?;
        let session_hex = hex(&session);
        match action {
            "preflight" => preflight(&config, &session_hex),
            "run" => run(config, session, session_hex),
            _ => Err(invalid("unknown action")),
        }
    }

    fn preflight(config: &Config, session: &str) -> io::Result<()> {
        run_owned_probe(config, session, false)?;
        run_owned_probe(config, session, true)?;
        let directory = config.prefix.join("drive_c/bridge/sessions").join(session);
        fs::create_dir(&directory)?;
        fs::set_permissions(&directory, fs::Permissions::from_mode(0o700))?;
        let stop = directory.join("host-preflight.stop");
        let stop_windows = windows_path(&config.prefix, &stop)?;
        let arguments = vec![
            config.windows_host.path.as_os_str().to_owned(),
            OsString::from("--rpi0-preflight"),
            stop_windows.into(),
        ];
        let cohort = Cohort::launch(config, session, &arguments)?;
        cohort.verify()?;
        atomic_new(&stop, b"stop\n")?;
        cohort.await_retired(Duration::from_secs(15))?;
        cleanup_session(&directory)?;
        println!("RPI0_PREFLIGHT_PASS session={session}");
        Ok(())
    }

    fn run(config: Config, session: [u8; 16], session_hex: String) -> io::Result<()> {
        let directory = config
            .prefix
            .join("drive_c/bridge/sessions")
            .join(&session_hex);
        fs::create_dir(&directory)?;
        fs::set_permissions(&directory, fs::Permissions::from_mode(0o700))?;
        let identity = BridgeIdentity {
            class: PROCESSOR_CLASS,
            module: config.plugin.sha256,
        };
        let architecture = ArchitectureHandshake::native(page_size()?);
        atomic_new(&directory.join("rpi0.arch.native"), &architecture.encode())?;
        let bridge_directory = directory.clone();
        let delay = config.bridge_frames;
        let bridge =
            thread::spawn(move || Instance::open_bound(bridge_directory, session, identity, delay));

        let ready = directory.join(format!("{session_hex}.ready"));
        let gate = directory.join(format!("{session_hex}.gate"));
        let expected = handshake(&config, &session_hex);
        let module = windows_path(&config.prefix, &config.plugin.path)?;
        let arguments = host_arguments(&config, &session_hex, &module);
        let cohort = Cohort::launch(&config, &session_hex, &arguments)?;
        wait_exact(&ready, expected.as_bytes(), Duration::from_secs(15))?;
        let peer_bytes = wait_bounded(
            &directory.join("rpi0.arch.windows"),
            64,
            Duration::from_secs(15),
        )?;
        let peer = ArchitectureHandshake::decode(&peer_bytes)
            .ok_or_else(|| invalid("Windows architecture handshake extent"))?;
        if !architecture.accepts(&peer) {
            return Err(invalid("Windows architecture handshake differs"));
        }
        atomic_new(&gate, expected.as_bytes())?;
        let instance = bridge
            .join()
            .map_err(|_| invalid("backend open thread panicked"))??;

        let control = Control::new();
        let mut jack = Client::open(&config.jack_client, &instance, &control)?;
        let buses = instrument_bus_contract();
        let traits = instance.setup(jack.buffer_size(), 48_000.0, &buses)?;
        instance.activate(256)?;
        instance.start()?;
        jack.activate()?;
        let ports = jack.ports();
        println!(
            "RPI0_READY midi={} left={} right={} jack_frames={} bridge_frames={} vendor_frames={} total_frames={}",
            ports[0], ports[1], ports[2], jack.buffer_size(), config.bridge_frames,
            traits.vendor_frames, traits.total_frames
        );
        println!("RPI0_LATENCY tail_frames={}", traits.tail_frames);
        println!(
            "Commands: open | close | save /absolute/path | restore /absolute/path | status | quit"
        );
        let result = command_loop(&instance, &control, &cohort, &mut jack);

        let jack_frames = jack.buffer_size();
        let deactivate = jack.deactivate();
        drop(jack);
        let metrics = &control.metrics;
        println!(
            "RPI0_CALLBACK callbacks={} failures={} deadline_misses={} callback_ns_max={} missing_frames={} expired_frames={} gaps={} delivered_frames={} priming_frames={} paused_frames={} unsupported_midi={} malformed_midi={} overflow_midi={}",
            metrics.callbacks.load(Ordering::Acquire), metrics.process_failures.load(Ordering::Acquire),
            metrics.deadline_misses.load(Ordering::Acquire), metrics.callback_ns_max.load(Ordering::Acquire),
            metrics.missing_frames.load(Ordering::Acquire), metrics.expired_frames.load(Ordering::Acquire),
            metrics.gaps.load(Ordering::Acquire), metrics.delivered_frames.load(Ordering::Acquire),
            metrics.priming_frames.load(Ordering::Acquire), metrics.paused_frames.load(Ordering::Acquire),
            metrics.unsupported_midi.load(Ordering::Acquire),
            metrics.malformed_midi.load(Ordering::Acquire), metrics.overflow_midi.load(Ordering::Acquire)
        );
        let callback_summary = format!(
            "{{\"event\":\"rpi0_callback_summary\",\"callbacks\":{},\"process_failures\":{},\"deadline_misses\":{},\"callback_ns_max\":{},\"missing_frames\":{},\"expired_frames\":{},\"gaps\":{},\"delivered_frames\":{},\"priming_frames\":{},\"paused_frames\":{},\"unsupported_midi\":{},\"malformed_midi\":{},\"overflow_midi\":{},\"jack_frames\":{},\"bridge_frames\":{}}}\n",
            metrics.callbacks.load(Ordering::Acquire), metrics.process_failures.load(Ordering::Acquire),
            metrics.deadline_misses.load(Ordering::Acquire), metrics.callback_ns_max.load(Ordering::Acquire),
            metrics.missing_frames.load(Ordering::Acquire), metrics.expired_frames.load(Ordering::Acquire),
            metrics.gaps.load(Ordering::Acquire), metrics.delivered_frames.load(Ordering::Acquire),
            metrics.priming_frames.load(Ordering::Acquire), metrics.paused_frames.load(Ordering::Acquire),
            metrics.unsupported_midi.load(Ordering::Acquire),
            metrics.malformed_midi.load(Ordering::Acquire), metrics.overflow_midi.load(Ordering::Acquire),
            jack_frames, config.bridge_frames
        );
        let stop = instance.stop();
        let off = instance.deactivate();
        let stats = instance.stats();
        let close = instance.close();
        let unit = cohort.identity().unit.clone();
        let retire = cohort.await_retired(Duration::from_secs(15));
        let cleanup = if close.is_ok() && retire.is_ok() {
            preserve_evidence(
                &directory.join("rpi0.performance.jsonl"),
                &config.evidence_path,
                callback_summary.as_bytes(),
            )
            .and_then(|_| read_journal(&unit))
            .and_then(|bytes| atomic_new(&config.windows_evidence_path, &bytes))
            .and_then(|_| cleanup_session(&directory))
        } else {
            Err(invalid(
                "session retained because close or cohort retirement failed",
            ))
        };
        result?;
        deactivate?;
        stop?;
        off?;
        if let Ok(stats) = stats {
            println!(
                "RPI0_TRANSPORT fault={} processed={} request_high={} result_high={} position={} epoch={}",
                stats.fault, stats.processed, stats.request_high, stats.result_high, stats.position, stats.epoch
            );
        }
        close?;
        retire?;
        cleanup?;
        println!("RPI0_CLEAN_SHUTDOWN session={session_hex}");
        Ok(())
    }

    fn command_loop(
        instance: &Instance,
        control: &Control,
        cohort: &Cohort,
        jack: &mut Client,
    ) -> io::Result<()> {
        let generation = instance.gui_generation()?;
        instance.gui_capabilities(generation, 7)?;
        let (sender, receiver) = mpsc::channel();
        thread::spawn(move || {
            let input = io::stdin();
            for line in input.lock().lines() {
                if sender
                    .send(line.unwrap_or_else(|_| "quit".to_owned()))
                    .is_err()
                {
                    break;
                }
            }
        });
        let mut native_view = 0u64;
        let mut view_epoch = 0u32;
        loop {
            drain_gui(instance, generation, control, &mut view_epoch)?;
            match receiver.recv_timeout(Duration::from_millis(5)) {
                Ok(line) => {
                    let line = line.trim();
                    if line == "quit" {
                        return Ok(());
                    }
                    if line == "status" {
                        let pids = cohort.verify()?;
                        let memory_peak =
                            fs::read_to_string(cohort.identity().cgroup.join("memory.peak"))
                                .unwrap_or_else(|_| "unavailable".into());
                        let cpu = fs::read_to_string(cohort.identity().cgroup.join("cpu.stat"))
                            .unwrap_or_else(|_| "unavailable".into())
                            .replace('\n', ",");
                        println!(
                            "RPI0_STATUS cohort_processes={} memory_peak={} cpu_stat={}",
                            pids.len(),
                            memory_peak.trim(),
                            cpu.trim_end_matches(',')
                        );
                    } else if line == "open" {
                        native_view = native_view
                            .checked_add(1)
                            .ok_or_else(|| invalid("editor generation exhausted"))?;
                        let mut message = Message {
                            kind: 1,
                            activation: 1,
                            native_view,
                            ..Message::default()
                        };
                        instance.gui_command(generation, &mut message)?;
                    } else if line == "close" {
                        if native_view == 0 {
                            return Err(invalid("no editor generation to close"));
                        }
                        let mut message = Message {
                            kind: 2,
                            native_view,
                            view_epoch,
                            ..Message::default()
                        };
                        instance.gui_command(generation, &mut message)?;
                    } else if let Some(path) = line.strip_prefix("save ") {
                        let path = absolute_state_path(path)?;
                        let mut state = vec![0; STATE_CAPACITY];
                        let count = instance.capture_state(&mut state)?;
                        atomic_new(&path, &state[..count])?;
                        println!("RPI0_STATE_SAVED bytes={count}");
                    } else if let Some(path) = line.strip_prefix("restore ") {
                        let path = absolute_state_path(path)?;
                        let state = fs::read(path)?;
                        let mut output = vec![0; STATE_CAPACITY];
                        jack.pause_processing()?;
                        instance.stop()?;
                        instance.deactivate()?;
                        let count = instance.restore_state(&state, &mut output)?;
                        instance.activate(256)?;
                        instance.start()?;
                        jack.resume_processing();
                        println!("RPI0_STATE_RESTORED bytes={count}");
                    } else if !line.is_empty() {
                        return Err(invalid("unknown command"));
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => return Ok(()),
            }
        }
    }

    fn drain_gui(
        instance: &Instance,
        generation: u64,
        control: &Control,
        view_epoch: &mut u32,
    ) -> io::Result<()> {
        for _ in 0..128 {
            let mut message = Message::default();
            instance.gui_take(generation, &mut message)?;
            if message.kind == 0 {
                break;
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
            if message.kind == 108 {
                *view_epoch = message.view_epoch;
                println!(
                    "RPI0_EDITOR native_view={} epoch={} lifecycle={} result={}",
                    message.native_view, message.view_epoch, message.lifecycle, message.result
                );
                if message.lifecycle == 3 {
                    let mut focus = Message {
                        kind: 5,
                        activation: message.activation,
                        native_view: message.native_view,
                        view_epoch: message.view_epoch,
                        target_x11: message.target_x11,
                        focus_result: 4,
                        ..Message::default()
                    };
                    instance.gui_command(generation, &mut focus)?;
                }
            }
        }
        Ok(())
    }

    fn host_arguments(config: &Config, session: &str, module: &str) -> Vec<OsString> {
        let hash = |bytes: &[u8]| OsString::from(hex(bytes));
        let ready = format!(r"C:\bridge\sessions\{session}\{session}.ready");
        let gate = format!(r"C:\bridge\sessions\{session}\{session}.gate");
        vec![
            config.windows_host.path.as_os_str().to_owned(),
            "--session".into(),
            session.into(),
            "--scanner-sha256".into(),
            hash(&config.windows_host.sha256),
            "--implementation-source-manifest-sha256".into(),
            hash(&config.source_manifest_sha256),
            "--module".into(),
            module.into(),
            "--module-sha256".into(),
            hash(&config.plugin.sha256),
            "--bundle-manifest-sha256".into(),
            hash(&config.plugin.sha256),
            "--ready".into(),
            ready.into(),
            "--gate".into(),
            gate.into(),
            "--max-classes".into(),
            "256".into(),
            "--stdout-cap".into(),
            "1048576".into(),
            "--mode".into(),
            "ap9-commercial".into(),
            "--component-case".into(),
            format!("class:{}", hex(&PROCESSOR_CLASS)).into(),
        ]
    }

    fn handshake(config: &Config, session: &str) -> String {
        format!(
            "schema=linux-vst-bridge-wf0-handshake/v1\nsession={session}\nscanner_sha256={}\nmodule_sha256={}\nbundle_manifest_sha256={}\nimplementation_source_manifest_sha256={}\nmode=ap9-commercial\ncomponent_case=class:{}\nrun_ordinal=1\n",
            hex(&config.windows_host.sha256), hex(&config.plugin.sha256), hex(&config.plugin.sha256),
            hex(&config.source_manifest_sha256), hex(&PROCESSOR_CLASS)
        )
    }

    fn windows_path(prefix: &Path, path: &Path) -> io::Result<String> {
        let root = prefix.join("drive_c");
        let relative = path
            .strip_prefix(&root)
            .map_err(|_| invalid("plug-in is outside selected Wine C: drive"))?;
        let value = relative
            .to_str()
            .ok_or_else(|| invalid("plug-in path encoding"))?;
        if value.contains('\\')
            || value
                .split('/')
                .any(|part| part.is_empty() || part == "." || part == "..")
        {
            return Err(invalid("plug-in path shape"));
        }
        Ok(format!(r"C:\{}", value.replace('/', r"\")))
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
                Err(error) => return Err(error),
            }
        }
    }

    fn atomic_new(path: &Path, bytes: &[u8]) -> io::Result<()> {
        if !path.is_absolute() || bytes.len() > 16 * 1024 * 1024 {
            return Err(invalid("bounded absolute output required"));
        }
        let temporary = path.with_extension("rpi0-tmp");
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&temporary)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        fs::rename(temporary, path)
    }

    fn absolute_state_path(value: &str) -> io::Result<PathBuf> {
        let path = PathBuf::from(value);
        if !path.is_absolute() || path.file_name().is_none() {
            return Err(invalid("state path must be an absolute file path"));
        }
        Ok(path)
    }

    fn random_session() -> io::Result<[u8; 16]> {
        let mut value = [0; 16];
        let mut file = fs::File::open("/dev/urandom")?;
        use std::io::Read;
        file.read_exact(&mut value)?;
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
            let metadata = entry.metadata()?;
            if !metadata.is_file() {
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
