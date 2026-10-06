# Paced Pure LoFi priority comparison: aborted after two first attempts

The declared A/B/B/A comparison is **incomplete**. Its first ordinary-native and
RR5-native windows both retain unexplained stereo-zero spans and Bitwig/PipeWire
ERR increments. B's recorder reached its finite limit before the normal-stop
marker, so the declared recording-loss abort was honored: windows03-B/04-A were
not performed, and no run or recorder-limit adjustment followed. This one pair
does not establish a priority cause, rate reduction or dependable continuity.

The [exact sanitized records and retained-file hashes](2026-10-04-paced-priority-abort.json)
bind unchanged source `d970eda9`, engine `add49d7a`, paired Windows host `c8587d8c`,
package `0.12.0refresh5deck`, Pure LoFi1.0.0.6121 and its SameCallback publication.
The working project used the same single S1 four-note110BPM clip and visible
Little Pleasures editor, graph512/48k and explicitly linked Bitwig-only F32LE
stereo capture. Recorder graph quantum1024 is a separate observation. Actual
VST3 N and device deadline remain unobserved. Host-version metadata comes from
the prior baseline; per-window power readback was not retained. No power change
was made, and sparse editor screenshots do not prove constant graphics workload.

The reviewed protocol fixed four180-second windows in A/B/B/A order. A changed
only the authenticated native worker to OTHER|RESET_ON_FORK, priority0; B used
RR|RESET_ON_FORK, priority5. The stopped-transport A→B restoration check succeeded.
Sampled readbacks retain the declared native policy, Windows RR5, nice0,
affinity0–7 and RTTIME soft/hard200000µs. The consumer was inside Flatpak and the
Windows worker outside. Existing managed-keeper startup is recorded, not cold.

| First attempt | Monotonic bracket | Byte-snapshot extent | Inside 512-frame stereo-zero spans | Active Bitwig ERR increase | Whole retained output |
|---|---:|---:|---:|---:|---:|
| 01 A: native ordinary | 180.034148881s | 179.946666667s | 5 | 7 | 266.933333333s |
| 02 B: native RR5 | 180.001579702s | 179.968s | 4 | 4 | 360s |

Both entire files were scanned:60,185,600 finite sample values, with three
additional isolated512-frame spans outside A's bracket and one outside B's;
initial setup-zero intervals are also retained. The unchanged near-zero and
sample-step scans are diagnostics, not a sample-exact vendor oracle or universal
click/glitch test. Vendor tails/noise and note rests remain opaque. Four of A's
five and three of B's four inside spans have nearby ERR associations under the
byte-observation bracket expanded by±1second; these are neither precise overlaps
nor proven corresponding xruns. Recorder ERR stays0, active sink ERR stays1,
and sampled bridge missing/gap/expired counters stay0.

A's recorder was normally interrupted after stop. B's recorder produced exactly
360seconds and then ended before its stop marker; the retained monitor records
`capture ended before normal stop`. Both recorder statuses are1 with empty
stderr; [PipeWire1.6.4 record-loop behavior](https://raw.githubusercontent.com/PipeWire/pipewire/1.6.4/src/tools/pw-cat.c)
is consistent with signal/finite-limit termination without the drain-success
flag. All sampled recorder checks inside both brackets were alive. The B cutoff
precedes its stop marker by about4.225seconds, but the actual GUI-stop instant and
any uncaptured audible duration are unobserved. Full-lifecycle B capture remains
incomplete; its captured bracket does not erase that failure.

The native worker was restored to RR5 before normal stop/save/quit. Exact owned
native/Windows processes retired; DSP/maintenance are0 with cleanup confirmed.
The healthy product keeper remains. Registry/software/preferences, predecessor
history and original project are exact; the working project is saved separately.
Vendor exit0 and reopen are not claimed. GUI/stream are closed, builders/VM stay
off, and no source, global privilege or scheduler-policy repair occurred.

The reviewed scheduler-trace helper is staged but **unexecuted**: tracefs is
root-only and noninteractive sudo is unavailable. The retired target is retained
as history, with no live TARGET. When the user is locally available, a fresh
stopped RR5 session and authenticated target/capture setup must precede the
reviewed local OS-authenticated command. No audio session is left running while
waiting. Historical outside-Flatpak SDK RT refusal remains a separate unknown;
this Bitwig session's product promotion and manual restoration succeeded.
