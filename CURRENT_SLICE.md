# Current Slice: WC0 — Windows VST3 Processor Component Admission

## Status

```text
status: active_implementation_slice
authority_phase: implementation
implementation_authorized: true
slice: WC0
target: main
selection_basis_commit: 1f4b946178319887950bdb764114543a1a7b845b
selection_basis_tree: 75b91e9f4d8b059aef76abeb9ff6518cf0c310bc
design_gate: required
design_revision: wc0-design-v2
design_status: approved_for_implementation
design_card: docs/slices/WC0/IMPLEMENTATION_DESIGN.md
v1_design_commit: e712956b14f7e216a1f83db5888701a5ee072eb6
v1_design_tree: da1095db0def3b7387c0514ba4ae884e71fb5b5e
v1_design_blob: bacccf99040565eee9339af3bedfe0e04a8ebcf8
v1_design_sha256: 047f0bb86c6c1991cf8a9addbccbf3a6bfd2c1a72f2fe247953b68eb9b17364e
v1_review: 5091717689 / DESIGN_REPAIR_REQUIRED
v2_design_commit: 064db624056f0fdf4daeda3b5ae394ab6210bee2
v2_design_tree: 6d7baf83e29704e803a76106c36d346cdbf7be03
v2_design_blob: df31b8467af9dcd97bc06b6afdf5b4b8d6be7018
v2_design_sha256: ca68cde6f68b02320e3c950b445aca9db99fdac1301fa7dfb50d510f83c7d78c
v2_review: 5092052158 / DESIGN_CLEAR
design_approval: docs/slices/WC0/DESIGN_APPROVAL.md
implementation_branch: codex/wc0-windows-vst3-processor-component-admission
implementation_basis: exact design-authority merge commit and tree after readback
successor_selection_authorized: false
```

## Primary claim

> On the exact accepted Steam Deck fixture, the existing supervised Windows VST3 host path creates the exact AGain processor class as one `IComponent`, verifies its declared controller class ID, initializes it with a minimal repository-owned `IHostApplication`, terminates and releases it correctly, and then completes the already-proven factory/module shutdown with no remaining object, process, environment, or protected-state residue.

## Exact new boundary

```text
accepted AGain factory and class census
    -> create the exact AGain processor as IComponent
    -> read and verify its declared controller class ID
    -> initialize it with one minimal host context
    -> terminate it
    -> release it
    -> reuse accepted factory/module shutdown and process cleanup
```

WC0 owns one processor object lifecycle. It does not create the controller object or enter audio, state, parameter, bus, editor, proxy, or IPC work.

## Approved implementation envelope

The immutable `wc0-design-v2` card and approval receipt authorize exactly:

```text
new owners: 2
new plug-in call operations: 5
component lifecycle states: 13
focused proof rows: 29
blocked results: 13
source/configuration paths: 19
evidence paths: 14
total tracked implementation paths: 33
```

The implementation branch must begin from the exact merged design-authority commit and tree read back after this authority merges. Its source commit must be one direct child of that basis and bind the exact 19-record `linux-vst-bridge-wc0-implementation-source/v1` manifest frozen in the design.

## Accepted infrastructure reused without redesign

WC0 consumes these accepted WF0 capabilities as prerequisites:

- Windows Server 2022 / Visual Studio 2022 / MSVC build plane;
- private Actions artifact custody on the Mac;
- exact implementation-source Git-bundle handoff;
- ordinary SSH source/artifact transfer to the Steam Deck;
- clean detached Deck execution worktree;
- content-addressed Windows artifact admission;
- Runtime 4 / Proton 11 supervision;
- disposable environment ownership;
- module entry, factory acquisition, reverse factory release, module exit, unload, process drainage, and protected-state verification.

During WC0, those mechanisms receive exact identity preflight and one bounded regression only. Their architecture and complete WF0 proof matrix are not reopened.

## Binding implementation clarifications

