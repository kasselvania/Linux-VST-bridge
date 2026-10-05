# Current outcome: extract native late-note-off recovery onto trunk

The operator selected split landing of the retained audio stack on 2026-10-04.
This isolated branch begins at main `13ed1d85e830d581ec297760e04e9f9433bfd671`,
tree `f281a2e518298f466b2486f3ca7c01a80e84aa28`. The original #200–#207 branches,
history and evidence remain preserved; this is the first independent code piece.

## Claim, basis and scope

The actual native SDK Processor translates a negative-timestamp note-off to
sample zero of a nonempty callback, preserving its voice fields and event order.
Subsequent valid audio continues on the same Processor. Negative note-ons, future
note offsets and zero-frame notes gain no new acceptance. The existing Rust/wire
validators and failure posture remain unchanged.

Basis: the current operator's selective landing instruction; AGENTS.md “Keep the
engineering safeguards” and “Verification and review”; GOVERNANCE.md “What evidence
means”; architecture 2.2 “C++20 owns the VST3 SDK edge” and 7.5 “Events and automation”;
fixture card F0. The detailed contract is in
[the native comparison note](native-vst3-proxy/host/LATE_NOTE_OFF.md).

Production scope is the two native SDK Processor files from `2e159ca7`, with no
manager, IPC, runner, prefix, editor, buffering or Windows processing change.
SDK tests are adapted to main. The independent comparison host includes its
`bad5c69c` latency/early-retirement correction. The existing audit preload is
enabled for the real Processor regression, and CI builds the comparison host.

## Acceptance and evidence

Pinned SDK tests must cover -1661/-1 releases, exact voice fields and following
valid processing, with zero prohibited callback effects. Negative note-on,
future note-off and negative/nonnegative zero-frame note-off remain refused.
Run the existing admission/backend and Linux native suites on this extracted
tree; retained stacked results do not replace those checks.

[Source readback](evidence/audio-split/native-note-off-source.json) retains the
original-source provenance and local observations. The retained original
Pure LoFi physical comparison belongs to the old stacked tree. This extraction
makes no installed Deck, commercial DAW, deadline, editor, soak or beta claim.

## Remaining integration and custody

The full repaired IPC15/AP23 ABI2 engine still requires whole-block/event,
SDK lifecycle/failure, reusable descriptor/admission and coherent configuration
foundations. Preserve the complete notification/render/zero-frame/START/order/
final-SDK repairs together; `559efe8f` is not a standalone cherry-pick.
The [approved split strategy](https://github.com/kasselvania/Linux-VST-bridge/blob/a8e63010/docs/AUDIO_RECOVERY_ROADMAP.md)
remains the source-landing basis. The detailed dependency map is retained as
the separate maintainer review artifact `lvb-audio-split/064bcf0f.md`, SHA-256
`4f8f3c6da0c15f24c82404ac5fdf3cdf671eb3d63a10df11a09da4b47570b90c`.

No shared machine or qualification branch mutation is selected here. Use local
checks and CI; remote builder use requires coordination with its executor.
Review the exact extracted PR head and required CI before merge. Reverting this
PR is the rollback; installed publications and vendor state remain outside it.
