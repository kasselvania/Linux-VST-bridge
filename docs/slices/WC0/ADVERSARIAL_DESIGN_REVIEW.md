# WC0 adversarial design review — V1

```yaml
slice: WC0
reviewed_design_commit: e712956b14f7e216a1f83db5888701a5ee072eb6
reviewed_design_tree: da1095db0def3b7387c0514ba4ae884e71fb5b5e
reviewed_design_path: docs/slices/WC0/IMPLEMENTATION_DESIGN.md
reviewed_design_blob: bacccf99040565eee9339af3bedfe0e04a8ebcf8
reviewed_design_sha256: 047f0bb86c6c1991cf8a9addbccbf3a6bfd2c1a72f2fe247953b68eb9b17364e
reviewed_revision: wc0-design-v1
github_review_id: 5091717689
review_result: DESIGN_REPAIR_REQUIRED
implementation_authorized: false
```

## Review provenance

This record materializes the supplied independent technical-lead review at
[GitHub review 5091717689](https://github.com/kasselvania/Linux-VST-bridge/pull/27#pullrequestreview-5091717689).
It does not claim that the design-authoring agent independently reviewed its own
design. The review found exactly the following two priority-one findings and no
additional reconnaissance requirement.

## P1 findings

### P1-1 — Nonzero component release lacks a stable physical state

- **Violated invariant:** module shutdown is legal only after every created
  plug-in object and host-context reference is proved absent. Process
  containment is not proof of clean in-process object retirement.
- **Concrete unsafe or false-acceptance condition:** V1 blocks an ordinary
  nonzero `release_component` result but has no stable state for the still-live
  reference and can continue into inherited factory release, `ExitDll`, and
  `FreeLibrary`. A nonzero `FUnknown::release` result proves an object reference
  remains; unloading its module could remove code beneath that object and could
  falsely report a clean shutdown.
- **Required V2 repair:** add `component_retirement_incomplete`, define one
  exact `object_quiescence` predicate, and gate every inherited factory release,
  `ExitDll`, and `FreeLibrary` attempt on that predicate. When absence is false
  or unknown, retain the primary WC0 blocker, record inherited calls as
  `not_attempted_object_quiescence_unproved`, and use only accepted process-tree
  and disposable-environment containment without a clean-unload claim. Never
  retry `Release` to force zero.
- **Adjacent operations reviewed under the same invariant:** anomalous
  failure/non-null create cleanup; controller-ID failure and mismatch cleanup;
  initialize-return failure; terminate-return failure; component release;
  final host-owner release; host-reference equality; initialize, terminate, and
  release timeout/crash attribution; inherited factory releases; `ExitDll`;
  `FreeLibrary`; process drainage; and environment retirement.
- **Additional reconnaissance required:** false.

### P1-2 — WC0 source, build, and handoff identities are not frozen

- **Violated invariant:** source, build, artifact, Deck execution, and evidence
  may join only through one exact immutable implementation identity. An
  implementation agent may not invent authority-critical branch, schema,
  manifest, artifact, bundle, ref, or receipt values.
- **Concrete unsafe or false-acceptance condition:** V1 places the existing
  hard-coded WF0 identity owners in scope and says they will receive “WC0
  identities,” but does not freeze those values or state which inherited WF0
  wire identities remain unchanged. That permits stale 26-record or `wf0-v7`
  material to be accepted, permits name-only or mutable-head selection, or
  forces an unreviewed identity design during implementation.
- **Required V2 repair:** freeze the WC0 implementation branch and Git ref, the
  19-record source schema and literal sorted roster, Actions artifact name,
  source-bundle name and ref, source/evidence handoff schemas and filenames,
  Mac and Deck roots, and every receipt join. Explicitly retain the accepted WF0
  infrastructure schemas, wire filenames, payload paths, environment-stage
  shape, algorithms, and owners unchanged while requiring their new receipts to
  embed the exact WC0 source identity.
- **Adjacent operations reviewed under the same invariant:** workflow path
  filtering; source-manifest construction; build receipt and artifact manifest;
  Actions upload and exact-ID selection; Mac custody; source bundle and receipt;
  SSH handoff; Deck ref, detached worktree, and artifact cache; component and
  callback evidence; evidence handoff; and the final evidence-only commit.
- **Additional reconnaissance required:** false.

## Result

```text
DESIGN_REPAIR_REQUIRED
implementation_authorized=false
```
