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
amendment_basis_commit: 15523c69567d24b256cb3c65cb6f06bfa07854be
amendment_basis_tree: a6e2fc8c7a564f50e9033094fde847387478de62
reviewed_amendment_pr: #18
reviewed_amendment_head: 38a5a87239ef91cb4537ec23aca1e72829ef5861
design_revision: wf0-design-v5
design_commit: 6fc3632a04fdba9b8fc20794205e97881c4313f9
design_tree: 0e6b666b28edf6be510d9d29dfa501b5460312bc
design_card: docs/slices/WF0/IMPLEMENTATION_DESIGN_V5.md
design_card_git_blob: 74fd8dc5616bf6a5bca45753503947681b35cfaf
design_card_sha256: 7d6db4dde4f0e5795757699a46f4443a5cbc19ceb917b5dc789b29c931499cc5
adversarial_review: docs/slices/WF0/ADVERSARIAL_DESIGN_REVIEW_V5.md
adversarial_review_git_blob: 8abbbdaf60fc00e00eb0a036fe9463ab4da35984
adversarial_review_github_id: 5083590311
adversarial_review_result: DESIGN_CLEAR
design_approval: docs/slices/WF0/DESIGN_APPROVAL_V5.md
design_approval_git_blob: aabf50ae545a4a4a3d7efddc4abec00d04dfc452
design_authority_merge_pr: #19
design_authority_merge_commit: 07aa098d4cee2397e221742e8284aa1c6ba3b1c8
design_authority_merge_tree: 2e1c04925a4bf728469b15b0e9ae8fd3bcb76abf
authority_readback_branch: status/wf0-v5-authority-readback
implementation_branch: codex/wf0-windows-vst3-factory-census
implementation_branch_state: awaiting_exact_authority_readback_merge
implementation_basis_rule: exact_current_main_after_this_authority_readback_merges
successor_selection_authorized: false
```

PR #18 retains the reviewed V3/V4/V5 amendment history. Because GitHub's draft-ready transition failed, the exact approved head was merged through ordinary PR #19 as design-authority commit `07aa098d4cee2397e221742e8284aa1c6ba3b1c8`, tree `2e1c04925a4bf728469b15b0e9ae8fd3bcb76abf`.

This file is a post-merge authority readback only. It does not amend `wf0-design-v5`. The implementation branch must begin from the exact current `main` commit and tree after this readback merges; the technical-lead handoff supplies those identities. The design, reconciliation, review, and approval records are immutable implementation inputs.

## Primary claim

> On the exact accepted Steam Deck fixture, a repository-owned supervised Windows x86_64 factory probe, built against the pinned official VST3 SDK and executed through the accepted Runtime 4 / Proton 11 lane in a disposable WF0-owned scan environment, loads the exact pinned AGain VST3 bundle, obtains its plug-in factory, retains deterministic factory metadata and the complete expected three-class census, unloads cleanly, and leaves the accepted WR0 environment and every protected fixture unchanged.

The claim is not accepted until an exact implementation head passes independent pre-PR audit, technical-lead review, and merge.

## Approved V5 authority

```text
fixture reconciliation:
  docs/slices/WF0/BITWIG_6_1_FIXTURE_RECONCILIATION.md

immutable design:
  docs/slices/WF0/IMPLEMENTATION_DESIGN_V5.md
  revision: wf0-design-v5
  commit: 6fc3632a04fdba9b8fc20794205e97881c4313f9
  tree: 0e6b666b28edf6be510d9d29dfa501b5460312bc
  Git blob: 74fd8dc5616bf6a5bca45753503947681b35cfaf
  SHA-256: 7d6db4dde4f0e5795757699a46f4443a5cbc19ceb917b5dc789b29c931499cc5

independent review:
  docs/slices/WF0/ADVERSARIAL_DESIGN_REVIEW_V5.md
  Git blob: 8abbbdaf60fc00e00eb0a036fe9463ab4da35984
  GitHub review: 5083590311
  result: DESIGN_CLEAR

approval receipt:
  docs/slices/WF0/DESIGN_APPROVAL_V5.md
  Git blob: aabf50ae545a4a4a3d7efddc4abec00d04dfc452
