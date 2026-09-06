# Current work: AP7 — Keep playback alive through a transient underrun

## Goal

Keep the prepared two-instance bridge usable during ordinary playback. A late audio block must not automatically destroy a healthy Windows instance or require manual state recovery. Missing sound must still be reported accurately. Continue PR #65 / issue #64 under AGENTS.md; this is the lead's correction to the existing task, not a new slice or permission cycle.

AP6 remains the accepted baseline at `c861993b886dc89e1358a3005be899f46492bfb7`. AP7's diagnostic-mutex repair is useful but its desktop result is incomplete. Preserve [that result](docs/AP7_RESULT.md), AP5's unexplained loss and all earlier evidence. Review 5126115612 records the source-grounded diagnosis and this behavioral correction.

## Pinpointed defects

`backend/src/queued.rs::Callback::process` calls `s.fail(UNDERFLOW, position)` when due output is absent. The worker treats that flag as terminal and closes the transport; `source/processor.cpp::process` latches Failed. One late result can therefore become minutes of rejected callbacks. The existing `absent_due_output_latches_and_never_replays` test requires that destructive behavior. Change the behavior and its test; preserving no-replay does not require preserving that policy.

`backend/src/lib.rs::Session::open_bound` enables the reference Witness automatically for normal owner-backed sessions. `Session::process` performs its reference comparison/fingerprinting before the worker can publish output. PR #65 removed one observer mutex wait, not this remaining deadline dependency.

## Implement the correction

Distinguish **output underrun** from **failed session**. On a temporarily missing due result, silence the missing host-time span, count it and advance the timeline. Keep the instance, transport and state alive. Continue processing inputs/control exactly once in order. When valid late output arrives, discard only frames whose presentation time has passed and resume at the correct absolute position. Handle partial blocks and bound queue draining. Do not replay requests, skip state evolution, shift the declared latency, or restart/reprime the plug-in to hide the gap.

Actual disconnection, request timeout, wrong generation/sequence, corrupt data and exhausted capacity remain explicit failures; finite request/backlog limits still handle sustained lack of progress. Keep AP6 recovery for those failures. Underrun counters must remain distinct from successful silence, initial latency priming, numerical mismatch and terminal failure. Filling a callback buffer is not a claim that no audio was missed.

Remove optional comparison, fingerprinting and diagnostic-reader work from the audio-completion dependency chain. Merely moving it after publication still delays the next request. Use a bounded nonblocking observation handoff to a separate consumer or an explicit separate verification mode; report lost observation coverage honestly. Retain mandatory validity/correlation checks and real Windows DSP/state. Do not violate component thread rules to make state calls concurrent.

## Verify the changed behavior, not another campaign

Use a deterministic delayed-but-valid result to show the old terminal behavior and the corrected timeline behavior. Check late partial blocks, real disconnect/corruption and sibling continuity. Update the existing tests rather than add a proof framework. Then use a focused normal Applications-launched Bitwig check, recording actual host-time gaps and failures as well as correct samples.

The historical 30.474-ms aggregate cannot identify that particular stall's origin. Do not label it a proven Proton, scheduler or hashing defect. If ordinary service remains late, split the specific request into dequeue/send/reply-validation/result-publication timing and read back the participating threads' actual scheduling policies before selecting a scheduling repair. The whole audio service path matters, not only the DAW callback. Keep this targeted investigation within the same implementation task.

Keep float32 stereo, 48 kHz, up to 256 frames and 1024-sample latency. Reuse the existing owner, transport, SSH/Moonlight and artifacts; rebuild only changed code. Necessary implementation, ordinary debugging and focused checks are authorized under normal tool permissions and spending safeguards. Preserve user projects and clean only owned resources. No arbitrary larger buffer, automatic process-restart loop, blanket priority escalation, compatibility expansion or duplicate diagnostic/acceptance run.

Leave PR #65 unmerged for review. A survivable deliberate gap is progress, not proof that ordinary playback never underruns; report that distinction and any remaining limitation. No production repair or new device result is claimed by this work-order correction.
