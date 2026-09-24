# RPI2 CPU efficiency — startup phase trace OFF/ON/OFF

The operator selected CPU efficiency before latency tuning and authorized a
separate implementation agent, with technical direction and review in the
coordinating conversation. This replaces the unrelated NAD2 task in this
isolated branch only.

## Basis and primary claim

- Base commit: 886414518d6dcff033027412095212fb2294ad5a.
- Base tree: e885ab8bdde24b679e7001033cc7b60e6b2dd1c3.
- AGENTS.md: "Work to an outcome" and "Keep the engineering safeguards".
- docs/ARCHITECTURE.md: "6.4 Thread affinity", "7.3 Audio shared memory",
  and "7.4 Real-time failure posture".
- docs/experiments/RPI2.md: "24 AM warm-up and editor closed/open/closed
  comparison" establishes the demanding workload, not a successful baseline.

The buffered recorder prerequisite reduced native recorder CPU by about 94%,
but the candidate measured pass had 24 attack-time gaps. That is not an audio
reliability win. The primary claim now is an explicit startup phase trace
selector, default OFF, and one identical candidate compared OFF/ON/OFF to
measure native phase observer cost and repeat drift. Existing Windows/transport
diagnostics remain; OFF is not a fully uninstrumented runtime. Governor and
multicore changes remain deferred pending this baseline.

## Fixed fixture

Use the existing Pi 5/ShieldXL ARM Wine/FEX installation, activated Pigments
7.0.1.6772, GE-Proton11-7-aarch64 and installed bridge binaries from the base
campaign. Verify identities before use. Preserve the licensed prefix and runner.
Use the existing saved 24 AM state, Poly 4, master 0.35, notes 60/64/67/71 at
velocity 96, the existing 12-second hold and 20-second recording. Keep the editor
closed. Keep 48 kHz, 512-frame JACK callback, 256-frame vendor quantum and
2,048-frame reserve fixed. Preserve all DSP quality and multicore settings.

## Implementation and experiment

1. Read LVB_RPI1_PHASE_TRACE once at ordinary startup: absent/off disables,
   on enables, invalid fails. OFF skips phase event production, phase-only
   clocks/conversion, recorder thread and file. Preserve scalar failure/status
   counters, callback deadline timing and transport clocks/fault semantics.
   Readback must state mode/recorder/file; disabled mark reports unavailable.
2. Test disabled no clock/event/file/thread and enabled records/marker, preserving
   buffered writer byte identity and write/flush/sync failure checks.
3. Reuse native incremental build cache, preserve installed panel.rs source drift
   and original generic build output and helper default. Stage one identified candidate; no dependency,
   Windows or runtime build. Finish build before first live launch.
4. Review comparison helper before launch. Run OFF1, ON, OFF2 with this same binary,
   same fixture warm-up and measured hold, ondemand and schedstats=0. Use the same
   near-drained live-counter rule in every mode; no phase freshness gate. ON alone
   has buffered recorder and five-second marks, an explicit observer difference.
5. Sample external CPU/frequency/temperature and real audio/counters in all modes.
   Compare whole held 2–14 seconds including attack and stable 3–14 seconds, retain
   warm-ups. Report captured alignment; graph receipt is not exact MIDI timing.
   Phase attribution is unavailable OFF. Retain repeat drift and failures. No extra
   live repeats without review. Restore state/master/graph, preserve original
   default binary and 75 C/current power-warning stops; then stop for review.

The implementation agent is the sole writer and live experiment owner for this
slice. The coordinating agent reviews source/evidence and selects the next
intervention. No other agent should concurrently control this Pi experiment.

## Scope and delivery

Allowed: native-vst3-proxy/backend/src/{queued.rs,rpi1_phase.rs},
rpi0/standalone/src/jack.rs, rpi1/standalone/src/{main.rs,phase_capture.rs},
existing rpi2 helpers and focused analysis/tests, sanitized evidence/rpi2 results,
the RPI2 experiment documentation and this card. No dependency or Windows/runtime
changes. Native incremental build only; no authentication needed.

Excluded: kernel/NTSYNC installation, runner replacement, FEX tuning, affinity
or priority changes, multicore changes, DSP simplification, buffer/latency
changes, queue recovery changes, new plug-ins, GUI/computer-control/VNC actions,
and modification of PR #145 or its original worktree.

Acceptance: reproducible comparison with the controlled differences explicit;
honest measurement availability; audio/state correctness assessed separately
from process liveness; failed or confounded runs never counted as wins; exact
cleanup readback. One good run is not sustained-performance qualification.

Keep proprietary state, audio and raw private paths on the private fixture.
Commit and push the focused source and sanitized results on codex/rpi2-cpu-governor and
update draft PR #150 based on codex/rpi2-native-arm-runtime. Do not merge.
Return the measured result, limitations and one evidence-backed next action.
