# Prompt — Adversarial Product Design Review

Use this in a fresh context for a product-contract design. Proof-harness-only work uses `PROOF_HARNESS_MAINTENANCE_REVIEW.md`.

```text
Act as the independent adversarial reviewer for one product-contract design.

Repository:
kasselvania/Linux-VST-bridge

Slice:
<SLICE_ID — TITLE>

Exact design head/tree/card identity:
<HEAD / TREE / BLOB / SHA-256>

Do not implement. Do not run Windows or Deck workloads. Do not redesign the
proof harness merely because it could be cleaner.

1. VERIFY AUTHORITY

Confirm exact repository, basis, selection receipt, design identity, changed
paths, worktree/PR posture, and implementation_authorized=false.

2. REVIEW THE PRODUCT CONTRACT

Attack:

- primary claim and claim ceiling;
- product owner map;
- lifecycle states and transitions;
- exact VST3 operation roster, order, and return interpretation;
- product-facing normalization;
- fixture semantics;
- thread/reentrancy obligations;
- containment and shutdown claims;
- protocol, trust, authorization, privacy, and security boundaries;
- product proof matrix and nonclaims.

3. REVIEW DEVELOPMENT ECONOMICS

The card must separately define:

- deterministic/local work;
- whether a DiagnosticCampaignIdentity is needed;
- diagnostic batch budget;
- bounded diagnostic result and failure closure;
- one exact AcceptanceCandidateIdentity;
- acceptance batch budget;
- Windows producer budget;
- live negative budget;
- separate budget owners.

Reject a design that uses acceptance as the edit/debug loop or gives every Deck
contact one undifferentiated budget.

4. DIAGNOSTIC INTEGRITY

Verify diagnostic results are permanently:

execution_class: diagnostic_non_authoritative
acceptance_eligible: false

They may not create product evidence, set product proof rows to PASS, update the
accepted frontier, or be promoted.

5. HARNESS-MAINTENANCE BOUNDARY

The card must state which harness defects may be repaired through focused
maintenance without reopening the product design. Return design repair only if
that boundary would allow a real product-contract change to escape review.

6. RETURN-TO-PRODUCT-DESIGN RULE

Require exact return when any product claim, VST3 call/interpretation,
ownership, lifecycle, Windows product behavior, fixture meaning, product
normalization, claimed containment/shutdown semantics, protocol/security
boundary, proof requirement, or compatibility claim changes.

Do not demand product redesign for a diagnostic-retention, evidence-rendering,
transaction-bookkeeping, or orchestration defect that leaves every product
fact unchanged.

7. VERDICT

Return:

DESIGN_CLEAR
or
DESIGN_REPAIR_REQUIRED
or
DESIGN_REVIEW_BLOCKED

For each finding provide severity, exact design section, product invariant,
concrete defect, and the smallest repair. Do not inflate proof rows, blockers,
paths, or authority cycles merely to appear exhaustive.
```
