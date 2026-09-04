# PX2 — Class-Aware Proof Transaction Core

```yaml
schema: linux-vst-bridge-proof-harness-maintenance/v1
repository: kasselvania/Linux-VST-bridge
maintenance_id: PX2
title: Class-Aware Proof Transaction Core
risk_class: high_risk
basis_commit: 98110016c1d50d578b2cf44d439830442f917b68
basis_tree: 6e0964a6c506a743ce4fc229940f1e6ed982acc5
maintenance_branch: codex/px2-class-aware-proof-backend
product_slice: PC0
product_contract_identity: pc0-selection-v2 with the unchanged Initialized-state pre-setup census claim
product_contract_unchanged: true
vst3_operation_roster_unchanged: true
product_owner_lifecycle_unchanged: true
windows_product_behavior_unchanged: true
fixture_semantics_unchanged: true
product_normalization_unchanged: true
containment_shutdown_claim_unchanged: true
security_privacy_boundary_unchanged: true
maintenance_claim: >-
  The repository owns one class-aware proof transaction core that accepts only
  a canonical validated delegation, durably records execution class, stable
  campaign or candidate identity, exact per-run source, acceptance eligibility,
  and its separately owned budget before workload reservation; performs cheap
  and read-only checks before reservation; makes diagnostic product-evidence
  publication structurally unreachable; permits tracked product evidence only
  from an admitted acceptance candidate; and closes unsuccessful observations
  with one bounded private diagnostic pair and concise disposition.
allowed_changed_paths:
  - CURRENT_SLICE.md
  - docs/maintenance/PX2_CLASS_AWARE_PROOF_BACKEND.md
  - docs/maintenance/PX2_MAINTENANCE_REVIEW.md
  - tools/classified_proof_backend.py
  - tools/proof-run.py
  - tools/proof_execution_policy.py
  - tools/test_classified_proof_backend.py
  - tools/test_proof_execution_policy.py
prohibited_changed_paths:
  - tools/host-proof.py
  - tools/wf0-factory-census/**
  - windows-factory-probe/**
  - windows-fixtures/**
  - .github/workflows/wf0-windows-msvc-build.yml
  - docs/slices/PC0/**
  - evidence/**
  - SDK or vendor source
  - product architecture or compatibility claims
deterministic_proof:
  - exact delegation is persisted before any workload reservation
  - read-only and cheap preflight completes before reservation
  - DiagnosticCampaignIdentity budget survives diagnostic source revisions
  - each diagnostic batch retains its exact source through a distinct execution record
  - diagnostic output is permanently acceptance-ineligible
  - diagnostic execution cannot invoke the product-evidence renderer
  - acceptance execution is bound to one exact AcceptanceCandidateIdentity and one batch
  - unsuccessful acceptance cannot invoke the product-evidence renderer
  - only an admitted successful acceptance may invoke an injected acceptance renderer
  - diagnostic and acceptance budgets do not consume or reset one another
  - unknown or lost acknowledgement does not authorize a duplicate launch
  - failure closure is one bounded JSON object and sidecar with actual-or-unknown effects
  - direct legacy live execution remains blocked
  - unsupported plan adapters fail before budget reservation
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
  source_transfers: 0
  artifact_transfers: 0
  fixture_seeds: 0
failure_closure:
  bounded_diagnostic_pair: true
  diagnostic_json_max_bytes: 65536
  success_evidence_on_failure: false
  full_audit_on_failure: false
  product_design_return_on_harness_failure: false
review_path: docs/prompts/PROOF_HARNESS_MAINTENANCE_REVIEW.md
review_record: docs/maintenance/PX2_MAINTENANCE_REVIEW.md
review_must_precede_implementation: true
implementation_authorized: true
live_execution_authorized: false
operator_approval_required: false
operator_approval_text: >-
  The operator directed the technical leads to take the next step, prepare the
  repository, and provide the PX2 implementation prompt under the newly accepted
  proof-harness-maintenance process.
successor_selection_authorized: false
```

## Classification

