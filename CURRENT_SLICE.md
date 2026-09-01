# Current Slice: WR0A — Post-Merge Final WR0 Repair Reconciliation and Adoption

## Status

```text
status: active_design_slice
authority_phase: reconnaissance_and_design
implementation_authorized: false
slice: WR0A
branch: codex/wr0a-final-repair-reconciliation-design
basis commit: b517f96bafa388107ea28c9d2e91528fd37b00d9
basis tree: 839cda6145f592642f06b5273855b04ffa0562ac
target: main
design_gate: required
successor_selection_authorized: false
```

WR0A owns read-only reconciliation and implementation design for one exact
post-merge WR0 event. It does not authorize source adoption, evidence
replacement, live-environment mutation, workload execution, or another slice.

## Authority receipts

The operator explicitly approved selecting WR0A and replacing the
no-active-slice card with this bounded reconnaissance-and-design authority.
The selection receipt is:

```text
docs/slices/WR0A/SLICE_SELECTION.md
```

The implementation design is:

```text
docs/slices/WR0A/IMPLEMENTATION_DESIGN.md
```

The supplied technical-lead reviews of design revisions v1 and v2 are
materialized at:

```text
docs/slices/WR0A/ADVERSARIAL_DESIGN_REVIEW.md
```

Revisions v1 and v2 received `DESIGN_REPAIR_REQUIRED`. Revision v3 is proposed
for a fresh independent adversarial review. Implementation still requires a
`DESIGN_CLEAR` record against the exact immutable v3 blob, a separate operator
approval receipt, an implementation-authority update to this card, and merge
of that design authority into main. No implementation prompt, archived branch,
live fixture, or apparent simplicity substitutes for those transitions.

## Exact basis and reconciliation sources

Current accepted main:

```text
commit: b517f96bafa388107ea28c9d2e91528fd37b00d9
tree:   839cda6145f592642f06b5273855b04ffa0562ac
```

Merged WR0 historical implementation:

```text
commit: 9228217b2abf7314b9dfaecc5fc4323d5f3d7a89
tree:   8da6817eba1f259d3565e377fcb50098ab8f3cf2
merge:  8237b96ce7c885edcf4e7a0923f2ac78d05a928d
```

Immutable final-repair source:

```text
GitHub branch: archive/wr0-final-repair-52be943
commit:        52be94316664f88a19df630e164105a0ca50b875
tree:          21601814bd352114fe2e55833c88a81e47e13e41
parent:        3deb414a54174cd95432c84e117a642f30c482fe
```

The technical lead established GitHub source authority. The Deck independently
verified the exact local commit objects, trees, and archive parent. A failed
public HTTPS fetch was an authentication limitation, not source authority; no
credential, token, origin change, agent forwarding, or alternate source was
used.

## Eventual primary claim

The separately authorized implementation may establish only:

> Current main adopts the exact content-addressed final WR0 repair represented
> by archive commit `52be94316664f88a19df630e164105a0ca50b875`
> without re-running or mutating the live WR0 environment.

The archive is immutable source material, not an authority to cherry-pick its
obsolete active-slice card or bypass DG0.

## Exact accepted live fixture

Read-only reconnaissance verified the accepted WR0 environment at:

```text
<HOME>/.local/share/linux-vst-bridge/environments/wr0-proton11
```

Expected and observed identities:

- transaction: `wr0-20260901T053317Z-183afd5bbf0736e4`;
- environment: `d3ed38d7ed53e9a5973cbf22fc504f2479dbdd15616fe1bfc1d5e1cd0bf5f4c4`;
- runner/runtime: `2d64df1d36786ca2d0e955c553005423dc2b5bdd714bd0a17872622e33912547`;
- contract source: `linux-vst-bridge-wr0-contract-source/v1` /
  `c6543004fbcd70252393f0e0cab4f9ad7a31e85f433ce3e903e690244cc79541`;
- workload: `4518ca37b8d7e005f01b441de5273447b70fa7c8c9c3d2b9c194a91782cfecac`;
- retired replacement record:
  `917c939c605f457376b0bdd53e06d252d4aecee4b39728850b2e1a61aa102b3e`.

The live environment is owned by the archive transaction. It is not drift to
be restored to the older merged identity. Design work may inspect it only
through bounded accepted read-only paths.

## Reconciliation ownership law

- Commit `9228217...` remains the historical merged WR0 fact.
- Commit `52be943...` is the exact post-merge final-repair source fact.
- The archive transaction owns the current live environment fact.
- Current main is incomplete relative to that exact live/source/evidence fact.
- Adoption must use exact archive path/mode/blob identities; it must never
  rewrite the archive commit or infer equivalent hand-edited source.
- The archived `CURRENT_SLICE.md` is obsolete implementation authority and must
  never be imported.
- DG0 governance on current main remains authoritative and byte-identical.
- Historical evidence remains recoverable from Git history. The eventual
  canonical WR0 evidence packet must truthfully represent the live/archive
  final repair, with a separate WR0A reconciliation packet recording the
  supersession relation.

## Design-phase changed paths

Only these paths may change in this phase:

```text
CURRENT_SLICE.md
docs/slices/WR0A/SLICE_SELECTION.md
docs/slices/WR0A/RECONNAISSANCE.md
docs/slices/WR0A/IMPLEMENTATION_DESIGN.md
docs/slices/WR0A/ADVERSARIAL_DESIGN_REVIEW.md
```

Do not create `docs/slices/WR0A/DESIGN_APPROVAL.md` in this phase. The existing
adversarial-review file records the supplied v1 and v2 repair verdicts. A fresh
independent reviewer may later append the disposition of the exact immutable
v3 design; the design card itself must remain byte-identical.

## External mutation posture

External mutation is prohibited. Do not:

- launch Proton, Wine, the Windows workload, Bitwig, Serum, or a validator;
- write, rename, replace, repair, or regenerate the WR0 environment or receipt;
- create a WR0 transaction sibling;
- modify `.wine`, Steam compatdata/configuration, Flatpak state, runner/runtime
  files, accepted evidence, or another fixture;
- import archive source/evidence into current main;
- change GitHub main, the immutable archive, or open/merge an implementation PR.

Read-only Git object inspection, path/mode/blob comparison, bounded process
guards, bounded fixture readback, and design-record creation are permitted.

## Design-gate and material-discovery stop law

The gate is mandatory because WR0A crosses historical/source/evidence
ownership, content-addressed identity, durable user-state reconciliation, and
accepted-governance boundaries.

Stop and return to the design gate if reconnaissance or later implementation
changes any owner, state, identity, adoption blob, evidence representation,
live-fixture fact, governance path, mutation root, recovery law, proof matrix,
claim ceiling, or changed-path envelope. In particular, stop if the archive no
longer closes the predecessor-backup defect, if the live environment no longer
matches the archive packet, if exact final blobs cannot be imported without
rewriting them, or if DG0 would need modification.

The next expected record is a fresh-context v3 result of either `DESIGN_CLEAR`
or `DESIGN_REPAIR_REQUIRED`. After a clear review, source adoption remains
forbidden until the exact v3 design is separately approved, the authority
records are committed, and the design-authority branch is merged into main.

## Explicit nonclaims

WR0A design does not implement or prove a new runner, environment replacement,
Windows workload run, VST3 load/scan/host, Serum or Bitwig behavior,
authorization, audio, GUI, proxy, bridge, IPC, shared memory, manager, broker,
Rust component, real-time safety, product-runner selection, Steam-independent
distribution, general Linux support, or successor-slice capability.
