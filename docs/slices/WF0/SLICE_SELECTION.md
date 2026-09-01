# WF0 Slice Selection Receipt

```yaml
schema: linux-vst-bridge-slice-selection/v1
repository: kasselvania/Linux-VST-bridge
basis_commit: 745ca63bdd8641ade85cb9a024c1dc842681192d
basis_tree: 275521adff574f165bb2b3e883c2ec909bae4c66
selected_slice: WF0
selected_title: Supervised Windows VST3 Factory Census Probe
primary_claim: >-
  On the exact accepted Steam Deck fixture, a repository-owned supervised
  Windows x86_64 factory probe, built against the pinned official VST3 SDK and
  executed through the accepted Runtime 4/Proton 11 lane in a disposable
  WF0-owned scan environment, loads the exact pinned AGain VST3 bundle,
  obtains its plug-in factory, retains deterministic factory metadata and the
  complete expected three-class census, unloads cleanly, and leaves the
  accepted WR0 environment and every protected fixture unchanged.
exact_fixture:
  host:
    device: Steam Deck Galileo
    operating_system: SteamOS 3.8.16
    architecture: x86_64
    graphical_session: KDE Wayland
  runner:
    proton_version: 1787334450 proton-11.0-2-x86_64
    runtime: Steam Linux Runtime 4 / 4.0.20260805.254769
    pressure_vessel: 0.20260805.0
    runner_runtime_digest: 2d64df1d36786ca2d0e955c553005423dc2b5bdd714bd0a17872622e33912547
  accepted_environment:
    path: <HOME>/.local/share/linux-vst-bridge/environments/wr0-proton11
    environment_identity: d3ed38d7ed53e9a5973cbf22fc504f2479dbdd15616fe1bfc1d5e1cd0bf5f4c4
    contract_source: c6543004fbcd70252393f0e0cab4f9ad7a31e85f433ce3e903e690244cc79541
    posture: protected_read_only
  scan_environment:
    posture: new_disposable_wf0_owned
    exact_path_and_lifecycle: to_be_frozen_by_design
  vst3_sdk:
    root_commit: 3cdf9ca5d1f5b1b21e0a86832aa4abe55607bd96
    public_sdk_commit: 586dc5e6c8012c3e4b01c79389375cbe96bdb1da
    cmake_commit: 054c9143cbb8d47fc4694e473f2ee3b4d951a8f5
  reference_module:
    source: official Steinberg AGain example
    entry_blob: 13b920b4b7a74137301bf213cf048e96e82861d4
    cid_blob: d32d1640ea187e718baba9cbb039d3a436d53cce
    expected_class_count: 3
    expected_classes:
      - 84E8DE5F92554F5396FAE4133C935A18
      - D39D5B65D7AF42FA843F4AC841EB04F0
      - 41347FD6FED64094AFBB12B7DBA1D441
authority_phase: reconnaissance_and_design
implementation_authorized: false
design_gate: required
design_gate_reason: >-
  WF0 creates the first Windows VST3 module and factory owner, scanner-stage
  lifecycle, supervised Windows process, disposable scan environment, exact
  class-identity census, cross-toolchain build contract, and bounded factory
  compatibility evidence.
design_card_path: docs/slices/WF0/IMPLEMENTATION_DESIGN.md
design_approval_path: null
allowed_changed_paths:
  - CURRENT_SLICE.md
  - docs/slices/WF0/SLICE_SELECTION.md
  - docs/slices/WF0/RECONNAISSANCE.md
  - docs/slices/WF0/IMPLEMENTATION_DESIGN.md
permitted_external_mutation:
  - bounded read-only Steam Deck and repository inspection
  - bounded read-only Flatpak SDK and extension availability inspection
  - bounded read-only runner and accepted-environment identity verification
  - primary-source VST3 SDK, AGain, build-system, and licensing inspection
protected_state:
  - accepted WR0 environment and receipts
  - accepted Runtime 4 and Proton 11 assets
  - .wine and observed Serum files
  - HP0 native publication and build receipts
  - HP1 Bitwig state and evidence
  - Bitwig and Flatpak configuration
  - Steam compatdata
  - SteamOS read-only state
  - all repository paths outside the four-path design envelope
explicit_nonclaims:
  - no package, Flatpak extension, compiler, SDK, dependency, or runtime installation
  - no Windows build or VST3 bundle creation
  - no Proton, Wine, scanner, validator, Bitwig, or Serum execution
  - no WF0 scan-environment creation or mutation
  - no accepted WR0 environment mutation
  - no class instantiation
  - no component or controller lifecycle
  - no interface census beyond factory-level interfaces
  - no parameter, bus, event, state, or process setup
  - no audio
  - no editor or GUI
  - no native proxy
  - no IPC or C ABI
  - no Rust product service
  - no Serum or Bitwig compatibility claim
  - no product-runner selection
  - no general Windows VST3 or Linux compatibility claim
material_discovery_requires_stop: true
successor_selection_authorized: false
operator_approval_text: >-
  I explicitly approve selecting WF0 — Supervised Windows VST3 Factory Census
  Probe and replacing the no-active-slice card with its bounded
  reconnaissance-and-design authority. This approval does not authorize
  implementation. Implementation requires a separate approved design revision.
approved_at: 2026-09-01T11:51:47-07:00
```

## Authority effect

This receipt selects WF0 and authorizes only the four-path reconnaissance-and-design phase recorded above. It does not authorize source implementation, toolchain installation, builds, Windows execution, live-environment mutation, or a compatibility claim.
