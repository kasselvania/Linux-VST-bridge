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
reviewed_design_pr: #21
reviewed_design_head: 1d13fefc60ad6c2c49e384cd30631f60be2a3de2
design_revision: wf0-design-v7
design_commit: 1d13fefc60ad6c2c49e384cd30631f60be2a3de2
design_tree: 752b885643c730378ceefab99c7d7ec9277fdf56
design_card: docs/slices/WF0/IMPLEMENTATION_DESIGN_V7.md
design_card_git_blob: 3c8ac56cfaa8c95cb99fa327b392ed447453a87b
design_card_sha256: 844d646509933516ff60eeb2c6213cd1c2e89a8d22b5c986ac986fb104a19d77
adversarial_review: docs/slices/WF0/ADVERSARIAL_DESIGN_REVIEW_V7.md
adversarial_review_git_blob: 36f6f7579f3e31e2d52a8d8e86b2950f56eaec90
adversarial_review_github_id: 5084789559
adversarial_review_result: DESIGN_CLEAR
design_approval: docs/slices/WF0/DESIGN_APPROVAL_V7.md
design_approval_git_blob: f10bf084de447be468254aed82237e2ecbbdad42
design_authority_branch: authority/wf0-v7-implementation
design_authority_merge_pr: pending
design_authority_merge_commit: pending
design_authority_merge_tree: pending
implementation_branch: codex/wf0-windows-vst3-factory-census-v7
implementation_branch_state: awaiting_exact_authority_readback_merge
implementation_basis_rule: exact_current_main_after_authority_readback_merges
successor_selection_authorized: false
```

The immutable V7 design is approved. This authority transition does not establish the WF0 technical claim; it authorizes implementation only after the V7 design authority and its exact post-merge readback are present on `main`.

## Primary claim

> On the exact accepted Steam Deck fixture, a repository-owned supervised Windows x86_64 factory probe, built against the pinned official VST3 SDK and executed through the accepted Runtime 4 / Proton 11 lane in a disposable WF0-owned scan environment, loads the exact pinned AGain VST3 bundle, obtains its plug-in factory, retains deterministic factory metadata and the complete expected three-class census, unloads cleanly, and leaves the accepted WR0 environment and every protected fixture unchanged.

WF0 stops before `createInstance`.

## Approved three-plane boundary

```text
MacControlPlane
    -> private GitHub repository and implementation source
    -> exact private Actions run/artifact custody
    -> exact self-contained implementation-source Git bundle
    -> ordinary SSH source/artifact handoff
    -> evidence retrieval and Git publication

WindowsBuildPlane
    -> GitHub-hosted windows-2022
    -> Visual Studio 2022 / v143 / x64
    -> Windows SDK 10.0.19041.0
    -> exact pinned VST3 SDK and AGain
    -> two clean builds and exact artifact envelope

SteamDeckExecutionPlane
    -> exact local source-bundle admission under fixed non-GitHub ref
    -> clean detached exact-commit execution worktree
    -> exact content-addressed artifact admission
    -> Runtime 4 / Proton 11 execution
    -> held-gate, negative-family, and positive AGain proofs
    -> cleanup, protected-state verification, and bounded evidence return
```

The Deck has no ordinary WF0 GitHub login, fetch, push, API, pull-request, or Actions-artifact-download dependency.

## Approved repository and custody posture

```text
repository: kasselvania/Linux-VST-bridge
visibility: private
owner type: User

GitHub artifact attestation: not required or claimed
GitHub Enterprise Cloud: not required
repository visibility/ownership change: prohibited
replacement signing system: not authorized
cryptographic provenance / SLSA / code signing: not claimed
```

Private Actions artifact custody binds the exact repository, workflow path and blob, implementation head SHA, run ID and attempt, artifact ID, artifact name, Actions digest, exact-ID API download, three-file inner envelope, build receipt, payload digest, artifact-manifest digest, Mac custody receipt, and Deck byte readback.

The upload action's human-facing artifact URL and the REST artifact object's API URL are distinct typed values. They are not compared for literal equality; they are joined through the exact artifact ID, repository, run, head SHA, name, and digest.

## Approved source-handoff law

The exact implementation source reaches the Deck through one self-contained Git bundle and `linux-vst-bridge-wf0-source-handoff/v1` receipt over ordinary SSH with no credential or agent forwarding.

The Deck imports only under:

```text
refs/handoff/wf0-v7-source/<implementation-commit>
```

and executes only from one clean detached worktree at:

```text
<HOME>/.local/share/linux-vst-bridge/worktrees/wf0/<implementation-commit>/
```

Before every held-gate, fault, positive, normalization, or evidence run, the Deck reproduces the exact 26-record implementation-source manifest. Source, Windows build, artifact custody, execution, evidence handoff, and final evidence-only publication must agree on one implementation commit and source-manifest digest.

## Approved implementation envelope

```text
tracked implementation paths: 40
implementation/configuration paths: 26
evidence paths: 14
owners: 14
material operations: 25
proof-matrix rows: 62
blocked results: 29
```

The exact path roster, state machines, custody schemas, scanner lifecycle, proof matrix, blocked taxonomy, protected-state law, external mutation envelope, and material-stop conditions are owned by `docs/slices/WF0/IMPLEMENTATION_DESIGN_V7.md`.

A need for another tracked path, a different builder, an SDK/VSTGUI patch, another positive fixture, a Deck GitHub dependency, a changed Runtime/Proton route, class instantiation, IPC, proxy, audio, GUI, Bitwig execution, Serum, authorization, packaging, or a widened claim requires `RETURN_TO_DESIGN_GATE`.

## Binding scanner laws

1. Every ordinary return from all 15 closed call operations is followed immediately by its paired synchronously flushed `call_completed` record before any later lifecycle event or call attempt.

2. Required `GetPluginFactory` is resolved and checked before optional `InitDll`. A missing export never invokes `InitDll`.

3. With a present export, optional entry handling completes, its lifecycle result is published, then `factory_export_found` is published. Only an entry-absent or entry-succeeded branch invokes the retained factory export.

4. Factory-release, `ExitDll`, `FreeLibrary`, and process-cleanup failures remain secondary when an earlier primary stage failure exists.

5. No class-instantiation operation exists in the scanner command, event enum, or retained schema.

## Protected state

Protected state includes:

- the accepted WR0 environment and receipts;
- exact Runtime 4 / Proton 11 assets;
- `.wine` and Steam compatdata;
- SteamOS read-only posture;
- all historical SR0, HP0, HP1, WR0, and WR0A source/evidence;
- current Bitwig 6.1 application, runtime, scope, shadow, overrides, permissions, and configuration;
- Serum/vendor material;
- the preserved stopped provisional source and bundle;
- unrelated worktrees and every repository path outside the 40-path envelope.

Bitwig remains unlaunched protected state only. WF0 makes no Bitwig 6.1 behavior claim.

## Explicit nonclaims

WF0 does not establish class instantiation, component/controller lifecycle, connections, host context, buses, parameters, events, state, processing, audio, timing, automation, presets, GUI, native proxy publication, IPC, Bitwig behavior, Serum behavior, authorization, packaging, release signing, distribution, or general VST3/Windows/Linux compatibility.

## Next lawful action

Merge the exact V7 authority records, perform one exact post-merge authority readback, create the implementation branch from that exact `main` commit and tree, and issue a bounded implementation handoff. The implementation agent may not merge or select a successor.
