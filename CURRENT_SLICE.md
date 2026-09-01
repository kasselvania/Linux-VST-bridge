# Current Slice: WR0A — Post-Merge Final WR0 Repair Reconciliation and Adoption

## Status

```text
status: implementation_authority_ready_for_merge
authority_phase: implementation
implementation_authorized: true
slice: WR0A
design_branch: codex/wr0a-final-repair-reconciliation-design
selection_basis_commit: b517f96bafa388107ea28c9d2e91528fd37b00d9
selection_basis_tree: 839cda6145f592642f06b5273855b04ffa0562ac
target: main
implementation_branch: codex/wr0a-final-repair-reconciliation
implementation_basis: exact future design-authority merge commit and tree
design_gate: cleared
successor_selection_authorized: false
```

Implementation may begin only after this design authority is merged into
`main`. The implementation branch must begin from that exact merge commit and
tree. The unmerged design branch and the original selection basis are not
implementation bases.

## Exact authority

Selection receipt:

```text
docs/slices/WR0A/SLICE_SELECTION.md
```

Immutable approved design:

```text
path: docs/slices/WR0A/IMPLEMENTATION_DESIGN.md
revision: wr0a-design-v3
commit: 19a1314ec3d750c01edeaf0fee6a5b4bc1aa6791
tree: 79bfdbbeeecb18893c5b35738fde7353aacf9c19
Git blob: 18759d2297b3e0bc0e69d0939f72391b373ce9ec
SHA-256: add191cf0dee588e9fce08c8b97f9b5a87aa6102eb3f5376d2aa2dfb4b455261
```

Adversarial-review history:

```text
path: docs/slices/WR0A/ADVERSARIAL_DESIGN_REVIEW.md
file blob before authority finalization: 9110f9ef24682f90eb1dca6bb98eb8a8dfdd7bb1
V3 DESIGN_CLEAR review: GitHub review 5080493577
superseded over-scoped V3 review: GitHub review 5080390572
```

Design approval:

```text
path: docs/slices/WR0A/DESIGN_APPROVAL.md
Git blob: 40179957b349390f8484e114b4d23dc14a7c1a13
implementation_authorized: true
```

The immutable V3 design does not need another revision. Review `5080493577`
correctly classifies WR0A as an exact Git reconciliation with a read-only live
fixture, not as a new live environment transaction.

## Primary claim

> Current main adopts the exact content-addressed final WR0 repair represented
> by archive commit `52be94316664f88a19df630e164105a0ca50b875`
> without re-running or mutating the live WR0 environment.

## Exact source and fixture

```text
merged historical WR0:
  commit: 9228217b2abf7314b9dfaecc5fc4323d5f3d7a89
  tree: 8da6817eba1f259d3565e377fcb50098ab8f3cf2
  merge: 8237b96ce7c885edcf4e7a0923f2ac78d05a928d

immutable final-repair archive:
  commit: 52be94316664f88a19df630e164105a0ca50b875
  tree: 21601814bd352114fe2e55833c88a81e47e13e41
  parent: 3deb414a54174cd95432c84e117a642f30c482fe

live WR0 environment:
  path: <HOME>/.local/share/linux-vst-bridge/environments/wr0-proton11
  transaction: wr0-20260901T053317Z-183afd5bbf0736e4
  environment identity: d3ed38d7ed53e9a5973cbf22fc504f2479dbdd15616fe1bfc1d5e1cd0bf5f4c4
  runner/runtime: 2d64df1d36786ca2d0e955c553005423dc2b5bdd714bd0a17872622e33912547
  contract source: c6543004fbcd70252393f0e0cab4f9ad7a31e85f433ce3e903e690244cc79541
  workload: 4518ca37b8d7e005f01b441de5273447b70fa7c8c9c3d2b9c194a91782cfecac
  retired record: 917c939c605f457376b0bdd53e06d252d4aecee4b39728850b2e1a61aa102b3e
```

The live environment is read-only in WR0A. It must not be repaired, replaced,
renamed, regenerated, or rerun.

## Implementation shape

The approved implementation has one tracked reconciliation owner:

```text
tools/wr0a-reconciliation/README.md
tools/wr0a-reconciliation/reconcile.py
```

It must:

