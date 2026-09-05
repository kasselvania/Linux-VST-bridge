# Governance

## Purpose

This repository must be strict about accepted product claims without turning development into a sequence of constitutional ceremonies. The process distinguishes:

- what product behavior is being claimed;
- how the proof harness is maintained;
- whether a live run is diagnostic or authoritative;
- which identity owns each expensive budget.

The detailed lifecycle is owned only by `docs/DEVELOPMENT_PROCESS.md`. This file owns authority and truth classes.

## Current scoped exception: PC0-D1 recovery

The 2026-09-04 recovery ruling in `AGENTS.md`, with its executable task in `docs/campaigns/PC0_D1.md`, takes precedence over the generic process below for the existing AGain diagnostic only. At the operator's direction, the technical lead has set a cumulative eight-reservation ceiling for the SAME campaign (two already consumed, at most six additional), authorized the policy/backend and diagnostic-retention repairs, and delegated tested source rebinding and diagnostic continuation to the implementation agent. No extra design, approval, or pre-run maintenance-review handoff is required inside that task. Code review is required before eventual merge. Acceptance execution and the accepted WA0 frontier are unchanged.

The new ceiling is not a permission to bypass executable controls: the policy and existing stored-budget migration must be corrected and locally tested before another live reservation. Old unknown results and consumed reservations remain historical. Bounded privacy-safe observations may be checkpointed before normalization and cleanup; the final report's validation cannot be the sole path that retains debugging information. The one-pair closure rule limits final reporting, not necessary diagnostic capture or ordinary local investigation.

## Authority precedence

1. explicit operator product or process ruling;
2. `AGENTS.md`;
3. `GOVERNANCE.md`;
4. `CURRENT_SLICE.md`;
5. exact approval or maintenance receipt;
6. exact approved design, when required;
7. `docs/DEVELOPMENT_PROCESS.md`;
8. accepted architecture and decision records;
9. fixture cards and dossier;
10. issue, PR, or implementation convenience.

A lower authority may narrow work but may not widen a higher authority.

## Truth classes

### Product design

Human-facing intended behavior and product laws. It does not prove implementation or select a slice.

### Product slice contract

One bounded claim, exact fixture, owner/lifecycle boundary, changed paths, nonclaims, and acceptance conditions.

### Proof-harness maintenance

Changes to diagnostics, orchestration, result admission, evidence rendering, transaction bookkeeping, recovery, or deterministic harness tests that leave the product contract unchanged.

### Diagnostic observation

A bounded live development observation marked permanently `acceptance_eligible: false`. It may guide repairs but cannot establish a product claim.

### Acceptance observation

A bounded live observation from one exact frozen acceptance candidate. Only this class may support retained product evidence and merge.

### Implementation fact

Code, tests, build definitions, and accepted PRs state what exists. They do not silently amend product design.

### Retained evidence

Exact fixture observations with declared execution class, identities, conditions, result, and claim ceiling.

### Compatibility claim

A bounded statement over exact versions and capabilities, never universal truth.

## Recognized authority phases

```text
no_active_slice
reconnaissance_and_design
implementation
proof_harness_maintenance
documentation_and_governance
```

`CURRENT_SLICE.md` names the active phase and must separately state:

```text
product_implementation_authorized
live_execution_authorized
permitted_execution_class
```

Silence authorizes none of them.

## Mandatory two-axis classification

Every change has one change class:

```text
PRODUCT_CONTRACT_CHANGE
PROOF_HARNESS_MAINTENANCE
MECHANICAL_MAINTENANCE
```

Every external operation has one execution class:

```text
READ_ONLY_RECONCILIATION
DIAGNOSTIC_NON_AUTHORITATIVE
ACCEPTANCE_CANDIDATE
```

The axes are independent. Proof-harness maintenance may need a diagnostic or acceptance run; a product change may use diagnostics before its final acceptance run.

## Product-design approval

A product-contract change requires:

```text
selection
→ bounded design
→ independent adversarial review
→ exact operator approval
→ merged authority
```

Return to product design only when the change alters or invalidates:

- the primary claim or claim ceiling;
- VST3 calls, order, or interpretation;
- object ownership or reference-count law;
- product lifecycle;
- Windows product/ABI behavior;
- fixture meaning;
- product-facing normalization;
- containment or shutdown semantics claimed by the product;
- protocol, trust, authorization, privacy, or security boundary;
- product proof requirements or compatibility claim.

## Proof-harness maintenance authority

Proof-harness maintenance does not reopen product selection when all product-contract facts remain unchanged.

### Mechanical maintenance

