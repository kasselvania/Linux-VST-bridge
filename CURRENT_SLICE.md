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
reviewed_design_pr: #15
design_authority_merge_pr: #16
design_authority_merge_commit: df102033292e815e26103e08929c1183fb1c260a
design_authority_merge_tree: f1ed5493b51518705c759c1b8f7dab7407414038
design_revision: wf0-design-v2
design_commit: 4a04d5b52d1fa8e2d309ed1e2883ff96a7963ca6
design_tree: b14b216ad5964ec68a1cf32d33c4201670d3ffd7
design_card: docs/slices/WF0/IMPLEMENTATION_DESIGN.md
design_card_git_blob: d618cbf6b397f10947d50fd4824cd3e06ef55726
design_card_sha256: f323e2b429c4f91c1821488d980b249901b8d82c3cceaa7bf9be9d874a455e24
adversarial_review: docs/slices/WF0/ADVERSARIAL_DESIGN_REVIEW.md
adversarial_review_git_blob: 7ee98c4c0fb0d28b11d43d33b46e2ab5fa57c299
adversarial_review_github_id: 5082923871
adversarial_review_result: DESIGN_CLEAR
design_approval: docs/slices/WF0/DESIGN_APPROVAL.md
design_approval_git_blob: a84df2044b8a78b44b7c004b53b620d474994221
implementation_branch: codex/wf0-windows-vst3-factory-census
implementation_basis_rule: exact_current_main_after_this_authority_readback_merges
successor_selection_authorized: false
```

PR #15 retains the reviewed V1/V2 design history. Because GitHub's draft-ready transition failed, the byte-identical approved branch head was merged through ordinary PR #16 as design-authority commit `df102033292e815e26103e08929c1183fb1c260a`, tree `f1ed5493b51518705c759c1b8f7dab7407414038`.

This file is a post-merge authority readback only. It does not amend `wf0-design-v2`. The implementation branch must begin from the exact current `main` commit and tree after this readback merges; the technical-lead handoff supplies those identities. The design, review, selection, reconnaissance, and approval records are immutable implementation inputs.

## Primary claim

> On the exact accepted Steam Deck fixture, a repository-owned supervised Windows x86_64 factory probe, built against the pinned official VST3 SDK and executed through the accepted Runtime 4/Proton 11 lane in a disposable WF0-owned scan environment, loads the exact pinned AGain VST3 bundle, obtains its plug-in factory, retains deterministic factory metadata and the complete expected three-class census, unloads cleanly, and leaves the accepted WR0 environment and every protected fixture unchanged.

The claim is not accepted until an exact implementation head passes independent pre-PR audit and technical-lead review and is merged.

## Approved design authority

```text
selection receipt:
  docs/slices/WF0/SLICE_SELECTION.md

reconnaissance:
  docs/slices/WF0/RECONNAISSANCE.md

immutable design:
  docs/slices/WF0/IMPLEMENTATION_DESIGN.md
  revision: wf0-design-v2
  Git blob: d618cbf6b397f10947d50fd4824cd3e06ef55726
  SHA-256: f323e2b429c4f91c1821488d980b249901b8d82c3cceaa7bf9be9d874a455e24

independent review:
  docs/slices/WF0/ADVERSARIAL_DESIGN_REVIEW.md
  Git blob: 7ee98c4c0fb0d28b11d43d33b46e2ab5fa57c299
  GitHub review: 5082923871
  result: DESIGN_CLEAR

approval receipt:
  docs/slices/WF0/DESIGN_APPROVAL.md
  Git blob: a84df2044b8a78b44b7c004b53b620d474994221

design authority merge:
  PR: #16
  commit: df102033292e815e26103e08929c1183fb1c260a
  tree: f1ed5493b51518705c759c1b8f7dab7407414038
```

## Binding implementation clarifications

1. Each `call_completed` record is synchronously flushed immediately after the bounded return value is captured and before any later lifecycle event or call attempt. A retained call is “in flight” when no validated completion was observed; WF0 does not claim instruction-level knowledge after abnormal process death.
2. The required `GetPluginFactory` export is resolved and checked before optional `InitDll` is invoked. A missing required export follows `factory_export_missing` without executing `InitDll`. Optional `ExitDll` cleanup remains governed by the loaded-module cleanup law.

These clarifications are part of the approved design and do not create a V3 revision.

## Exact implementation boundary

WF0 implements only:

```text
exact user-scope Freedesktop SDK / MinGW lock
    -> exact Windows x86_64 scanner and AGain reference builds
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

## Approved tracked path envelope

The implementation PR may change exactly the 40 paths listed in Section 14 of `docs/slices/WF0/IMPLEMENTATION_DESIGN.md`:

