# DX0 adversarial design review — V1

```yaml
slice: DX0
reviewed_revision: dx0-design-v1
reviewed_design_commit: 17304ef3b22d44eee9ebb5bbbe16bb237e9bad96
reviewed_design_tree: 98b1e6cfa5b039eeada5aee1c2f88670a7af9b9e
reviewed_design_path: docs/slices/DX0/IMPLEMENTATION_DESIGN.md
reviewed_design_blob: b71024010e828fba9acc5c9a2f82546a5bb6e6c9
reviewed_design_sha256: de40a98822bc7ea4fda8684047f77007177626e5b8a49cdfeda7bed2d845682e
github_review_id: 5096569104
review_result: DESIGN_REPAIR_REQUIRED
implementation_authorized: false
```

This record materializes the supplied independent technical-lead review. The
design author does not claim to have independently reviewed its own design.
The review accepted the primary claim, three owners, ten implementation paths,
five evidence paths, and one-build/one-live-exercise ceilings, subject to these
two repairs.

## R1 — Reusable-result admission and original execution provenance

Violated invariant: a cached proof result is admissible only when its complete
typed content proves the exact original observation and every required
producer, execution, consumer, input, custody, lifecycle, cleanup, and
protected-state join.

Unsafe condition: V1 named a result schema but did not freeze its fields or
predicate. Producer source P, Deck execution source E, and renderer/consumer
source C could be collapsed, or an incomplete, failed, truncated, or
misattributed result could be accepted by filename, hash, schema label, or a
nominal success flag.

Required V2 repair: freeze the bounded result and consumer-admission contracts,
the exact positive-regression predicate, truthful P/E/C reporting, and an
acyclic hash order with an initial nonce distinct from the final digest.
Refine proof rows 3, 8, and 14 without adding a live exercise.

Adjacent operations reviewed under the same invariant: identity derivation,
host custody, source handoff, Deck admission, result retention, evidence
rendering, and transaction closure.

Additional reconnaissance required: false.

## R2 — Remote-outcome recovery without duplicate work

Violated invariant: loss of a local acknowledgement cannot authorize a second
expensive remote side effect whose first outcome is unresolved.

Unsafe condition: V1 did not persist operation intent before dispatch or Deck
execution, pin the dispatch response contract, reconcile accepted-but-
unacknowledged work, retain a completed Deck result before acknowledgement, or
exclude concurrent duplicate producers.

Required V2 repair: persist stable phase nonces and exact intent, use the
pinned GitHub dispatch run ID, reconcile unknown outcomes before repetition,
atomically retain Deck completion before acknowledgement, and add a bounded
single-writer rule with fail-closed stale-lock and partial-publication
behavior. Refine proof rows 9 and 12 without increasing the matrix.

Adjacent operations reviewed under the same invariant: workflow eligibility
and dispatch, workflow/result readback, artifact custody, Deck launch,
process/environment cleanup, result publication and retrieval, resume, and
concurrent driver invocation.

Additional reconnaissance required: false.
