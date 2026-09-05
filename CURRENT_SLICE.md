# Current Work: AP0 — Offline AGain Sample Processing

**Active and authorized for implementation.** Basis: `82bc56fb16ddb999430f6cab6cd0a5c3598da91c`. Branch: `codex/ap0-offline-again-processing`. PC0 is accepted through PR #47; preserve its code/evidence and completed budgets. The active ruling in AGENTS.md authorizes this whole task before merge, including the new processing calls and changed Windows host build.

## Goal and reason

The product ultimately lets a native Linux DAW use a Windows plug-in. The next unresolved capability is actual Windows-side sample processing under the Linux runtime, not more metadata discovery. Prove that first, without conflating it with the later Linux proxy/IPC/audio-device integration.

AP0's claim: the supervised Windows host on the existing Steam Deck/Runtime 4/Proton 11 fixture configures the retained AGain processor, processes deterministic **32-bit stereo** buffers offline at a documented sample rate and block size, obtains numerically correct results and shuts down cleanly.

## Required proof

Use distinct, bounded left/right input patterns containing positive, negative and zero samples. In one processing session use at least two blocks with different non-unity gains, **0.5 and 0.25**, supplied at block boundaries through the SDK's normal gain-parameter input. Add one silent block. Keep bypass off and send no musical events. This is a fixture-specific parameter queue, not a general automation/controller subsystem.

Read the actual output buffers. A small independent checker must calculate expected samples from the input recipe and requested gain, not from the plug-in implementation, a reported PASS flag or output-derived gain. Compare every output sample, record maximum error and reject nonfinite, missing, stale, unchanged or channel-swapped data. Prefer exactly representable binary-fraction inputs so multiplication by these gains has no tolerance ambiguity. Document the comparison rule before the device run; do not loosen it to fit a failure. Out-of-place buffers with sentinels/guards should expose unwritten samples, input modification and boundary overwrite.

Validate the actual processing lifecycle, return values, thread ownership and teardown. Stop processing and join any processing thread before deactivation, interface/component release or module unload. Preserve timeouts, owned-process cleanup, stage retirement and protected-state checks. Keep evidence bounded and useful: configuration, requested gains, actual sample results/comparison, lifecycle and cleanup. This proves offline correctness only, not real-time performance or arbitrary memory safety.

## Implementation freedom and known seams

Choose the smallest extension of the existing C++ SDK host and shared Python runner. Keep buffers allocated before processing; collect result logging outside the processing calls. Normal setup, bus activation, setActive/setProcessing, process, teardown and supporting SDK interfaces are in scope. Optional latency/tail/context queries may be used where required by the chosen SDK-correct flow, not as a separate milestone. Follow the pinned SDK's thread/lifecycle rules rather than copying PC0's Initialized-only call counts.

Two concrete integration details must not cause another approval stop:

- PC0's reused Windows artifact was produced at `7ac6095a488d0077fcc78fedc5abd870b5ffb1cb`, with retained Deck helpers at `309b8918c128c0b9e6701d0453dc841a111d5ac5`. Current main's C++ tree is not automatically that producer's full PC0 host. Reuse/port the relevant implementation explicitly; do not merge an archived branch or restore its old authority.
- AGain sends text messages during activation/deactivation. Support or correctly handle the applicable standard host callbacks using existing SDK facilities; PC0's exact callback-count expectation is not an AP0 processing contract. No editor or connected controller is needed for this fixture test.

A new Windows host binary is necessary. Use the existing MSVC/CMake producer and artifact-delivery machinery, adapting its stale branch guards, input rosters and narrow AP0 registration as necessary. Do not point at the old no-processing artifact and expect it to process. Reuse the unchanged AGain binary, SDK pin and deployed runtime. No fixture rebuild/reseed, runtime replacement, Bitwig/Serum execution, native proxy, shared-memory audio transport, live speakers, UI, state/presets, 64-bit processing or broad harness refactor.

## Execute and finish

Focused local tests should catch numerical/ordering/cleanup and runner failures before device use. Reuse existing regression suites without replaying all prior live proofs. Necessary implementation, build, artifact delivery, troubleshooting and exact source/plan/candidate binding belong to this agent; no intermediate design or permission handoff.

Use new AP0 scoped receipts with real computed identities through the existing classified command. The default mapping below deliberately grants no unspecified workload; it does not block this task. Thin plan/adapter extensions are included, not another execution framework.

Task ceilings: up to **four Windows producer attempts**, **eight diagnostic batches in one AP0 campaign**, and **two acceptance candidates maximum, one fresh batch each**. Reuse an unchanged build/artifact; do not repeat successful work to consume an allowance. The second acceptance candidate is permitted only after a specific failed/inconclusive first attempt has a tested correction and current cleanup is established. Preserve the first record and all cumulative counts. A lost acknowledgement is reconciled, not relaunched; a reporting-only failure reuses its retained result. No budget can be reset or silently increased. Track task totals in the existing records and a concise result note; do not build a new budgeting service.

Fix routine errors and continue within those limits. Stop live execution for unknown process containment, changed protected state, a genuinely different product/runtime boundary or exhausted allowance; local diagnosis remains permitted. Open one non-draft implementation PR, leave it unmerged and report actual numerical results first. No result means no AP0 acceptance; PC0 remains the accepted frontier pending review.

## Source basis

Repository SDK pin: `cmake/WF0DependencyLock.cmake`; public.sdk commit `586dc5e6c8012c3e4b01c79389375cbe96bdb1da`.

- [AGain parameter handling, activation and processing](https://github.com/steinbergmedia/vst3_public_sdk/blob/586dc5e6c8012c3e4b01c79389375cbe96bdb1da/samples/vst/again/source/again.cpp)
- [AGain sample multiplication](https://github.com/steinbergmedia/vst3_public_sdk/blob/586dc5e6c8012c3e4b01c79389375cbe96bdb1da/samples/vst/again/source/againprocess.h)
- [Pinned processing interfaces and thread/state requirements](https://github.com/steinbergmedia/vst3_pluginterfaces/blob/4f547e8e102b47de4a8b8aaf343c73b700786372/vst/ivstaudioprocessor.h)

## Authority

This compatibility mapping is only the disabled **default CLI guard**. Active AP0 implementation is authorized above and in AGENTS.md; explicitly bound AP0 diagnostic/candidate receipts authorize its live commands. Existing guard tests need not become another workstream.

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
current_product_target: AP0
pc0_status: accepted
diagnostic_campaign_authorized: false
acceptance_candidate_authorized: false
successor_selection_authorized: false
```
