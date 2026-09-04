# PX1 — Proof Execution Class Enforcement

```yaml
schema: linux-vst-bridge-proof-harness-maintenance/v1
repository: kasselvania/Linux-VST-bridge
maintenance_id: PX1
title: Proof Execution Class Enforcement
risk_class: high_risk
basis_commit: 39cad26ca914c9b89a9f79a3d2c25aabd4c06f30
basis_tree: 90d30e4f7fe720f3aac3be8e4ae9cc1af105d36c
product_slice: PC0
product_contract_identity: pc0-selection-v2
product_contract_unchanged: true
vst3_operation_roster_unchanged: true
product_owner_lifecycle_unchanged: true
windows_product_behavior_unchanged: true
fixture_semantics_unchanged: true
product_normalization_unchanged: true
containment_shutdown_claim_unchanged: true
security_privacy_boundary_unchanged: true
maintenance_claim: >-
  Direct unclassified workload execution is impossible through the repository's
  ordinary proof entry point; live proof commands require one exact execution
  class and authority, while the existing unclassified backend remains disabled
  until it can durably retain class, identity, eligibility, and separate budgets.
allowed_changed_paths:
  - CURRENT_SLICE.md
  - docs/maintenance/PX1_EXECUTION_CLASS_ENFORCEMENT.md
  - tools/host-proof.py
  - tools/proof-run.py
  - tools/proof_execution_policy.py
  - tools/test_proof_execution_policy.py
deterministic_proof:
  - current authority disables live execution
  - direct legacy run and fixture seeding are blocked
  - diagnostic authority always yields acceptance_eligible=false
  - acceptance authority binds exactly one candidate identity and budget
  - missing or mismatched authority and unclassified backend fail closed
  - local-only legacy blob is content-addressed and has no executable worktree path
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
implementation_authorized: true
live_execution_authorized: false
operator_approval_required: false
operator_approval_text: >-
  The operator explicitly directed the technical lead to fix the repository
  process, eliminate redundant loops, and prevent recurrence.
successor_selection_authorized: false
```

## Scope

PX1 is a circuit breaker and authority parser. It does not yet implement a live
diagnostic backend. That omission is intentional: the accepted legacy backend
does not retain execution class or eligibility and therefore cannot safely run
under the new process.

Cheap local `plan` and `validate` compatibility load the exact accepted legacy
Git blob `beb57c62b94f3d99fb87f56db3b20b20553064c4` from repository history and
verify it before execution. There is no legacy executable path in the worktree,
so the old live command cannot bypass the policy wrapper.

`run` and `seed-fixture` are blocked at the ordinary entry point.
`tools/proof-run.py` validates future diagnostic or acceptance authority but
refuses live delegation until a class-aware backend exists.

## Product nonclaims

PX1 changes no VST3 call, owner, lifecycle, Windows binary, fixture, runtime,
normalization, containment claim, or product capability. The accepted frontier
remains WA0 and PC0 remains suspended.
