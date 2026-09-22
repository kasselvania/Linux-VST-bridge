# RPI1 bounded MIDI repair and audio retirement — 2026-09-22

## Result and custody

**One repaired editor-closed Pigments audio/retirement gate passed.** The
operator confirmed hearing the note, both channels measured nonzero audio, and
normal quit completed without a host/transport fault. Overall RPI1 remains
pending. A new transient undervoltage event and startup gaps are preserved.

Executed source: `1db3478292fda7839d051f703992c8ddc0040653`, tree
`f7edd886464f5791b86332375661248f8ffc6fd9`. The result-only documentation commit
does not change that tested source identity. Binary digests, sanitized records
and health aggregates are in `evidence/rpi1/pigments-cc123-repair.json`.

This was exactly one newly authorized Pigments session on the already-connected
replacement supply, with the USB MIDI controller disconnected and editor closed.
The fixture remained 48 kHz, JACK period 512, bridge delay 2048 and vendor quantum
256 frames; reported vendor latency is a separate 48-frame value. Proton,
Box64, Pigments, Windows host, bus contract and editor lifetime were unchanged.

## Repair

The prior attempt's final CC123 was incorrectly converted into an RPI0 reference
instrument parameter ID. Pigments does not own that ID; the Windows host correctly
rejected it. RPI1 now selects an explicit tracked-note policy in the existing
JACK callback. RPI0 selects its separate reference-instrument policy and retains
its owned controller parameter identities.

For RPI1, CC123 produces at most 128 note-offs on its own channel, retaining
each tracked note's ID, pitch and event offset. The parser checks destination
capacity before writing or clearing notes. Refusal preserves its tracked state;
the callback reports overflow and does not automatically retry. Translation uses
the existing preallocated event array. Empty-channel CC123 is an accepted no-op.

CC64 and other controllers remain explicitly unsupported until an authoritative
commercial mapping exists. A second note-on for an already active channel/pitch
is refused rather than discarding the older note identity. This is a bounded
note-input contract, not general MIDI qualification.

## Validation

The shared standalone suite passed 18 tests locally and 20 on ARM with JACK
enabled; RPI1 passed 7 tests on each platform. Tests cover channel/identity/offset
preservation, all 128 notes, atomic capacity refusal, malformed/duplicate messages,
the exact qualification sequence without parameter events, unchanged reference
policy and allocation-free 128-note expansion. The ARM release build passed;
all five changed source files matched their local hashes before building.

Local all-target Clippy passed for both crates with only the pre-existing
`clippy::new_without_default` lint allowed. This is not a strict zero-exception
Clippy claim. No new hosted-check result is claimed here.

The unchanged qualification client sent note 60 on channel 1 at velocity 96,
then note-off and CC123. All three messages were sent and accepted. Each channel
measured 240,000 samples, 72,928 nonzero samples, 144 nonzero windows, peak
0.19765145 and RMS 0.076079156529. Internal counters agreed. Nonfinite samples,
JACK xruns, process failures, callback deadline misses and terminal faults were
all zero. The operator confirmed: "Yes, I heard it".

Because note-off precedes CC123, this physical sequence exercises CC123 with no
remaining tracked note. Active-note expansion is tested offline, not independently
demonstrated by this physical run.

The host completed 1,268 blocks (324,608 frames), stopped processing, joined its
worker without an exception, deactivated and unmapped its endpoint. Native
retirement reported all milestones (`127`), sequence `1269`, position `324608`,
then `RPI1_CLEAN_SHUTDOWN`. The outer unit exited 0 with result `success`.

## Residual timing and power findings

The callback retained 2,560 missing and expired frames in one startup gap episode.
Ten 256-frame gap records cover positions 0 through 2304. Maximum callback time
was 54,001 ns; delivered frames were 320,000, with 2048 priming and 512 paused
frames. Zero JACK xruns does not turn those gaps into a timing/performance pass.
Live processed counts are blocks; request/result high fields are queue occupancy
high-water marks, not sequence frontiers.

During 25 one-Hz samples with the outer unit present, temperature reached 57.85 C,
supply readings ranged 4.79586–4.96336 V and MemAvailable stayed at least
5,611,448 KiB. Peak memory was 55,644,160 bytes for the outer unit and
2,017,857,536 bytes for the translated cohort.

The kernel reported a new undervoltage event at monotonic 1835.204308 and voltage
normalization at 1837.220044. Firmware spot samples remained historical
`0x50000`; they do not negate the kernel event. The machine stayed responsive
without a restart. The remote Wi-Fi ping record received all 240 packets.
This establishes a recovered voltage event during a successful short session,
not the cause of the earlier whole-machine failure or adequate sustained margin.

## Setup, cleanup and stopping point

Before the attempt, the idle JACK process was present but clients could not
connect. Restarting only its existing service restored the exact graph. The
cause of that setup condition was not established. No additional tone or Pigments
session was used as a setup probe.

The native executable ran directly under a user unit with journal output and a
private command FIFO. One-Hz health recording and remote journal/ping recording
ran outside the translated cohort. The approved temporary runtime-watchdog
disable was restored to its original 60 seconds; no persistent watchdog change
was made. The previous inactive outer-unit configuration was restored afterward.

The successful native process and translated cohort retired, their transport
directory and command FIFO were removed, and their JACK ports/routes disappeared.
The health and remote journal readers stopped. The prior failed session remains
privately retained, and its original evidence is unchanged. No proprietary
payload, account state or raw machine log is published.

Stop after this result. The next planned layer is an editor-closed state round-trip.
State, soak, editor, physical controller, their final combination and overall
RPI1 qualification remain open; this run does not authorize or claim those gates.
