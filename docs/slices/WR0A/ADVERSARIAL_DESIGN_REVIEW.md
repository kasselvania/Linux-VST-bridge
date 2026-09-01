# WR0A adversarial design review

## Reviewed design identity

This file materializes the technical lead's supplied review. The design agent
is recording that external verdict; it is not claiming to have independently
reviewed its own design.

```text
reviewed_design_commit:
  fce6078ab3c6e7ae9ad8bf116daefb62682ee093
reviewed_design_tree:
  a8cfe44a08781c6bcbf7c5057fb2d3c47e108c8d
reviewed_design_path:
  docs/slices/WR0A/IMPLEMENTATION_DESIGN.md
reviewed_design_blob:
  2d4e2c662e616461b233d398a3e40c5e3e248573
reviewed_design_sha256:
  a822d2671b253f386721f3756f684a6065072c353ea4d98e64c1bcd82f85d939
reviewed_revision:
  wr0a-design-v1
review_result:
  DESIGN_REPAIR_REQUIRED
implementation_authorized:
  false
```

## Finding 1 — design merge and implementation basis conflict (P1)

- Violated invariant: reviewed and approved design authority must merge before
  implementation begins, and implementation must start from that accepted
  design-authority basis.
- Concrete failure state: v1 allowed a cumulative implementation PR from
  `b517f96...`, making either unmerged-design implementation or a wrong basis
  unavoidable.
- Why v1 proof missed it: its state machine jumped from design approval to an
  implementation branch without a durable design-authority merge state.
- Required revision: add `DESIGN_V2_REVIEWED_CLEAR`,
  `DESIGN_APPROVAL_RECORDED`, `DESIGN_AUTHORITY_MERGED`, and
  `IMPLEMENTATION_BRANCH_CREATED`; make the future merge commit/tree the sole
  implementation basis.
- Adjacent operations under the same audit: branch creation, immutable
  authority records, implementation path envelope, PR base, and status
  closure.
- Reconnaissance required: false.

## Finding 2 — design-card review status is self-invalidating (P1)

- Violated invariant: an exact reviewed design blob must remain immutable.
- Concrete failure state: changing v1's `pending` review/approval fields after
  review would create an unreviewed blob; leaving them unchanged would conflict
  with later authority.
- Why v1 proof missed it: the card itself was assigned ownership of mutable
  phase disposition.
- Required revision: keep v2 permanently
  `proposed_for_adversarial_review`; make the review file, approval receipt,
  and `CURRENT_SLICE.md` the only later phase-state owners.
- Adjacent operations under the same audit: exact review identity, approval
  identity, current-slice transition, and implementation preflight.
- Reconnaissance required: false.

## Finding 3 — no tracked reconciliation owner (P1)

- Violated invariant: consequential production behavior and its verification
  require a rerunnable repository-owned implementation owner.
- Concrete failure state: v1 left exact import, live inspection, evidence
  rendering, and negative tests in an ephemeral session script.
- Why v1 proof missed it: it prohibited a tool path while naming abstract
  owners with no durable interface.
- Required revision: authorize only
  `tools/wr0a-reconciliation/README.md` and
  `tools/wr0a-reconciliation/reconcile.py`, with fixed bounded modes and no
  arbitrary normal-mode identities or paths.
- Adjacent operations under the same audit: authority verification, adoption
  planning/staging, candidate verification, read-only live inspection,
  evidence rendering, and negative tests.
- Reconnaissance required: false.

## Finding 4 — WR0A implementation source is not identity-bound (P1)

- Violated invariant: the implementation used to produce evidence must remain
  exact across an evidence-only amendment.
- Concrete failure state: v1 verified only the ten-file WR0 contract source;
  a changed WR0A reconciler could retain the same `c6543004...` result.
- Why v1 proof missed it: it conflated imported WR0 source identity with the
  new reconciliation implementation identity.
- Required revision: define
  `linux-vst-bridge-wr0a-implementation-source/v1` over the two WR0A tool
  files and reproduce it at the final evidence-only head.
- Adjacent operations under the same audit: clean intermediate commit, import
  manifest, DG0/design-authority protection, evidence amendment, and final-head
  verification.
- Reconnaissance required: false.

## Finding 5 — acceptance was scheduled before acceptance (P1)

- Violated invariant: only the merged, reviewed implementation may become an
  accepted decision.
- Concrete failure state: v1 modified `docs/DECISION_REGISTER.md` inside the
  candidate implementation PR and thereby predeclared acceptance.
- Why v1 proof missed it: its mutation ledger merged candidate truth with
  post-merge status closure.
- Required revision: prohibit Decision Register edits in the implementation
  PR; append the accepted result only in a separate post-merge status closure.
