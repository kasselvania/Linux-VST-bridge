# Prompt — Choose the Next Product Slice

Use this in a fresh technical-lead context only after the repository is in no-active-product-slice posture and no mandatory proof-harness repair remains.

```text
Act as technical lead for the Linux VST Bridge.

This task is analysis only. Do not edit the repo, create a branch, run a
fixture, or authorize implementation.

Repository:
kasselvania/Linux-VST-bridge

Expected main:
<COMMIT / TREE>

1. VERIFY BASIS

Confirm exact main, accepted product frontier, no active product implementation,
no open successor PR, and no unresolved process/harness repair that must precede
new live product work.

If process repair remains mandatory, return:

NEXT_SLICE_BLOCKED_BY_PROCESS_REPAIR

2. STATE THE ACCEPTED FRONTIER

Separate:

- accepted product capability;
- retained acceptance evidence;
- proof-harness capability;
- diagnostic history;
- open product decision;
- explicit nonclaims.

3. IDENTIFY ONE PRODUCT EDGE

Find the smallest meaningful next product boundary with one comprehensible
failure owner. Do not select tooling merely because it is untidy. Do not combine
independent product uncertainties.

4. PRESENT AT MOST THREE CANDIDATES

For each candidate state:

- one primary product claim;
- owner/lifecycle boundary;
- VST3 operations;
- exact fixture;
- accepted prerequisites;
- product unknowns;
- source envelope estimate;
- deterministic development work;
- whether a diagnostic campaign is needed;
- diagnostic batch budget;
- final acceptance batch budget;
- Windows producer budget;
- live negative-fixture budget;
- success evidence;
- meaningful blocked result;
- explicit nonclaims.

5. RECOMMEND ONE

Choose one candidate. Explain why it is the smallest coherent product proof and
why the others wait.

6. CLASSIFY THE PRODUCT DESIGN GATE

Return DESIGN_GATE_REQUIRED when the slice changes product ownership,
lifecycle, VST3 calls, product normalization, fixture meaning, process/security
boundary, or compatibility claim.

A waiver must positively prove none of those change.

7. REQUIRE A DEVELOPMENT PLAN

The recommended slice must include this sequence:

implement locally
→ deterministic validation
→ optional bounded diagnostic campaign
→ proof-harness maintenance without product redesign when needed
→ freeze AcceptanceCandidateIdentity
→ one strict acceptance transaction
→ audit and merge

Do not permit acceptance as the edit/debug loop.

8. FREEZE SEPARATE BUDGET OWNERS

State:

Windows producer budget owner: WindowsBuildInputIdentity
Diagnostic budget owner: DiagnosticCampaignIdentity
Acceptance budget owner: AcceptanceCandidateIdentity
Live negative budget owner: FaultPlanIdentity
Evidence render owner: EvidenceRendererIdentity + admitted acceptance result

Diagnostic and acceptance budgets must not consume one another.

For the open AGain fixture, diagnostic campaigns default to at most two live
batches. Acceptance defaults to one batch per exact candidate.

9. DRAFT THE SELECTION RECEIPT

Include:

repository
basis commit/tree
slice ID/title
primary claim
claim ceiling
exact fixture
product design gate
allowed paths
protected state
change_class: PRODUCT_CONTRACT_CHANGE
diagnostic_required
diagnostic_plan_id
diagnostic_batch_budget
acceptance_plan_id
acceptance_batch_budget: 1
windows_producer_budget
live_negative_budget
failure_closure: bounded diagnostic only
implementation_authorized: false
live_execution_authorized: false
successor_selection_authorized: false

10. OUTPUT

Return:

NEXT_SLICE_SELECTION_READY
or
NEXT_SLICE_SELECTION_BLOCKED
or
NEXT_SLICE_BLOCKED_BY_PROCESS_REPAIR

Then provide:

BASIS
ACCEPTED_FRONTIER
FIRST_UNPROVED_PRODUCT_EDGE
CANDIDATES
RECOMMENDATION
DESIGN_GATE
DEVELOPMENT_AND_DIAGNOSTIC_PLAN
SEPARATE_BUDGETS
DRAFT_SELECTION_RECEIPT
EXACT_OPERATOR_APPROVAL_SENTENCE

Keep the analysis bounded. Do not write an implementation prompt before an
approved design exists.
```