Technical-lead review `5092052158` binds these implementation details without creating a V3 design:

1. Zero-initialize the complete controller `TUID` before `getControllerClassId`; consume or retain it only after an ordinary successful result.
2. Callback/reference-count evidence storage must outlive the self-deleting `MinimalHostApplication`; no field may be read through the host pointer after its final `release()` returns zero.
3. Deterministic unmatched-call attribution must cover all five new operations, including `create_component` and `get_controller_class_id`, without expanding the live fault family.

## Object-quiescence law

Only `object_quiescence=true` permits inherited factory-interface release, `ExitDll`, and `FreeLibrary`.

For an initialized component, quiescence requires all nine design facts: successful initialization; exactly one ordinary terminate return; component release return zero; no component call in flight; cleared unusable component pointer; host reference count returned to owner baseline; final host-owner release returned zero; no host callback in flight; and a closed non-overflowed callback ledger.

A nonzero release enters `component_retirement_incomplete`, is never retried, suppresses inherited in-process shutdown as `not_attempted_object_quiescence_unproved`, and leaves physical containment to the accepted process/environment owners without a clean-unload claim.

## Anti-distraction law

WC0 may not redesign or re-prove:

- GitHub authentication;
- the Windows build plane;
- Actions artifact custody;
- Mac-to-Deck transport;
- Deck source admission;
- SteamOS persistence;
- Runtime or Proton selection;
- process-supervisor architecture;
- Bitwig installation;
- the WF0 factory census.

WC0 may not absorb:

- edit-controller instantiation;
- `IConnectionPoint` pairing;
- `IAudioProcessor` interface census;
- bus enumeration or activation;
- parameters, state, process setup, events, or audio;
- GUI/editor behavior;
- native Linux proxy, C ABI, IPC, or shared memory;
- Bitwig execution;
- Serum execution or authorization;
- packaging or runner selection.

A need for any of those owners is a material discovery and returns WC0 to the design gate.

## Exact fixture

```text
accepted WF0 source commit:
8b76ab886fd75079c72e3f820781beb5d1b36ae9

accepted WF0 evidence commit:
0096010a36ebf31a36149064d64059142ce7cfed

accepted WF0 source-manifest SHA-256:
03c3c017f7d6eb357ae657e992ef3c3988932f6870ac3a18192dfd9e343ee05f

Runtime 4 / Proton 11 digest:
2d64df1d36786ca2d0e955c553005423dc2b5bdd714bd0a17872622e33912547

AGain module SHA-256:
60aa9ff6b9918d4330449e7b3ab34b588dd93cba09f37413a3cd91f6e7d2e18f

processor class ID:
84E8DE5F92554F5396FAE4133C935A18

processor raw Windows TUID:
5FDEE8845592534F96FAE4133C935A18

requested interface:
Steinberg::Vst::IComponent

expected controller class ID:
D39D5B65D7AF42FA843F4AC841EB04F0

expected controller raw Windows TUID:
655B9DD3AFD7FA42843F4AC841EB04F0
```

Bitwig 6.1 remains unlaunched protected state only.

## Protected state

Protected state includes the accepted WF0 source, evidence, build, custody, and runtime identities; the accepted WR0 environment; Runtime 4 and Proton 11; current Bitwig 6.1 and its Flatpak configuration; historical SR0/HP0/HP1/WR0/WR0A/WF0 evidence; Serum and vendor material; `.wine`; Steam compatdata; SteamOS read-only posture; and every repository path outside the bounded WC0 authority.

## Next lawful action

Merge this exact approved design authority into `main`, read back the resulting commit and tree, create `codex/wc0-windows-vst3-processor-component-admission` from that exact basis, and implement only the immutable `wc0-design-v2` claim and 33-path envelope.

A material design discovery returns WC0 to the design gate. Ordinary defects within the approved component, host-context, event, focused-fixture, normalizer, supervisor-extension, or evidence owners are implementation repairs and must not reopen infrastructure design.
