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
unresolved_p0_findings: 0
unresolved_p1_findings: 0
unresolved_p2_findings: 0
implementation_authorized: false
```

This file materializes the independent GitHub technical-lead review of the exact immutable V7 design. It does not authorize implementation by itself. Implementation requires the separately retained operator approval, authority transition, merge, exact `main` readback, and implementation handoff.

## Review result

V7 closes both and only the two P1 custody defects identified in the V6 review.

### Private Actions artifact custody

The design binds the actual repository posture:

```yaml
repository: kasselvania/Linux-VST-bridge
visibility: private
owner_type: User
```

It removes mandatory GitHub artifact attestation, GitHub Enterprise Cloud, repository visibility or ownership changes, Sigstore, replacement signing, code signing, and release signing from the WF0 dependency and claim set.

The accepted custody route instead joins:

```text
exact private repository
+ exact workflow path and Git blob
+ exact implementation commit and source manifest
+ exact workflow run ID and attempt
+ exact Actions artifact ID, name, digest, size, run and head identity
+ raw exact-artifact-ID download
+ exact three-file inner envelope
+ build-receipt hash
+ payload hash
+ artifact-manifest hash
+ credential-free Mac custody receipt
+ Mac/Deck transfer readback
```

This establishes exact byte and identity custody only. It does not establish cryptographic provenance, trusted-builder identity, SLSA, code signing, release signing, or product distribution.

### Exact implementation-source handoff

The design supplies the missing source route from the Mac control plane to the Steam Deck execution plane:

```text
exact clean 26-path implementation-source commit
    -> self-contained one-ref Git bundle
    -> canonical source-handoff receipt
    -> ordinary SSH transfer without credential or agent forwarding
    -> exact Deck local handoff ref
    -> clean detached execution worktree
    -> exact 26-record source-manifest reproduction before every Deck run
```

The Windows build receipt, artifact manifest, Mac custody receipt, Deck source worktree, runtime receipts, evidence handoff, and final evidence commit must all bind the same implementation commit and source-manifest digest.

The Deck performs no GitHub fetch, push, API, pull-request, or Actions-artifact operation.

## Design invariants accepted

The following V7 decisions are coherent and implementable:

- Mac control plane owns Git, GitHub, exact private Actions artifact custody, exact source-bundle creation, SSH handoff, evidence retrieval, and Git publication.
- Windows build plane uses explicit `windows-2022`, Visual Studio 2022, `v143`, x64, Windows SDK `10.0.19041.0`, the exact pinned VST3 SDK, two clean builds, PE inspection, and one exact Actions artifact.
- Steam Deck execution plane owns only exact local source and artifact admission, Runtime 4 / Proton 11 execution, cleanup, protected-state verification, and bounded evidence generation.
- The stopped MinGW route remains historical and correctly classified; no SDK or VSTGUI patch is introduced.
- The exact AGain positive fixture, scanner lifecycle, 15 closed call operations, paired `call_completed` law, required-export/optional-entry ordering, no-`createInstance` boundary, held-gate proof, process cleanup law, protected fixtures, and fourteen-file evidence roster remain unchanged.
- The tracked implementation envelope remains exactly 26 implementation/configuration paths plus 14 evidence paths.

## Binding implementation clarification

The upload action's human-facing `artifact-url` and the REST artifact object's API `url` are distinct typed URLs and must not be compared for literal string equality.

They must be joined through the exact:

```text
artifact ID
repository
workflow run
run attempt
head SHA
artifact name
artifact digest
```

The Mac custody receipt's `url` field is the exact REST API artifact URL. The upload action's human-facing URL may be retained separately as bounded transport metadata.

This clarification changes no owner, schema purpose, tracked path, proof boundary, or claim ceiling.

## Count and envelope review

```yaml
owners: 14
material_operations: 25
proof_matrix_rows: 62
blocked_results: 29
implementation_source_paths: 26
evidence_paths: 14
total_tracked_paths: 40
```

The counts and exact path envelopes agree across the design. The source handoff and private artifact-custody work fit within the existing implementation surfaces and do not create a manager, daemon, build farm, deployment service, source synchronizer, or general CI product.

## Claim ceiling

WF0 remains limited to:

```text
exact Windows VST3 module loading
    -> required GetPluginFactory
    -> factory metadata
    -> exact ordered class census
    -> reverse-order factory release
    -> optional ExitDll
    -> FreeLibrary
    -> exact process cleanup
```

WF0 does not establish class instantiation, component/controller lifecycle, host context, buses, parameters, events, state, processing, audio, timing, automation, presets, GUI, native proxy publication, IPC, Bitwig behavior, Serum behavior, authorization, packaging, signing, distribution, or general VST3/Windows/Linux compatibility.

## Disposition

No P0, P1, or P2 design defect remains. No further reconnaissance or design revision is required.

```text
DESIGN_CLEAR
implementation_authorized=false
```
