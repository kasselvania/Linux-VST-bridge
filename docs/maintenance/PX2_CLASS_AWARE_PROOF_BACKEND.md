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
  a canonical validated delegation, binds one exact frozen product contract and
  plan digest, durably records execution class, stable campaign or candidate
  identity, exact per-run source, acceptance eligibility, and its independently
  owned budget before workload launch; performs every cheap and read-only check
  before reservation; prevents duplicate physical launch after crash or lost
  acknowledgement; makes diagnostic product-evidence publication structurally
  unreachable; permits tracked product evidence only from a strictly admitted
  successful acceptance candidate; and closes unsuccessful observations with
  one bounded private diagnostic pair and concise disposition.
allowed_changed_paths:
  - .github/workflows/proof-policy.yml
  - CURRENT_SLICE.md
  - README.md
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
required_transaction_order:
  - validate authority and exact delegation
  - resolve one closed adapter and exact plan content
  - run cheap local and read-only preflight
  - reconcile prior reservation, result, or unknown outcome
  - acquire budget-owner single-writer lock
  - durably reserve exactly one batch
  - invoke the adapter at most once
  - retain class-specific result or bounded failure
  - retain cleanup, protected-state, and actual-or-unknown effects
  - close the reservation monotonically
  - render product evidence only for a strictly admitted successful acceptance
state_progression:
  - NEW
  - PREFLIGHTED
  - RESERVED
  - LAUNCHING
  - OBSERVED
  - OUTCOME_UNKNOWN
  - RESULT_RETAINED
  - FAILURE_RETAINED
  - CLOSED
  - ACCEPTANCE_EVIDENCE_RENDERED
state_law: >-
  State transitions are monotonic. OUTCOME_UNKNOWN may reconcile to an already
  existing observation or retained result but may never transition to NEW or
  authorize another physical launch. A durable reservation consumes the batch
  even when the process crashes, times out, acknowledgement is lost, or outcome
  remains unknown.
durability_law: >-
  Canonical state is written to a same-directory temporary file, flushed and
  fsynced, atomically renamed, and followed by a durable parent-directory
  update where supported. The implementation must document the exact guarantee
  achieved and must not claim stronger crash consistency than it proves.
single_writer_law: >-
  Budget ownership uses a real cross-process exclusive lock or equivalent
  filesystem primitive. Tests must use competing processes, not only in-process
  mocks. A stale owner is reconciled from durable state; it is never treated as
  permission to reset consumption.
authority_model: >-
  Use a finite closed table of legal authority-phase, change-class,
  execution-class, live-authorization, product-implementation-authorization,
  and backend-readiness combinations. Unknown values and contradictory
  combinations fail closed. No generic policy language is authorized.
plan_binding: >-
  A readable plan ID is insufficient. The closed adapter mapping binds the
  product-contract identity, canonical plan content digest, execution class,
  exact source, artifact requirement, fixture requirement, runtime requirement,
  and campaign or candidate identity. A correct result under the wrong plan is
  inadmissible.
deterministic_proof:
  - current main authority disables live execution
  - direct host-proof run is blocked before historical-backend loading
  - direct host-proof seed-fixture is blocked before historical-backend loading
  - exact delegation and plan digest are persisted before reservation
  - illegal authority phase, change class, execution class, and readiness combinations fail closed
  - unsupported adapter and failed preflight consume zero workload budget
  - DiagnosticCampaignIdentity budget survives focused diagnostic source revisions
  - each diagnostic batch retains its exact source through a distinct execution record
  - a two-batch diagnostic campaign refuses a third reservation
  - diagnostic output is permanently acceptance-ineligible
  - diagnostic execution has no product-evidence renderer capability
  - acceptance is bound to one exact AcceptanceCandidateIdentity, source, plan digest, and one batch
  - source change requires a new acceptance candidate
  - diagnostic and acceptance budgets do not consume or reset one another
  - competing processes for the final batch yield exactly one durable reservation
  - crash after durable reservation leaves the batch consumed
  - lost acknowledgement and OUTCOME_UNKNOWN reconcile without duplicate launch
  - retained result is re-admitted without re-execution
  - diagnostic, failed acceptance, and inconclusive acceptance cannot invoke the product renderer
  - only a strictly admitted successful acceptance invokes the renderer exactly once
  - failure closure is canonical, sidecar-verified, no larger than 65536 bytes, and retains unknown effects as unknown
  - failure closure does not invoke the product proof matrix, product audit, or success evidence renderer
  - direct legacy live execution remains blocked
ci_requirement: >-
  Add one Linux-only policy/core workflow running Python compilation and the two
  deterministic test modules. It performs no Windows build, artifact operation,
  Deck contact, Runtime/Proton launch, plug-in workload, or fixture mutation.
branch_rule_requirement: >-
  After the named policy/core check exists and passes, configure main to require
  pull requests and that check, block force pushes and deletion, and retain an
  explicit maintainer break-glass path. If repository administration cannot be
  changed through the available authority, return the exact blocked result and
  required settings rather than claiming protection.
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
  repository_rule_update_after_green_ci: true
failure_closure:
  bounded_diagnostic_pair: true
  diagnostic_json_max_bytes: 65536
  success_evidence_on_failure: false
  full_audit_on_failure: false
  product_design_return_on_harness_failure: false
