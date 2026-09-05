# Current Work: AP1 Accepted

**AP1 is accepted.** PR #51 merged as `af4f988420cefe3c4cc20c01c3d1b0d29d11efaa` after technical-lead review `5121807388` of exact head `32379eec7ea8964d4f3ab6f1e918630c02637e34`. AP0 and PC0 remain accepted prerequisites. No implementation successor is active.

## Accepted capability

A native Linux Rust caller supplies samples and gain requests through one shared mapping and authenticated loopback control connection. The supervised Windows host processes them through one retained AGain instance. Linux reads actual mapped output and checks it against independently determined expectations.

The second acceptance on the Steam Deck / Steam Linux Runtime 4 / explicitly bound Proton 11.0-2c composition verified **1,502/1,502 float32 samples with maximum absolute error 0.0** across ten blocks. The original eight changing-input/gain/length cases remain, including silence. Zero gain on non-silent input returned zero samples and actual output mask 3; correctly formed left-channel silence returned exact samples and mask 0; the all-silent case returned mask 3. Output flags are validated independently of input flags, including all 64 SDK bits and the samples of any channel claimed silent. Protocol 1.1 retains those actual flags in Done.

One host, instance, mapping and connection served the session. The reported execution stopped and joined processing before deactivation/release/unload, retired both mappings, exited both endpoints cleanly, removed owned processes and staging, and left protected state unchanged. Local peer/fault tests cover malformed/stale responses, disconnects, deadlines, invalid output claims and failure retention without replay. These are not live hostile-plug-in tests.

This is bounded offline numerical and transport correctness on the recorded fixture. It is not a DAW-ready or real-time bridge, a portable shared-memory proof, general plug-in compatibility, an editor/state implementation or a sandbox-security certification.

## Evidence and runtime history

- [Current contract](docs/slices/AP1/CONTRACT.md), [usage](docs/slices/AP1/USAGE.md), and [second acceptance result](docs/slices/AP1/RESULT_A2.md).
- [Second acceptance packet](evidence/ap1-linux-windows-audio-roundtrip-a2/FINDINGS.md).
- [Preserved first result](docs/slices/AP1/RESULT.md) and [first packet](evidence/ap1-linux-windows-audio-roundtrip/FINDINGS.md).
- Executed source: `a2e12c52df0db34c6476c1b337065e1f53e92638`.
- Second acceptance result: `c018835588a05a3e8fb7092ef95f262338b003060897b7f0d7a1c6f470f0a628`.
- Proton: `1788504981 proton-11.0-2c-x86_64`, build `25118279`, depot `2978628887517351791`; runtime contract `20064247dc297d0228bbf63d5a49050238c518d14fbb77b61a8c73351db5ebf7`.

The runtime transition was explicitly authorized and bound for AP1; historical WR0/PC0/AP0 and the first AP1 runtime records remain unchanged. The reported new selection matched preflight, launch and post-run. The first 1,344-sample result remains valid for its original code/runtime/vectors; it does not prove the later silence repair. Because code and Proton both changed, the second result is not an isolated comparison of Proton versions.

Review inspected the repaired C++ result handler, Rust protocol/checker, independent Python checker, runtime verifier/binding, source comparison and retained evidence/report. Final CI run `33973617513`, job `101326364729`, passed compilation, 95 Python/integration tests (91 focused tests plus four native mapped-peer tests), eight Rust tests and C++ protocol/result-handler checks on merge candidate `bd968f01c35d0319940df96c31bc80929357de9f`. Windows producer `33971421519` also succeeded. The lead did not independently rerun the Deck workload or recompute every sample from private raw stores during review.

The evidence packets retain their publication-time pending-review status. This record and the merge review establish subsequent acceptance; do not rewrite immutable observations merely to update status.

## Completed task

Cumulative usage remains **2/6 Windows producers, 2/10 diagnostics, 2/2 acceptance candidates**. Both diagnostics remain acceptance-ineligible. Both acceptance candidates retain their original source, authority and result. No budget, reservation or unknown outcome is reset. AP1 is complete: no further build, diagnostic or acceptance workload is authorized by its former work order or this closure. Read-only inspection and supported retained-result reporting remain available without relaunch.

## Next product direction

Move this working audio crossing toward the native Linux VST3 proxy and a real DAW consumer, rather than another fixed-buffer fixture. The next work order should close the necessary session ownership and callback behavior against those actual consumers. AP1's offline socket waits, allocation and logging must not simply be placed on a real-time DAW callback. Reuse the working Windows processing and mapped-data implementation; no successor execution is authorized by this review.

## Authority — default command guard

The default guard remains closed for unspecified work. Historical scoped receipts do not renew completed task permission or permit duplicate execution.

```yaml
status: no_active_slice
authority_phase: no_active_slice
change_class: PROOF_HARNESS_MAINTENANCE
maintenance_id: AP1
maintenance_status: complete
repository: kasselvania/Linux-VST-bridge
maintenance_implementation_authorized: false
product_implementation_authorized: false
live_execution_authorized: false
permitted_execution_class: none
classified_backend_core_ready: true
classified_backend_ready: true
production_adapter_registry: pc0_ap0_ap1_diagnostic_and_acceptance
accepted_product_frontier: AP1
current_product_target: none
pc0_status: accepted
ap0_status: accepted
ap1_status: accepted
diagnostic_campaign_authorized: false
acceptance_candidate_authorized: false
successor_selection_authorized: false
```
