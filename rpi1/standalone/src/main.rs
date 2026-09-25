#[cfg(not(all(target_os = "linux", target_arch = "aarch64", feature = "jack-runtime")))]
fn main() {
    eprintln!("lvb-arm-pigments-standalone requires Linux AArch64 and --features jack-runtime");
    std::process::exit(64);
}

#[cfg(all(target_os = "linux", target_arch = "aarch64", feature = "jack-runtime"))]
fn main() {
    if let Err(error) = appliance::entry() {
        eprintln!("RPI1 refusal: {error}");
        std::process::exit(1);
    }
}

#[cfg(all(target_os = "linux", target_arch = "aarch64", feature = "jack-runtime"))]
mod phase_capture;

#[cfg(all(target_os = "linux", target_arch = "aarch64", feature = "jack-runtime"))]
mod appliance {
    use super::phase_capture::PhaseCapture;
    use ap2_backend::rpi0::{Identity as BridgeIdentity, Instance, Message, STATE_CAPACITY};
    use lvb_arm_pigments_standalone::{
        config::{hex, Config},
        contract::{handshake, host_arguments},
        master,
        panel::{Action as PanelAction, Model as PanelModel},
        panel_linux::Hardware as PanelHardware,
        panel_state,
        retirement::RetirementStatus,
        supervisor::{read_journal, Cohort},
    };
    use lvb_arm_standalone::{
        audio::ParameterUpdate,
        jack::{Client, Control},
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
        let args = env::args_os().collect::<Vec<_>>();
        if args.len() != 2 || args[1].is_empty() {
            return Err(invalid(
                "usage: lvb-arm-pigments-standalone /absolute/config",
            ));
        }
        let config = Config::load(Path::new(&args[1]))?;
        let session = random_session()?;
        run(config, session, hex(&session))
    }