review_path: docs/prompts/PROOF_HARNESS_MAINTENANCE_REVIEW.md
review_record: docs/maintenance/PX2_MAINTENANCE_REVIEW.md
reviewed_authority_head: 665f98b507b779e8bbada97984ede37b674d48ba
review_record_git_blob: 367934660d456e5b27ac630a7a1ae636c2b87c9e
review_result: PROOF_HARNESS_MAINTENANCE_CLEAR
review_must_precede_implementation: true
review_status: clear
implementation_authorized: true
live_execution_authorized: false
operator_approval_required: false
operator_approval_text: >-
  The operator directed the technical leads to take the next step, prepare the
  repository, correct the process defects that can be corrected immediately,
  and provide the engineering prompt under the accepted proof-harness-
  maintenance process.
successor_selection_authorized: false
```

## Classification

PX2 is high-risk proof-harness maintenance because it changes authority
admission, durable budget accounting, output eligibility, crash recovery, and
failure closure. It does not alter the PC0 product contract, VST3 calls, Windows
product source, fixture meaning, product normalization, or claimed containment
semantics.

A finding that changes any product fact returns `RETURN_TO_PRODUCT_DESIGN`. A
finding confined to this receipt is a focused maintenance repair or
`PROOF_HARNESS_MAINTENANCE_BLOCKED`.

## Core boundary

PX2 provides the transaction core around a closed plan adapter. It does not
implement or enable a live PC0 adapter.

The core accepts one exact
`linux-vst-bridge-proof-execution-delegation/v1` value produced by
`proof_execution_policy.py`. Before any workload reservation it must retain and
re-admit that exact value together with:

- authority digest;
- product-contract identity;
- exact canonical plan digest;
- execution class;
- stable campaign or candidate identity;
- exact source commit for this batch;
- artifact, fixture, and runtime requirements supplied by the adapter;
- `acceptance_eligible`;
- budget owner, maximum, consumed count, reservation identity, and state.

The backend is not a generic workflow language. Plan adapters come from a
closed in-repository mapping and expose bounded typed preflight, invoke,
reconcile, admit, cleanup, and acceptance-render capabilities.

## Stable campaign and candidate identities

A `DiagnosticCampaignIdentity` owns the campaign budget and remains stable
through focused diagnostic-harness source revisions. Each batch separately
retains its exact source through a distinct execution record. A source edit does
not create a new campaign or reset its count.

An `AcceptanceCandidateIdentity` binds one exact frozen source, one exact plan
digest, and one acceptance batch. A source or plan-content change requires a
new candidate and cannot mutate the prior candidate.

## Required backend order

```text
validate authority and exact delegation
→ resolve closed adapter and exact plan digest
→ local/read-only preflight
→ reconcile prior reservation/result/unknown outcome
→ acquire budget single-writer
→ durably reserve one batch
→ invoke adapter once
→ retain diagnostic, success result, or bounded failure
→ retain cleanup/protected-state/effects
→ close reservation
→ acceptance renderer only after strict successful-candidate admission
```

Preflight and unsupported-adapter failures consume no workload budget. Once
`RESERVED` is durable, success, conclusive failure, inconclusive failure,
crash, timeout, lost acknowledgement, and unknown outcome all consume the
batch. Reconciliation may recover existing work; it may not start replacement
work.

## Structural output separation

### Diagnostic path

A diagnostic record states:

```text
execution_class: DIAGNOSTIC_NON_AUTHORITATIVE
acceptance_eligible: false
```

The diagnostic adapter path receives no product-evidence rendering capability.
It cannot create or update tracked product evidence, set a product proof row to
PASS, advance the frontier, or later be relabelled as acceptance.

### Acceptance path

An acceptance record states:

```text
execution_class: ACCEPTANCE_CANDIDATE
acceptance_eligible: true
```

Tracked evidence is available only after strict admission of a successful
result bound to the same authority, product contract, candidate, source, plan
digest, reservation, artifact, fixture/runtime requirements, cleanup, and
protected-state result. Failed and inconclusive acceptance candidates receive
bounded failure closure only.

## Failure closure

One unsuccessful classified observation may retain only:

- one canonical private JSON diagnostic no larger than 64 KiB;
- one exact external SHA-256 sidecar;
- execution class and identity;
- exact source and plan digest;
- primary classification;
- cleanup and protected-state disposition;
- actual effect counts, or explicit `unknown` where not proven;
- remaining budget and concise next action.

It does not render success evidence, replay a product proof matrix, run a
product pre-PR audit, or create a product-design amendment merely to describe
the failure.

## Hosted CI and repository enforcement

PX2 adds one cheap Linux-only policy/core check. After that check is green, the
repository should require a PR and the check on `main`, prohibit force pushes
and deletion, and retain an explicit maintainer break-glass route. This is
repository administration, not product evidence.

## README correction

The repository README previously stopped its accepted-boundary summary at WR0.
The reviewed authority corrects that factual summary to include WF0, WC0, WA0,
DX0/PX0/PX1, and PC0's suspended posture. This does not reopen product design.

## PX2 completion

PX2 is complete when:

- the production transaction core and policy integration pass deterministic
  tests with injected adapters;
- direct legacy live execution remains disabled;
- the Linux-only policy/core CI passes;
- main protection is configured or an exact administration blocker is retained;
- no live plan is enabled.

After merge:

```text
classified_backend_core_ready: true
classified_backend_ready: false
live_execution_authorized: false
accepted_product_frontier: WA0
current_product_target: PC0
```

A later PC0 maintenance branch may add exactly one closed PC0 adapter and a
bounded diagnostic campaign without reopening the unchanged PC0 product
contract.

## Deferred cleanup

The exact historical Git blob used for local `plan` and `validate` remains a
migration bridge during PX2. It is not a permanent architecture. After visible
current equivalents are implemented and proven equivalent, remove runtime
execution of historical source in a separate mechanical cleanup. Do not block
PX2 on that work.
