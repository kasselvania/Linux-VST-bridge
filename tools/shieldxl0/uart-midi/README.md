# ShieldXL Pi 5 TRS MIDI input

This original Rust helper reads the Pi 5 RP1 header UART0 at 31,250 baud, 8N1,
and publishes one ALSA sequencer source named `ShieldXL UART:TRS In`. The existing
JACK ALSA sequencer driver exposes it with the stable alias
`ShieldXL-UART:midi/playback_1`. Numeric ALSA client IDs and JACK system port
numbers can change: resolve the exact name against the current roster.

The helper is input-only. It does not transmit to the ShieldXL MIDI OUT jack,
connect itself to a synthesizer, create audio, or change any Pigments policy.
ALSA's byte-stream parser handles running status and interleaved real-time
messages. SysEx is deliberately discarded and counted in bounded fragments;
ordinary channel messages and system real-time messages are forwarded. Forwarding
a controller message does not establish that a downstream instrument accepts it.

The device must be `/dev/ttyAMA0` backed by the exact Pi 5 RP1 UART0 node. The
debug-console alias `/dev/serial0` is not used. `TCSETS2/BOTHER` selects the actual
MIDI rate directly; the old `midi-uart0` clock trick and 38,400 setting are not used.
CTS/RTS remains disabled (GPIO17 belongs to the ShieldXL codec reset).

The serial descriptor is exclusive and nonblocking. Each read is bounded to 256
bytes; polling wakes within 100 ms. ALSA output is nonblocking and an output error
terminates the service explicitly. Normal stop closes the ALSA source, restores
the prior UART settings with readback, and releases the descriptor. The helper
does no audio callback work. No timing or lossless-under-load claim is made.

## Build and check on the Pi

Requires the existing ARM64 Rust toolchain, pkg-config and libasound2-dev.

```sh
cargo test --locked --manifest-path tools/shieldxl0/uart-midi/Cargo.toml
cargo clippy --locked --manifest-path tools/shieldxl0/uart-midi/Cargo.toml --all-targets -- -D warnings
cargo build --locked --release --manifest-path tools/shieldxl0/uart-midi/Cargo.toml
```

Tests use ALSA's real parser for fragmented notes/running status with interleaved
clock, plus a pseudo-terminal to check custom-rate configuration and restoration.
They do not prove the physical TRS wiring or actual baud accuracy.

## Install on the established ShieldXL Pi 5 fixture

First verify GPIO14/15 are unused and `/dev/ttyAMA0` has no existing owner.
The live overlay enables only UART0 TX/RX and can be removed after stopping the
helper. This does not alter the existing debug console, audio overlay or kernel.

```sh
sudo dtoverlay uart0-pi5
sudo udevadm settle
sudo install -m 0755 tools/shieldxl0/uart-midi/target/release/shieldxl-uart-midi /usr/local/libexec/shieldxl0/shieldxl-uart-midi
sudo install -m 0644 tools/shieldxl0/systemd/shieldxl-uart-midi@.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl start "shieldxl-uart-midi@$USER.service"
```

The established appliance user already belongs to `audio` and `dialout`. The
system unit runs as that user, permits only the UART and ALSA sequencer devices,
and restricts filesystem writes, privileges, memory and process count. It does
not auto-restart on failure.

Read `SHIELDXL_UART_READY`, verify the exact client name/PID with `aconnect -l`,
then use `aseqdump` on that observed address. Verify the corresponding JACK
alias with `jack_lsp -A`; observe actual messages with `jack_midi_dump` before
connecting to an instrument. Keep the controller externally powered.

### Audible wiring test

For hands-on diagnosis, use `tools/shieldxl0/midi_tone.rs` on the existing JACK
server. Build natively with `rustc --edition=2021 -O -D warnings midi_tone.rs -o
midi-tone`, and pass the exact current JACK source resolved from the UART alias.
Run it in a user service named `shieldxl-midi-tone.service`. It connects only that
MIDI source and `system:playback_1/2`, plays three startup beeps, then stays on for
30 minutes. Notes make a held monophonic tone; CC changes retrigger a 200 ms beep.
Clock/active-sensing traffic is silent. Oscillator data is preallocated, the
callback does no allocation/I/O, and the output has a short gain ramp.

The companion `midi_tone_display.py` shows `MIDI ON RX:<count>` through the existing
OLED service and changes to `MIDI TEST OFF` when that exact tone session ends.
The operator can swap controllers/cables/jacks without coordinating individual
listening windows. Stop the named tone/display services to end the test early.
This fixture is for audible wiring feedback only: it is not Pigments, a general
synth, a MIDI timing validator or a proof of clean audio. Startup beeps establish
only the output route; physical MIDI requires received messages afterward.

Boot persistence uses the provided `boot.conf` as
`/boot/firmware/shieldxl-uart-midi.conf`, included once from the `[all]` section of
`config.txt` after backing up that file. Install `modules.conf` as
`/etc/modules-load.d/shieldxl-uart-midi.conf`, then enable this service for the
appliance user. The module entry makes the ALSA sequencer available before the
restricted service starts. These files and enablement were installed on the
fixture after the OMX-27 physical pass. Do not claim reboot verification until
it actually happens.
For rollback, stop/disable only this service, remove its own boot include, and
remove its own module-load entry and the live `uart0-pi5` overlay if present and
unowned. Keep the existing
ShieldXL audio/control/OLED configuration intact.

## Source basis

- [Pi UART documentation](https://www.raspberrypi.com/documentation/computers/configuration.html#configuring-uarts)
- Installed Raspberry Pi `overlays/README`: `uart0-pi5` maps GPIO14/15.
- [ALSA Rust MIDI byte-stream parser](https://docs.rs/alsa/0.12.1/alsa/seq/struct.MidiEvent.html)
- Pinned ShieldXL board `eace215e99e8e0c3cb06dcd029ff14964da433c2`: J6 MIDI IN
  feeds GPIO15 through the dual-polarity optocoupler circuit; J3 MIDI OUT uses
  GPIO14 and the output-only A/B switch.

The legacy `okyeron/ttymidi` source at
`06f18e0f4a8f09bb13130fba2295c2a3c534c03c` was inspected as prior art. It does not
accept 31,250 directly and uses a custom parser; none of its source is copied.
Dependency versions/checksums are pinned in `Cargo.lock`; see the parent
`THIRD_PARTY_NOTICES.md` for dependency licensing.
