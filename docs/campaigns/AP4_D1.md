# AP4-D1 — Lead-issued diagnostic execution authority

Issued by the technical lead in response to the operator's explicit request to resolve the AP4 approval block and continue the existing work. This is an actual scoped execution authorization, not an implementation agent claiming permission for itself. It does not override Codex tool approvals, sandbox restrictions or administrator policy.

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
authorized_source_commit: da3e0f3c1b0afbaa405abdf97451614969480531
authorized_source_tree: e5a130579f2513e1d2f90d8acfaad657ffd40c5d
authorized_plan_id: ap4-state-recall-diagnostic-v1
authorized_product_contract_identity: ap4-plugin-state-recall-v1
authorized_product_contract_sha256: f8d45e3ae48d573e520a9b03fb1b81de08378feb1c84a25f17b12ac9ddd8d89f
authorized_plan_content_sha256: 2d25e6cf1ca6c43b3342315ca8045756ad0cdf8222f14733bd1c41d88a689f75
diagnostic_campaign_identity: db719088048bf8da92c0060a4bb852a82945e515a345a369b1c2ab06ece686f5
diagnostic_batch_budget: 10
accepted_product_frontier: AP3
current_product_target: AP4
```

## Exact scope and continuation

Consume this receipt with the existing `proof-run.py diagnose` command and its `--preflight-only` option first. Execute source `da3e0f3c1b0afbaa405abdf97451614969480531`, not this later authority commit. A clean source worktree may read this file from a separate authority checkout. The built source and CURRENT_SLICE.md are unchanged by issuing this receipt.

The selected adapter is the source's existing `ap4-state-three-sdk` diagnostic: capture and fresh-session state restoration through the SDK-loaded proxy and actual Windows AGain. It is not the full Bitwig recall result. Use the bound Windows artifact `9978188887`, Windows build input `cf19a6afa65d0a51360b9e48393664741c46741e09ade2b6c3d3282ae87d8ebe`, and native manifest `24f40e2509a464188ed53b89d86c5a44c6c879a8cb425ffe385c0935e12c110c`. The source's descriptor binds the retained AGain fixture and explicit Proton 11.0-2c / Runtime 4 inputs. The ordinary source/artifact/runtime/cleanup checks still apply.

At issuance the operator reports 2/6 Windows producer attempts, 0/10 diagnostic reservations and 0/2 acceptance candidates consumed. These are not replacement ledgers; reconcile actual counters before launch. Do not create a second campaign if an AP4 reservation already exists. The previously selected task allowance is unchanged. Missing permission-file writes consumed no plug-in reservation.

Retain useful failures and complete owned cleanup. A successful diagnostic remains permanently acceptance-ineligible. The companion AP4_A1.md authorizes the separately measured full acceptance candidate at this exact source after the diagnostic and ordinary readiness checks pass; do not promote this observation.

The agent may continue ordinary local implementation/testing under the existing work order. This exact receipt does not authorize a mismatched source or plan. If a repair changes a binding, obtain the operator's normal explicit approval for the exact replacement before live execution; retain the same campaign, counts and historical receipts. A tool denial must use the normal user-approval route, not another tool, renamed permission file, relaxed policy or disabled safeguard. This is an execution-permission issue, not a reason to restart product design.
