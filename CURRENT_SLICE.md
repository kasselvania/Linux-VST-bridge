# Current Slice: WF0 — Supervised Windows VST3 Factory Census Probe

## Status

```text
status: active_design_slice
authority_phase: reconnaissance_and_design
implementation_authorized: false
slice: WF0
target: main
selection_basis_commit: 745ca63bdd8641ade85cb9a024c1dc842681192d
selection_basis_tree: 275521adff574f165bb2b3e883c2ec909bae4c66
design_branch: codex/wf0-windows-vst3-factory-census-design
design_gate: required
selection_receipt: docs/slices/WF0/SLICE_SELECTION.md
design_card: docs/slices/WF0/IMPLEMENTATION_DESIGN.md
design_approval: absent
successor_selection_authorized: false
```

The operator explicitly selected WF0 for bounded reconnaissance and implementation design. Product implementation is not authorized by this card.

## Operator approval

```text
I explicitly approve selecting WF0 — Supervised Windows VST3 Factory Census Probe and replacing the no-active-slice card with its bounded reconnaissance-and-design authority. This approval does not authorize implementation. Implementation requires a separate approved design revision.
```

## Eventual primary claim

> On the exact accepted Steam Deck fixture, a repository-owned supervised Windows x86_64 factory probe, built against the pinned official VST3 SDK and executed through the accepted Runtime 4/Proton 11 lane in a disposable WF0-owned scan environment, loads the exact pinned AGain VST3 bundle, obtains its plug-in factory, retains deterministic factory metadata and the complete expected three-class census, unloads cleanly, and leaves the accepted WR0 environment and every protected fixture unchanged.

This is the claim to be designed. It is not yet an implementation or compatibility claim.

## Accepted prerequisites

```text
HP0 / HP1:
  native Linux VST3 build, validation, publication, Bitwig discovery, and native instance admission

WR0:
  exact Runtime 4 / Proton 11 controlled Windows-command execution
  isolated project-owned Windows environment
  exact process ownership and cleanup
  durable environment replacement and recovery

WR0A:
  repository truth reconciled to the accepted final WR0 source, evidence, and live environment
```

Accepted runner and environment identities:

```text
runner/runtime digest: 2d64df1d36786ca2d0e955c553005423dc2b5bdd714bd0a17872622e33912547
accepted WR0 environment: d3ed38d7ed53e9a5973cbf22fc504f2479dbdd15616fe1bfc1d5e1cd0bf5f4c4
final WR0 contract source: c6543004fbcd70252393f0e0cab4f9ad7a31e85f433ce3e903e690244cc79541
```

The accepted WR0 environment is protected and read-only during WF0.

## Design-phase objective

Reconnaissance and the implementation design must establish the smallest lawful design for:

```text
exact Windows cross-toolchain
    -> exact Windows AGain reference bundle
    -> disposable WF0 scan environment
    -> supervised Windows scanner process
    -> module open
    -> module entry
    -> GetPluginFactory / IPluginFactory
    -> factory metadata
    -> complete factory class enumeration
    -> module exit and unload
    -> exact process cleanup
```

The scanner stages must remain distinguishable:

```text
module_open
module_entry
factory_get
factory_info
class_enumeration
module_exit
module_unload
process_cleanup
```

WF0 stops before class instantiation.

## Exact design-phase changed paths

Only these paths may change during reconnaissance and design:

```text
CURRENT_SLICE.md
docs/slices/WF0/SLICE_SELECTION.md
docs/slices/WF0/RECONNAISSANCE.md
docs/slices/WF0/IMPLEMENTATION_DESIGN.md
```

The independent reviewer may later add:

```text
docs/slices/WF0/ADVERSARIAL_DESIGN_REVIEW.md
```

A design approval may later add:

```text
docs/slices/WF0/DESIGN_APPROVAL.md
```

Neither later file exists or is authorized by this design-phase activation.

## Permitted design-phase activity

- Read all governing repository documents and accepted HP0, HP1, WR0, and WR0A evidence.
- Perform bounded read-only inspection of the Steam Deck, installed Flatpak SDK/runtime inventory, runner assets, accepted WR0 environment identity, and relevant local source checkouts.
- Inspect the pinned official VST3 SDK, AGain source, Windows hosting implementation, CMake support, and applicable license material.
- Determine the exact MinGW/toolchain candidate and whether it is available without installing it.
- Define the owner map, scanner state machine, process topology, disposable-environment lifecycle, exact census schema, failure classifications, changed-path envelope, proof matrix, and claim ceiling.

## Prohibited during design

- Installing a package, Flatpak extension, compiler, SDK, dependency, or runtime.
- Building a Windows executable or VST3 bundle.
- Launching Proton, Wine, a Windows workload, scanner, validator, Bitwig, or Serum.
- Creating or mutating a WF0 scan environment.
- Modifying the accepted WR0 environment, `.wine`, Steam compatdata, Bitwig/Flatpak settings, HP0 publication, or any accepted evidence.
- Reading vendor credentials, authorization state, proprietary plug-in contents, or private project data.
- Implementing scanner, host, proxy, IPC, Rust product code, audio, GUI, or Serum support.

## Protected state

- Current accepted `main` and all governance records outside the four-path design envelope.
- Accepted WR0 environment and receipts.
- Runtime 4 and Proton 11 assets.
- Existing `.wine` and observed Serum files.
- HP0 native publication and build receipts.
- HP1 Bitwig state and evidence.
- Bitwig and Flatpak configuration.
- Steam compatdata and SteamOS read-only state.

## Design questions that must be answered

1. Which exact user-space Windows cross-toolchain is selected and how is it pinned?
2. Can the pinned VST3 SDK and AGain fixture be built through that toolchain without source patching or undeclared downloads?
3. What exact AGain bundle, binary path, dependencies, class order, and factory metadata are expected?
4. What is the narrowest C++20 scanner executable and which official SDK helpers does it own?
5. What disposable environment does WF0 use without touching the accepted WR0 environment?
6. Which WR0 supervisor, ancestry, timeout, and cleanup laws are reused, and what scanner-specific additions are needed?
7. What bounded normalized schema represents factory metadata and every class exactly?
8. Which blocked result owns each scanner stage?
9. How is the reference fixture independently validated before scanner results are trusted?
10. Which SDK/example notices and generated artifacts may be retained or redistributed?

## Material-discovery stop law

Return to the design gate if reconnaissance changes the proposed owner boundary, runner route, scan-environment mutation root, module lifecycle, toolchain route, exact fixture, primary claim, claim ceiling, changed paths, or proof ownership.

Do not patch forward into implementation.

## Required next gate

The design agent must produce:

```text
docs/slices/WF0/RECONNAISSANCE.md
docs/slices/WF0/IMPLEMENTATION_DESIGN.md
```

A fresh independent context must then review the exact design revision. Implementation remains forbidden until a separate exact design approval is retained and merged.

## Explicit nonclaims

WF0 design authority does not prove or authorize class instantiation, component/controller lifecycle, interface census beyond factory-level interfaces, buses, parameters, events, state, process setup, audio, editor behavior, a native proxy, IPC, a Rust service, Serum execution, Bitwig execution, product-runner selection, Steam-independent distribution, another plug-in, or general Windows VST3/Linux compatibility.
