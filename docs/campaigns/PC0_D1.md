# PC0-D1 — First Classified Diagnostic Campaign

## Authority

```yaml
status: active_diagnostic_campaign
authority_phase: proof_harness_maintenance
change_class: PROOF_HARNESS_MAINTENANCE
campaign_id: PC0-D1
repository: kasselvania/Linux-VST-bridge
product_implementation_authorized: false
maintenance_implementation_authorized: false
live_execution_authorized: true
permitted_execution_class: DIAGNOSTIC_NON_AUTHORITATIVE
classified_backend_core_ready: true
classified_backend_ready: true
acceptance_eligible: false
authorized_source_commit: e247e9f11f39d8ed5a47ed5eea3686dde994d317
authorized_source_tree: 64212ef19e3b0c39fa86c42315b19a5dcee9f0bf
authorized_plan_id: pc0-pre-setup-processing-contract-diagnostic-v1
authorized_product_contract_identity: pc0-selection-v2
authorized_product_contract_sha256: daa042e4184cb5fffdf1ff08d59cc51f7755b4635c85e02597adc2584c4b4c1d
authorized_plan_content_sha256: a5303e12d644fefba2ca4003555ebe30f578e60c0a334e5f0bb96b7498decc92
diagnostic_campaign_identity: be62c45243de799f2474b26d4b20ac3593f16fc0392f58f42a10ce922500281b
diagnostic_batch_budget: 2
diagnostic_campaign_authorized: true
acceptance_candidate_authorized: false
windows_workload_authorized: false
deck_workload_authorized: true
accepted_product_frontier: WA0
current_product_target: PC0
```

## Scope and next action

The operator approved this bounded campaign after PX3 review 5118022473 and merge.
Run the first diagnostic to obtain an actionable PC0 observation. Stop after that
observation. A second reservation is reserved for verifying a specific repair,
not blind repetition; bind its exact reviewed source here before using it.
The campaign identity and cumulative two-batch ceiling never reset on source edits.

Reuse only the closed adapter's exact retained artifact (9915439437), AGain,
Runtime/Proton, and stopped-source primitives. No Windows production, artifact
download, custody reconstruction, fixture seed, product implementation, acceptance
execution, or tracked product evidence is authorized. Missing stores or historical
worktrees are a bounded preflight blocker, not permission to rebuild them.

## Source and authority are separate

Use a clean detached checkout of `authorized_source_commit` for execution. Read
this receipt from the current clean main checkout, never from the frozen checkout
or an edited copy. CURRENT_SLICE.md's default CLI remains live-disabled.

For this campaign, the authorized entry is the already-accepted classified API:
`load_authority(receipt)` -> `authorize_live_request(...)` ->
`ClassifiedProofBackend(production_state_root(), PRODUCTION_ADAPTERS).execute(...)`.
It retains the same current-file re-admission, closed plan, canonical delegation,
fixed OS-account ledger, and reservation enforcement as `proof-run.py`. Do not
patch module globals, alter the ledger root, weaken checkout checks, or invoke
the historical driver. This is a scoped invocation, not another backend.

## Failure and completion

Same-reservation read-only reconciliation is allowed; a lost acknowledgement
never authorizes another launch. Preserve unresolved locks and unknown effects.
After required containment/readback, retrieve one bounded diagnostic pair,
validate it once, and report in at most 20 lines. No success packet, full audit,
proof-matrix replay, or polished failure dossier. Report what ran, where it failed,
cleanup/protected state, actual effects, budget remaining, and the next fix.
Diagnostic success remains non-authoritative: WA0 stays accepted and PC0 unaccepted.
