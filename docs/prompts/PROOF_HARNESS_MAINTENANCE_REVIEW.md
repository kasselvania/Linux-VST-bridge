# Prompt — Proof-Harness Maintenance Review

Use this in a fresh context for mechanical or high-risk proof-harness maintenance. This review is intentionally narrower than product-design review.

```text
Act as the independent proof-harness maintenance reviewer.

Repository:
kasselvania/Linux-VST-bridge

Maintenance receipt:
<PATH AND IDENTITY>

Exact candidate head/tree:
<HEAD>
<TREE>

Do not run a product workload unless the receipt explicitly authorizes a
DIAGNOSTIC_NON_AUTHORITATIVE batch. Do not treat diagnostics as acceptance.

1. VERIFY AUTHORITY

Confirm exact repository, basis, candidate, changed paths, receipt identity,
CURRENT_SLICE authority, and live budgets.

2. PROVE PRODUCT CONTRACT IS UNCHANGED

Compare the maintenance candidate with the exact approved product contract.
Return RETURN_TO_PRODUCT_DESIGN if it changes any:

- primary claim or claim ceiling;
- VST3 operation roster, order, or interpretation;
- product ownership or reference-count law;
- product lifecycle;
- Windows product/ABI behavior;
- fixture semantics;
- product-facing normalization;
- containment/shutdown semantics claimed by the product;
- protocol, trust, authorization, privacy, or security boundary;
- product proof requirements or compatibility claim.

3. REVIEW THE MAINTENANCE CLAIM

Inspect production code, not only tests. Verify the exact bounded maintenance
claim, failure states, recovery, identity joins, process cleanup, privacy,
and evidence eligibility.

4. DIAGNOSTIC LAW

When diagnostics are authorized, verify:

- exact DiagnosticCampaignIdentity;
- batch count within budget;
- acceptance_eligible=false;
- no tracked product evidence generated;
- no product proof row marked PASS;
- exact cleanup/protected-state disposition;
- no promotion path from diagnostic to acceptance.

5. FAILURE-CLOSURE LAW

A failed diagnostic or inconclusive acceptance attempt may retain only one
bounded diagnostic pair, cleanup/protected-state disposition, actual effect
counts, and one concise report. Reject work that creates a success evidence
packet, full proof-matrix replay, or multi-hour failure audit.

6. DETERMINISTIC PROOF

Verify tests exercise production helpers and prove:

- maintenance behavior;
- product contract unchanged;
- diagnostic/acceptance separation;
- exact budget ownership;
- no duplicate launch after acknowledgement loss;
- old evidence not relabelled.

7. VERDICT

Return exactly one:

PROOF_HARNESS_MAINTENANCE_CLEAR
PROOF_HARNESS_MAINTENANCE_REPAIR_REQUIRED
RETURN_TO_PRODUCT_DESIGN
PROOF_HARNESS_MAINTENANCE_BLOCKED

Include exact head reviewed, changed paths, product-contract comparison,
external-effect counts, diagnostic eligibility, and required action.
```
