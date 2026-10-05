# Current task: stop the audio dropouts

## Goal

Pure LoFi plays in Bitwig on the Deck at 512 frames / 48 kHz, SameCallback,
editor open, for ten minutes without a dropped cycle; then thirty minutes with
normal interaction. Preserve projects, licensed state and the machine budget.

## Still broken

The same-candidate three-minute editor-open and editor-closed comparisons each
had three graph errors and three 512-frame stereo-zero observations. The latest
closed scheduler capture failed early and the recorder had errors; its no-zero
suffix is not a clean pass. The native continuous-tone control was clean after
one global frequency fit.

## Best explanation (about 60%)

Windows processing plus ordinary-priority caller/Wine contention consumes time
while the Linux SDK caller waits. Caller ready delays occasionally add milliseconds.
The two longest measured calls have different runnable/sleeping composition; all
covered calls remained below the nominal cycle, so editor load alone is not yet
our explanation. We need the Windows processing time for those same calls.

## First change

Reuse the existing Windows process_ns reply and correlate it with each native
whole-call record. Add only bounded preallocated telemetry where that correlation
is missing. Measure three minutes on the same saved device/clip, without a kernel
trace. If Windows processing dominates, act on that side; if it is small, follow
the bridge/Wine waiting time. Compare one change at a time and keep only changes
that improve the musician's result. No measuring-tool repair project.

Execution/source/SSH/Deck: Sol6.1 xhigh; GUI: Sol6.1 high; root orchestrates.
Landing work uses a separate owned worktree. Raw diagnostics stay outside Git.