PX2 is high-risk proof-harness maintenance because it changes authority
admission, durable budget accounting, output eligibility, and failure closure.
It does not alter the PC0 product contract, VST3 calls, Windows product source,
fixture meaning, product normalization, or claimed containment semantics.

A finding that requires any such product change returns
`RETURN_TO_PRODUCT_DESIGN`. A finding confined to this receipt is either a
maintenance repair or `PROOF_HARNESS_MAINTENANCE_BLOCKED`.

## Core boundary

PX2 provides the transaction core around a **closed plan adapter**. It does not
implement or enable a live PC0 plan adapter.

The core accepts one exact
`linux-vst-bridge-proof-execution-delegation/v1` value produced by
`proof_execution_policy.py`. Before any workload reservation, it must retain and
re-admit that exact value together with:

- authority digest;
- execution class;
- stable campaign or candidate identity;
- exact source commit for this batch;
- closed plan ID;
- `acceptance_eligible`;
- budget owner, maximum, consumed count, and reservation state.

The backend is not a generic workflow language. Plan adapters are selected from
a closed in-repository mapping and expose only bounded typed methods.

## Stable campaign and per-run source

A `DiagnosticCampaignIdentity` owns the campaign budget and remains stable
through focused diagnostic-harness source revisions. Each batch separately
retains its exact source in a `DiagnosticExecutionIdentity` or equivalent
canonical execution record.

Changing source therefore cannot reset the campaign budget, and the historical
source of every diagnostic remains explicit.

An `AcceptanceCandidateIdentity` binds one exact frozen source and owns one
acceptance batch. A source change requires a new candidate; it does not mutate
or reuse the prior candidate.

## Required backend ordering

```text
validate authority and delegation
→ resolve one closed adapter
→ perform local/read-only preflight
→ reconcile prior reservation or result
→ acquire exact budget single-writer
→ durably reserve one batch
→ invoke one workload through the adapter
→ retain class-specific result or bounded failure diagnostic
→ close reservation and cleanup disposition
→ render tracked product evidence only for an admitted successful acceptance
```

A preflight or unsupported-adapter failure consumes no workload budget.
A reserved batch is consumed by success, conclusive failure, inconclusive
failure, crash, timeout, or unknown outcome. Lost acknowledgement triggers
reconciliation, not another reservation.

## Structural output separation

### Diagnostic

A diagnostic result must state:

```text
execution_class: DIAGNOSTIC_NON_AUTHORITATIVE
acceptance_eligible: false
```

The diagnostic code path receives no product-evidence rendering capability and
cannot create or update a tracked product evidence packet or product proof-row
PASS disposition.

### Acceptance

An acceptance result must state:

```text
execution_class: ACCEPTANCE_CANDIDATE
acceptance_eligible: true
```

Tracked product evidence is permitted only after strict admission of a
successful result bound to the same authority, candidate, source, plan, budget
reservation, fixture/runtime inputs supplied by the future adapter, cleanup,
and protected-state result.

An inconclusive or failed acceptance receives bounded failure closure only.

## Failure closure

One unsuccessful classified observation may retain only:

- one canonical private JSON diagnostic no larger than 64 KiB;
- one exact external SHA-256 sidecar;
- execution class and identity;
- exact source and plan;
- primary classification;
- cleanup and protected-state disposition;
- actual effect counts, or explicit `unknown` where not proven;
- remaining budget.

It does not render success evidence, replay the product proof matrix, run a
product pre-PR audit, or create a product-design amendment merely to describe
the failure.

## PX2 completion

PX2 is complete when the production transaction core and policy integration
pass deterministic tests with injected adapters, the focused independent
maintenance review is retained, direct legacy live execution remains disabled,
and no live plan is enabled.

After merge:

```text
classified_backend_core_ready: true
classified_backend_ready: false
live_execution_authorized: false
```

A later PC0 maintenance/product branch may add one closed PC0 adapter and exact
diagnostic or acceptance authority without reopening the unchanged PC0 product
contract.