    fn run(config: Config, session: [u8; 16], session_hex: String) -> io::Result<()> {
        let quantum = match env::var("LVB_RPI2_PROCESS_QUANTUM") {
            Err(env::VarError::NotPresent) => 256,
            Ok(v) if v == "256" => 256,
            Ok(v) if v == "512" && cfg!(feature = "rpi2-quantum") => 512,
            _ => return Err(invalid("unsupported processing quantum")),
        };
        let directory = config
            .prefix()
            .join("drive_c/bridge/sessions")
            .join(&session_hex);
        fs::create_dir(&directory)?;
        fs::set_permissions(&directory, fs::Permissions::from_mode(0o700))?;
        let retirement = RetirementStatus::create(&directory, &session)?;
        let identity = BridgeIdentity {
            class: config.binding.class,
            module: config.plugin.sha256,
        };
        let architecture = ArchitectureHandshake::native(page_size()?);
        atomic_new(&directory.join("rpi0.arch.native"), &architecture.encode())?;
        let bridge_directory = directory.clone();
        let delay = config.bridge_frames;
        let bridge =
            thread::spawn(move || Instance::open_bound_quantum(bridge_directory, session, identity, delay, quantum));

        let ready = directory.join(format!("{session_hex}.ready"));
        let gate = directory.join(format!("{session_hex}.gate"));
        let expected = handshake(&config, &session_hex)?;
        let arguments = host_arguments(&config, &session_hex)?;
        let cohort = Cohort::launch(&config, &session_hex, &arguments)?;
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

        // The mapping/host handshake precedes commercial module inspection.
        // Keep that startup work out of the ordinary Configure reply budget.
        let initialization = Instant::now();
        println!("RPI1_INITIALIZATION_WAIT timeout_seconds=120");
        lvb_arm_pigments_standalone::startup::await_initialization(
            &cohort,
            &session_hex,
            Duration::from_secs(120),
        )?;
        println!(
            "RPI1_INITIALIZED wait_ms={}",
            initialization.elapsed().as_millis()
        );

        let control = Control::new();
        let mut jack = Client::open_with_io(
            &config.jack_client,
            &instance,
            &control,
            lvb_arm_standalone::midi::ControllerPolicy::TrackedNoteOffs,
            config.binding.stereo_input,
            config.binding.midi_input,
        )?;
        let phase = PhaseCapture::start(
            instance.phase_rings()?,
            &config.evidence_directory,
            &session_hex,
        )?;
        println!("RPI1_PHASE_TRACE mode={} recorder={} file={}",
            if phase.enabled() { "on" } else { "off" }, phase.enabled(), phase.enabled());
        let buses = &config.binding.buses;
        let traits = instance.setup(jack.buffer_size(), 48_000.0, buses)?;
        let state_directory = config.environment_root.join(format!(
            "panel-state-{}-{}",
            hex(&config.plugin.sha256),
            hex(&config.binding.class)
        ));
        let restored = if config.binding.surface.is_some() {
            if let Some(state) = panel_state::load(&state_directory, STATE_CAPACITY)? {
                let mut readback = vec![0; STATE_CAPACITY];
                instance.restore_state(&state, &mut readback)?;
                println!("RPI2_PANEL_RESTORED bytes={}", state.len());
                true
            } else {
                false
            }
        } else {
            false
        };
        instance.activate(instance.processing_quantum())?;
        instance.start()?;
        jack.activate()?;
        let ports = jack.ports();
        println!(
            "RPI1_READY midi={} left={} right={} jack_frames={} bridge_frames={} vendor_frames={} total_frames={}",
            ports[0], ports[1], ports[2], jack.buffer_size(), config.bridge_frames,
            traits.vendor_frames, traits.total_frames
        );
        if let Some(inputs) = jack.audio_input_ports() {
            println!("RPI1_INPUT left={} right={}", inputs[0], inputs[1]);
        }
        println!("RPI2_PROCESSING quantum={} map_version={} capacity={} sample_rate=48000",
            instance.processing_quantum(), if cfg!(feature = "rpi2-quantum") { 2 } else { 1 },
            if cfg!(feature = "rpi2-quantum") { 512 } else { 256 });
        println!("RPI1_LATENCY tail_frames={}", traits.tail_frames);
        println!("RPI1_MIDI cc123=tracked_note_offs cc64=unsupported duplicate_note_on=refused");
        println!(
            "Commands: open | close | save /absolute/path | slot save | restore /absolute/path | master [normalized] | parameter id [normalized] | trim [linear] | bypass [on|off] | status | mark | quit"
        );
        let result = command_loop(&instance, &control, &cohort, &mut jack, &phase, &config, restored);

        let jack_frames = jack.buffer_size();
        let deactivate = jack.deactivate();
        drop(jack);
        if let Err(error) = print_audio_metrics(&instance, &control) {
            eprintln!("RPI1_AUDIO_UNAVAILABLE {error}");
        }
        let metrics = &control.metrics;
        println!(
            "RPI1_CALLBACK callbacks={} failures={} deadline_misses={} callback_ns_max={} missing_frames={} expired_frames={} gaps={} delivered_frames={} priming_frames={} paused_frames={} unsupported_midi={} malformed_midi={} overflow_midi={}",
            metrics.callbacks.load(Ordering::Acquire), metrics.process_failures.load(Ordering::Acquire),
            metrics.deadline_misses.load(Ordering::Acquire), metrics.callback_ns_max.load(Ordering::Acquire),
            metrics.missing_frames.load(Ordering::Acquire), metrics.expired_frames.load(Ordering::Acquire),
            metrics.gaps.load(Ordering::Acquire), metrics.delivered_frames.load(Ordering::Acquire),
            metrics.priming_frames.load(Ordering::Acquire), metrics.paused_frames.load(Ordering::Acquire),
            metrics.unsupported_midi.load(Ordering::Acquire),
            metrics.malformed_midi.load(Ordering::Acquire), metrics.overflow_midi.load(Ordering::Acquire)
        );
        let callback_summary = format!(
            "{{\"event\":\"rpi1_callback_summary\",\"callbacks\":{},\"process_failures\":{},\"deadline_misses\":{},\"callback_ns_max\":{},\"missing_frames\":{},\"expired_frames\":{},\"gaps\":{},\"delivered_frames\":{},\"priming_frames\":{},\"paused_frames\":{},\"unsupported_midi\":{},\"malformed_midi\":{},\"overflow_midi\":{},\"jack_frames\":{},\"bridge_frames\":{}}}\n",
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
        let retirement_result = retirement.await_ready(Duration::from_secs(30));
        let unit = cohort.identity().unit.clone();
        let retire = cohort.retire();
        drop(retirement);
        let phase_result = phase.finish();
        let cleanup = if close.is_ok() && retirement_result.is_ok() && retire.is_ok() {
            preserve_evidence(
                &directory.join("rpi0.performance.jsonl"),
                &config
                    .evidence_directory
                    .join(format!("{session_hex}-native.jsonl")),
                callback_summary.as_bytes(),
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
                "session retained because close, retirement authority, or cgroup cleanup failed",
            ))
        };
        result?;
        phase_result?;
        deactivate?;
        stop?;
        off?;
        if let Ok(stats) = stats {
            println!(
                "RPI1_TRANSPORT fault={} processed={} request_high={} result_high={} position={} epoch={}",
                stats.fault, stats.processed, stats.request_high, stats.result_high, stats.position, stats.epoch
            );
        }
        close?;
        let retirement = retirement_result?;
        println!(
            "RPI1_RETIREMENT milestones={} epoch={} sequence={} position={} generation={}",
            retirement.milestones,
            retirement.epoch,
            retirement.sequence,
            retirement.position,
            retirement.generation
        );
        retire?;
        cleanup?;
        println!("RPI1_CLEAN_SHUTDOWN session={session_hex}");
        Ok(())
    }

