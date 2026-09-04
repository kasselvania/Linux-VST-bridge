# PX0 Process Ruling — Diagnostic Development and Acceptance Separation

## Status

```yaml
ruling: accepted_by_explicit_operator_direction
basis_commit: 7ed1fbf985b5bb717883e0cd2132620b960e3aa6
basis_tree: 07efd2d95c7f72205ac5c201361c4291bb8eaf6d
accepted_product_frontier: WA0
pc0_status: suspended
live_execution_authorized: false
```

## Finding

DX0 successfully separated Windows build, Deck execution, and evidence-renderer identities. The remaining process still conflated development diagnostics with authoritative acceptance observations.

That conflation caused an inconclusive PC0 proof-harness failure to trigger repeated product-design, review, approval, authority-finalization, and execution cycles. It also made failure reporting itself an expensive engineering deliverable.

## Ruling 1 — Change class and execution class are separate

```text
change class:
  PRODUCT_CONTRACT_CHANGE
  PROOF_HARNESS_MAINTENANCE
  MECHANICAL_MAINTENANCE

execution class:
  READ_ONLY_RECONCILIATION
  DIAGNOSTIC_NON_AUTHORITATIVE
  ACCEPTANCE_CANDIDATE
```

## Ruling 2 — Product design is reopened only for product changes

Return to product design only for changes to the claim, VST3 contract, product ownership/lifecycle, Windows product behavior, fixture meaning, product normalization, claimed containment/shutdown semantics, protocol/security boundary, product proof requirement, or compatibility claim.

Harness-only diagnostics, orchestration, result admission, evidence rendering, bookkeeping, and recovery use proof-harness maintenance authority.

## Ruling 3 — Diagnostics are permanently non-authoritative

Every diagnostic result states:

```text
execution_class: diagnostic_non_authoritative
acceptance_eligible: false
```

Diagnostics never create final product evidence, mark product proof rows PASS, update the accepted frontier, or become acceptance by relabelling.

## Ruling 4 — Default diagnostic budget

For the open AGain fixture:

```text
2 diagnostic batches maximum per DiagnosticCampaignIdentity
```

One batch diagnoses; one verifies the harness repair. More requires explicit operator authority.

## Ruling 5 — Acceptance budget

```text
1 acceptance batch per AcceptanceCandidateIdentity
```

Acceptance begins only after deterministic work and diagnostics are complete and the exact candidate is frozen.

## Ruling 6 — Failure closure is bounded

A failed diagnostic or inconclusive acceptance attempt receives only:

- one bounded diagnostic JSON object and sidecar;
- cleanup/protected-state disposition;
- actual effect counts;
- one concise report.

No success evidence, full proof-matrix replay, full audit, or product-design amendment is produced merely to document failure.

## Ruling 7 — Budgets have separate owners

| Budget | Owner |
|---|---|
| Windows producer | `WindowsBuildInputIdentity` |
| Diagnostic workload | `DiagnosticCampaignIdentity` |
| Acceptance workload | `AcceptanceCandidateIdentity` |
| Live fault replay | `FaultPlanIdentity` |
| Evidence render | `EvidenceRendererIdentity` plus admitted result |
| Read-only reconciliation | orchestration accounting only |

## Ruling 8 — PC0 history

The stopped PC0 source is development history only:

```text
309b8918c128c0b9e6701d0453dc841a111d5ac5
```

It is archived at:

```text
refs/heads/codex/archive/pc0-v3-stopped-309b8918
```

It is not accepted source or evidence. The PC0 product claim remains suspended and may resume only after diagnostic/acceptance lane enforcement exists.

## Required next enforcement

Before another live product workload:

1. merge the human process repair;
2. implement execution-class and eligibility enforcement in the proof driver;
3. test separate diagnostic and acceptance budgets deterministically;
4. ensure failed diagnostics cannot trigger success evidence or full audit;
5. start a new PC0 diagnostic campaign only if real-fixture debugging remains necessary;
6. freeze a fresh acceptance candidate after diagnostics succeed.