1. verify the merged design authority and immutable archive;
2. commit the two reconciliation-tool files before they act;
3. import exactly 13 archive blobs and never archive `CURRENT_SLICE.md`;
4. preserve the exact 11 already-identical WR0 paths;
5. reproduce the final WR0 contract-source digest `c6543004...`;
6. retain the exact canonical final WR0 evidence packet;
7. perform bounded read-only live/archive equality checks;
8. create a separate eight-file WR0A reconciliation packet;
9. preserve DG0 and all immutable authority records;
10. obtain an independent pre-PR audit before technical-lead merge.

Unexpected disposable implementation-worktree state is not a live product
recovery problem. Stop, preserve unknown work, and recreate an owned clean
implementation worktree from the exact design-authority merge rather than
expanding WR0A into another transaction framework.

## Exact implementation changed paths: 23

```text
docs/WR0_RUNNER_LOCK.md
tools/wr0-proton-bootstrap/README.md
tools/wr0-proton-bootstrap/launch.py
evidence/wr0-proton-bootstrap/BASIS.md
evidence/wr0-proton-bootstrap/ENVIRONMENT.md
evidence/wr0-proton-bootstrap/EXIT_PROPAGATION.md
evidence/wr0-proton-bootstrap/LAUNCH_CONTRACT.md
evidence/wr0-proton-bootstrap/NEGATIVE_TESTS.md
evidence/wr0-proton-bootstrap/PROCESS_GUARDS.md
evidence/wr0-proton-bootstrap/RUN_1.md
evidence/wr0-proton-bootstrap/RUN_2.md
evidence/wr0-proton-bootstrap/fixture.json
evidence/wr0-proton-bootstrap/hashes.sha256
tools/wr0a-reconciliation/README.md
tools/wr0a-reconciliation/reconcile.py
evidence/wr0a-final-repair-reconciliation/BASIS.md
evidence/wr0a-final-repair-reconciliation/SOURCE_DELTA.md
evidence/wr0a-final-repair-reconciliation/ARCHIVE_AUDIT.md
evidence/wr0a-final-repair-reconciliation/LIVE_READBACK.md
evidence/wr0a-final-repair-reconciliation/GOVERNANCE.md
evidence/wr0a-final-repair-reconciliation/FINDINGS.md
evidence/wr0a-final-repair-reconciliation/fixture.json
evidence/wr0a-final-repair-reconciliation/hashes.sha256
```

The implementation PR may not modify this file, the selection receipt,
reconnaissance, design card, adversarial-review record, design approval,
`docs/DECISION_REGISTER.md`, or any other governance/architecture path.

## External mutation

Permitted:

```text
owned implementation Git worktree/index/commits
read-only inspection of the exact live WR0 environment
bounded WR0A transient cache that is removed after use
configured branch push and ordinary non-draft pull request metadata
```

Prohibited:

- Proton, Wine, Windows workload, Bitwig, Serum, or validator execution;
- live WR0 environment or receipt mutation;
- `.wine`, Steam compatdata, Flatpak, runner/runtime, or vendor mutation;
- archive `CURRENT_SLICE.md` adoption;
- another slice or product capability;
- implementation-agent merge.

## Material-discovery stop law

Return to the design gate only if implementation changes a material approved
fact: archive/source identity, live fixture, owner model, exact import set,
read-only boundary, implementation path envelope, primary claim, claim ceiling,
or proof ownership.

Ordinary implementation defects within the approved V3 design are repaired
under that design. The implementation must not grow new Git-runtime, lease,
filesystem-transaction, or governance frameworks merely to defend disposable
unaccepted worktree state.

## Completion and closure

The implementation agent opens an ordinary non-draft PR and leaves it
unmerged. After independent pre-PR audit and technical-lead exact-head review,
the technical lead may merge. A separate status closure then changes only:

```text
CURRENT_SLICE.md
docs/DECISION_REGISTER.md
```

That closure records WR0A acceptance and restores no-active-slice posture.

## Nonclaims

WR0A does not create a new WR0 execution result, run or replace the environment,
load or scan a Windows VST3, operate Serum or Bitwig, process audio, implement a
proxy, bridge, IPC, shared memory, manager, broker, GUI, activation flow, Rust
component, real-time path, final product runner, Steam-independent distribution,
or general Linux compatibility.
