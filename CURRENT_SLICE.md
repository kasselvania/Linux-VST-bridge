# Current Work: PC0 Accepted

**PC0 is accepted.** PR #47 merged as `ece01f02e6a6070a6d9f181c718cafc636300a3a` after technical-lead review `5119395068` of exact head `02e3e4d812546788a15df61844729a4d0b1a6d12`. The accepted product frontier is now PC0, not WA0.

## Accepted capability

On the exact retained AGain/Runtime 4/Proton 11 fixture, the supervised Windows host initializes the component, acquires IAudioProcessor, queries the pre-setup contract and shuts down cleanly:

- audio buses: one input and one output, each two-channel stereo (`0x3`);
- event buses: one one-channel input, no output;
- Stereo In, Stereo Out and Event In are main/default-active buses;
- kSample32 and kSample64 each return zero (supported);
- complete 33-call paired ledger, including eleven pre-setup calls and no processing-state mutation;
- exit zero, complete interface/component/factory/module shutdown, no owned processes, stage absent and protected state unchanged.

Default-active is a reported flag, not proof of activation. Sample-format support is not processed audio. Audio processing, bridge IPC, a Windows plug-in inside Bitwig, commercial plug-ins and real-time performance remain unproven.

## Review and retained result

Executed source: `c4eda373caf0fc9d4c965e05f1e16706c06ddd3d`.
Acceptance reservation: `4534d5b772cac1241cb822471566b71b668256de941a50671973d771517af214`.
Immutable result SHA-256: `a920ef5ba3c712d7d19dbfd1948c85130d5b45d73c9985ae12c63b07fe5d7ae2`.
Evidence: `evidence/pc0-windows-vst3-pre-setup-processing-contract/`.

Review covered the class-separated shared runner, source/runtime bindings, strict contract and shutdown validation, error retention and local renderer recovery. GitHub Actions run `33934425860`, job `101219455861`, compiled the code and passed all 72 tests on the reviewed PR merge candidate. The lead did not independently run the Deck workload or access the private Mac receipt. GitHub refused a self-approval through the author's account; the technical verdict is recorded as a review comment, followed by the pinned-head merge.

The acceptance packet's pending-review wording is its historical publication state. Do not rewrite or rerender that immutable packet to update project status; this file and the merge review record the subsequent acceptance.

## Completed work and budgets

PC0-A1 used its one acceptance reservation. PC0-D1 remains at three consumed out of eight; earlier diagnostic results remain acceptance-ineligible and the unknown result remains unknown. No budget is reset, transferred or renewed by this merge. Do not rerun either completed task for status closure.

The PC0 completion directions in AGENTS.md and the candidate receipt describe completed work. There is no remaining adapter or evidence task to prepare before declaring PC0 accepted.

## Next product goal

The next useful milestone is a small offline known-buffer processing test: configure AGain, process deterministic stereo samples and compare the returned samples with an independently calculated expected result, followed by clean shutdown. That is a new processing task, not another PC0 census. Prefer one concrete host experiment over a general audio/IPC framework. Preserve the working runner and build/fixture reuse; a new host binary is justified only by actual Windows host-source changes.

This review closes PC0; it does not silently reuse its exhausted reservation for the new processing calls. A subsequent scoped work order must authorize that distinct operation. No new workload was run during this review.

## Authority

This mapping keeps the default CLI from launching an unspecified workload. Historical explicit candidate receipts remain available for read-only verification and permitted retained-result rendering, not duplicate execution.

```yaml
status: no_active_slice
authority_phase: no_active_slice
change_class: PROOF_HARNESS_MAINTENANCE
maintenance_id: PC0-A1
maintenance_status: complete
repository: kasselvania/Linux-VST-bridge
maintenance_implementation_authorized: false
product_implementation_authorized: false
live_execution_authorized: false
permitted_execution_class: none
classified_backend_core_ready: true
classified_backend_ready: true
production_adapter_registry: pc0_diagnostic_and_acceptance
accepted_product_frontier: PC0
current_product_target: none
pc0_status: accepted
diagnostic_campaign_authorized: false
acceptance_candidate_authorized: false
successor_selection_authorized: false
```