- 26 implementation and configuration paths;
- 14 evidence paths.

The implementation-source identity is:

```text
schema: linux-vst-bridge-wf0-implementation-source/v1
record_count: 26
records: exact lexically sorted path / Git mode / Git blob
```

All builds and live runs must use one clean committed 26-path implementation identity. The final evidence-only head must reproduce the same manifest exactly. A source/configuration change after live execution invalidates the artifacts and live evidence and requires rebuild and rerun.

No implementation edit is authorized outside the exact 40-path envelope.

## Approved external mutations

WF0 implementation may:

- install the exact user-scope Flatpak ref `org.freedesktop.Sdk.Extension.mingw-w64/x86_64/25.08` only at approved commit `f15a5a88eb09557f645860bfcb3c4a6bc267683fce06cb68436bd76376fca694`;
- create the declared WF0 build and verified-artifact cache roots;
- create and retire exact marker-bound disposable WF0 scan roots;
- launch the exact supervised Runtime 4 / Proton 11 scanner workloads and bounded source-owned negative fixtures;
- create bounded owned runtime transients and remove them after exact cleanup;
- push the implementation branch and open an ordinary non-draft PR.

No other package, compiler, SDK, runner, system, Flatpak, Bitwig, Serum, `.wine`, Steam-compatdata, accepted-WR0-environment, HP0, HP1, or user-content mutation is authorized.

## Protected fixture

```text
host:
  Steam Deck Galileo
  SteamOS 3.8.16
  x86_64
  SteamOS read-only

runner/runtime digest:
  2d64df1d36786ca2d0e955c553005423dc2b5bdd714bd0a17872622e33912547

accepted WR0 environment:
  d3ed38d7ed53e9a5973cbf22fc504f2479dbdd15616fe1bfc1d5e1cd0bf5f4c4
  posture: protected_read_only

VST3 SDK root:
  3cdf9ca5d1f5b1b21e0a86832aa4abe55607bd96

positive fixture:
  official pinned AGain Windows VST3
```

The accepted WR0 environment is never reused as the WF0 scan environment.

## Proof obligations

The implementation must satisfy the approved 35-row proof matrix and 23-result blocked taxonomy. Material obligations include:

- exact installed toolchain identity;
- clean pinned SDK and exact 26-path implementation source;
- two truthful reproducibility builds;
- PE32+ x86_64, export, import, dependency, and bundle closure;
- valid checked DLL-search calls;
- first supervised launch as sole prefix-initialization owner;
- causal held-gate proof with no module-open attempt or AGain mapping;
- exact factory metadata and ordered three-class census;
- exact pre-call attribution for all 15 closed operations;
- no `createInstance`, proven by source/contract and tripwire;
- reverse-order releases, optional exit, unload, and cleanup;
- zero owned descendants and unrelated-process survival;
- exact disposable-environment retirement;
- accepted-WR0 and protected-fixture equality;
- bounded sanitized fixed-roster evidence.

A production exercise may satisfy several proof rows where the retained mapping is explicit. An arbitrary test-count target is not required.

## Material-discovery stop law

Implementation must stop and return to the design gate if evidence changes the approved:

- owner map;
- scanner or build lifecycle;
- mutation root or recovery boundary;
- process/thread topology;
- call/event or census contract;
- identity or dependency-search law;
- security, privacy, licensing, or distribution posture;
- exact MinGW, SDK, AGain, Runtime, Proton, or protected fixture;
- primary claim or claim ceiling;
- 40-path tracked envelope;
- 35-row proof matrix.

A normal defect within the approved owner/model/path envelope is repaired in the implementation branch and does not require redesign.

## Required implementation topology

The implementation branch should preserve the approved two-phase commit posture:

```text
clean source commit:
  exactly 26 implementation/configuration paths
  -> freeze implementation-source manifest
  -> install exact toolchain
  -> build twice
  -> run negatives
  -> held-gate proof
  -> positive AGain factory census

final evidence-only commit:
  exactly 14 evidence paths
  -> reproduce identical 26-path source manifest
  -> retain bounded evidence
```

The implementation PR must remain open, ordinary/non-draft, and unmerged pending independent pre-PR audit and technical-lead review.

## Explicit nonclaims

WF0 does not establish or implement class instantiation, component/controller lifecycle, connection points, instantiated-interface census, host contexts, buses, parameters, MIDI/events, state, process setup, audio, timing, automation, presets, editor/GUI behavior, a native Linux proxy, C ABI, IPC, shared memory, a Rust service, Bitwig execution, Serum execution or compatibility, installation/authorization, product-runner selection, Steam-independent distribution, another plug-in or format, general Windows VST3 support, or general Linux compatibility.

No successor slice or adjacent feature is authorized.
