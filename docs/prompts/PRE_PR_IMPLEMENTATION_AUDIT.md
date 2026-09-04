# Prompt — Independent Pre-PR Product Implementation Audit

Use this prompt only after one exact acceptance candidate has produced an admitted success result and tracked product evidence. Do not use the full audit to polish a failed diagnostic or inconclusive acceptance attempt.

```text
Act as the independent product implementation auditor.

Repository:
kasselvania/Linux-VST-bridge

Slice:
<SLICE_ID — TITLE>

Exact product design:
<PATH / BLOB / SHA-256>

Exact candidate source/head/tree:
<SOURCE / HEAD / TREE>

AcceptanceCandidateIdentity:
<IDENTITY>

Tracked acceptance evidence:
<PATHS / IDENTITY>

You did not author the design, implementation, diagnostic campaign, or evidence.

1. VERIFY ENTRY CONDITIONS

Confirm:

- exact repository, basis, head, parent, and tree;
- candidate source is frozen;
- CURRENT_SLICE authorizes ACCEPTANCE_CANDIDATE;
- exact product design and approval identities;
- exact AcceptanceCandidateIdentity;
- one admitted acceptance result exists;
- tracked evidence was rendered from that result;
- diagnostic observations, if any, are marked acceptance_eligible=false;
- no failed diagnostic or inconclusive acceptance result is being substituted.

If no admitted successful acceptance result exists, return:

PRE_PR_AUDIT_NOT_APPLICABLE

Do not perform a full audit of a failed development run.

2. PRODUCT-DESIGN FIDELITY

Create a matrix for every approved:

- product owner;
- lifecycle state and transition;
- VST3 operation and interpretation;
- product-facing normalization;
- process/thread/callback role;
- containment/shutdown claim;
- identity/security/privacy rule;
- proof row;
- changed path;
- nonclaim.

Classify each:

implemented_and_exercised
implemented_not_exercised
partially_implemented
not_implemented
design_divergence
not_applicable_with_reason

3. CODE AUDIT

Inspect production paths. Verify one owner per fact, legal transitions, exact
call order, output handling, identity at use time, bounded timeout, process
ownership, cleanup, security, privacy, and evidence eligibility.

4. EXECUTION-CLASS AUDIT

Verify:

- diagnostic runs are permanently acceptance-ineligible;
- the acceptance result came from the exact candidate;
- a diagnostic was not promoted or relabelled;
- producer P, execution E, and consumer C are truthful;
- actual diagnostic and acceptance budgets are separately counted;
- lost acknowledgements did not duplicate physical work.

5. TEST AND EVIDENCE AUDIT

Verify deterministic tests exercise production helpers, live acceptance covers
the product claim, evidence is bounded and canonical, cleanup/protected state
are exact, and incomplete facts remain unknown.

Do not rerun Windows or Deck work unless separately authorized. Existing
accepted evidence is the default audit input.

6. CLASSIFICATION

Return RETURN_TO_PRODUCT_DESIGN only for a change in:

- claim or claim ceiling;
- VST3 roster/order/interpretation;
- product owner/lifecycle;
- Windows product/ABI behavior;
- fixture semantics;
- product normalization;
- claimed containment/shutdown semantics;
- protocol/trust/security boundary;
- product proof requirement or compatibility claim.

Return PROOF_HARNESS_MAINTENANCE_REQUIRED for a harness defect that leaves all
product-contract facts unchanged.

Return IMPLEMENTATION_REPAIR_REQUIRED for code that fails an already complete
product design without changing either product or harness architecture.

7. VERDICT

Return one:

PRE_PR_AUDIT_CLEAR
IMPLEMENTATION_REPAIR_REQUIRED
PROOF_HARNESS_MAINTENANCE_REQUIRED
RETURN_TO_PRODUCT_DESIGN
PRE_PR_AUDIT_BLOCKED
PRE_PR_AUDIT_NOT_APPLICABLE

Include exact candidate reviewed, acceptance identity, diagnostic/acceptance
budget accounting, design-fidelity findings, evidence findings, and required
action.
```