- Adjacent operations under the same audit: implementation PR path envelope,
  technical-lead merge readback, `CURRENT_SLICE.md` closure, and successor
  selection.
- Reconnaissance required: false.

## Finding 6 — read-only live call graph was unspecified (P1)

- Violated invariant: read-only fixture evidence needs an exact callable owner
  and a mechanically enforceable non-mutation boundary.
- Concrete failure state: v1 referred to an accepted inspector even though the
  same module also owns workload launch, writes, publication, deletion, and
  environment replacement.
- Why v1 proof missed it: it named a module, not the exact allowed functions,
  arguments, before/after identities, and prohibited call set.
- Required revision: verify exact blob `215718...`, import through a fixed
  local-file loader, allow only named read-only functions, add a WR0A-owned
  read-only wrapper and transient-cache law, and reject any disallowed call.
- Adjacent operations under the same audit: process/sibling guards, runner and
  contract verification, marker/snapshot/receipt/record reads, protected
  fixtures, cache cleanup, and no-live-mutation proof.
- Reconnaissance required: false.

## Finding 7 — proof matrix was over-aggregated (P1)

- Violated invariant: independently fallible ownership boundaries require
  distinct positive and negative proofs.
- Concrete failure state: nine broad rows allowed path rosters, blob/mode
  refusals, source identities, live joins, governance, and publication to hide
  behind aggregate success.
- Why v1 proof missed it: it organized proof by narrative topic rather than
  production transition and failure owner.
- Required revision: provide at least 43 separate rows, each naming production
  owner/helper, fixture, positive proof, negative/fault proof, retained
  evidence, and claim ceiling.
- Adjacent operations under the same audit: all exact source, adoption,
  evidence, live-readback, governance, Git-publication, and closure boundaries.
- Reconnaissance required: false.

## Finding 8 — final-head evidence ownership was circular (P2)

- Violated invariant: a committed evidence packet cannot contain its own final
  Git commit identity as an authoritative field.
- Concrete failure state: amending evidence to record the final head creates a
  new final head indefinitely.
- Why v1 proof missed it: it did not separate payload-owned provenance from
  Git/PR/merge readback authority.
- Required revision: retain the clean implementation commit and source/payload
  manifests inside evidence; assign final PR head to branch/PR and independent
  review, and merge/acceptance identities to merge readback and status closure.
- Adjacent operations under the same audit: evidence rendering, hash manifest,
  final-head audit, PR review, merge readback, and decision entry.
- Reconnaissance required: false.

## Disposition

The eight findings are one cross-invariant repair: the approved design,
reconciliation implementation, evidence, implementation basis, and acceptance
status each require one durable owner and a non-circular transition order.
Revision v2 must receive a fresh independent review. This v1 verdict does not
authorize implementation or design approval.

---

## V2 adversarial design review

This section materializes the technical lead's GitHub review of revision v2.
It is distinct from, and does not alter, the v1 review above. The design agent
is recording the supplied review; it is not reviewing its own design.

```text
reviewed_design_commit:
  2aeac55f0297b5d8c6b843eb5ce489d936999923
reviewed_design_tree:
  6be95aaca2eddf8008a0177bf07bac48921133c0
reviewed_design_path:
  docs/slices/WR0A/IMPLEMENTATION_DESIGN.md
reviewed_design_blob:
  9375172252f6dca73636916536a1bcbd8641115d
reviewed_design_sha256:
  0ccf184612d7312334e65658181f1bb32ae5d3ddb847cb8bab042d470533b15e
reviewed_revision:
  wr0a-design-v2
review_result:
  DESIGN_REPAIR_REQUIRED
github_review_id:
  5080009434
implementation_authorized:
  false
```

### V2 finding 1 — selection authority identity is stale (P1)

- Violated invariant: every authority identity must name the exact bytes the
  design consumes.
- Concrete failure state: v2 named the original selection receipt blob
  `fe7b3f84...`, while the reviewed v2 tree contains the clarified receipt at
  blob `4f3131c9...`.
- Why v2 proof missed it: the lifecycle clarification changed the receipt but
  the design's identity ledger was not remeasured against the reviewed tree.
- Required v3 revision: bind the current selection receipt's exact path, mode,
  blob, and SHA-256; retain its lineage from the original operator receipt; and
  freeze the six-path design-authority finalization envelope.
- Adjacent operations receiving the same audit: v3 review, operator approval,
  implementation-authority update, design merge, and implementation preflight.
- Reconnaissance required: false.

### V2 finding 2 — the reconciler cannot bootstrap under its own clean-worktree law (P1)

