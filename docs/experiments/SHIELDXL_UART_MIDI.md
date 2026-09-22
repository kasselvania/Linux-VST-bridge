# ShieldXL Pi 5 UART MIDI input — 2026-09-22

The operator selected the ShieldXL physical TRS MIDI input for RPI1, with Monolit,
Chord ATK or OMX-27 as possible senders. Audio-artifact diagnosis was explicitly
deferred because the operator suspects their listening interface. Pigments,
Proton and Box64 remain stopped during this hardware step.

## Scope and implementation

This hardware change is based on SHIELDXL0 head
`32c8d2af4d83c9bccdbc6c5f0364a02de071399e`, tree
`e73f3cfde3e38265d031a89b54872ef063d2f58c`. It implements the optional UART input
in SHIELDXL0 Phase 6. It does not modify RPI0/RPI1 hosts, audio, activation,
vendor policy, the existing codec/control/OLED overlay or the kernel.

The initial live inspection matched the historical deferred status: no header
UART0, no UART MIDI service, and only MIDI Through in ALSA/JACK. `serial0` was
the Pi 5 debug UART `ttyAMA10`, already owned by the debug console. GPIO14/15
had no UART function.

The live, removable `uart0-pi5` overlay exposed RP1 UART0 on GPIO14/15. A small
original Rust input service configures and reads back actual 31,250 baud, raw
8N1, then uses ALSA's existing MIDI stream parser and sequencer. JACK's existing
ALSA sequencer backend exposes the stable `ShieldXL-UART:midi/playback_1` alias.
The implementation does not copy the legacy ttymidi parser or clock workaround.
SysEx is explicitly discarded; MIDI output from the physical OUT jack is absent.

The service runs as the existing appliance user, with UART/ALSA device access,
bounded serial reads and polling, nonblocking ALSA output, no auto-restart and
explicit errors. Normal retirement restores and verifies the prior serial
configuration. It neither generates audio nor connects to an instrument.

## Validation and live status

- Native ARM tests passed: running status/interleaved clock and UART rate/restore
  against a pseudo-terminal.
- Strict all-target ARM Clippy and the ARM release build passed.
- Existing SHIELDXL0 deterministic suite: 44 tests passed.
- The installed service published readiness with baud readback and the expected
  ALSA source; JACK exposed the corresponding alias.
- A preliminary service generation stopped with exit 0 and restarted. The final
  generation additionally requires explicit restoration readback before a clean
  retirement record.

Physical MIDI acceptance is pending. The operator connected Monolit and was
asked to move a slider. At the initial check the final service had received
zero bytes; no note/CC delivery is claimed from a visible software port.
Boot configuration and service enablement have not yet been changed.

The first observer attempted to reuse an ALSA client number after restart and
attached to the wrong client. It was stopped without an input-pass claim. The
replacement resolved the exact `ShieldXL UART` name and current service PID
before observing; the JACK observer used the current exact alias. Numeric port
identifiers must not be reused across service generations.

Source/build/installation and rollback instructions are in
[`tools/shieldxl0/uart-midi/README.md`](../../tools/shieldxl0/uart-midi/README.md).
The historical `hardware-contract.json` remains unchanged until physical input
has actually been proved. No reboot, physical output direction, MIDI stress,
Pigments controller mapping or overall appliance pass is claimed.
