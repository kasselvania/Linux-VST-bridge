# Implementation Design Card — `<SLICE_ID>`

Use this only for a product-contract change that requires a design gate. Proof-harness-only work uses `PROOF_HARNESS_MAINTENANCE_RECEIPT.md`.

Remove all placeholder text before approval.

## 1. Identity and authority

```yaml
slice_id: <ID>
title: <TITLE>
change_class: PRODUCT_CONTRACT_CHANGE
design_revision: <REVISION>
design_status: proposed | review_required | clear | superseded
repository: kasselvania/Linux-VST-bridge
basis_commit: <COMMIT>
basis_tree: <TREE>
selection_receipt_path: <PATH>
authority_phase: reconnaissance_and_design
implementation_authorized: false
```

## 2. Primary product contract

```text
primary_claim:
claim_ceiling:
accepted_prerequisites:
exact_fixture:
```

State the exact end-to-end boundary and everything a successful demo still does not prove.

## 3. Product-contract ownership

| Fact | Owner | Creation | Mutation | Retirement | Exact readback |
|---|---|---|---|---|---|
|  |  |  |  |  |  |

## 4. Product lifecycle

| State | Authoritative facts | External reality | Entry proof | Exit operation |
|---|---|---|---|---|
|  |  |  |  |  |

List legal and forbidden transitions. Include only product-significant states here. Proof-transaction bookkeeping belongs to the harness section.

## 5. VST3 and product operation roster

| Order/type | Operation | Coordinates | Legal lifecycle | Return/output interpretation | Failure owner |
|---|---|---|---|---|---|
|  |  |  |  |  |  |

This roster is part of the product contract. A later change returns to product design.

## 6. Product-facing normalization

Freeze exact caps, enum interpretation, string handling, consistency rules, and immutable output schema.

## 7. Process, thread, callback, and shutdown law

State exact process/thread roles, reentrancy, timeouts, ownership, clean in-process shutdown claims, and physical containment claims.

## 8. Fixture, security, privacy, and licensing

State exact permitted fixture use and what may never enter Git, logs, diagnostics, screenshots, or support bundles.

## 9. Source and mutation envelope

```text
allowed tracked paths:
permitted external mutation:
protected state:
prohibited mutation:
```

## 10. Deterministic/local development lane

List all checks that must pass before any live workload:

- compiler/syntax checks;
- production-owner state tests;
- schema and canonical-JSON tests;
- selected/prohibited-call checks;
- source/build/Deck/renderer invalidation;
- result-admission tests;
- driver dry-run external plan.

## 11. Diagnostic campaign

State whether live diagnostic development is required.

```yaml
diagnostic_required: true | false
diagnostic_campaign_schema: linux-vst-bridge-diagnostic-campaign/v1
diagnostic_campaign_identity_inputs:
  - product_contract_identity
  - windows_artifact_identity
  - fixture_identity
  - runtime_identity
  - closed_diagnostic_plan
  - diagnostic_harness_source_identity
diagnostic_plan_id: <ID OR null>
diagnostic_batch_budget: <0, 1, OR 2 BY DEFAULT>
acceptance_eligible: false
```

For each diagnostic batch define:

- bounded result/diagnostic schema;
- cleanup and protected-state law;
- actual effect accounting;
- invalidation conditions;
- permanent prohibition on promotion into acceptance.

### Diagnostic failure closure

A failed diagnostic may produce only one bounded diagnostic pair, cleanup/protected-state disposition, effect counts, and one concise report. No success evidence packet, product proof-matrix replay, or full pre-PR audit.

## 12. Proof-harness maintenance boundary

State which changes may be repaired under `PROOF_HARNESS_MAINTENANCE` without reopening this product design.

Confirm that maintenance must leave unchanged:

- claim and nonclaims;
- VST3 calls/order/interpretation;
- product owners and lifecycle;
- Windows product behavior;
- fixture semantics;
- product normalization;
- containment/shutdown claims;
- protocol/security boundary.

## 13. Acceptance candidate

```yaml
acceptance_candidate_schema: linux-vst-bridge-acceptance-candidate/v1
acceptance_identity_inputs:
  - product_contract_identity
  - exact_candidate_source
  - windows_build_input_identity
  - artifact_and_custody_identity
  - fixture_identity
  - runtime_identity
  - closed_acceptance_plan
  - acceptance_authority
acceptance_plan_id: <ID>
acceptance_batch_budget: 1
```

Define exact entry conditions, result schema, evidence roster, cleanup, protected state, and claim ceiling.

A diagnostic result may not be promoted. Acceptance requires a fresh observation.

## 14. Budget ledger

| Budget | Identity owner | Maximum | Reuse law | Failure law |
|---|---|---:|---|---|
| Windows producer | `WindowsBuildInputIdentity` |  |  |  |
| Diagnostic workload | `DiagnosticCampaignIdentity` |  |  |  |
| Acceptance workload | `AcceptanceCandidateIdentity` | 1 |  |  |
| Live negative replay | `FaultPlanIdentity` |  |  |  |
| Source transfer | source-handoff identity |  |  |  |
| Evidence render | renderer + retained result |  |  |  |

Do not combine these budgets.

## 15. Proof matrix

| Product claim | Deterministic proof | Diagnostic role | Acceptance proof | Production helper | Retained product evidence | Claim ceiling |
|---|---|---|---|---|---|---|
|  |  |  |  |  |  |  |

Diagnostic observations cannot satisfy the acceptance column.

## 16. Failure classification

Distinguish at least:

```text
IMPLEMENTATION_REPAIR_REQUIRED
PROOF_HARNESS_MAINTENANCE_REQUIRED
DIAGNOSTIC_BUDGET_EXHAUSTED
DIAGNOSTIC_CLOSURE_BLOCKED
ACCEPTANCE_CANDIDATE_INCONCLUSIVE
ACCEPTANCE_CANDIDATE_FAILED
RETURN_TO_PRODUCT_DESIGN
```

For each, state preserved state, cleanup, remaining budgets, and next lawful action.

## 17. Material product-design stop conditions

Implementation returns to product design only for changes to:

- primary claim or claim ceiling;
- VST3 roster/order/interpretation;
- product ownership/refcount law;
- product lifecycle;
- Windows product/ABI behavior;
- fixture semantics;
- product normalization;
- claimed containment/shutdown semantics;
- protocol/trust/security boundary;
- product proof requirements or compatibility claim.

Harness-only discoveries enter proof-harness maintenance.

## 18. Implementation sequence

```text
implement locally
→ deterministic validation
→ bounded diagnostics if required
→ harness maintenance if required
→ freeze acceptance candidate
→ one acceptance transaction
→ evidence
→ independent audit
→ exact-head review
```

## 19. Review disposition and approved identity

```yaml
adversarial_review_path: <PATH>
adversarial_review_result: DESIGN_CLEAR | DESIGN_REPAIR_REQUIRED
card_path: <PATH>
card_git_blob: <BLOB>
card_sha256: <SHA-256>
approval_receipt_path: <PATH>
implementation_authorized: false
```

The card does not approve itself.