# AP0 result — fresh verification complete, pending review

Fresh acceptance compared **96/96 float32 samples with maximum absolute error 0.0**. The three out-of-place stereo blocks ran at 48000 Hz, 16 frames each, in kOffline:

| Block | Gain | Input | Samples compared | Maximum absolute error |
| --- | --- | --- | --- | --- |
| 0 | 0.5 | Distinct left/right binary-fraction patterns | 32 | 0.0 |
| 1 | 0.25 | Different distinct left/right patterns | 32 | 0.0 |
| 2 | 0.25 | Silence on both channels | 32 | 0.0 |

Every process call returned zero. Inputs and guards remained unchanged; no output was missing, unwritten or nonfinite; silent output flags were 3. Expected samples came from the independent fixed recipe in [CONTRACT.md](CONTRACT.md), with zero tolerance established before execution.

Setup/activation used the owner thread. A distinct processing thread issued the start/stop notifications and three process calls, then joined before owner-thread deactivation, interface release, component termination/release and module unload. The two notifications returned the pinned SDK default kNotImplemented (0x80004001), retained explicitly. Exit was zero; in-process shutdown and owned-process containment completed; the stage was absent. Protected-state digest before and after was `307e53f193155ee471e5a7d8ae5ab106ddc9ea253e0a42d203f5429b66eef305`.

## Exact verification

- Executed source: `d9f4a0dec920b71b50fa07460b288d9e7407313b`.
- Fresh acceptance candidate: `7968963e0756e148da9e6c3156c8b444a92ff48cc1639db431abdbf2459a5405`.
- Reservation: `6aca9f9406a6c8cc36def2850294a356320f347743d9dff7241e13b8a391503b`.
- Result: `7d4e76d6dae11b3993968b76a7b0ac2c705ab5b8ee48cbbb0bba17d3c7d4649e`; disposition `ACCEPTANCE_EVIDENCE_RENDERED`.
- Windows producer: [run 33942379173](https://github.com/kasselvania/Linux-VST-bridge/actions/runs/33942379173), source `0add6e51bb9c0f2063906630679777503967f9fc`, artifact `9962278098`. Exact source/build-input equivalence and custody were checked before execution. AGain and the deployed runtime were reused unchanged.
- [Five-file acceptance packet](../../../evidence/ap0-offline-again-processing/FINDINGS.md) contains actual sample words, lifecycle, identities, effects and hashes. It was independently read back and validated after retrieval. Later commits bind authority and report this result; they do not change executed implementation bytes.

## Repairs and cumulative costs

The existing Windows host gained fixed buffers/parameter queues and a bounded offline lifecycle. The runner reuses the existing worker, supervisor, retention, custody and atomic renderer. The inherited PC0 census implementation was ported from its accepted producer, as authorized; PC0 evidence and historical budgets remain unchanged.

| Resource | Consumed / ceiling | Remaining |
| --- | --- | --- |
| Windows producer attempts | 4 / 4 | 0 |
| AP0 diagnostic batches, one campaign | 4 / 8 | 4 |
| Acceptance candidates, one batch each | 1 / 2 | 1, conditional and unused |

Producer 1 was superseded by corrected callback attribution before device use. Diagnostic 1 exposed standard activation callbacks missing from stream admission. Diagnostic 2 exposed incorrect rejection of the SDK's no-op setProcessing return. Diagnostic 3 retained all 96 correct samples and clean shutdown, but failed the host's inherited total-release-count check. Producer 4 scoped that check to component termination; diagnostic 4 and the separate fresh acceptance then passed. Each repair had a focused local regression. Both exact artifact/plan revisions used the existing locked atomic budget path, retaining all consumed reservations and before/after records; no campaign/count was reset.

Focused local suites passed 82 tests, including actual worker/adapter/retention flow, native release attribution, numerical corruption, shutdown ordering, policy, budget preservation and lost acknowledgement without duplicate execution. Local tests consumed no live reservations. Raw diagnostics/checkpoints and custody records remain in the private proof stores; no diagnostic was promoted or copied into this acceptance packet.

PC0 remains the accepted frontier until AP0 review and merge. This verifies bounded offline numerical correctness only: no DAW, proxy/IPC, audio device, real-time performance, editor, commercial plug-in, state/preset or arbitrary-memory-safety claim.
