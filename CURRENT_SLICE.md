# Current Slice: WF0 — Supervised Windows VST3 Factory Census Probe

## Status

```text
status: active_implementation_slice
authority_phase: implementation
implementation_authorized: true
slice: WF0
target: main
selection_basis_commit: 745ca63bdd8641ade85cb9a024c1dc842681192d
selection_basis_tree: 275521adff574f165bb2b3e883c2ec909bae4c66
stopped_v5_implementation_basis_commit: 67026ad7160a584cbf9cdb4bf0db7b8dbfa60136
stopped_v5_implementation_basis_tree: 2021dfcbf000d934a462d40efef6bbe7b54e0862
mandatory_stop_result: RETURN_TO_DESIGN_GATE
v6_design_commit: 1e14eb8c318263b7307fe8e07aae13628402e0c1
v6_design_tree: 63f9ce1edfe663848e2a4832c7facad881b83aac
v6_design_blob: e153943a302e3f9b38c74c9048f70a1d65c7b238
v6_review_github_id: 5084539680
v6_review_result: DESIGN_REPAIR_REQUIRED
design_revision: wf0-design-v7
design_commit: 1d13fefc60ad6c2c49e384cd30631f60be2a3de2
design_tree: 752b885643c730378ceefab99c7d7ec9277fdf56
design_card: docs/slices/WF0/IMPLEMENTATION_DESIGN_V7.md
design_card_git_blob: 3c8ac56cfaa8c95cb99fa327b392ed447453a87b
design_card_sha256: 844d646509933516ff60eeb2c6213cd1c2e89a8d22b5c986ac986fb104a19d77
adversarial_review: docs/slices/WF0/ADVERSARIAL_DESIGN_REVIEW_V7.md
adversarial_review_git_blob: 962ba4ba031bcec79cb57ae234985ee962bb16ce
adversarial_review_github_id: 5084789559
adversarial_review_result: DESIGN_CLEAR
design_approval: docs/slices/WF0/DESIGN_APPROVAL_V7.md
design_approval_git_blob: 95e1a19cc965044ae9176d202abb799e36506176
design_authority_pr: #21
design_authority_merge_commit: pending_exact_merge_readback
design_authority_merge_tree: pending_exact_merge_readback
authority_readback_required: true
implementation_branch: codex/wf0-windows-vst3-factory-census-v7
implementation_branch_state: not_created_until_exact_authority_readback
implementation_basis_rule: exact_current_main_after_authority_readback_merges
successor_selection_authorized: false
```

The immutable V7 design is technically clear and separately approved. This card authorizes implementation only after the V7 authority is merged and its exact merge commit/tree are recorded through a post-merge authority readback. The implementation branch must begin from the exact resulting `main`; it may not begin from the stopped V5 basis, the V6/V7 design branch, or the preserved provisional MinGW commit.

## Primary claim

> On the exact accepted Steam Deck fixture, a repository-owned supervised Windows x86_64 factory probe, built against the pinned official VST3 SDK and executed through the accepted Runtime 4 / Proton 11 lane in a disposable WF0-owned scan environment, loads the exact pinned AGain VST3 bundle, obtains its plug-in factory, retains deterministic factory metadata and the complete expected three-class census, unloads cleanly, and leaves the accepted WR0 environment and every protected fixture unchanged.

The claim is not accepted until an exact V7 implementation head passes the approved proof matrix, independent pre-PR audit, technical-lead exact-head review, and merge.

## Approved V7 authority

```text
immutable design:
  path: docs/slices/WF0/IMPLEMENTATION_DESIGN_V7.md
  revision: wf0-design-v7
  commit: 1d13fefc60ad6c2c49e384cd30631f60be2a3de2
  tree: 752b885643c730378ceefab99c7d7ec9277fdf56
  Git blob: 3c8ac56cfaa8c95cb99fa327b392ed447453a87b
  SHA-256: 844d646509933516ff60eeb2c6213cd1c2e89a8d22b5c986ac986fb104a19d77

independent review:
  path: docs/slices/WF0/ADVERSARIAL_DESIGN_REVIEW_V7.md
  Git blob: 962ba4ba031bcec79cb57ae234985ee962bb16ce
  GitHub review: 5084789559
  result: DESIGN_CLEAR

approval receipt:
  path: docs/slices/WF0/DESIGN_APPROVAL_V7.md
  Git blob: 95e1a19cc965044ae9176d202abb799e36506176
```

The V6 design and review remain immutable history. V7 supersedes V6 only for the actual private-repository artifact-custody boundary and the exact Mac-to-Deck implementation-source handoff. The scanner, factory, process, Runtime/Proton, evidence, and claim-ceiling laws inherited from V5/V6 remain binding.

