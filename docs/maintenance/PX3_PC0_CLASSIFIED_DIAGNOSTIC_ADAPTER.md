# PX3 — Closed PC0 Classified Diagnostic Adapter

```yaml
schema: linux-vst-bridge-proof-harness-maintenance/v1
repository: kasselvania/Linux-VST-bridge
maintenance_id: PX3
change_class: PROOF_HARNESS_MAINTENANCE
basis_commit: 78b4ce3a5dc6bef8ab6de766a39ecb6c191165b0
basis_tree: 6bb0547c817df2fc073a2ea2ee10c959d7231494
branch: codex/px3-pc0-classified-diagnostic-adapter
claim: >-
  The production PX2 registry contains exactly one closed PC0 diagnostic-only
  adapter. It performs strict read-only preflight and reconciliation, invokes
  a current reservation-bound worker over the stopped source's environment,
  supervision, normalization, and cleanup primitives after PX2 reservation,
  strictly admits bounded private observations, and can never render product
  evidence or produce an acceptance-eligible result.
plan_id: pc0-pre-setup-processing-contract-diagnostic-v1
product_contract_identity: pc0-selection-v2
product_contract_sha256: daa042e4184cb5fffdf1ff08d59cc51f7755b4635c85e02597adc2584c4b4c1d
plan_content_sha256: a5303e12d644fefba2ca4003555ebe30f578e60c0a334e5f0bb96b7498decc92
product_claim_unchanged: true
accepted_product_frontier: WA0
pc0_status: suspended_pending_separately_approved_diagnostic_campaign
stopped_pc0_source: 309b8918c128c0b9e6701d0453dc841a111d5ac5
windows_build_input_identity: 575d3bd9183be1ec0fe0311cff48bc0107c4299a3355748c470e3d195d284849
allowed_changed_paths:
  - .github/workflows/proof-policy.yml
  - CURRENT_SLICE.md
  - docs/maintenance/PX3_PC0_CLASSIFIED_DIAGNOSTIC_ADAPTER.md
  - tools/pc0_proof_adapter.py
  - tools/pc0_diagnostic_worker.py
  - tools/proof-run.py
  - tools/proof_execution_policy.py
  - tools/test_pc0_proof_adapter.py
  - tools/test_proof_execution_policy.py
no_live_budget:
  windows_workloads: 0
  deck_workloads: 0
  diagnostic_campaigns: 0
  acceptance_candidates: 0
tests:
  - exact production registry and diagnostic-only structure
  - authority, source, plan, artifact, fixture, and runtime refusal before reservation
  - actual retained-store sidecar reader and stdin worker with pinned helper contracts
  - two distinct reservations on one frozen artifact; same reservation resumes; third refused
  - unknown remains consumed and reconcilable; later exact result with one launch
  - retrieval survives failed launch safety; historical stores remain untouched
  - current retrieval effects are zero rather than historical publication counts
  - canonical bounded diagnostic/sidecar admission and unknown effects
  - no renderer, legacy full transaction, or product evidence path
  - Linux-only PX2 proof policy core workflow
stop_boundary: >-
  Any need for a live campaign authority, acceptance adapter, product-contract
  change, legacy full transaction, second budget owner, tracked product
  evidence, Windows work, or changed PC0 behavior stops this maintenance task.
repair_review: 5117828668
reviewed_head: b765ce1d074592345e47cec0acfd4a717e54ee1d
validation_scope: local_effect_free_dependencies_and_linux_policy_core
live_execution_authorized: false
diagnostic_campaign_authorized: false
acceptance_candidate_authorized: false
```
