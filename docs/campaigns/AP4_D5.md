# AP4-D5 — Lead-approved GUI-window repair and full diagnostic

The technical lead approves execution source `871e074e9ec3d3b6652bbcb7278b5812fdc6d12d` under the existing AP4 task. The reviewed repair gives interactive Bitwig stages a declared 180-second post-gate window; automated stages retain 120 seconds. Plug-in-call, readiness and owned-cleanup deadlines remain separate. This is permission to verify, not product acceptance or an override of tool/sandbox approvals.

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
authorized_source_commit: 871e074e9ec3d3b6652bbcb7278b5812fdc6d12d
authorized_source_tree: 10cfdea375dcbe203657831176ce36de90062fa3
authorized_plan_id: ap4-state-recall-diagnostic-v1
authorized_product_contract_identity: ap4-plugin-state-recall-v1
authorized_product_contract_sha256: f8d45e3ae48d573e520a9b03fb1b81de08378feb1c84a25f17b12ac9ddd8d89f
authorized_plan_content_sha256: 1917c0475dd109e4a3430fee1321ee7d676c57f1b33a0c7c2c42a13ad9c38068
diagnostic_campaign_identity: db719088048bf8da92c0060a4bb852a82945e515a345a369b1c2ab06ece686f5
diagnostic_batch_budget: 10
diagnostic_plan_revision_authorized: true
diagnostic_previous_plan_content_sha256: 82764316e38d95d7f55cbefe1d92bfab4d984501980c78fcdc7c1b25d9f81e21
diagnostic_previous_product_contract_sha256: f8d45e3ae48d573e520a9b03fb1b81de08378feb1c84a25f17b12ac9ddd8d89f
accepted_product_frontier: AP3
current_product_target: AP4
```

## Continue the existing task

Use the existing `proof-run.py diagnose` path, with read-only preflight first, from a clean checkout of the exact execution source above. Read this receipt from the newer authority checkout. The full registered operation is `ap4-state-three-sdk-and-three-bitwig`; `FULL_DIAGNOSTIC=True`. Its descriptor now includes `ap4_gui_post_gate_seconds: 180`. This is the same campaign and product contract, not a reset or a new workload family.

Reuse Windows artifact `9978188887`, Windows build input `cf19a6afa65d0a51360b9e48393664741c46741e09ade2b6c3d3282ae87d8ebe`, and native manifest `0add784f5f8c996768909792243f7a18ea8d9e08bf148db166fb57d9455fb017`, with the existing exact AGain and Proton 11.0-2c / Runtime 4 bindings. The reviewed timing patch changes orchestration and tests, not the native or Windows build inputs. Verify normal input equality; do not rebuild unchanged binaries for receipt or timing changes.

Reported consumed counts at issuance are 2/6 Windows producers, 4/10 diagnostics and 1/2 acceptance candidates. Reconcile the actual ledger before launch. Preserve all four diagnostic reservations, the first acceptance outcome, original errors and every earlier receipt. A different ledger does not authorize silently selecting another campaign or changing consumed counts. AP4_D1 through AP4_D4 remain historical; this receipt provides the exact continuation they require.

Have the approved Moonlight control session usable before the GUI stage. Follow the existing required sequence: make a genuine nondefault change for the initial capture, save, Quit normally, then verify fresh-process gain recall before editing; save mute, Quit and verify the next unchanged-project reopen. Carry the same saved project between those sessions. Prepare the GUI action sequence before launching, and leave verbose inspection/reporting until after Quit; do not consume the interactive window on unrelated work. Do not replace normal Quit with a forced exit or omit any required state/audio/cleanup check.

A complete diagnostic pass at this exact source, including all three SDK and all three Bitwig stages and final cleanup, permits the still-unreserved candidate in the updated `AP4_A2.md` without another lead handoff. SDK-only success does not satisfy this condition. The diagnostic stays acceptance-ineligible. The same two-candidate task ceiling and existing bounds remain in force.

If the diagnostic fails, preserve its useful result and cleanup; do not spend A2 on an incomplete rehearsal or blindly repeat the failure. Ordinary local repair remains permitted, but changed executable bindings require proper exact approval before new live work. Tool or sandbox refusals use the normal specific approval route, never a renamed action or disabled safeguard. No extra design package or framework is requested.