```

The prior V1 through V4 design and review records remain immutable history. V5 is the current implementation authority.

## Current protected Bitwig fixture

Bitwig is protected state only and is never launched by WF0:

```text
version: 6.1
ref: app/com.bitwig.BitwigStudio/x86_64/stable
application commit: 8a048e733e74dda8b897339436153a2d5df952f29d362dfcaca9d8e0d6f6c231
scope: system
origin: flathub
user shadow: absent
runtime ref: org.freedesktop.Platform/x86_64/25.08
runtime commit: bd44a6230581917d04f89812a4c21090c304d390edb73995af1c2f9fd8abf4e8
user override SHA-256: 1b4a6a6ed688f69dd3c36dcac8db008c5a41ed52170ea3e23dee984b0aae6a1e
system override SHA-256: e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
permission-output SHA-256: c7f5a34dce104cc3d347dcaf89e135cc5ad73b891101d7587f78423858ad5c73
```

Historical SR0, HP0, HP1, WR0, and WR0A evidence remains exact for its original Bitwig `6.0.11` fixture. This authority does not promote any historical result into a Bitwig `6.1` behavioral claim.

## Binding implementation laws

1. Immediately after every ordinary return from any of the 15 closed call operations, capture the bounded return value, emit the exact paired `call_completed`, and synchronously flush it before any later lifecycle event or call attempt.

2. Resolve and check required `GetPluginFactory` before optional `InitDll`. A missing required export must emit `factory_export_missing`, must not resolve or invoke `InitDll`, and retains `WF0_FACTORY_GET_BLOCKED` as primary while applicable loaded-module cleanup proceeds.

3. When the required export is present, retain successful resolution internally, complete optional entry handling and its paired completion, publish the entry lifecycle result, then publish `factory_export_found`. Only an entry-absent or entry-succeeded branch may invoke the already-resolved factory export. The later lifecycle publication is not evidence of when the export was checked.

4. Cleanup failure remains secondary to the primary stage failure.

## Exact implementation boundary

WF0 implements only:

```text
exact user-scope Freedesktop SDK / MinGW lock
    -> exact Windows x86_64 scanner and pinned AGain builds
    -> exact PE, export, import, dependency, bundle, and reproducibility checks
    -> disposable WF0-owned scan environment
    -> supervised Runtime 4 / Proton 11 scanner launch
    -> causal held-gate proof before module load
    -> explicit Win32 module loading
    -> required GetPluginFactory / IPluginFactory acquisition
    -> factory metadata and IPluginFactory1/2/3 support
    -> exact ordered three-class census
    -> reverse-order factory release
    -> optional ExitDll and module unload
    -> exact process cleanup and environment retirement
    -> bounded sanitized evidence
```

WF0 stops before `createInstance`.

## Approved implementation envelope

```text
tracked paths: 40
implementation/configuration paths: 26
evidence paths: 14
implementation-source schema: linux-vst-bridge-wf0-implementation-source/v1
proof-matrix rows: 35
blocked results: 23
material operations: 14
owners: 10
```

The exact paths, owners, lifecycle, mutation law, proof matrix, external mutations, and nonclaims are defined by `docs/slices/WF0/IMPLEMENTATION_DESIGN_V5.md`.

The exact approved user-scope MinGW extension installation is permitted during implementation:

```text
org.freedesktop.Sdk.Extension.mingw-w64/x86_64/25.08
commit: f15a5a88eb09557f645860bfcb3c4a6bc267683fce06cb68436bd76376fca694
```

No host package-manager or system-scope installation is authorized.

## Protected state

The accepted WR0 environment, Runtime 4 and Proton 11 assets, `.wine`, Steam compatdata, SteamOS read-only state, HP0 publication and evidence, HP1 evidence, historical Bitwig-bound evidence, current Bitwig 6.1 installation and configuration projection, Serum/vendor material, unrelated worktrees, and every repository path outside the exact implementation envelope are protected.

Any protected mismatch is `WF0_PROTECTED_FIXTURE_DRIFT`. Implementation must stop and may not repair, downgrade, upgrade, launch, or reconfigure Bitwig or another protected fixture.

## Material-discovery stop law

Return to the design gate if implementation requires a different toolchain, SDK patch, positive fixture, Runtime/Proton route, accepted-WR0 mutation, separate bootstrap workload, class instantiation, new call operation, materially different process topology, persistent product environment, IPC, native proxy, audio, GUI, Bitwig execution, Serum, authorization, packaging, a forty-first tracked path, changed proof owner, or widened claim ceiling.

A normal defect inside the approved owners, lifecycle, paths, and claim is repaired in the implementation branch without redesign.

## Explicit nonclaims

WF0 does not establish class instantiation, component/controller lifecycle, connection points, instantiated-interface census, host contexts, buses, parameters, MIDI/events, state, process setup, audio, timing, automation, presets, editor/GUI behavior, a native Linux proxy, C ABI, IPC, shared memory, a Rust service, Bitwig behavior, Serum execution or compatibility, installation/authorization, product-runner selection, Steam-independent distribution, another plug-in or format, general Windows VST3 support, or general Linux compatibility.

No successor slice or adjacent feature is authorized.
