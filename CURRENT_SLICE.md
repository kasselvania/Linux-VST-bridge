# Current work: AP6 complete — no successor selected

## Accepted capability

AP6 is the reviewed recovery capability delivered by PR #63, closing #62. This status accompanies that implementation's merge; no separate closure task is required.

The prepared native Linux bridge can explicitly recover one failed Windows AGain endpoint from its identified, last confirmed complete component-state snapshot without restarting Bitwig, re-entering settings or interrupting its healthy sibling. Replacement requires positive owned containment and retirement. Full-state readback and controller/host synchronization precede resumed processing; fresh endpoint, generation and queues prevent old-audio replay. Missing state, failed restore and uncertain retirement remain failures.

[AP6 results](docs/AP6_RESULT.md) and [the original sanitized observations](evidence/ap6-instance-recovery/result.json) retain the actual build, desktop, test and cleanup provenance. Real Windows fault/recovery ran on native source `5791256dc0e14237c008f85847cf380605d840bf`; final source `428870b3345177e722548a144558f1a516d5dc9a` passed focused loaded recovery cases and normal-desktop save/reopen after the snapshot-notification correction. The result/publication commit was `f7152c64c1b9ac151ce0fdf717c62e7d376e62d5`. These are distinct observations, not a claim that all tests ran on one source.

AP5 independent instances and AP4 state/recall remain accepted. Their evidence is unchanged. AP6's observed recovery, sibling and reopened intervals compared 33,282,560 returned samples at zero error without gain edits. That is scoped development evidence, not general reliability qualification.

## Remaining limits

- Recovery restores the last confirmed snapshot, which may predate later edits; it does not restore lost audio or tails. Snapshot storage lives in the native component, not a separate durable service.
- Replacement startup remains slow and synchronous on the SDK owner thread. Healthy-sibling audio survived the observed recovery, but control interactions may wait.
- The historical AP5 transport loss and the initial AP6 native queue underflow remain unresolved. Controlled recovery and a substituted late-response test do not establish their causes.
- The delivered preview is prepared AGain/X11, float32 stereo at 48 kHz, up to 256-frame callbacks and 1024 added samples. Commercial plug-ins, Windows editors, instruments, reboot persistence, packaging and low-latency/reliability qualification remain unclaimed.

## Next work

No new implementation or live experiment is selected by this closure. The technical-lead recommendation is to address ordinary playback reliability using the retained first-fault evidence and focused timing/queue investigation before expanding compatibility or packaging. Select the concrete outcome with the operator; do not reopen historical process campaigns or replay accepted observations merely to change their label.
