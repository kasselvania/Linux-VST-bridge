# Prompt — Adversarial Implementation Design Review

Use this prompt in a fresh context that did not author the implementation design card.

```text
/goal

Act as the independent adversarial design reviewer for one selected slice of
the Linux Audio Compatibility Bridge project.

Review design only. Do not implement code, edit the repository, launch a live
fixture, or invent a replacement architecture without identifying the exact
defect in the proposed design.

Repository:
kasselvania/Linux-VST-bridge

Pinned design-review basis:
commit: <EXACT COMMIT>
tree:   <EXACT TREE>

Slice:
<SLICE_ID> — <TITLE>

Selection receipt:
<PATH AND IDENTITY>

Implementation design card:
<PATH>
expected Git blob: <BLOB>
expected SHA-256: <SHA-256>
expected revision: <REVISION>

────────────────────────────────────────────────────────────────────
1. VERIFY AUTHORITY
────────────────────────────────────────────────────────────────────

Confirm:

- repository and exact basis;
- CURRENT_SLICE names this slice;
- authority_phase is reconnaissance_and_design;
- implementation_authorized is false;
- selection receipt matches the card's claim, fixture, path, and mutation
  envelope;
- design card path/blob/SHA/revision are exact;
- no implementation diff is being smuggled into the design review.

On mismatch return:

DESIGN_REVIEW_BLOCKED

Do not repair authority implicitly.

────────────────────────────────────────────────────────────────────
2. READ THE APPLICABLE AUTHORITY
────────────────────────────────────────────────────────────────────

Read:

- AGENTS.md
- GOVERNANCE.md
- CURRENT_SLICE.md
- docs/DEVELOPMENT_PROCESS.md
- the selection receipt
- the complete implementation design card
- applicable architecture sections
- applicable fixture cards and research basis
- accepted predecessor implementation/evidence needed by this design

Do not use the full dossier as implementation authority.

────────────────────────────────────────────────────────────────────
3. OWNER AUDIT
────────────────────────────────────────────────────────────────────

For every consequential fact ask:

- Is there exactly one authoritative owner?
- Who creates, mutates, reads, retires, and recovers it?
- Is durable representation distinct from cached/read-model state?
- Can an in-memory flag, path name, process name, UI state, test result, or
  evidence record compete with the real owner?
- What happens when physical state and memory disagree?

Report missing, overlapping, circular, or implicit owners.

────────────────────────────────────────────────────────────────────
4. COMPLETE STATE-SPACE AUDIT
────────────────────────────────────────────────────────────────────

Enumerate the proposed states independently.

For each pair of adjacent fallible operations ask:

> If operation N succeeds and operation N+1 fails, what exact physical state
> exists and which named state represents it?

Inspect:

- rename before parent fsync;
- write before file/directory fsync;
- spawn before ownership proof;
- publish before readback;
- promote before commit;
- commit before retirement;
- deletion before absence durability;
- callback before acknowledgement;
- authorization before persistence;
- host save before project commit;
- evidence rendering before publication.

Report every externally observable intermediate state missing from the card.

Test crash, reboot, cancellation, retry, duplicate invocation, stale process
identity, and concurrent actor behavior at every durable/intermediate state.

────────────────────────────────────────────────────────────────────
5. MUTATION AND FAULT AUDIT
────────────────────────────────────────────────────────────────────

For every mutation-ledger row verify:

- precondition is exact;
- operation is bounded;
- immediate physical result is stated;
- durability barrier is sufficient;
- readback checks the right identity;
- next authority state begins at the correct point;
- failure classification names the correct owner;
- recovery cannot destroy both predecessor and replacement;
- unknown objects are refused rather than adopted;
- post-commit failure never enters pre-commit rollback;
- cleanup is idempotent or explicitly non-repeatable.

Perform a cross-invariant scan: when one defect is found, inspect every other
operation governed by the same invariant.

────────────────────────────────────────────────────────────────────
6. PROCESS, THREAD, CALLBACK, AND REAL-TIME AUDIT
────────────────────────────────────────────────────────────────────

Where applicable inspect:

- real observed versus assumed topology;
- parent, ancestry, process group, session, namespace, and lifetime;
- exact process/object identity and stale-ID protection;
- escaped descendants and unrelated same-name protection;
- timeout and final empty proof;
- thread affinity;
- callback direction and recursive/reentrant calls;
- lock ordering;
- bounded waits;
- allocation/logging/filesystem/network behavior on real-time paths;
- editor versus processor failure ownership;
- shutdown while callbacks are outstanding.

If real topology is not yet knowable, require bounded reconnaissance instead of
allowing the design to pretend.

────────────────────────────────────────────────────────────────────
7. IDENTITY AND AUTHORIZATION AUDIT
────────────────────────────────────────────────────────────────────

For every dangerous operation verify that the complete identity is sufficient.
Reject designs that rely on:

- friendly name;
- basename or path suffix;
- PID alone;
- path alone;
- ambient latest version;
- one in-memory boolean;
- vendor/product family where exact build/class is required;
- successful stdout without process/owner proof.

Inspect machine identity, licensing, browser/deep-link callbacks, recurring
verification, content roots, and secret handling where applicable.

────────────────────────────────────────────────────────────────────
8. SECURITY, PRIVACY, LICENSING, AND CLEAN-ROOM AUDIT
────────────────────────────────────────────────────────────────────

Verify:

- untrusted process boundaries;
- path containment and symlink law;
- command/profile injection resistance;
- diagnostic allow lists and redaction;
- no credentials, tokens, license material, account state, proprietary
  binaries, presets, or paid content in Git/evidence;
- no unsupported DRM or authorization bypass;
- exact third-party license/distribution obligations;
- no accidental yabridge/GPL source copying;
- no false sandbox claim.

────────────────────────────────────────────────────────────────────
9. PROOF-MATRIX AUDIT
────────────────────────────────────────────────────────────────────

For every primary and supporting claim verify:

- positive proof matches the claim;
- negative/fault proof reaches the production owner;
- synthetic versus live fixture choice is justified;
- test names do not substitute for behavior;
- output, screenshot, or log alone is not overclaimed;
- incomplete/capped observations propagate to unknown;
- evidence can be regenerated without rewriting history;
- the claim ceiling blocks adjacent inference.

Identify any test that merely restates the condition in a parallel toy helper.

────────────────────────────────────────────────────────────────────
10. CODE-TOPOLOGY AUDIT
────────────────────────────────────────────────────────────────────

Verify the proposed code topology keeps owners distinct. One source file may
contain multiple early-proof owners, but their interfaces and state must remain
explicit.

Report any component that would implicitly own several unrelated concerns such
as runtime selection, process supervision, durable transaction state, evidence
rendering, and UI policy without an approved boundary.

────────────────────────────────────────────────────────────────────
11. MATERIAL-DISCOVERY AND STOP-LAW AUDIT
────────────────────────────────────────────────────────────────────

Verify that the card names the observations that would invalidate it and that
implementation must stop rather than patch forward.

Add missing slice-specific stop conditions.

────────────────────────────────────────────────────────────────────
12. VERDICT
────────────────────────────────────────────────────────────────────

Return one of:

DESIGN_CLEAR

DESIGN_REPAIR_REQUIRED

DESIGN_REVIEW_BLOCKED

A clear design must have no unresolved P0/P1 finding and no material unknown
masquerading as design fact.

For every finding provide:

- severity: P0 / P1 / P2;
- exact design heading;
- violated invariant;
- concrete failure state;
- why existing proof would miss it;
- required design change;
- every adjacent operation that must receive the same cross-invariant audit;
- whether reconnaissance is required.

Do not provide implementation patches.

────────────────────────────────────────────────────────────────────
13. OUTPUT FORMAT
────────────────────────────────────────────────────────────────────

VERDICT

AUTHORITY_CHECK

OWNER_FINDINGS

STATE_SPACE_FINDINGS

MUTATION_AND_FAULT_FINDINGS

PROCESS_THREAD_REALTIME_FINDINGS

IDENTITY_AUTHORIZATION_FINDINGS

SECURITY_PRIVACY_LICENSING_FINDINGS

PROOF_MATRIX_FINDINGS

CODE_TOPOLOGY_FINDINGS

MATERIAL_DISCOVERY_FINDINGS

CROSS_INVARIANT_AUDIT

REQUIRED_DESIGN_REVISIONS

RECONNAISSANCE_REQUIRED

APPROVAL_RECOMMENDATION

REVIEWED_DESIGN_IDENTITY
```