    fn print_audio_metrics(instance: &Instance, control: &Control) -> io::Result<()> {
        let metrics = &control.metrics;
        let stats = instance.stats()?;
        println!(
            "RPI1_AUDIO midi_accepted={} output_nonzero_l={} output_nonzero_r={} output_peak_l={} output_peak_r={} output_nonfinite={} xruns={} process_failures={} bridge_processed={} request_high={} result_high={} fault={} fault_site={} bridge_processed_frames={} processing_quantum={}",
            metrics.accepted_midi.load(Ordering::Acquire),
            metrics.output_nonzero[0].load(Ordering::Acquire),
            metrics.output_nonzero[1].load(Ordering::Acquire),
            f32::from_bits(metrics.output_peak_bits[0].load(Ordering::Acquire) as u32),
            f32::from_bits(metrics.output_peak_bits[1].load(Ordering::Acquire) as u32),
            metrics.output_nonfinite.load(Ordering::Acquire),
            metrics.xruns.load(Ordering::Acquire),
            metrics.process_failures.load(Ordering::Acquire),
            stats.processed, stats.request_high, stats.result_high, stats.fault,
            ap2_backend::rpi1_phase::site_name(instance.fault_site().unwrap_or(0)),
            instance.processed_frames()?, instance.processing_quantum()
        );
        Ok(())
    }

    fn print_live_callback_metrics(control: &Control) {
        let metrics = &control.metrics;
        println!(
            "RPI1_CALLBACK_LIVE callbacks={} failures={} deadline_misses={} callback_ns_max={} missing_frames={} gaps={} delivered_frames={} paused_frames={} unsupported_midi={} malformed_midi={} overflow_midi={}",
            metrics.callbacks.load(Ordering::Acquire),
            metrics.process_failures.load(Ordering::Acquire),
            metrics.deadline_misses.load(Ordering::Acquire),
            metrics.callback_ns_max.load(Ordering::Acquire),
            metrics.missing_frames.load(Ordering::Acquire),
            metrics.gaps.load(Ordering::Acquire),
            metrics.delivered_frames.load(Ordering::Acquire),
            metrics.paused_frames.load(Ordering::Acquire),
            metrics.unsupported_midi.load(Ordering::Acquire),
            metrics.malformed_midi.load(Ordering::Acquire),
            metrics.overflow_midi.load(Ordering::Acquire),
        );
    }

