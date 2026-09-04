# Proof-Harness Maintenance Receipt

Use this instead of a product-design amendment when the product contract is unchanged and the work repairs diagnostics, orchestration, result admission, evidence rendering, recovery, or deterministic harness tests.

```yaml
schema: linux-vst-bridge-proof-harness-maintenance/v1
repository: kasselvania/Linux-VST-bridge
maintenance_id: <ID>
title: <TITLE>
risk_class: mechanical | high_risk
basis_commit: <COMMIT>
basis_tree: <TREE>
product_slice: <SLICE_ID OR none>
product_contract_identity: <EXACT DESIGN/CLAIM IDENTITY>
product_contract_unchanged: true
vst3_operation_roster_unchanged: true
product_owner_lifecycle_unchanged: true
windows_product_behavior_unchanged: true
fixture_semantics_unchanged: true
product_normalization_unchanged: true
containment_shutdown_claim_unchanged: true
security_privacy_boundary_unchanged: true
maintenance_claim: >-
  <ONE BOUNDED HARNESS CLAIM>
allowed_changed_paths:
  - <PATH>
prohibited_changed_paths:
  - <PATH OR CLASS>
deterministic_proof:
  - <PROOF>
diagnostic_campaign:
  required: false
  campaign_identity: null
  batch_budget: 0
  acceptance_eligible: false
acceptance_candidate_required_after_maintenance: true
permitted_external_effects:
  read_only_reconciliation: true
  diagnostic_batches: 0
  acceptance_batches: 0
  windows_producers: 0
  artifact_downloads: 0
  custody_operations: 0
  fixture_seeds: 0
failure_closure:
  bounded_diagnostic_pair: true
  success_evidence_on_failure: false
  full_audit_on_failure: false
review_path: docs/prompts/PROOF_HARNESS_MAINTENANCE_REVIEW.md
implementation_authorized: true | false
live_execution_authorized: true | false
operator_approval_required: true | false
operator_approval_text: <TEXT OR null>
successor_selection_authorized: false
```

## Classification rule

This receipt is invalid and work returns to product design if any proposed change alters:

- the primary product claim or claim ceiling;
- VST3 calls, order, or interpretation;
- product ownership or lifecycle;
- Windows product or ABI behavior;
- fixture meaning;
- product-facing normalization;
- containment or shutdown semantics claimed by the product;
- product protocol, trust, authorization, privacy, or security boundary;
- product proof requirements or compatibility claim.

## Live diagnostic rule

When a maintenance receipt permits diagnostics, each result must state:

```text
execution_class: diagnostic_non_authoritative
acceptance_eligible: false
```

A diagnostic result cannot support product merge and cannot be promoted. Final product acceptance requires a fresh exact AcceptanceCandidateIdentity.