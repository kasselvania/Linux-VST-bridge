# Current work: AP7 — Keep playback alive through a transient underrun

## Outcome and permission to finish

Fix PR #65 / issue #64 on `codex/ap7-playback-underflow`. A temporarily late audio result must cause a counted gap, not destroy an otherwise healthy Windows instance or require manual recovery. Remove optional verification work from the audio-delivery dependency chain. The operator approved this correction; implement, build, debug and verify it under AGENTS.md without another selection or approval-document handoff. Normal tool permissions and explicit spending limits remain separate.

AP6 remains accepted at `c861993b886dc89e1358a3005be899f46492bfb7`. Keep AP7's useful diagnostic-mutex repair and [original incomplete result](docs/AP7_RESULT.md). Earlier AP5/AP6/AP7 failures remain recorded. Review 5126115612 explains the correction; it does not identify the cause of the initial scheduling/transport delay.

## Change the behavior, not just the error message

Start in `native-vst3-proxy/backend/src/queued.rs` (`Callback::process`, worker and the terminal-underflow test), `source/processor.cpp`, `backend/src/lib.rs` (`Session::open_bound` / `Session::process`) and `backend/src/state.rs` (reference Witness).

**Late output is not a terminal session fault.** Silence only the missing host-time span, count it and advance the output timeline. Keep instance/state/connection and ordered input/control processing alive. Do not latch the C++ processor into Failed for a counted underrun. When correctly correlated output arrives late, discard its already-expired portion and deliver its still-due portion at the original absolute sample position. Cover partial blocks and bound callback work. Discard obsolete output, not unprocessed input: a stateful plug-in must still receive each admitted input/control exactly once in order. Do not replay audio, shift latency or restart/reprime the component to hide a gap.

Disconnection, bounded request timeout, corrupt or mismatched responses, and exhausted capacity remain explicit failures. Preserve finite progress/backlog limits and AP6 recovery for those failures. Keep underrun frames/discontinuities distinct from terminal rejection, valid silence, priming and sample-comparison errors. A callback returning a filled buffer must not be reported as gap-free playback when part was missing.

**Optional observation must not gate audio service.** Remove reference comparison, fingerprinting and diagnostic-reader waits from both current-result publication and subsequent-request progress. Use a bounded nonblocking observation handoff to a separate consumer, or an explicit separate verification mode. Retain mandatory bounds, validity and correlation checks. Report observation overflow or disabled comparison honestly; unchecked audio cannot count as verified. Preserve actual Windows DSP/state and SDK thread rules. Private implementation details belong to the engineer; no new telemetry/broker framework.

## Evidence that finishes this repair

Replace the test requiring permanent failure after an empty due queue. Through the actual callback/worker/proxy path, force a delayed-but-valid response and verify the exact missing span, unchanged connection/state, correct post-gap alignment, and unaffected sibling. Cover a partially late block and a real disconnect/mismatch. A paused observation consumer must not stall playback. Focused tests may share one scenario; no prescribed test count or separate acceptance campaign.

Then check two real Windows instances in Bitwig launched normally from Applications. Record gaps and failures as well as correct delivered audio. Reuse unaffected state/recall results; retest only affected behavior. If ordinary playback still misses deadlines, continue this task: correlate the specific late request's queue wait, send, reply, validation and publication, and inspect actual participating-thread scheduling before choosing a targeted repair. The historical 30.474-ms maximum alone cannot establish its cause. Do not stop at another aggregate-only report or claim a quiet run explains a historical failure.

Keep the current float32 stereo, 48-kHz, up-to-256-frame and 1024-sample-latency configuration. No larger buffer to conceal the defect, automatic process-restart loop or unsupported priority escalation. Reuse SSH/Moonlight, owner, transport and artifacts; rebuild only changed components. Necessary native/Windows changes supported by the diagnosis are in scope, not blocked by the previous binary's pin.

Preserve projects and original observations, restore temporary setup and clean only owned resources. Publish the repair, focused evidence and remaining limits in this same PR; leave it unmerged for review. Report survivable injected gaps separately from ordinary-playback reliability. No arbitrary sample quota, GUI deadline or replay of historical campaigns.

## Implementation status for review

The playback repair is implemented in PR #65. The final native source passes delayed/partially late output, complete-state preservation, sibling continuity, disconnect recovery and corruption checks. The retained normal Applications-launch desktop check recorded zero gaps or terminal rejections and complete zero-error sample comparison. [The repair result](docs/AP7_PLAYBACK_REPAIR.md) identifies the desktop and final SDK sources separately, cleanup and remaining limits. Original results above remain unchanged. Review/acceptance is pending; this status does not claim a historical root cause or select another slice.
