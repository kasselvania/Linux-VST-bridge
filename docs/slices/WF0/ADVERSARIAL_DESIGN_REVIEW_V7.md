# WF0 V7 Adversarial Design Review

## Review identity

```yaml
slice: WF0
review_kind: github_technical_lead_review
reviewed_design_commit: 1d13fefc60ad6c2c49e384cd30631f60be2a3de2
reviewed_design_tree: 752b885643c730378ceefab99c7d7ec9277fdf56
reviewed_design_path: docs/slices/WF0/IMPLEMENTATION_DESIGN_V7.md
reviewed_design_blob: 3c8ac56cfaa8c95cb99fa327b392ed447453a87b
reviewed_design_sha256: 844d646509933516ff60eeb2c6213cd1c2e89a8d22b5c986ac986fb104a19d77
reviewed_revision: wf0-design-v7
github_review_id: 5084789559
review_result: DESIGN_CLEAR
unresolved_p0: 0
unresolved_p1: 0
unresolved_p2: 0
implementation_authorized: false
```

This review binds the exact immutable V7 design. It does not authorize implementation; implementation authority requires a separate exact operator approval receipt, authority transition, merge, and exact `main` readback.

## Findings resolved

V7 closes both and only the two P1 custody defects identified in the V6 review:

1. **Private Actions artifact custody.** The design binds the actual private, user-owned repository posture and removes mandatory GitHub artifact attestation, Enterprise Cloud, visibility or ownership changes, Sigstore, replacement signing, code signing, and release signing from the workflow, schemas, proofs, blockers, and claim set. Exact custody instead joins one fixed private-repository workflow run, exact source and workflow identities, one Actions artifact, exact artifact ID and digest, exact-ID API download, the three-file inner envelope, build, payload, and manifest hashes, and a credential-free Mac custody receipt. No cryptographic provenance, trusted-builder, SLSA, code-signing, or release-signing claim follows.

2. **Exact implementation-source handoff.** The Mac creates a self-contained one-ref Git bundle and source-handoff receipt for the exact 26-path implementation commit and merged V7 authority. The bundle is transferred over ordinary SSH without credential or agent forwarding. The Deck verifies and imports it under the fixed local handoff ref, creates a clean detached execution worktree at the exact implementation commit, and reproduces the 26-record path/mode/blob source manifest before every Deck run. Source, Windows build, artifact custody, execution, evidence, and the final evidence-only commit must agree on one implementation commit and source-manifest digest. The Deck performs no GitHub fetch, push, API, PR, or Actions-artifact operation.

## Design assessment

```text
primary claim:                    sound
claim ceiling:                   sound
private repository posture:      exact
Windows/MSVC build plane:        sound
Actions artifact custody:        sound
Mac source-bundle custody:       sound
Deck source admission:           sound
no-Deck-GitHub boundary:         complete
source/build/artifact join:      complete
scanner lifecycle:               unchanged and sound
process supervision:             unchanged and sound
protected-state law:             sound
implementation path envelope:    complete
further reconnaissance:          not required
further design revision:         not required
```

The Mac/Windows/Deck plane split is coherent. The supported `windows-2022` / Visual Studio 2022 / `v143` / x64 / Windows SDK `10.0.19041.0` build route remains distinct from the accepted Runtime 4 / Proton 11 execution route. The valid MinGW stop and preserved provisional source remain historical inputs only. Scanner semantics, pinned AGain, required-export and optional-entry ordering, all 15 paired-completion laws, no-`createInstance` boundary, process supervision, held-gate proof, protected fixtures, 14-file evidence roster, and the WF0 claim ceiling remain unchanged.

Counts and envelopes agree:

```text
owners:                            14
material operations:              25
proof-matrix rows:                 62
blocked results:                   29
implementation/configuration:      26 paths
evidence:                          14 paths
total tracked implementation:      40 paths
```

## Binding implementation clarification

The upload action's human-facing `artifact-url` and the REST artifact object's API `url` are distinct typed URLs and must not be compared for literal string equality.

They must each be retained or validated in their declared field and joined by the exact:

```text
artifact ID
repository
workflow run
head SHA
artifact name
artifact digest
```

The Mac custody receipt's `url` field is the exact REST API artifact URL. The upload action's human-facing URL may be retained separately as bounded transport metadata. This clarification changes no owner, schema purpose, path, proof boundary, or claim ceiling.

## Verdict

No P0, P1, or P2 design defect remains. No additional reconnaissance or design revision is required.

```text
DESIGN_CLEAR
implementation_authorized=false
```