May receive an explicit waiver and ordinary code review.

### High-risk proof-harness maintenance

Changes diagnostic durability, recovery, process supervision, identity joins, or evidence admission. It requires:

```text
one focused maintenance receipt
→ one independent maintenance review
→ exact technical-lead disposition
```

Operator approval is required only when the maintenance changes fixture semantics, security/privacy boundaries, external mutation, or live budgets, except where an explicit scoped ruling delegates that decision. It does not require a new product design card merely because the proof harness is high risk. PC0-D1 uses the review timing specified in its current recovery ruling.

## Diagnostic execution authority

A diagnostic campaign must freeze:

- `DiagnosticCampaignIdentity`;
- exact product contract identity;
- source, artifact, fixture, runner, and closed diagnostic plan;
- live diagnostic budget;
- bounded output and cleanup law.

For the open AGain fixture, the default is two diagnostic batches per campaign unless an explicit scoped ruling sets another ceiling. PC0-D1 recovery authorizes eight cumulative reservations. Other increases require an explicit operator ruling or specifically delegated technical-lead decision. Commercial or stateful fixtures default to one or zero.

Every diagnostic result must state:

```text
execution_class: diagnostic_non_authoritative
acceptance_eligible: false
```

It cannot create final product evidence, set product proof rows to PASS, update the accepted frontier, or later be promoted.

## Acceptance authority

An acceptance candidate must freeze:

- `AcceptanceCandidateIdentity`;
- exact candidate source;
- build/artifact, fixture, runner, and closed acceptance plan;
- one acceptance batch by default;
- evidence and cleanup requirements.

A failed or inconclusive acceptance observation remains historical. A harness-inconclusive result enters proof-harness maintenance and requires a new acceptance candidate after repair; it does not invalidate the product contract by itself.

## Failure closure

Failure reporting is bounded work, not another slice.

After process cleanup and protected-state readback, a failed diagnostic or inconclusive acceptance attempt may perform only:

1. one bounded diagnostic publication/retrieval and validation;
2. cleanup/protected-state disposition;
3. actual effect-count accounting;
4. one concise report.

No success evidence packet, full proof-matrix replay, pre-PR audit, or product-design amendment is created merely to prove that a development run failed. Incremental capture authorized by the recovery ruling happens before this final closure and is not another product observation.

## Budget ownership

| Expensive operation | Budget owner |
|---|---|
| Windows producer | `WindowsBuildInputIdentity` |
| Artifact download/custody | exact artifact and custody identity |
| Diagnostic workload | `DiagnosticCampaignIdentity` |
| Acceptance workload | `AcceptanceCandidateIdentity` |
| Live fault replay | `FaultPlanIdentity` |
| Source transfer | exact source-handoff identity |
| Evidence rendering | `EvidenceRendererIdentity` plus retained result |
| Read-only reconciliation | orchestration accounting only |

Budgets do not silently consume one another.

## Evidence eligibility

- Diagnostic observations are permanently ineligible for acceptance.
- Acceptance evidence must come from the exact candidate and plan it names.
- Producer P, execution E, and consumer C remain distinct.
- Old evidence is never rewritten for new source.
- A renderer-only change may reuse an admitted result without another live run.
- Incomplete observations remain unknown.
- Every workload, diagnostic or acceptance, retains exact cleanup and protected-state facts.

## Independent review

- Product design review uses a fresh context.
- Final implementation audit uses a fresh context.
- High-risk proof-harness maintenance uses a focused fresh-context review proving the product contract is unchanged, with timing governed by an explicit scoped ruling where present.
- A review must inspect the exact head, not a branch name or summary.

## Merge posture

A product implementation may merge only from a successful acceptance candidate with exact evidence, audit, and claim ceiling.

Proof-harness maintenance may merge without product evidence when deterministic tests and any explicitly authorized diagnostic evidence establish the maintenance claim. Diagnostic results themselves never advance the product frontier.

After a successful product merge, a separate status closure records the accepted capability and returns to no-active-slice posture. No successor is automatic.

## Process anti-patterns

Prohibited:

- using acceptance runs as the edit/debug loop;
- treating all Deck contact as one acceptance budget;
- returning to product design for a harness-only defect;
- promoting diagnostics into evidence;
- unlimited diagnostic retries;
- spending hours producing polished failure evidence;
- replaying accepted proof matrices without a changed invariant;
- manually copying run IDs, artifact IDs, SSH paths, or receipts during ordinary use;
- hiding cumulative failed attempts;
- weakening cleanup or evidence integrity because a run is diagnostic.
