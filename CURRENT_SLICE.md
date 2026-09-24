# RPI2 CPU efficiency — buffered phase recorder prerequisite

The operator selected CPU efficiency before latency tuning and authorized a
separate implementation agent, with technical direction and review in the
coordinating conversation. This replaces the unrelated NAD2 task in this
isolated branch only.

## Basis and primary claim

- Base commit: 540eb13f39d7d89815d078b28c4c5f815afd5dda.
- Base tree: f73c235b20024e7699d91d6a7d24dc7cff776719.
- AGENTS.md: "Work to an outcome" and "Keep the engineering safeguards".
- docs/ARCHITECTURE.md: "6.4 Thread affinity", "7.3 Audio shared memory",
  and "7.4 Real-time failure posture".
- docs/experiments/RPI2.md: "24 AM warm-up and editor closed/open/closed
  comparison" establishes the demanding workload, not a successful baseline.

The coordinator selected a specific prerequisite after two retained governor
attempts: remove excessive ordinary-thread recorder write granularity without
changing phase records or audio behavior. Demonstrate byte-identical records,
flush/sync error propagation, and a short same-ondemand original/candidate
comparison. The original governor A/B/A is deferred, not achieved: attempt 02
already sampled approximately 2.4 GHz throughout its active window, while the
audio worker consumed 97.6% of one core with little scheduler wait. Those samples
do not isolate DSP, translation or spinning and do not prove a universal ceiling.

## Fixed fixture

Use the existing Pi 5/ShieldXL ARM Wine/FEX installation, activated Pigments
7.0.1.6772, GE-Proton11-7-aarch64 and installed bridge binaries from the base
campaign. Verify identities before use. Preserve the licensed prefix and runner.
Use the existing saved 24 AM state, Poly 4, master 0.35, notes 60/64/67/71 at
velocity 96, the existing 12-second hold and 20-second recording. Keep the editor
closed. Keep 48 kHz, 512-frame JACK callback, 256-frame vendor quantum and
2,048-frame reserve fixed. Preserve all DSP quality and multicore settings.

## Implementation and experiment

1. Add BufWriter on the ordinary phase drain thread only, with explicit flush
   before the existing underlying sync_data. Keep records/schema, markers,
   incident/finish boundaries and errors. No callback, DSP or routing changes.
2. Check byte-identical records and reduced underlying write calls with a
   counting writer. Exercise write, flush and sync errors.
3. Reuse the native incremental build cache. Preserve the original installed
   binary and stage a separate identified candidate. No clean runtime build,
   dependency changes or unrelated repairs. Return a large-build dependency.
4. Run one original and one candidate session under existing ondemand, each
   with the same saved 24 AM, warm-up/measurement stimulus and mark cadence.
   Scheduler statistics stay disabled and wait is unavailable. Use existing
   live callback/paused-frame/completion counters for a bounded drain check,
   reporting separate-read uncertainty; trace-file delivery is not a live barrier.
5. Compare block completion/throughput, Windows and native recorder CPU, phase
   delivery/coverage and audio separately. Retain failed/incomplete observations.
   Restore operator state and JACK graph, leave the original default binary
   intact, preserve the 75 C/current power-warning stops, then stop for review.

The implementation agent is the sole writer and live experiment owner for this
slice. The coordinating agent reviews source/evidence and selects the next
intervention. No other agent should concurrently control this Pi experiment.

## Scope and delivery

Allowed: rpi1/standalone/src/phase_capture.rs, existing rpi2/ helpers and focused analysis/tests, sanitized
evidence/rpi2/ results, the RPI2 experiment documentation, and this task card.
No dependency or Windows/runtime changes. Only the native recorder candidate is
authorized for a focused incremental build. No new authentication is needed.

Excluded: kernel/NTSYNC installation, runner replacement, FEX tuning, affinity
or priority changes, multicore changes, DSP simplification, buffer/latency
changes, queue recovery changes, new plug-ins, GUI/computer-control/VNC actions,
and modification of PR #145 or its original worktree.

Acceptance: reproducible comparison with the controlled differences explicit;
honest measurement availability; audio/state correctness assessed separately
from process liveness; failed or confounded runs never counted as wins; exact
cleanup readback. One good run is not sustained-performance qualification.

Keep proprietary state, audio and raw private paths on the private fixture.
Commit and push the focused source and sanitized results on a new branch, and
open a draft follow-on PR based on codex/rpi2-native-arm-runtime. Do not merge.
Return the measured result, limitations and one evidence-backed next action.
