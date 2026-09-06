# Current work: AP5 — Two independent plug-in instances

## Accepted baseline

**AP4 is accepted and merged.** PR #59 was reviewed at `93a00ede0044c4d9e0ac28bee0836d97675865ce` (review 5124120833) and merged with operator approval as `a45a30b916f847d1cc683ef7e07121b57d52491c`; #57 is complete. Normal Applications launches saved and recalled 0.1650 through actual Windows component state, with 5,250,048 fresh-recall samples at zero error and no control edits. The interactive instance survived ordinary waiting and Moonlight disconnection. See [result](docs/AP4_RESULT.md) and [preview setup](docs/AP4_PREVIEW.md). Their pending-review text records publication time; this later review/merge closes AP4. D9 and failed A2 stay unchanged. No further AP4 verification is needed.

## Goal and scope

Use two copies of the bridged Windows AGain effect in one Bitwig project, with independent audio, controls and saved state. Save, close and reopen both correctly; changing or removing one must not disturb the other. Issue #60; implementation branch `codex/ap5-independent-instances`.

The lead has selected this successor under the operator's request. The implementation task includes necessary builds, focused tests and routine repairs. AP4's consumed limits remain its history, not a renewed campaign or a gate on AP5. Use normal development commands; do not create per-source approval receipts or a diagnostic/acceptance replay. Normal tool approvals and spending safeguards in AGENTS.md still apply.

The existing queued backend has global ACTIVE/BUSY state; the private owner rejects every second connection. Replace those single-instance assumptions in the working path. Each live instance owns its queues, mapping, state, reports, connection and Windows process/environment. Share immutable artifacts/runtime, not mutable instance data. Preserve stable class identity and project-state format; temporary instance IDs do not belong in saved blobs. Support two instances within one Linux plug-in process as well as separate host processes; do not depend on Bitwig's Individually mode to hide globals. Sibling creation, callbacks and teardown must not block or fail unrelated audio. An explicit capacity limit is fine if at least two work and excess is refused safely. Private types and implementation organization belong to the engineer.

## Enough evidence

Use focused local tests for two loaded instances in one process, including concurrent callbacks and a failure/close of one while the other continues. In normally launched Bitwig, use two separate tracks with distinct inputs and gains, check their respective returned audio, save/close/reopen without re-entering values, and remove one while the other keeps playing. Exercise a shared native hosting mode and record the actual topology; no all-modes matrix. Compare enough samples to expose crossed routes or shared state, with no arbitrary sample-count target. Reuse unaffected AP4 evidence; fix and retest only affected behavior unless a real integration uncertainty requires more.

## Boundaries and delivery

Keep actual Windows DSP/state, nonblocking audio callbacks, independent owned cleanup, prepared runtime and truthful 1024-sample per-instance latency. Retain float32 stereo at 48 kHz and up-to-256-frame blocks. Reuse the preview owner and transport; no general broker, installer, reboot-persistence, editor, commercial-plug-in or latency-tuning project. Serum remains the commercial target, not a claim made by AP5.

Use established SSH/Moonlight access and Bitwig from Applications. Preserve existing projects and restore only temporary test settings/publication. One implementation PR referencing #60, with observed results, tests and limitations; leave it unmerged for review.

Targeted basis: [architecture](docs/ARCHITECTURE.md) §§5.7–5.9 and 6.5; `native-vst3-proxy/backend/src/queued.rs`, `backend/src/preview.rs` under the same directory, and `tools/ap4_preview.py`; [Bitwig hosting modes](https://www.bitwig.com/userguide/latest/vst_plug-in_handling_and_options/).

## Implementation result pending review

PR #61 contains the AP5 implementation and [focused result](docs/AP5_RESULT.md). Two fresh recalled instances in one native Bitwig host compared 61,495,808 samples at zero error with no gain edits; removing A left B processing in the same Windows process. Both recalled instances cleaned up. An earlier isolated transport loss remains unexplained and is retained as a limitation; the terminal-fault diagnostic gap was repaired. AP5 is not marked accepted.
