# Current Slice: WC0 — Windows VST3 Processor Component Admission

## Status

```text
status: active_design_slice
authority_phase: reconnaissance_and_design
implementation_authorized: false
slice: WC0
target: main
selection_basis_commit: 1f4b946178319887950bdb764114543a1a7b845b
selection_basis_tree: 75b91e9f4d8b059aef76abeb9ff6518cf0c310bc
design_gate: required
design_revision: wc0-design-v2
design_status: proposed_for_adversarial_review
design_card: docs/slices/WC0/IMPLEMENTATION_DESIGN.md
v1_design_commit: e712956b14f7e216a1f83db5888701a5ee072eb6
v1_design_tree: da1095db0def3b7387c0514ba4ae884e71fb5b5e
v1_design_blob: bacccf99040565eee9339af3bedfe0e04a8ebcf8
v1_design_sha256: 047f0bb86c6c1991cf8a9addbccbf3a6bfd2c1a72f2fe247953b68eb9b17364e
v1_review: 5091717689 / DESIGN_REPAIR_REQUIRED
implementation_branch: not_created
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

## Design-phase authority

The active design phase may change only:

```text
CURRENT_SLICE.md
docs/slices/WC0/SLICE_SELECTION.md
docs/slices/WC0/IMPLEMENTATION_DESIGN.md
```

It may perform bounded read-only inspection of the merged WF0 implementation and the exact pinned VST3 SDK interfaces needed for:

- `IPluginFactory::createInstance`;
- `IPluginBase::initialize` and `terminate`;
- `IHostApplication`;
- `IComponent::getControllerClassId`;
- AGain processor and controller class identities.

No external mutation or workload is authorized during design.

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

Create `docs/slices/WC0/IMPLEMENTATION_DESIGN.md` as one compact implementation design centered only on:

- component and host-context ownership;
- exact create/read-controller-ID/initialize/terminate/release ordering;
- focused failure attribution and cleanup precedence;
- a bounded changed-path envelope;
- a focused proof matrix;
- the explicit claim ceiling.

Implementation remains unauthorized until that exact design receives fresh independent review and separate operator approval.
