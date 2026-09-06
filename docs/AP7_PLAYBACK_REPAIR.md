# AP7 playback repair — implemented, awaiting review

PR #65 now preserves a healthy instance through transient late output. The native callback fills only the missing presentation span with silence, counts it, advances host time, and discards expired portions of subsequently returned output. Still-due samples retain their original absolute position. Every admitted input and control remains ordered through the same Windows instance; latency stays **1024 samples**. Disconnect, bounded request timeout, corrupt/mismatched responses and exhausted capacity remain terminal failures with AP6 recovery.

The demonstrated defect was the old policy: an empty due-output queue permanently failed and retired a healthy connection. The actual previous SDK-loaded binary fails the new delayed-request test at the first missing callback. The repaired binary survives both a **128-frame partial-block gap** and a **768-frame gap spanning complete blocks**, resumes with exact samples, preserves the complete state, processes a subsequent control edit once, and closes the original connection normally. Its sibling has no gap or failure. The deterministic driver explicitly waits until the peer holds the real request, advances presentation time, then releases it; this establishes queue/lifecycle behavior, not ordinary wall-clock service latency.

Reference comparison and fingerprints now run on a separate consumer. The transport publishes validated output first, then makes a bounded copy into a 64-entry observation queue. Neither the next request nor result publication waits for comparison or diagnostic readers. Overflow loses observation coverage, invalidates the reference until another actual state readback, and does not count unchecked samples as verified. Activation no longer issues a state request solely to seed observation. Bounds, sequence/epoch/position, guards, unchanged inputs, finite output and silence claims remain mandatory.

Callback work is bounded by the existing 2048 descriptors and 256 frames, with preallocated storage, scalar operations, atomics and monotonic timestamp reads. It adds no allocation, blocking, logging, process or transport I/O. Optional traces connect a missing span to a particular request's queue wait, preparation, send, reply, validation and publication. Only the first 32 span traces are retained; missing trace coverage is reported. The final trace-only correction explicitly marks output discarded after an epoch stops as unpublished, with no publication timestamp. It leaves the tested publication predicate and playback behavior unchanged and passes the 23 Rust tests and Clippy. No queue enlargement, priority escalation, Windows rebuild or automatic restart was used.

## Focused verification

- 23 Rust tests pass, including partial/fully expired output, variable lengths, epoch restart, capacity/correlation failures, first-fault retention and observation-consumer/reader pauses. The actual mapped/TCP worker processes 128 requests correctly with the consumer and reader paused; observation fills and drops without delaying playback. Separate consumer checks account for dropped and unchecked samples.
- The final Linux SDK-loaded build passes both delayed-output cases, genuine disconnect with complete-state recovery and sibling continuity, and corrupt output-guard rejection. The callback audit records zero forbidden effects. All checked samples have zero error.
- Seven owner cleanup/isolation tests and Clippy with `-D warnings` pass. Hosted CI is tracked on PR #65.

The SDK-tested native build is source `60e2cf7a9db7d2071b289e27688cf7f5f82586de`, manifest `667c8cc73a2769a088347f892172f0eb7118ea35713fffbb70aef527339bd244`. The 23-test actual-worker regression source is `f2b0f6cfe686a0c89deebe75d7e92a7a16ac88a0`; only results documentation follows it in the native build source. The same final test driver rejects the previous native bundle (manifest `c2f47288fa8409f76519fcb9fec7880a5b3138609c7f2fb7ef5616df6d9e4a72`) at its terminal-underrun policy.

## Normal desktop result and provenance

Bitwig 6.1 was launched from KDE Applications through existing Moonlight. A disposable copy of the existing two-track project restored A=0.6750 and B=0.3500, with the original complete-state payload hashes, before playback. Two real Windows AGain endpoints ran behind **one native Bitwig plug-in host process**, with no `LVB_` launcher environment variables. Both distinct clips played, paused, resumed and closed without changing controls.

| Measured result | A | B |
| --- | ---: | ---: |
| Compared returned channel samples | 10,147,840 | 11,097,088 |
| Delivered channel samples | 10,141,696 | 11,092,992 |
| Maximum sample error | 0 | 0 |
| Missing presentation frames / gaps | 0 / 0 | 0 / 0 |
| Terminal rejected callbacks / silent frames | 0 / 0 | 0 / 0 |
| Observation drops / unchecked samples | 0 / 0 | 0 / 0 |
| Priming frames | 3,072 | 2,048 |
| Successful silent frames, including priming | 2,225,408 | 2,700,032 |

All returned output was checked, so comparison coverage includes every delivered sample. Returned totals also include output left in latency tails; delivered totals are separate. Successful silence includes stopped/idle input. These figures are processing/presentation evidence, not a claim of uninterrupted audible program for the entire native lifetime or physical acoustic listening.

**Desktop source is `a8a8318099ec6a733792e4f4ae5e8671cf2c5d6f`**, manifest `bcabc84cd2b29c412d0bbc9dca76cbeb6a5a6a36841a418129e281534c3fd837`, native binary SHA-256 `163da5498e9fd2f1f354d48598accfad9dec552f760731aacddc737404a5031c`. The final commit subsequently tightened the test gate and removed an observer-only conditional state query before activation. Both desktop instances had already performed real get/set state operations before activation, so that conditional query did not execute in the retained desktop check. The activation cleanup, subsequent actual-worker regression and redundant-closure lint cleanup were rebuilt together and passed the focused SDK checks; the desktop workload was not replayed under a new label.

Windows manifest `f00325df1cefe08c4496d3007211d7ecebdd02a49d4dc42a6a2e0a94881ec615`, AGain, Proton 11.0-2c and Steam Linux Runtime 4 were reused unchanged. Read-only scheduling showed the native transport/observer and Windows owner/processing threads at ordinary policy/priority; this is a during-playback snapshot, not a trace of a historical fault.

## Limits and cleanup

No ordinary underrun occurred in this bounded check. The new code survives injected gaps; the quiet desktop check does **not** explain the retained AP5 transport loss or AP6/original AP7 delay. The historical 30.474-ms maximum remains an aggregate, not a particular request's root cause. Actual late service can still produce counted gaps, and backlog exhaustion still fails explicitly. This is not a long-run reliability or commercial-plug-in qualification claim.

Original [AP7 incomplete results](AP7_RESULT.md), AP4/D9, AP5 and AP6 evidence remain unchanged. Their successful real save/reopen and recovery results retain their original source. Here, complete-state preservation after a gap and disconnect recovery were checked through the final SDK-loaded implementation; no duplicate recall ritual was performed. AP6's last-confirmed-snapshot/uncaptured-edit limitations remain.

Cleanup found zero owned stages or Bitwig/native/Windows processes. The owner stopped successfully; its socket and temporary publication were removed. Preferences and plug-in metadata were restored. Original AP4/AP5/AP6/AP7 project hashes are unchanged; the disposable repair copy is retained. Sunshine remains active. Build artifacts and private diagnostics are retained without publishing runtime binaries, private paths or credentials.

[Selected observations and exact capture hashes](../evidence/ap7-transient-playback/result.json), [before](../evidence/ap7-transient-playback/before.txt), [Rust](../evidence/ap7-transient-playback/rust.txt), [owner](../evidence/ap7-transient-playback/owner.txt), and [Clippy](../evidence/ap7-transient-playback/clippy.txt) identify the source of each result. PR #65 remains unmerged for review; issue #64 remains open pending review.
