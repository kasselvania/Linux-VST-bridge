# PC0 V1 adversarial design review history

```yaml
slice: PC0
reviewed_revision: pc0-design-v1
reviewed_design_commit: f6938e7dd501a4c86a382243670d82d060d1b75a
reviewed_design_tree: bacf308af5a7c993e1da45059497fb392a3b8845
reviewed_design_parent: f00824e196d2a71667d80a9cc622c03ea2cef403
reviewed_design_path: docs/slices/PC0/IMPLEMENTATION_DESIGN.md
reviewed_design_blob: 0f2d1c26aea3d009ce93d1d5086b52ca03fc8ce1
reviewed_design_sha256: 29db8b0f884407ba9ea75c80cab6ddd290908f2449d73e4502dd61347c490e86
github_review_id: 5105496167
github_review_result: PC0_DESIGN_V1_REPAIR_REQUIRED
review_authority: independent_technical_lead
additional_reconnaissance_required: false
implementation_authorized: false
```

This file materializes the supplied independent technical-lead review; it is
not a self-review by the design author.

## R1 — Accepted DX0 identity

**Violated invariant:** every cited authority and retained-evidence coordinate
must resolve to the exact accepted Git object and relationship.

**False-acceptance condition:** V1 cited nonexistent DX0 evidence commit
`85840927f49c2ead5d42cfe0014f4771c9d03e76`; lexical hash shape could therefore
pass while the accepted evidence object was unresolvable.

**Required V2 repair:** bind DX0 source
`be046dd2d44a7915ca408c01a06212639ccea51e`, tree
`1322e4eb7b5bd54244bd2fa7fc83327ea6a0c183`; evidence
`85840920844693f7611306492298817e16dd2a6c`, tree
`35ddde4e56a810ab1ce44970313bf2f42e585908`; merge
`1f71487717eabdb3cd5285a4559df2ce2915c8d8`; and status closure
`858c240b104e090aaed8bd23ace04fd9a0dfd20e`. Require real Git-object and
relationship resolution for every cited identity.

**Adjacent operations reviewed:** selection/activation lineage, WA0/DX0
source-evidence parentage, implementation merge containment, and current-main
ancestry.

## R2 — Observation/result/evidence ownership

**Violated invariant:** a live observation and a later consumer projection are
separate facts with separate owners and non-circular integrity.

**False-acceptance condition:** V1's private result key roster contained P/E but
its predicate also required C, while no tracked machine-readable file owned the
complete P/E/C, admission, call, quiescence, shutdown, cleanup, protected-state,
and proof-row closure. Prose or a bus-contract-only JSON could falsely imply a
complete transaction.

**Required V2 repair:** keep the private
`linux-vst-bridge-pc0-transaction-result/v1` as P/E original-observation truth;
render a separate `linux-vst-bridge-pc0-evidence-packet/v1` binding P/E/C into
`TRANSACTION.json`, with the processing contract nested, exact proof closure,
and acyclic projection hashes. Restrict `COST_AND_INVALIDATION.json` to cost and
invalidation. Freeze the durable output-writer failure boundary.

**Adjacent operations reviewed:** Deck result publication, strict Mac
admission, renderer-only reuse, consumer truthfulness, tracked hashes, and
physical containment versus clean in-process shutdown.

Both findings are bounded design-contract repairs. The primary claim, owner,
state machine, call surface, proof count, path envelope, cost ceiling, and
nonclaims remain sound. Fresh independent review is required for V2.
