# AP4-D6 — Approved exact float32 readback repair

The operator approved source `39af4e697dc66d11c313a36c145cf2d648b97502` for one fresh full diagnostic and the remaining acceptance candidate only if it passes unchanged. The repair compares the native logger's nine-digit decimal gain as exact float32 bits with the real Windows state payload; the complete payload digest must still match. It adds no tolerance and rejects adjacent float32 values, nonfinite/range-invalid gains and changed hidden state. The plan, 180-second GUI window and both binaries are unchanged.

## Authority

```yaml
status: active_diagnostic_campaign
authority_phase: proof_harness_maintenance
change_class: PROOF_HARNESS_MAINTENANCE
repository: kasselvania/Linux-VST-bridge
product_implementation_authorized: false
maintenance_implementation_authorized: true
live_execution_authorized: true
permitted_execution_class: DIAGNOSTIC_NON_AUTHORITATIVE
classified_backend_core_ready: true
classified_backend_ready: true
acceptance_eligible: false
authorized_source_commit: 39af4e697dc66d11c313a36c145cf2d648b97502
authorized_source_tree: 312682da3d5c9cdfde473970636601ef82269eee
authorized_plan_id: ap4-state-recall-diagnostic-v1
authorized_product_contract_identity: ap4-plugin-state-recall-v1
authorized_product_contract_sha256: f8d45e3ae48d573e520a9b03fb1b81de08378feb1c84a25f17b12ac9ddd8d89f
authorized_plan_content_sha256: 1917c0475dd109e4a3430fee1321ee7d676c57f1b33a0c7c2c42a13ad9c38068
diagnostic_campaign_identity: db719088048bf8da92c0060a4bb852a82945e515a345a369b1c2ab06ece686f5
diagnostic_batch_budget: 10
diagnostic_plan_revision_authorized: true
diagnostic_previous_plan_content_sha256: 1917c0475dd109e4a3430fee1321ee7d676c57f1b33a0c7c2c42a13ad9c38068
diagnostic_previous_product_contract_sha256: f8d45e3ae48d573e520a9b03fb1b81de08378feb1c84a25f17b12ac9ddd8d89f
accepted_product_frontier: AP3
current_product_target: AP4
```

## Execution and preservation

Run the existing `proof-run.py diagnose` path with read-only preflight first, from the clean exact-source checkout and this newer authority file. Complete all three SDK and three Bitwig sessions under `ap4-state-three-sdk-and-three-bitwig`. Follow AP4_D5's GUI preparation, real initial parameter change, normal Save/Quit, fresh-process gain recall before edits, mute Save/Quit, fresh mute recall and later deliberate edit without overwriting the saved mute project. Inspect extended logs after Quit.

Reuse Windows artifact `9978188887`, input `cf19a6afa65d0a51360b9e48393664741c46741e09ade2b6c3d3282ae87d8ebe`, native manifest `0add784f5f8c996768909792243f7a18ea8d9e08bf148db166fb57d9455fb017`, and the existing exact AGain/Proton/Runtime 4 inputs. Verify ordinary input equality; no unchanged binary rebuild.

Five of ten diagnostic reservations, one of two acceptance candidates and two of six Windows producers are consumed. Preserve all earlier receipts, counts and outcomes. D5 retained all six normal exits, matching saved/restored state and project hashes and 14,982,656 zero-error comparisons, but failed its decimal-to-float32 validator. That diagnostic remains failed and acceptance-ineligible; no relabelling or replay.

Only a complete fresh diagnostic pass at this exact source, including actual Windows/Bitwig recall, normal cleanup and unchanged protected state, permits the updated still-unreserved AP4_A2 candidate. Restore original preferences and remove only the exact temporary publication after each batch; preserve the private project. No new campaign or allowance. All earlier ownership, call/readiness/cleanup bounds and tool approvals remain applicable. A changed executable binding requires exact approval before further live work.
