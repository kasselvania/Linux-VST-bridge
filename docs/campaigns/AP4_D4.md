# AP4-D4 — Approved exact state-capture and recall verification

The operator explicitly approved tested source `6ad538cf8c750c3ed2f196e9fad4b3e1aa952c61` for the next full diagnostic and the remaining acceptance candidate only after an unchanged full pass. The repair distinguishes the two actual capture sessions from the final unchanged-project reopen. Captures still require fresh Windows snapshots; final recall requires actual restoration, matching saved/restored payload bytes, independently checked audio and unchanged project bytes. Native and Windows binaries, product contract and plan digest are unchanged.

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
authorized_plan_id: ap4-state-recall-diagnostic-v1
diagnostic_campaign_identity: db719088048bf8da92c0060a4bb852a82945e515a345a369b1c2ab06ece686f5
diagnostic_batch_budget: 10
diagnostic_plan_revision_authorized: true
diagnostic_previous_plan_content_sha256: 82764316e38d95d7f55cbefe1d92bfab4d984501980c78fcdc7c1b25d9f81e21
diagnostic_previous_product_contract_sha256: f8d45e3ae48d573e520a9b03fb1b81de08378feb1c84a25f17b12ac9ddd8d89f
authorized_source_commit: 6ad538cf8c750c3ed2f196e9fad4b3e1aa952c61
authorized_source_tree: 6590b786d8c32062e477efd0b6bbde64119f7681
authorized_product_contract_identity: ap4-plugin-state-recall-v1
authorized_product_contract_sha256: f8d45e3ae48d573e520a9b03fb1b81de08378feb1c84a25f17b12ac9ddd8d89f
authorized_plan_content_sha256: 82764316e38d95d7f55cbefe1d92bfab4d984501980c78fcdc7c1b25d9f81e21
accepted_product_frontier: AP3
current_product_target: AP4
```

## Execution and preservation

Use the existing `proof-run.py diagnose` route from a clean exact-source checkout, with this newer authority file. Run `--preflight-only` first. The operation remains `ap4-state-three-sdk-and-three-bitwig`, with native manifest `0add784f5f8c996768909792243f7a18ea8d9e08bf148db166fb57d9455fb017`, Windows artifact `9978188887`, unchanged Windows input `cf19a6afa65d0a51360b9e48393664741c46741e09ade2b6c3d3282ae87d8ebe`, and the descriptor's exact retained AGain/runtime inputs. No rebuild is required.

Three of ten diagnostics, two of six Windows producers and one of two acceptance candidates are consumed. Preserve historical receipts, reservations and outcomes. D3 observed real gain recall and zero-error samples but failed its capture requirement because the unchanged state was cached; it is not promoted. Preserve the saved private project. Make an actual manual parameter change before the first new capture, and observe each later restored value/playback before editing. Do not resend saved values through a driver.

Keep the work order's bounds, owned supervision, useful failure retention, stage retirement and protected-state checks. The tested sixty-second post-clean-exit GUI-note collection remains; missing notes fail without erasing native records. Restore original preferences and remove only temporary publication afterward.

Only after the complete diagnostic and ordinary readiness checks pass at this exact source may the still-unreserved AP4_A2.md execute. The total campaign and acceptance allowances remain unchanged. A further source replacement requires normal explicit approval under AP4_D1.md; tool/sandbox approvals remain applicable. This diagnostic remains permanently acceptance-ineligible.
