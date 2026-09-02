# WC0 Slice Selection Receipt

```yaml
schema: linux-vst-bridge-slice-selection/v1
repository: kasselvania/Linux-VST-bridge
basis_commit: 1f4b946178319887950bdb764114543a1a7b845b
basis_tree: 75b91e9f4d8b059aef76abeb9ff6518cf0c310bc

selected_slice: WC0
selected_title: Windows VST3 Processor Component Admission

primary_claim: >-
  On the exact accepted Steam Deck fixture, the existing supervised Windows
  VST3 host path creates the exact AGain processor class as one IComponent,
  verifies its declared controller class ID, initializes it with a minimal
  repository-owned IHostApplication, terminates and releases it correctly,
  and then completes the already-proven factory/module shutdown with no
  remaining object, process, environment, or protected-state residue.

exact_fixture:
  repository_main:
    commit: 1f4b946178319887950bdb764114543a1a7b845b
    tree: 75b91e9f4d8b059aef76abeb9ff6518cf0c310bc

  accepted_wf0:
    implementation_merge: e694cc84344394553c4a3eff6b13f34226b368ae
    source_commit: 8b76ab886fd75079c72e3f820781beb5d1b36ae9
    evidence_commit: 0096010a36ebf31a36149064d64059142ce7cfed
    source_manifest_sha256: 03c3c017f7d6eb357ae657e992ef3c3988932f6870ac3a18192dfd9e343ee05f

  runtime_proton_digest: 2d64df1d36786ca2d0e955c553005423dc2b5bdd714bd0a17872622e33912547

  module:
    name: AGain VST3
    sha256: 60aa9ff6b9918d4330449e7b3ab34b588dd93cba09f37413a3cd91f6e7d2e18f

  processor:
    class_id: 84E8DE5F92554F5396FAE4133C935A18
    raw_windows_tuid: 5FDEE8845592534F96FAE4133C935A18
    requested_interface: Steinberg::Vst::IComponent

  expected_controller:
    class_id: D39D5B65D7AF42FA843F4AC841EB04F0
    raw_windows_tuid: 655B9DD3AFD7FA42843F4AC841EB04F0

  bitwig:
    version: "6.1"
    posture: protected_unlaunched_state_only

authority_phase: reconnaissance_and_design
implementation_authorized: false

design_gate: required
design_gate_reason: >-
  WC0 introduces the first created VST3 class object, a repository-owned
  IHostApplication context, COM-style component ownership, initialize and
  terminate lifecycle, third-party failure boundaries, and a new component
  compatibility claim.

design_card_path: docs/slices/WC0/IMPLEMENTATION_DESIGN.md
design_approval_path: docs/slices/WC0/DESIGN_APPROVAL.md

allowed_changed_paths:
  - CURRENT_SLICE.md
  - docs/slices/WC0/SLICE_SELECTION.md
  - docs/slices/WC0/IMPLEMENTATION_DESIGN.md

permitted_external_mutation:
  - none during reconnaissance and design

accepted_infrastructure_reuse:
  - WF0 Windows/MSVC build plane
  - WF0 private Actions artifact custody
  - WF0 Mac-to-Deck source and artifact handoff
  - WF0 detached Deck execution worktree
  - WF0 Runtime 4 / Proton 11 supervisor
  - WF0 disposable environment ownership
  - WF0 module/factory load and unload lifecycle

accepted_infrastructure_reproof:
  - exact identity preflight
  - one bounded regression only
  - no full WF0 proof-matrix replay
  - no infrastructure redesign

protected_state:
  - accepted WF0 source, evidence, build, custody, and runtime identities
  - accepted WR0 environment and receipts
  - exact Runtime 4 and Proton 11 assets
  - current Bitwig 6.1 installation and Flatpak configuration
  - historical SR0, HP0, HP1, WR0, WR0A, and WF0 evidence
  - Serum and all vendor material
  - Steam compatdata and .wine
  - SteamOS read-only posture
  - every repository path outside the bounded WC0 design authority

explicit_nonclaims:
  - no edit-controller instance
  - no IConnectionPoint pairing
  - no IAudioProcessor interface census
  - no bus enumeration or activation
  - no parameter enumeration
  - no component or controller state
  - no processing setup
  - no audio or event processing
  - no GUI or editor
  - no native Linux proxy
  - no C ABI or IPC
  - no Bitwig execution
  - no Serum execution or authorization
  - no packaging or runner selection
  - no general Windows VST3 or Linux compatibility claim

material_discovery_requires_stop: true
successor_selection_authorized: false

operator_approval_text: >-
  I explicitly approve selecting WC0 — Windows VST3 Processor Component
  Admission and replacing the no-active-slice card with its bounded
  reconnaissance-and-design authority. This approval does not authorize
  implementation. Implementation requires a separate approved design revision.
approved_at: 2026-09-02T07:20:00-07:00
```

## Technical-lead interpretation

This approval activates design only. WC0 is successful only if it remains one object lifecycle:

```text
create AGain processor as IComponent
    -> verify controller class ID
    -> initialize with minimal IHostApplication
    -> terminate
    -> release
```

Build, custody, transfer, Runtime/Proton, module loading, factory census, and process supervision are accepted WF0 infrastructure. They are reused, not redesigned.
