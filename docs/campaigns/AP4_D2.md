# AP4-D2 — Operator-approved replacement diagnostic binding

The operator explicitly approved tested source `e4143c4057f0d4b68b56e6d5ec2f2ea193a16dac` and plan digest `82764316e38d95d7f55cbefe1d92bfab4d984501980c78fcdc7c1b25d9f81e21` after the first acceptance attempt failed before saving the Bitwig project. This records that approval, as required by AP4_D1.md; it does not revise D1 or A1 or authorize mismatched executable bytes.

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
diagnostic_previous_plan_content_sha256: 2d25e6cf1ca6c43b3342315ca8045756ad0cdf8222f14733bd1c41d88a689f75
diagnostic_previous_product_contract_sha256: f8d45e3ae48d573e520a9b03fb1b81de08378feb1c84a25f17b12ac9ddd8d89f
authorized_source_commit: e4143c4057f0d4b68b56e6d5ec2f2ea193a16dac
authorized_source_tree: 9f010cb52a283f39e0a3aba8faacf267dbc92de1
authorized_product_contract_identity: ap4-plugin-state-recall-v1
authorized_product_contract_sha256: f8d45e3ae48d573e520a9b03fb1b81de08378feb1c84a25f17b12ac9ddd8d89f
authorized_plan_content_sha256: 82764316e38d95d7f55cbefe1d92bfab4d984501980c78fcdc7c1b25d9f81e21
accepted_product_frontier: AP3
current_product_target: AP4
```

## Execution

Run the normal `proof-run.py diagnose` command from a clean checkout of the exact source, reading this newer authority separately. Check `--preflight-only` first. The operation is `ap4-state-three-sdk-and-three-bitwig`: actual Windows AGain state capture, fresh SDK restoration, and disposable Bitwig gain/mute project recall. Native manifest `0add784f5f8c996768909792243f7a18ea8d9e08bf148db166fb57d9455fb017` contains the tested first-fault retention repair. Reuse Windows artifact `9978188887` and unchanged Windows input identity `cf19a6afa65d0a51360b9e48393664741c46741e09ade2b6c3d3282ae87d8ebe`; preserve the descriptor's exact AGain/runtime identities.

Use the existing locked atomic diagnostic-plan adjustment path. Preserve the one consumed diagnostic reservation and all historical delegations/results; the campaign ceiling remains ten, not ten additional. At approval, two of six Windows producers and one of two acceptance candidates are consumed. Reconcile the actual ledger before launch. No new campaign, count reset, receipt rewrite or replay of A1 is authorized.

Keep the work order's eight serial Windows-instance maximum, ten-minute active-audio ceiling, existing timeouts, original-failure retention, owned cleanup and protected-state checks. Preserve the saved disposable project; restore preferences and temporary publication afterward. This diagnostic is permanently acceptance-ineligible. Only after the full diagnostic and readiness checks pass unchanged may AP4_A2.md execute. A further source/plan repair requires exact replacement approval under AP4_D1.md. Tool and sandbox approvals remain applicable.