## Valid mandatory stop and preserved source

The stopped V5 implementation correctly found:

```text
SMTG VSTGUI integration forces VSTGUI_STANDALONE=ON
    -> pinned standalone target selects Win32 sources only under if(MSVC)
    -> MinGW selects no applicable source branch
    -> CMake: No SOURCES given to target: vstgui_standalone
```

The provisional source remains salvage material only:

```text
commit: 5b166f4902af5e1e3b9c2287512f1f8f099c51d0
parent: 67026ad7160a584cbf9cdb4bf0db7b8dbfa60136
tree: fb6a14a78bcab6dd11b5720a725e9df841714743
source/configuration paths: 26
evidence paths: 0
bundle SHA-256: c442d429ff20447d616c62c4273bbefaedbe4b28cd4631e3fc5683c4bb2d1143
status: preserved_unaccepted_salvage_material
```

It may be audited path by path as a source reference. It may not be merged, rebased, wholesale cherry-picked, or treated as accepted implementation authority. No MinGW retry or SDK/VSTGUI patch is authorized.

## Exact three-plane implementation boundary

```text
MacControlPlane
    -> private GitHub repository and branch authority
    -> exact implementation-source commit and manifest
    -> exact private Actions run and artifact-ID custody
    -> exact self-contained source Git bundle
    -> ordinary SSH source/artifact handoff
    -> evidence retrieval and Git publication

WindowsBuildPlane
    -> GitHub-hosted windows-2022
    -> Visual Studio 2022 / v143 / x64
    -> Windows SDK 10.0.19041.0
    -> exact pinned VST3 SDK and submodules
    -> two clean MSVC builds
    -> PE/import/export/dependency verification
    -> one exact private Actions artifact

SteamDeckExecutionPlane
    -> exact source-bundle verification and local handoff ref
    -> clean detached execution worktree at exact source commit
    -> exact 26-record source manifest before every run
    -> exact content-addressed artifact import
    -> Runtime 4 / Proton 11 execution
    -> held-gate, negative-family, and positive AGain proofs
    -> cleanup, protected-state equality, and bounded evidence return
```

The Deck has no normal WF0 GitHub login, fetch, push, API, pull-request, or Actions-artifact-download dependency.

## Private Actions artifact custody

The exact repository posture is:

```text
repository: kasselvania/Linux-VST-bridge
visibility: private
owner type: User
```

GitHub artifact attestation, GitHub Enterprise Cloud, repository visibility or ownership changes, Sigstore, replacement signing, code signing, and release signing are not required or claimed.

The Mac custody route must bind:

```text
exact repository
exact workflow path and Git blob
exact implementation head SHA and source-manifest digest
exact workflow run ID and attempt
exact artifact ID, name, size, digest, run and head identity
raw exact-artifact-ID API download
exact three-file inner envelope
build-receipt hash
payload hash
artifact-manifest hash
credential-free Mac custody receipt
Mac/Deck transfer equality
```

The upload action's human-facing artifact URL and the REST artifact object's API URL are distinct typed values and must not be compared as identical strings. They are joined by exact artifact ID, repository, run, head SHA, name, and digest. The Mac custody receipt's `url` field is the REST API artifact URL.

## Exact implementation-source handoff

The Mac must create a self-contained, one-ref Git bundle for the exact implementation-source commit and bind it through:

```text
schema: linux-vst-bridge-wf0-source-handoff/v1
advertised ref: refs/handoff/wf0-v7-source/<implementation-commit>
Deck local ref: refs/handoff/wf0-v7-source/<implementation-commit>
Deck execution worktree:
  <HOME>/.local/share/linux-vst-bridge/worktrees/wf0/<implementation-commit>/
```

The source bundle, handoff receipt, Windows build receipt, artifact manifest, Mac custody receipt, Deck worktree, runtime receipts, evidence handoff, and final evidence commit must all name one exact implementation commit and one exact 26-record source-manifest digest.

Every Deck execution occurs from the clean detached worktree. Individually copied scripts, the provisional MinGW worktree, a dirty checkout, or a GitHub-fetched Deck source are not authority.

## Binding scanner and execution laws

