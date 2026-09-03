# PC0 adversarial design review history

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
implementation_authorized_at_review: false
```

## V1 findings

### R1 — Accepted DX0 identity

V1 cited nonexistent DX0 evidence commit `85840927f49c2ead5d42cfe0014f4771c9d03e76`. V2 was required to bind the real accepted DX0 source `be046dd2d44a7915ca408c01a06212639ccea51e` / tree `1322e4eb7b5bd54244bd2fa7fc83327ea6a0c183`, evidence `85840920844693f7611306492298817e16dd2a6c` / tree `35ddde4e56a810ab1ce44970313bf2f42e585908`, merge `1f71487717eabdb3cd5285a4559df2ce2915c8d8`, and status closure `858c240b104e090aaed8bd23ace04fd9a0dfd20e`, with real Git-object and relationship validation.

### R2 — Observation/result/evidence ownership

V1 conflated the private original Deck observation with a later consumer projection. V2 was required to keep `linux-vst-bridge-pc0-transaction-result/v1` as producer-P/execution-E original-observation truth with no consumer C, while a separate tracked `linux-vst-bridge-pc0-evidence-packet/v1` in `TRANSACTION.json` binds P/E/C, strict admission, observation disposition, processing contract, calls, quiescence, shutdown, cleanup, protected state, proof rows, renderer identity, and acyclic integrity. The durable output-writer failure boundary was also required.

Both V1 findings were bounded contract repairs. The primary claim, owner, state machine, call surface, proof count, path envelope, cost ceiling, and nonclaims remained sound.

## V2 exact-head review

```yaml
reviewed_revision: pc0-design-v2
reviewed_design_commit: 996ee557d33ea55d6acf7ef2242f703c2c63f262
reviewed_design_tree: bb1fe157bef42854812ebeb0b73b1a5332c93cf6
reviewed_design_parent: f6938e7dd501a4c86a382243670d82d060d1b75a
reviewed_design_path: docs/slices/PC0/IMPLEMENTATION_DESIGN.md
reviewed_design_blob: ec0683fc66028239d7481640ba72e2dd9a060a2c
reviewed_design_sha256: 20e653a6b1fad720ea5fc888a8d531bda44c338840f5eb96e488612d197de992
github_review_id: 5105712590
github_review_result: PC0_DESIGN_V2_CLEAR
review_authority: independent_technical_lead
implementation_authorized_at_review: false
```

V2 closes R1 by binding the exact accepted DX0 objects and requiring object/type/relationship resolution. V2 closes R2 by separating the private P/E observation from tracked P/E/C evidence, freezing the five-file evidence roster and `TRANSACTION.json` ownership, and making durable writer failure part of call attribution.

Two binding implementation clarifications require no V3:

1. `COST_AND_INVALIDATION.json` uses schema `linux-vst-bridge-pc0-cost-and-invalidation/v1`; incompatible PC0 keys must not be published under the DX0 v1 label.
2. If durable `call_started` publication fails with no earlier PC0 blocker, the primary result is `PC0_EVIDENCE_BLOCKED`; no VST3 call occurred and only accepted physical containment may be claimed. If an earlier PC0 blocker exists, writer failure remains secondary and does not erase it.

The reviewed implementation ceiling remains one `PreSetupProcessingContractCensus` owner, nine states, four selected operation types, eleven positive calls, sixteen proof rows, ten blockers, fourteen source/configuration paths, five evidence paths, at most one Windows producer, at most one positive Deck batch, zero live negative exercises, zero AGain rebuilds or fixture reseeds, one ordinary Mac command, and zero manually copied identifiers.

`PC0_DESIGN_V2_CLEAR` is binding for implementation authority only when paired with the exact operator approval receipt and matching `CURRENT_SLICE.md`. No successor is selected by this review.
