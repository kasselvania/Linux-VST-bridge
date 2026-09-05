# Current Work: AP2 Accepted

**AP2 is accepted.** PR #53 merged as `7859202c2ddc37eb40611b270ae3fab8405a5d6d` after technical-lead review `5122151279` of exact head `59a420eaf4fb9e02f14f05ab61ba2a4b4dd9b2ad`. AP1, AP0 and PC0 remain accepted prerequisites. No implementation successor is active.

## Accepted capability

A native Linux SDK host loads `AGainOfflineBridge.vst3` through the official module/factory/component/audio-processor interfaces and receives actual Windows AGain output in its own buffers. The native C++ VST3 shell calls the reusable Rust session/transport backend through a versioned C ABI. External supervision retains process/environment ownership. The test host neither calls private backend exports nor accesses mapping offsets; the proxy does not substitute local gain DSP.

Fresh Deck acceptance on the retained Steam Linux Runtime 4 / Proton 11.0-2c composition verified **1,730/1,730 host-returned float32 samples with maximum absolute error 0.0**: 1,692 in 13 persistent audio blocks, plus 38 after fresh module/session reopen. Cases include changing gains and lengths, full and one-channel silence, zero gain on non-silent input, empty queues retaining gain, a zero-frame/zero-bus parameter update applied to the next audio block, same-channel in-place buffers, and default gain 1.0 after reopen. Output silence flags remain independent of input flags. Malformed/unsupported requests are rejected without advancing processing or changing retained gain.

The reported Windows processing threads stopped and joined before owner-thread deactivation/release/unload. Both native module unloads and endpoint mapping retirements completed. Owned process groups were empty, staging was removed, and protected state was unchanged. Separate local SDK-loaded proxy tests cover discovery without a backend, unavailable backend, zero-call start/stop, transport and output faults, failure latching and no duplicate dispatch. Those negative tests use substituted peers, not live hostile plug-ins.

This is an **offline, processor-only reference-fixture implementation**: float32 stereo, 48 kHz, 1–256 frames and at most 64 nonempty processing calls per activation. Unsupported realtime/prefetch and 64-bit processing are refused. It is not Bitwig publication or playback, a full VST3 validator result, general plug-in compatibility, state/editor/controller support, real-time performance or portable memory-safety certification.

## Evidence and review

- [Contract and ABI/lifecycle limits](docs/slices/AP2/CONTRACT.md).
- [Fresh result and cumulative costs](docs/slices/AP2/RESULT.md).
- [Local substituted-peer tests](docs/slices/AP2/LOCAL_TESTS.json).
- [Acceptance packet](evidence/ap2-native-vst3-offline-bridge/FINDINGS.md).
- Executed source: `6c3ccd5888c17f2d236f433d45fe0ee0b8dc70fb`, tree `35a73d7d15b47baa77448b266d16be61af3b3b0c`.
- Acceptance reservation: `d12c4783e0f6906e0625c9c20e57213385d0f7c6ee671ec4843c2322bee4f704`; result SHA-256: `67631a8816aae2936a6c6714890148a4aaf570bf6346140866be14ce537fe60f`.
- Windows producer: `33978589618`, artifact `9973099999`; the native build and exact input equivalence are recorded in the packet.
- Runtime: explicitly bound Proton 11.0-2c, build `25118279`, runtime identity `20064247dc297d0228bbf63d5a49050238c518d14fbb77b61a8c73351db5ebf7`.

Review inspected the actual proxy/SDK host, Rust C ABI and endpoint, Windows lifecycle integration, independent checker, two-session supervision, source bindings, published transaction and CI. The comparison from executed source to reviewed head changes only authority/result/evidence files. Final CI run `33979329796`, job `101341579562`, passed compilation, 101 Python/integration tests, eight native-client Rust tests, two AP2 backend tests and C++ protocol/result checks on merge candidate `bc1b159103c1999d04e0c0a8450ca6811f86d707`. The SDK-loaded proxy fault tests are separately retained local results, not claimed as part of that CI job. The lead did not rerun the Deck workload or independently recompute every sample from private stores.

The acceptance packet retains its publication-time pending-review status; this record and the merge review establish subsequent acceptance. Do not rewrite immutable observations merely to update status. All prior accepted evidence and runtime records remain intact.

## Completed task

Cumulative AP2 usage remains **2/6 Windows producer attempts, 1/10 diagnostic batches, 1/2 acceptance candidates**. The first producer's failure remains recorded. The diagnostic is permanently acceptance-ineligible. No budget or result has been reset, replayed or promoted. AP2 is complete: no further build, diagnostic or acceptance workload is authorized by its former work order or this closure. Read-only inspection and supported retained-result reporting remain available without relaunch.

## Next product direction

The bridge now has a host-loaded offline VST3 path to actual Windows processing. The next selection should move this same proxy toward real-time-safe sustained processing and a controlled real DAW consumer, rather than another fixed-vector client. Close callback deadlines, failure output, session lifetime and the minimal host-facing requirements together. Do not simply enable realtime on the existing blocking, allocating offline path, or begin a general transport/broker rewrite. No successor is activated by this review.

## Authority — default command guard

The default guard remains closed for unspecified work. Historical scoped receipts do not renew completed task permission or permit duplicate execution.

```yaml
status: no_active_slice
authority_phase: no_active_slice
change_class: PROOF_HARNESS_MAINTENANCE
maintenance_id: AP2
maintenance_status: complete
repository: kasselvania/Linux-VST-bridge
maintenance_implementation_authorized: false
product_implementation_authorized: false
live_execution_authorized: false
permitted_execution_class: none
classified_backend_core_ready: true
classified_backend_ready: true
production_adapter_registry: pc0_ap0_ap1_ap2_diagnostic_and_acceptance
accepted_product_frontier: AP2
current_product_target: none
pc0_status: accepted
ap0_status: accepted
ap1_status: accepted
ap2_status: accepted
diagnostic_campaign_authorized: false
acceptance_candidate_authorized: false
successor_selection_authorized: false
```
