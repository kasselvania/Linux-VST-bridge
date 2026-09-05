# Current Work: AP0 Accepted

**AP0 is accepted.** PR #49 merged as `af5aefbfc70e2d8d53c3355f8822ac1ad8683146` after technical-lead review `5119728219` of exact head `3d9e11495dae89d4129991b15772cbbfb1acb543`. The accepted product frontier is AP0; PC0 remains an accepted prerequisite.

## Accepted capability

The supervised Windows host processes the retained AGain plug-in under the tested Steam Deck / Runtime 4 / Proton 11 composition. Three offline float32 stereo blocks, each 16 frames at 48000 Hz, use gains 0.5 and 0.25 plus silence. All 96 scalar output samples match the independent recipe exactly, with maximum absolute error 0.0. Input words and boundary guards remain unchanged, and output silence flags match the test cases.

The processing thread stops and joins before owner-thread deactivation and release. The retained result shows exit zero, completed interface/component/factory/module shutdown, no owned processes, absent staging and unchanged protected state. The pinned SDK's two no-op setProcessing notification returns are retained as kNotImplemented; setup, activation and process calls require success.

This establishes bounded offline numerical correctness on this fixture. It does not establish a Linux proxy/IPC audio path, DAW integration, audio-device output, real-time performance, general automation, 64-bit processing or commercial plug-in compatibility.

## Evidence and review

- [Contract and independent comparison rule](docs/slices/AP0/CONTRACT.md).
- [Results, repairs and cumulative costs](docs/slices/AP0/RESULT.md).
- [Acceptance evidence](evidence/ap0-offline-again-processing/FINDINGS.md).
- Executed source: `d9f4a0dec920b71b50fa07460b288d9e7407313b`.
- Acceptance result SHA-256: `7d4e76d6dae11b3993968b76a7b0ac2c705ab5b8ee48cbbb0bba17d3c7d4649e`.

Review inspected the native processing path, independent checker, published sample words/lifecycle, shared-runner integration, runtime/artifact bindings and CI. GitHub Actions run `33942858668`, job `101243488337`, compiled the reviewed paths and passed all 82 focused tests on the PR merge candidate. The lead did not independently operate the Deck or access the private Mac stores.

The packet and candidate receipt retain their publication-time pending-review state. Do not rewrite or rerender immutable evidence to update status; this current-work record and merge review record subsequent acceptance.

## Completed task and costs

AP0 consumed four of four Windows producer attempts, four of eight diagnostic batches, and one of two permitted acceptance candidates. The second candidate was conditional on a failed/inconclusive first result; it is not needed after success. No further AP0 workload, build or replay is authorized by this closure. Keep all consumed counts, failed observations and private records intact. Earlier PC0/PC0-D1 evidence and budgets remain unchanged.

## Next product direction

The next useful proof is an offline native-Linux-to-Windows round trip: a Linux caller supplies varying sample buffers and gain requests; the Windows host processes them through AGain; the Linux caller receives and independently verifies the returned samples. This would prove the first audio-bearing process boundary rather than another self-contained Windows fixture. Preserve the existing processing owner, correct thread/lifetime rules and truthful failure reporting. It is not yet a claim of a real-time DAW bridge.

That successor has not been activated by this review. Its transport and ownership decisions belong in a focused work order, not a new general-purpose proof framework. No new device work occurred during review.

## Authority

The default CLI guard remains disabled for unspecified work. Historical receipts remain available for read-only verification and supported retained-result reporting, not duplicate execution or renewed budgets.

```yaml
status: no_active_slice
authority_phase: no_active_slice
change_class: PROOF_HARNESS_MAINTENANCE
maintenance_id: AP0
maintenance_status: complete
repository: kasselvania/Linux-VST-bridge
maintenance_implementation_authorized: false
product_implementation_authorized: false
live_execution_authorized: false
permitted_execution_class: none
classified_backend_core_ready: true
classified_backend_ready: true
production_adapter_registry: pc0_and_ap0_diagnostic_and_acceptance
accepted_product_frontier: AP0
current_product_target: none
pc0_status: accepted
ap0_status: accepted
diagnostic_campaign_authorized: false
acceptance_candidate_authorized: false
successor_selection_authorized: false
```
