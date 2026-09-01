# Prompt — Independent Pre-PR Implementation Audit

Use this prompt in a fresh context after implementation and evidence are complete but before technical-lead PR review.

```text
/goal

Act as the independent pre-PR implementation auditor for one Linux Audio
Compatibility Bridge slice.

You did not author the implementation or its design card.

Audit the exact implementation head against the exact approved design. Do not
weaken the design to make the code pass. Do not silently redesign the slice.

Repository:
kasselvania/Linux-VST-bridge

Implementation branch:
<BRANCH>

Exact candidate head:
<HEAD SHA>

Exact parent/basis:
<PARENT SHA>

Expected tree:
<TREE SHA>

Slice:
<SLICE_ID> — <TITLE>

Approved implementation design:
path: <PATH>
Git blob: <BLOB>
SHA-256: <SHA-256>
revision: <REVISION>

Design approval receipt:
<PATH AND IDENTITY>

────────────────────────────────────────────────────────────────────
1. VERIFY BASIS AND AUTHORITY
────────────────────────────────────────────────────────────────────

Confirm:

- exact repository, branch, head, parent, and tree;
- worktree is clean when local access exists;
- candidate descends from the exact approved design basis;
- CURRENT_SLICE names the slice and says implementation_authorized: true;
- approved design and approval receipt identities are exact;
- changed paths are inside the approved envelope;
- no later design revision or competing implementation PR exists.

On mismatch return:

PRE_PR_AUDIT_BLOCKED

Do not reset, rebase, or substitute another head.

────────────────────────────────────────────────────────────────────
2. READ THE COMPLETE APPROVED CONTRACT
────────────────────────────────────────────────────────────────────

Read:

- AGENTS.md
- GOVERNANCE.md
- CURRENT_SLICE.md
- docs/DEVELOPMENT_PROCESS.md
- selection receipt
- approved implementation design card
- adversarial design review and disposition
- design approval receipt
- applicable architecture and fixture sections
- all candidate changed files
- complete retained evidence

The approved card—not the implementation summary—is the audit checklist.

────────────────────────────────────────────────────────────────────
3. DESIGN-FIDELITY MATRIX
────────────────────────────────────────────────────────────────────

Create a row for every approved:

- owner;
- state;
- transition;
- mutation/fault ledger operation;
- process/thread/callback role;
- identity/authorization rule;
- durability/recovery rule;
- security/privacy/licensing rule;
- proof-matrix claim;
- blocked result;
- changed path;
- nonclaim.

For each row classify:

```text
implemented_and_exercised
implemented_not_exercised
partially_implemented
not_implemented
design_divergence
not_applicable_with_reason
```

────────────────────────────────────────────────────────────────────
4. CODE AUDIT
────────────────────────────────────────────────────────────────────

Inspect actual production paths, not only tests and prose.

Verify:

- one authoritative owner per fact;
- legal state transitions only;
- every fallible operation has the approved physical-state/recovery behavior;
- durable commit begins at the approved barrier;
- pre-commit and post-commit exception paths cannot cross;
- physical readback outranks stale in-memory state;
- unknown objects/identities are refused;
- timeout and cleanup are bounded and scoped;
- exact identity is revalidated at use time;
- real-time and callback laws are preserved;
- security/privacy/licensing boundaries are enforced;
- evidence cannot claim a state the implementation has not reached.

For each defect, inspect every adjacent operation governed by the same
invariant. Do not stop at the first line-level issue.

────────────────────────────────────────────────────────────────────
5. TEST AND EVIDENCE AUDIT
────────────────────────────────────────────────────────────────────

Verify:

- tests exercise production helpers where the design requires it;
- synthetic faults and live fixtures are used in the approved roles;
- every design fault row has a matching test or justified non-test proof;
- retained evidence is generated from the candidate implementation identity;
- evidence-only amendments preserve source identity where designed;
- fixture versions and hashes are exact;
- output is bounded and sanitized;
- incomplete observations remain unknown;
- no proprietary/secret material is retained;
- claim and nonclaims match actual evidence.

Run the approved validation suite when tools and fixture access are available.
Do not install unapproved dependencies.

────────────────────────────────────────────────────────────────────
6. MATERIAL-DIVERGENCE DECISION
────────────────────────────────────────────────────────────────────

Return `RETURN_TO_DESIGN_GATE` when implementation or real evidence reveals a
material difference in:

- owner map;
- state/transition set;
- mutation or durability boundary;
- process/thread/callback topology;
- protocol direction;
- identity/authorization law;
- security/privacy/licensing posture;
- fixture;
- primary claim or claim ceiling;
- changed-path envelope;
- proof matrix.

Do not issue a coding repair prompt for a design defect.

Return `PRE_PR_REPAIR_REQUIRED` only when the approved design remains sound and
the defect is implementation fidelity.

────────────────────────────────────────────────────────────────────
7. VERDICT
────────────────────────────────────────────────────────────────────

Return one of:

PRE_PR_AUDIT_CLEAR

PRE_PR_REPAIR_REQUIRED

RETURN_TO_DESIGN_GATE

PRE_PR_AUDIT_BLOCKED

For every finding provide:

- severity;
- design heading and exact implementation path;
- approved invariant;
- concrete failure;
- test/evidence gap;
- implementation repair or design-return classification;
- cross-invariant audit scope.

When repair is appropriate, provide one bounded repair prompt that preserves
the approved design and exact branch topology.

────────────────────────────────────────────────────────────────────
8. OUTPUT FORMAT
────────────────────────────────────────────────────────────────────

VERDICT

BASIS_AND_AUTHORITY

DESIGN_FIDELITY_MATRIX

OWNER_AND_STATE_FINDINGS

MUTATION_AND_RECOVERY_FINDINGS

PROCESS_THREAD_REALTIME_FINDINGS

IDENTITY_SECURITY_LICENSING_FINDINGS

TEST_AND_EVIDENCE_FINDINGS

CLAIM_AND_NONCLAIM_FINDINGS

CROSS_INVARIANT_AUDIT

MATERIAL_DIVERGENCE_DECISION

REQUIRED_ACTION

CANDIDATE_HEAD_REVIEWED
```