1. Every ordinary return from all 15 closed call operations must be followed immediately by its paired synchronously flushed `call_completed` record before any later lifecycle event or call attempt.
2. Required `GetPluginFactory` is resolved and checked before optional `InitDll`. A missing export never resolves or invokes `InitDll`.
3. With a present export, optional entry handling completes, its lifecycle result is published, then `factory_export_found` is published. Only an entry-absent or entry-succeeded branch invokes the retained factory export.
4. Factory-release, `ExitDll`, `FreeLibrary`, and process-cleanup failures remain secondary to the first primary stage failure.
5. No class-instantiation operation exists in the command, scanner event enum, or retained schema. `createInstance` is never called.
6. Scanner readiness and exact process identity must be established before the supervisor releases the module-load gate.
7. The first exact Runtime 4 / Proton 11 scanner launch owns any disposable-prefix initialization. No separate bootstrap workload is authorized.

## Approved implementation boundary

```text
owners: 14
material operations: 25
proof-matrix rows: 62
blocked results: 29
implementation/configuration paths: 26
evidence paths: 14
total tracked implementation paths: 40
```

The exact path rosters and schemas are owned by `docs/slices/WF0/IMPLEMENTATION_DESIGN_V7.md`. The implementation must contain one clean 26-path source commit followed by one 14-path evidence-only commit. A forty-first tracked path or source change after build/runtime proof is a material stop unless the exact design already classifies it as an ordinary repair within the approved envelope.

## Protected state

The following remain protected:

- accepted WR0 environment and receipts;
- Runtime 4 / Proton 11 assets and digest `2d64df1d36786ca2d0e955c553005423dc2b5bdd714bd0a17872622e33912547`;
- accepted WR0 environment identity `d3ed38d7ed53e9a5973cbf22fc504f2479dbdd15616fe1bfc1d5e1cd0bf5f4c4`;
- SteamOS read-only posture;
- `.wine` and Steam compatdata;
- historical SR0/HP0/HP1/WR0/WR0A source and evidence;
- current Bitwig 6.1 application/runtime/scope/shadow/override/permission projection;
- Serum and all vendor material;
- unrelated repositories and worktrees;
- every tracked path outside the exact 40-path envelope.

Bitwig remains unlaunched protected state only:

```text
version: 6.1
application commit: 8a048e733e74dda8b897339436153a2d5df952f29d362dfcaca9d8e0d6f6c231
runtime commit: bd44a6230581917d04f89812a4c21090c304d390edb73995af1c2f9fd8abf4e8
user override SHA-256: 1b4a6a6ed688f69dd3c36dcac8db008c5a41ed52170ea3e23dee984b0aae6a1e
system override SHA-256: e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
permission-output SHA-256: c7f5a34dce104cc3d347dcaf89e135cc5ad73b891101d7587f78423858ad5c73
```

No Bitwig 6.1 behavior is claimed.

## Material-discovery stop law

Implementation returns to the design gate if it requires:

- a build plane other than the approved Windows 2022 / MSVC route;
- an SDK or VSTGUI patch, copied upstream source, replacement target, or different positive fixture;
- mandatory artifact attestation or another signing system;
- Deck GitHub authentication or API access;
- a source route other than the approved self-contained Git bundle and exact local handoff ref;
- mutation or reuse of the accepted WR0 environment;
- a separate bootstrap workload;
- class instantiation or a new unapproved factory-call operation;
- a materially different Runtime/Proton process topology;
- Bitwig execution or historical-evidence rewrite;
- IPC, native proxy, audio, GUI, Serum, authorization, packaging, manager, broker, registry, release signing, or distribution ownership;
- a changed primary claim, owner model, proof ownership, path envelope, fixture, security posture, or claim ceiling.

An ordinary workflow, build, scanner, transfer, importer, supervisor, or evidence defect inside the approved owners and 40 paths is repaired in the implementation branch and does not by itself authorize another design revision.

## Claim ceiling and nonclaims

WF0 ends after:

```text
exact Windows module load
    -> required GetPluginFactory
    -> factory metadata and interface support
    -> exact ordered three-class census
    -> reverse-order factory release
    -> optional ExitDll
    -> FreeLibrary
    -> exact process cleanup and environment retirement
```

WF0 does not establish class instantiation, component/controller lifecycle, connections, host context, buses, parameters, events, state, processing, audio, timing, automation, presets, GUI, native proxy publication, IPC, Bitwig behavior, Serum behavior, authorization, packaging, cryptographic provenance, signing, distribution, a permanent build service, or general VST3/Windows/Linux compatibility.

## Next lawful action

1. Merge the exact V7 design/review/approval authority.
2. Record the actual merge commit/tree through a separate authority readback.
3. Create `codex/wf0-windows-vst3-factory-census-v7` from that exact post-readback `main` basis.
4. Issue the bounded V7 implementation handoff.
5. Leave implementation unaccepted until independent pre-PR audit, technical-lead review, and merge.

No successor slice or status closure is authorized.
