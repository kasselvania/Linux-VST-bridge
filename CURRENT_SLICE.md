# RPI2 CPU efficiency — fixed-workload governor comparison

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

Determine whether the performance governor materially improves completion of
the same Pigments workload compared with ondemand, and distinguish increased
clock availability from reduced CPU work. A supported negative or inconclusive
result is valid; do not manufacture an optimization claim.

## Fixed fixture

Use the existing Pi 5/ShieldXL ARM Wine/FEX installation, activated Pigments
7.0.1.6772, GE-Proton11-7-aarch64 and installed bridge binaries from the base
campaign. Verify identities before use. Preserve the licensed prefix and runner.
Use the existing saved 24 AM state, Poly 4, master 0.35, notes 60/64/67/71 at
velocity 96, the existing 12-second hold and 20-second recording. Keep the editor
closed. Keep 48 kHz, 512-frame JACK callback, 256-frame vendor quantum and
2,048-frame reserve fixed. Preserve all DSP quality and multicore settings.

## Implementation and experiment

1. Reuse existing supervision, audio capture, phase records and thread sampling.
   Add only the small adapters/analysis necessary for this comparison. Do not
   build a dashboard or a general benchmark framework.
2. Capture the original governor and scheduler-statistics setting. Temporarily
   enable scheduler statistics if existing privileges allow it; otherwise mark
   wait data unavailable. A disabled counter is never evidence of zero wait.
3. Run a short warmed ondemand/performance/ondemand comparison. Ensure each
   measured hold begins without inherited bridge backlog; use the same existing
   reset/start and warm-up procedure for every condition. Mark note-active and
   idle/release windows separately. Do not compare whole-session averages as if
   they were identical active DSP intervals.
4. Report vendor wall time (distribution and slow-block count), available
   caller/worker CPU, scheduling wait, actual sampled frequency, whole-process
   CPU, queue age, missing frames, captured-audio validity, temperature and power
   warning flags. Use existing FEX or caller-CPU counters where readily available;
   identify missing measurements rather than expanding scope to obtain them.
5. Restore original governor, scheduler-statistics setting, operator plug-in
   state and JACK graph; terminate only owned experiment processes. Use bounded
   runs and existing thermal/power limits. Retain failures and cleanup results.

The implementation agent is the sole writer and live experiment owner for this
slice. The coordinating agent reviews source/evidence and selects the next
intervention. No other agent should concurrently control this Pi experiment.

## Scope and delivery

Allowed: existing rpi2/ helpers and focused analysis/tests, sanitized
evidence/rpi2/ results, the RPI2 experiment documentation, and this task card.
No dependency changes. Prefer the installed binaries; a need for product-code
or runtime changes is a finding to return, not an invitation to widen this slice.

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
