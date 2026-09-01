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