- Violated invariant: the production reconciliation owner must be committed
  and source-bound before it acts.
- Concrete failure state: v2 ran `verify-authority`, `plan`, and
  `stage-adoption` while the two tool files were still untracked, even though
  those modes require a clean worktree and reject extra untracked files.
- Why v2 proof missed it: its state machine combined tool creation and archive
  adoption in one clean implementation commit after the tool had already run.
- Required v3 revision: create a clean tool-only commit first, measure and test
  its two-file source identity, run that committed tool to stage the archive,
  create a separate adoption commit, and add evidence in a third commit.
- Adjacent operations receiving the same audit: implementation basis,
  clean-worktree preflight, source manifest, production negative tests,
  adoption commit, evidence-only commit, and final-head verification.
- Reconnaissance required: false.

### V2 finding 3 — live readback lacks a durable evidence handoff (P1)

- Violated invariant: every transient input and output needs one explicit
  transaction owner and one restart law.
- Concrete failure state: v2 removed the `inspect-live` cache before the later
  `render-evidence` mode, leaving the renderer either to consume an undefined
  result or silently repeat the inspection.
- Why v2 proof missed it: it specified cache creation and deletion but not the
  authoritative input to retained evidence publication.
- Required v3 revision: make standalone `inspect-live` diagnostic-only; make
  `render-evidence` own the sole retained inspection, rendering, validation,
  publication, and cleanup within a unique marker-bound session with explicit
  physical states, rollback, cleanup blocker, and restart behavior.
- Adjacent operations receiving the same audit: process/sibling guards,
  before/after fixture identity, packet staging, sanitization, partial publish,
  cache cleanup, and evidence-only commit.
- Reconnaissance required: false.

### V2 finding 4 — ordinary module import may mutate protected source (P1)

- Violated invariant: a read-only dispatcher may not create source-tree
  artifacts outside the exact implementation envelope.
- Concrete failure state: an ordinary `importlib` load of archived `launch.py`
  may create `tools/wr0-proton-bootstrap/__pycache__/launch.*.pyc`.
- Why v2 proof missed it: the call allowlist constrained invoked functions but
  not Python's import-time bytecode-cache behavior.
- Required v3 revision: set `sys.dont_write_bytecode = True` before loading,
  use a fixed non-`__main__` module name, and prove no `__pycache__`, `.pyc`, or
  other source-tree side effect before and after the production dispatcher.
- Adjacent operations receiving the same audit: module loading, source status,
  implementation path envelope, dispatcher tests, and transient cleanup.
- Reconnaissance required: false.

### V2 finding 5 — archive staging recovery lacks syscall-boundary proof (P1)

- Violated invariant: Git-local adoption must restore the exact old worktree
  and index at every fallible write/mode/index/readback boundary.
- Concrete failure state: v2 promised partial-failure recovery without tests
  after the Nth file write, Nth mode update, Nth index update, or final index
  readback.
- Why v2 proof missed it: the proof matrix tested aggregate dirty-state
  refusal, not production-helper recovery across individual mutation seams.
- Required v3 revision: add narrow production-helper injection at all four
  seams and prove all thirteen old blobs/modes plus the original clean index
  are restored, unknown paths are preserved, and no partial import remains.
- Adjacent operations receiving the same audit: adoption journal ownership,
  literal blob writes, executable modes, index entries, post-stage verifier,
  restart recovery, and cleanup failure classification.
- Reconnaissance required: false.

### V2 finding 6 — post-adoption archive dependency is ambiguous (P2)

- Violated invariant: a mode's source prerequisites and long-term provenance
  claim must be explicit at each lifecycle phase.
- Concrete failure state: v2 simultaneously said current main would not depend
  on the archive branch and described modes that require the archive commit,
  without stating when that requirement ends or how the object stays reachable
  through acceptance.
- Why v2 proof missed it: it described final material independence but not the
  one-shot/pre-acceptance/post-closure mode lifecycle.
- Required v3 revision: classify archive-dependent one-shot modes, retain an
  exact provenance ref through status closure, define post-adoption verifiers
  over retained ledgers/current blobs, and avoid claiming a clean main-only
  clone can re-read an unavailable archive object.
- Adjacent operations receiving the same audit: authority verification,
  adoption planning/staging, candidate verification, pre-PR audit, status
  closure, retained provenance ledger, and later clean-clone verification.
- Reconnaissance required: false.

## V2 cross-invariant disposition

The six findings are one boundary class: the reconciliation owner must be
committed before it acts, every transient input/output must have one
transaction owner, and every authority/source identity must name the exact
bytes actually used. Revision v3 must receive a fresh independent review.
This v2 verdict does not authorize implementation or design approval.