    fn command_loop(
        instance: &Instance,
        control: &Control,
        cohort: &Cohort,
        jack: &mut Client,
        phase: &PhaseCapture,
        config: &Config,
        restored: bool,
    ) -> io::Result<()> {
        let binding = &config.binding;
        let state_directory = config.environment_root.join(format!(
            "panel-state-{}-{}",
            hex(&config.plugin.sha256),
            hex(&binding.class)
        ));
        let mut panel = binding
            .surface
            .clone()
            .map(|surface| {
                Ok::<_, io::Error>((
                    PanelModel::new(surface, &binding.controls),
                    PanelHardware::open()?,
                ))
            })
            .transpose()?;
        let generation = instance.gui_generation()?;
        instance.gui_capabilities(generation, 7)?;
        if let Some((model, _)) = &mut panel {
            request_parameter_snapshot(instance, generation)?;
            if restored {
                model.status = "RESTORED".into();
            }
            println!("RPI2_PANEL_READY controls=shieldxl state_slot=private");
        }
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
            drain_gui(
                instance,
                generation,
                control,
                &mut view_epoch,
                phase,
                binding,
                &mut panel,
            )?;
            if let Some((model, hardware)) = &mut panel {
                if !model.faulted {
                    if let Err(error) = hardware.poll(model) {
                        model.faulted = true;
                        model.status = "INPUT ERROR".into();
                        eprintln!("RPI2_PANEL_INPUT_ERROR {error}");
                    }
                }
                for action in model.actions(Instant::now()) {
                    match action {
                        PanelAction::Set(id, value) => {
                            if let Err(error) =
                                set_parameter(instance, generation, control, id, value)
                            {
                                model.faulted = true;
                                model.status = "CONTROL ERROR".into();
                                eprintln!("RPI2_PANEL_CONTROL_ERROR {error}");
                            } else {
                                println!("RPI2_PANEL_SET id={id} normalized={value:.17}");
                            }
                        }
                        PanelAction::Save => {
                            let result = (|| {
                                let mut state = vec![0; STATE_CAPACITY];
                                let count = instance.capture_state(&mut state)?;
                                panel_state::save(&state_directory, &state[..count])?;
                                Ok::<_, io::Error>(count)
                            })();
                            match result {
                                Ok(count) => {
                                    model.status = "SAVED".into();
                                    println!("RPI2_PANEL_SAVED bytes={count}");
                                }
                                Err(error) => {
                                    model.status = "SAVE FAILED".into();
                                    eprintln!("RPI2_PANEL_SAVE_ERROR {error}");
                                }
                            }
                        }
                    }
                }
                hardware.update_display(model);
            }
            match receiver.recv_timeout(Duration::from_millis(5)) {
                Ok(line) => {
                    let line = line.trim();
                    if line == "quit" {
                        return Ok(());
                    }
                    if line == "mark" {
                        if phase.mark("operator_incident")? {
                            println!("RPI1_OBSERVER_MARKER accepted");
                        } else {
                            println!("RPI1_OBSERVER_MARKER unavailable phase_trace=off");
                        }
                        continue;
                    }
                    if line == "status" {
                        if let Err(error) = print_audio_metrics(instance, control) {
                            eprintln!("RPI1_AUDIO_UNAVAILABLE {error}");
                        }
                        print_live_callback_metrics(control);
                        println!(
                            "RPI1_OUTPUT_ROUTE bypass={} trim_linear={:.6}",
                            control.bypass(),
                            control.output_trim()
                        );
                        let processes = cohort.verify()?;
                        let memory_peak =
                            fs::read_to_string(cohort.identity().cgroup.join("memory.peak"))
                                .unwrap_or_else(|_| "unavailable".into());
                        let cpu = fs::read_to_string(cohort.identity().cgroup.join("cpu.stat"))
                            .unwrap_or_else(|_| "unavailable".into())
                            .replace('\n', ",");
                        println!(
                            "RPI1_STATUS cohort_processes={} memory_peak={} cpu_stat={}",
                            processes.len(),
                            memory_peak.trim(),
                            cpu.trim_end_matches(',')
                        );
                    } else if let Some(arguments) = line.strip_prefix("parameter ") {
                        let words = arguments.split_whitespace().collect::<Vec<_>>();
                        if !(1..=2).contains(&words.len()) {
                            return Err(invalid("parameter id [normalized]"));
                        }
                        let id = words[0]
                            .parse::<u32>()
                            .map_err(|_| invalid("parameter id"))?;
                        binding.control(id)?;
                        if let Some(text) = words.get(1) {
                            let value = master::normalized(text)?;
                            set_parameter(instance, generation, control, id, value)?;
                            println!("RPI1_PARAMETER_QUEUED id={id} normalized={value:.17}");
                        } else {
                            request_parameter_snapshot(instance, generation)?;
                        }
                    } else if line == "trim" {
                        println!("RPI1_OUTPUT_TRIM linear={:.6}", control.output_trim());
                    } else if let Some(value) = line.strip_prefix("trim ") {
                        let trim = value
                            .parse::<f32>()
                            .map_err(|_| invalid("trim requires linear gain from 0 to 1"))?;
                        control.set_output_trim(trim)?;
                        println!("RPI1_OUTPUT_TRIM linear={trim:.6}");
                    } else if line == "bypass" {
                        println!("RPI1_BYPASS enabled={}", control.bypass());
                    } else if let Some(value) = line.strip_prefix("bypass ") {
                        if !binding.stereo_input {
                            return Err(invalid("bypass requires stereo effect input"));
                        }
                        let bypass = match value {
                            "on" => true,
                            "off" => false,
                            _ => return Err(invalid("bypass requires on or off")),
                        };
                        control.set_bypass(bypass);
                        println!(
                            "RPI1_BYPASS enabled={bypass} policy=current_dry_processing_continues"
                        );
                    } else if line == "master" || line.starts_with("master ") {
                        if !binding.legacy_master {
                            return Err(invalid("master command requires the Pigments binding"));
                        }
                        if let Some(text) = line.strip_prefix("master ") {
                            let value = master::normalized(text)?;
                            // Reuse the same bounded audio parameter queue and controller
                            // companion used by the existing host/editor boundary.
                            control
                                .parameters
                                .push(ParameterUpdate {
                                    id: master::ID,
                                    value,
                                })
                                .map_err(|_| invalid("parameter queue full"))?;
                            let mut set = Message {
                                kind: 3,
                                id: master::ID,
                                value,
                                ..Message::default()
                            };
                            instance.gui_command(generation, &mut set)?;
                            println!("RPI1_MASTER_QUEUED normalized={value:.17}");
                        }
                        let mut refresh = Message {
                            kind: 4,
                            ..Message::default()
                        };
                        instance.gui_command(generation, &mut refresh)?;
                    } else if line == "open" {
                        phase.mark("editor_open_requested")?;
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
                        phase.mark("editor_close_requested")?;
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
                        println!("RPI1_STATE_SAVED bytes={count}");
                    } else if line == "slot save" {
                        if binding.surface.is_none() {
                            return Err(invalid("slot requires a selected effect surface"));
                        }
                        let mut state = vec![0; STATE_CAPACITY];
                        let count = instance.capture_state(&mut state)?;
                        panel_state::save(&state_directory, &state[..count])?;
                        println!("RPI2_SLOT_SAVED bytes={count}");
                    } else if let Some(path) = line.strip_prefix("restore ") {
                        let path = absolute_state_path(path)?;
                        let state = fs::read(path)?;
                        let count = restore_payload(instance, jack, &state)?;
                        request_parameter_snapshot(instance, generation)?;
                        println!("RPI1_STATE_RESTORED bytes={count}");
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
                // VST3 kParamValuesChanged. A zero mask cannot retain a refresh
                // requested while EditorSession is publishing an earlier snapshot.
                flags: 1 << 2,
                ..Message::default()
            },
        )
    }

    fn set_parameter(
        instance: &Instance,
        generation: u64,
        control: &Control,
        id: u32,
        value: f64,
    ) -> io::Result<()> {
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
        request_parameter_snapshot(instance, generation)
    }

    fn restore_payload(instance: &Instance, jack: &mut Client, state: &[u8]) -> io::Result<usize> {
        let mut output = vec![0; STATE_CAPACITY];
        jack.pause_processing()?;
        instance.stop()?;
        instance.deactivate()?;
        let count = instance.restore_state(state, &mut output)?;
        instance.activate(instance.processing_quantum())?;
        instance.start()?;
        jack.resume_processing();
        Ok(count)
    }

    fn drain_gui(
        instance: &Instance,
        generation: u64,
        control: &Control,
        view_epoch: &mut u32,
        phase: &PhaseCapture,
        binding: &lvb_arm_pigments_standalone::binding::Binding,
        panel: &mut Option<(PanelModel, PanelHardware)>,
    ) -> io::Result<()> {
        for _ in 0..128 {
            let mut message = Message::default();
            instance.gui_take(generation, &mut message)?;
            if message.kind == 0 {
                break;
            }
            if binding.legacy_master && message.kind == 110 && message.id == master::ID {
                let value = master::readback(&message)?;
                println!(
                    "RPI1_MASTER_READBACK normalized={value:.17} revision={}",
                    message.revision
                );
            }
            if message.kind == 110 && binding.controls.iter().any(|p| p.id == message.id) {
                let readback = binding.readback(&message)?;
                if readback.metadata_changed {
                    println!(
                        "RPI1_PARAMETER_METADATA_CHANGED id={} revision={}",
                        message.id, message.revision
                    );
                }
                if let Some(value) = readback.value {
                    if let Some((model, _)) = panel {
                        model.readback(message.id, value);
                    }
                    println!(
                        "RPI1_PARAMETER_READBACK id={} normalized={value:.17} revision={}",
                        message.id, message.revision
                    );
                } else {
                    if let Some((model, _)) = panel {
                        model.unavailable(message.id);
                    }
                    println!(
                        "RPI1_PARAMETER_UNAVAILABLE id={} revision={}",
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
            if message.kind == 108 {
                phase.mark(&format!("editor_lifecycle_{}", message.lifecycle))?;
                *view_epoch = message.view_epoch;
                println!(
                    "RPI1_EDITOR native_view={} epoch={} lifecycle={} result={} target_x11=0x{:x}",
                    message.native_view,
                    message.view_epoch,
                    message.lifecycle,
                    message.result,
                    message.target_x11
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
        let temporary = path.with_extension("rpi1-tmp");
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
