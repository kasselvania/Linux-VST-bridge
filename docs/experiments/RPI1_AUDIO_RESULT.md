# RPI1 deterministic audio — 2026-09-22

## Observed result

**Exact Pigments generated measured and audible stereo audio. The complete gate
failed afterward on a specific MIDI control mapping, including normal retirement.**

The operator explicitly authorized testing the replacement supply already
connected. No further supply swap or installer/runtime change was made.
Executed head was `a7aa62281ba52537de7039c7b84da5267a1deeb1`, tree
`48676d6406a1ea5547d9c422b45b1d5691fe4755`; executable digests and sanitized
observations are in `evidence/rpi1/pigments-deterministic-audio.json`.

First, the two-second source-owned tone again measured nonzero stereo, zero
xruns, and clean port retirement; the operator again confirmed hearing it.
Then exactly one Pigments session ran with the editor closed and USB MIDI
unplugged, at 48 kHz / JACK 512 frames / bridge delay 2048 frames / vendor quantum
256 frames. The executable itself ran in its own user unit with journal output
and a private command FIFO; independent health and remote logging were active.

The qualification client sent channel-1 note 60 at velocity 96, note-off, and
CC123. All three writes succeeded and all three were accepted by the native
parser. Both internal output counters and the separate JACK meter recorded
72,928 nonzero samples per channel, peak 0.19765145. Each external channel
measured 240,000 samples, RMS 0.0760791565, and 144 nonzero callback windows.
There were no nonfinite samples and zero JACK xruns. The operator confirmed:
“Yes, I heard the Pigments note”.

The meter's success establishes nonzero output during its observation window;
it does not assert continuous transport or successful host retirement.

## Specific failure and diagnosis

After the third accepted MIDI event, native status changed from fault 0 to
fault 3. The Windows host published `controller update identity/value`, stopped
processing, joined its worker with an exception, and deactivated the component.
It completed 624 blocks of 256 frames (159,744 frames) before failure. The native
callback subsequently returned defined silence and accumulated 315 process
failures; it had zero callback deadline misses. Normal quit ended with
`RPI0 stop failed: 2`, outer exit 1, and no clean-retirement receipt.

The inherited parser in `rpi0/standalone/src/midi.rs` translates channel-1 CC123
to a parameter event with ID `0x52500200` (1,380,975,104) and value 1. That is an
RPI0 reference-instrument parameter identity, not a Pigments identity. The exact
host enumerated Pigments IDs 0 through 4445; the synthetic ID is absent.

`windows-factory-probe/source/mapped_processing.cpp` requires commercial
parameter events to pass `ControllerUpdates::publish`; its identity check in
`controller_updates.h` rejects this ID. A local source reproduction using the
actual Rust parser produced kind 2 / ID 1380975104 / value 1. The actual host
controller-update class, configured with the observed ID set, accepted a known
ID and rejected that synthetic ID. No second Windows/Pigments run was used.

The failing host record does not include the raw event ID. The deterministic
sequence has only one parameter-producing event, its accepted-event timing
matches the fault, and the exact source checks reproduce the rejection. This
identifies CC123 as the trigger without claiming a second physical isolation
run. The inherited CC64 path also emits reference-instrument IDs and must be
reviewed as part of the same commercial MIDI boundary.

The next source repair belongs at RPI1's MIDI interpretation: prevent reference
fixture parameter identities from entering commercial plug-in requests. Note
identities already reach Pigments correctly. Supported controllers need the
plug-in's authoritative VST3 MIDI assignment, or an explicitly bounded supported
translation; unsupported messages must be reported before submission. Do not
guess IDs from names, suppress the host's identity check, silently erase CC123
from this failed record, or change the bus/runner/installation to address it.
No mapping repair or additional physical attempt was made in this run.

## Power and cleanup evidence

During 61 one-Hz samples with the outer unit present, temperature peaked at
59.5 C, sampled voltage ranged 4.80524–4.97542 V, and MemAvailable remained at
least 5,535,424 KiB. Peak memory was 164,761,600 bytes for the outer unit and
2,319,323,136 bytes for the translated cohort. The Pi stayed responsive without
a reboot. Kernel records contain no new undervoltage events during the test;
sampled throttle flags remained the historical `0x50000` from boot.

This short observation neither proves the earlier hang's cause nor qualifies
the replacement supply for sustained operation. It establishes that the supply
as connected supported this audible-note test and that its observed terminal
failure has a specific software path.

All qualification and Pigments JACK ports/routes were removed. The native
process exited and the exact translated cohort was stopped; no Box64 or
wineserver process remained. The private command FIFO and independent health
recorder retired. The failed outer unit and session transport files are retained
for diagnosis because normal retirement failed. This is contained process
cleanup, not a successful product-retirement receipt.

The previously approved temporary runtime watchdog disable was used and its
original 60-second setting restored afterward. No persistent watchdog change
was made. No editor, physical controller, state test, soak, or second session
was started.

Counter interpretation correction: live `bridge_processed` counts completed
blocks, while `request_high` and `result_high` are queue high-water occupancies,
not request/result sequence frontiers. The original raw records are retained;
their units must not be inflated into completed-frame counts.
