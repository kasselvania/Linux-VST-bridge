# RPI1 headroom comparison interrupted by whole-unit failure — 2026-09-22

The original-versus-reduced Master Volume comparison did not complete. The Pi
stopped responding to SSH and ping during the original-level chord ladder. The
operator confirmed no power changes, reported a blank display, and described a
crash. The retained remote record contains active undervoltage and throttling
before access was lost. There is no clean-retirement result.

## What changed and what passed before launch

Source head `1173e816bdea7f334347e411ddb15b438f81467b`, tree
`d97e60f8ab04fc0e1604fd975a465f365db4bc4c`, adds a control-thread `master`
command and extends the existing source-owned JACK qualification fixture.
`master` requests the existing controller refresh, validates the exact
`Master Volume` / `dB` parameter at ID 0, and reports its normalized value.
A supplied normalized value uses the existing bounded audio-parameter queue and
controller companion. No alternative transport or audio engine was introduced.

The fixture's fixed 30-second ladder plays 1, 2, 4, 6 and 8 notes, held for three
seconds in six-second windows. It sends 47 channel-1 messages at velocity 96.
Optional stereo float32 recording allocates and touches writable pages before
JACK activation, copies within that fixed extent in the callback, and writes the
new private file only after callback retirement. This protects callback behavior
but cannot preserve a RAM-only incomplete capture across a hard crash.

Fourteen RPI1 tests passed locally and on ARM; five fixture tests passed locally
and on ARM. ARM release builds and local/ARM Clippy passed, retaining only the
existing `new_without_default` exception. All 54 transferred source files matched.
Native binary SHA-256:
`81a44a8eed7191b08655496dfe0b49c0c919208e9f7a600658bc37887a2006c7`.
Fixture SHA-256:
`a3fb302d8f648742a853d85cd4d6f3b8b0d46ecc70259d56272137553b19e861`.

The prior idle SSH connection had expired. JACK's process was alive but its
client IPC endpoints were unavailable; restarting the idle JACK service restored
them with the same configuration. The endpoint-loss cause was not established.
This is separate from the subsequent loaded run. No reboot or power change was
made. The controller was not routed to Pigments during the automatic sequence.

## Retained physical sequence

Pigments initialized and reported readiness at 48 kHz / JACK 512 frames / bridge
2048 frames / vendor processing quantum 256. The editor remained closed. The
real controller reported Master Volume `0.67308074235916138`, and a 381,326-byte
private state envelope was saved. Pass A began at that original level. Pass B
was planned at normalized `0.5` after restoring the same saved state, but it was
not reached in the retained record. Physical application of a changed master
value therefore remains unproved.

The last native status reported 38 accepted MIDI messages, 5891 processed blocks,
617,647 nonzero samples per channel and peak 1.3439976. It reported zero xruns,
process failures, nonfinite samples and terminal transport faults. This was an
intermediate status, not a final callback, audio or retirement receipt. The
fixture did not publish completion or a completed waveform record.

The 48 one-Hz remote health samples span monotonic 5518.607–5565.607 seconds.
The first already had historical `0x50000` flags. Active `0x50005` occurred:

- At 5528.613 seconds, **during initialization before the notes**, with 4.67392 V.
- At 5559.607 seconds, during the six-note hold, with 4.75834 V.
- At 5565.607 seconds, during the eight-note hold, with 4.70340 V.

Maximum sampled temperature was 60.6 C. Last available memory was about 5.3 GiB,
and swap remained unused. Firmware, voltage and frequency queries occur
sequentially; these are not simultaneous electrical measurements.

[Raspberry Pi's flag definition](https://www.raspberrypi.com/documentation/computers/os.html#get_throttled)
identifies bits 0 and 2 as active undervoltage and current throttling, and bits
16 and 18 as their historical counterparts. The records therefore establish
actual firmware-reported voltage/throttle events, not an inference from adapter
wattage. They do not identify the adapter, cable, connector or board as the cause.
The first active sample predates the chord test, so this is not evidence of a
simple eight-voice processing ceiling.

## Stop, custody and recovery

After active voltage flags were noticed in the remote record, state restoration
and normal quit were requested. That request was not acknowledged; the SSH
connection subsequently failed. No complete final metrics, clean retirement,
state-restoration acknowledgement or watchdog-restoration result was obtained.
The runtime watchdog had been temporarily disabled under the earlier operator
authorization. After the operator power cycle, its original 60-second setting was verified.

Remote journal/run/ping records are retained privately on the Mac. No vendor
state, presets, account material or raw machine logs are published. The user was
asked for one power cycle solely to recover logs and restore the idle setup.
Recovery completed: prior-boot kernel and user journals survived and were copied
privately to the Mac. Pigments and translated processes were absent. JACK and the
UART service were active after reboot. The interrupted unit definition and failed
session directory were preserved; the previous inactive unit definition was
restored, and only the verified stale command FIFO was removed. The recovered
waveform file is zero bytes; the private saved-state envelope remains 381,326
bytes. This is recovery of evidence and idle services, not clean retirement of
the failed session.

The recovery boot already showed historical voltage/throttle flags and reported
a 3000 mA connection limit. These facts do not identify a defective adapter or
cable. The operator later connected an inline meter at the Pi, observed idle
5.02 V / 0.63 A, and explicitly requested a separate Pigments reload. That retry
is a separate observation, not completion of either headroom pass.

The prior [physical UART/Pigments success](RPI1_UART_PIGMENTS.md) remains valid.
Clipping versus processing limits is still unanswered. A high aggregate USB-C wattage
rating alone did not establish adequate operation on this fixture. Neither the
failed ladder nor an idle meter reading isolates the power-delivery cause.

Sanitized record: [`headroom-interrupted.json`](../../evidence/rpi1/headroom-interrupted.json).
