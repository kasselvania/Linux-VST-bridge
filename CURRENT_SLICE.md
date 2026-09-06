# Current work: AP7 — Fix avoidable playback underflow

## Outcome and authorization

Keep the prepared two-instance Windows AGain bridge working during ordinary playback and transport transitions instead of requiring manual recovery after an avoidable queue/lifecycle failure. Issue #64; branch `codex/ap7-playback-underflow`; accepted AP6 basis `c861993b886dc89e1358a3005be899f46492bfb7` (PR #63).

The operator approved this outcome. Implement, debug, build and perform focused local/desktop verification under current AGENTS.md. Reuse existing binaries and helpers when unchanged. Private types, file layout and the targeted repair belong to the engineer. There is no separate authority PR, per-source permission, diagnostic campaign or compulsory replay. Normal tool approvals, spending safeguards and protection of user work remain in force.

## Starting facts, not a diagnosis

[AP6 results](docs/AP6_RESULT.md) and [original observations](evidence/ap6-instance-recovery/result.json) retain an underflow during initial setup before deliberate fault injection: `fault=1`, `first_position=9783808`, `processed=38215`. Its scheduling cause was not established; do not call it a first-buffer/startup defect merely because it happened during setup. The substituted late-response case demonstrates one way to underflow, not the cause of this event. AP5's earlier transport loss remains separately unexplained unless evidence connects them.

AP6 recovery, AP5 sibling isolation and AP4 state/recall remain accepted. Preserve their evidence and original provenance. Recovery still restores the last confirmed complete snapshot, possibly older than later edits; this task does not fix snapshot age or synchronous recovery UI startup.

## Engineering direction

Start with the callback/worker queue and epoch/control transitions in `native-vst3-proxy/backend/src/queued.rs`, `queue.rs`, and `source/processor.cpp`; follow the actual transport dependency only as needed. Use retained first-fault records and focused reproduction to distinguish incorrect queue/start-stop alignment, worker scheduling or response delay, and real peer/transport failure. Examine whether state/control traffic or diagnostic work contributes rather than assuming the plug-in or runtime is at fault.

Collect only the missing bounded timing/position/progress facts needed to discriminate those causes. Use callback-safe counters and non-real-time reporting, not logging, blocking, allocation or process operations inside audio callbacks. Reuse existing failure records; do not build a telemetry framework.

Fix demonstrated causes and compare the same stimulus before and after. Keep real Windows DSP, exact sample/epoch correlation, bounded callbacks, genuine-failure silence, per-instance ownership and AP6 recovery. Never turn underflow into success by hiding missing audio, replaying stale results, inventing output, disabling fault detection or adding an automatic restart loop. Keep the current float32 stereo / 48 kHz / up-to-256-frame / 1024-sample-latency contract. Do not enlarge latency or buffering merely to conceal the failure; a necessary contract change is a concrete lead decision, not another process redesign.

## Enough verification

Use targeted Rust/SDK-loaded regressions for the demonstrated queue/timing defect and affected start-stop/idle-resume behavior. Check that a genuine late, disconnected or mismatched peer is still reported correctly and cannot corrupt a sibling or replay old-generation audio. Retest state/recall and recovery only where the change affects them.

Then check the delivered fix with two actual Windows instances in normally Applications-launched Bitwig using a disposable copy of the existing project. Exercise the relevant playback/transport sequence and verify both actual output and sibling continuity. Account for callback rejections, silence and discontinuities, not just zero error among returned samples. Preserve valid sub-results; no arbitrary sample target, timed acceptance label or indefinite soak. If repeated trials teach nothing new, investigate the evidence or report the specific unresolved question rather than continue blind reruns. A quiet run alone does not establish a root cause or general reliability.

Reuse the prepared runtime/artifacts and established SSH/Moonlight under their permissions. Restore temporary settings and preserve original projects, credentials and observations; retire only owned test resources. No commercial plug-in, editor, instrument, installation, packaging, new broker or compatibility expansion.

## Delivery

Continue on this branch and publish one implementation PR referencing #64, left unmerged for technical review. Include the cause established, fix, focused before/after checks, actual desktop result, remaining limits and cleanup. Preserve failed observations; distinguish any new explanation from the still-unresolved historical AP5 loss. Update this status in the same PR. Use normal commits; no cosmetic history rewrite, separate audit artifact or closure PR. If no defensible fix is established, return concise findings and the next specific engineering decision without claiming AP7 complete.

## Current status — 2026-09-06

**AP7 remains incomplete.** The diagnostic-reader mutex can no longer block the audio transport worker; the same forced-contention regression fails before and passes after the change. Focused Rust and SDK-loaded checks pass, with genuine failure silence, sibling isolation and recovery preserved.

The normal Applications-launched desktop check still produced an underflow in A (epoch 3, due output absent, maximum service duration by reporting time 30.474 ms); B continued with zero rejected callbacks. A recovered its identified complete snapshot without gain entry, and both subsequently saved/reopened with correct processing. This establishes neither the remaining timing cause nor a historical AP5/AP6 root cause. No buffer/latency increase or Windows rebuild was used.

[AP7 results](docs/AP7_RESULT.md) retain the failure, before/after checks, callback silence/discontinuity counts, actual recall result and verified cleanup. The implementation PR references #64 and is left unmerged. The next engineering question is where the specific late request loses its output lead: native scheduling, peer response, or native post-response service. Decide that before selecting another repair; no general reliability or AP7 completion claim is made.
