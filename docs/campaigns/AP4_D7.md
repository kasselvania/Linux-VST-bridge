# AP4-D7 — Approved native failure-retention repair

The operator explicitly approved source `8e9b01a725dcea83fbbc8dfbff47850bb2acd98a` for the next full diagnostic, followed by the remaining acceptance candidate only if the full diagnostic passes unchanged. This tested repair checkpoints native observations and bounded sanitized error excerpts before cleanup, keeps the original failure separate from reporting/cleanup failures, and preserves the existing post-cleanup GUI-note wait. The plug-in calls, both binaries, plan and declared deadlines are unchanged.

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
authorized_source_commit: 8e9b01a725dcea83fbbc8dfbff47850bb2acd98a
authorized_source_tree: 8d1683d06374ebfe3654bb737423dac08e3ad465
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

Run the normal `proof-run.py diagnose` path from the clean exact-source checkout, reading this receipt from the newer authority checkout. Perform read-only preflight first. Complete all three SDK and all three Bitwig stages, with actual Windows AGain, normal Save/Quit and fresh-process gain/mute recall observed before editing. Follow AP4_D5's GUI sequence and inspect extended logs only after Quit.

Reuse Windows artifact `9978188887` and native manifest `0add784f5f8c996768909792243f7a18ea8d9e08bf148db166fb57d9455fb017`; their relevant inputs are unchanged. Reuse the retained AGain, Proton and Runtime 4 fixture.

Six of ten diagnostics, one of two acceptance candidates and two of six Windows producers are consumed. Preserve every historical reservation, failure, delegation and consumed count. D6 completed three SDK sessions with 3,151,872 zero-error comparisons, then Bitwig's engine crashed before its endpoint opened (native caller exit 134). Its underlying engine cause remains unknown; cleanup was complete and protected state unchanged. It remains failed and acceptance-ineligible.

Only a complete fresh pass at this exact source permits the updated, still-unreserved AP4_A2 candidate. Restore original preferences and remove only the exact temporary publication after each batch; preserve the private project unchanged between saves and reopens. No new campaign, budget reset, historical replay, or product-evidence promotion. All ownership and tool approvals remain applicable. A changed executable binding requires exact approval before further live work.
