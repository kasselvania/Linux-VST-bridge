# WA0 Slice Selection Receipt

```yaml
schema: linux-vst-bridge-slice-selection/v1
repository: kasselvania/Linux-VST-bridge
basis_commit: 68f52e5b9678d87b13547d3bb37078a0e6a1255c
basis_tree: 776a01340eca9ec456b9c063cf9201d31bd9f3e3

selected_slice: WA0
selected_title: Windows VST3 Audio-Processor Interface Admission

primary_claim: >-
  On the exact accepted Steam Deck fixture, the existing initialized AGain
  processor component exposes the mandatory Steinberg::Vst::IAudioProcessor
  interface through the exact interface query, the repository-owned Windows
  host acquires and retires exactly one such interface reference without
  calling any audio-processor method, and the accepted WC0 component, factory,
  module, process, environment, and protected-state shutdown remains exact.

exact_fixture:
  repository_main:
    commit: 68f52e5b9678d87b13547d3bb37078a0e6a1255c
    tree: 776a01340eca9ec456b9c063cf9201d31bd9f3e3

  accepted_wc0:
    implementation_merge: cb831c38e1be88f4bb6a0ab6f2fca2d94164891b
    source_commit: 9c0096930df86fc5b171cdebec40b306a198316a
    source_tree: e60286740a3aff7589e0dc9bb3b278e68a23374e
    evidence_commit: 77bb40dbf35e19fd93b9f79b5286a43d56b9cc21
    evidence_tree: 43fab0b6341e2549775b7f4223965031521ff338
    source_manifest_sha256: 38699a1d2026cb1078a569dc1997122b0111c78e608f294777dcf4afc49c8b25
    scanner_sha256: 51b899b7936921b24265ead9ff12180f14249d49b532ead9a18414559f5f83f7
    artifact_manifest_sha256: 25bd47471f01ef57b06b3c8232bb6cc7e40c767f281c30186fa5426130f8ae82

  runtime_proton_digest: 2d64df1d36786ca2d0e955c553005423dc2b5bdd714bd0a17872622e33912547

  module:
    name: AGain VST3
    sha256: 60aa9ff6b9918d4330449e7b3ab34b588dd93cba09f37413a3cd91f6e7d2e18f

  processor:
    class_id: 84E8DE5F92554F5396FAE4133C935A18
    raw_windows_tuid: 5FDEE8845592534F96FAE4133C935A18

  component_interface:
    name: Steinberg::Vst::IComponent
    logical_iid: E831FF31F2D54301928EBBEE25697802
    raw_windows_tuid: 31FF31E8D5F20143928EBBEE25697802

  audio_processor_interface:
    name: Steinberg::Vst::IAudioProcessor
    logical_iid: 42043F99B7DA453CA569E79D9AAEC33D
    raw_windows_tuid: 993F0442DAB73C45A569E79D9AAEC33D

  bitwig:
    version: "6.1"
    posture: protected_unlaunched_state_only

authority_phase: reconnaissance_and_design
implementation_authorized: false

design_gate: required
design_gate_reason: >-
  WA0 introduces a new COM-style interface lease on a live third-party VST3
  object, changes the component object-quiescence proof, adds query/release
  failure boundaries and a new compatibility claim, and therefore requires an
  exact owner, state, fault, identity, and proof design before implementation.

design_card_path: docs/slices/WA0/IMPLEMENTATION_DESIGN.md
design_approval_path: docs/slices/WA0/DESIGN_APPROVAL.md

design_branch: codex/wa0-windows-vst3-audio-processor-interface-design
implementation_branch: null

allowed_changed_paths:
  - CURRENT_SLICE.md
  - docs/slices/WA0/SLICE_SELECTION.md
  - docs/slices/WA0/IMPLEMENTATION_DESIGN.md

permitted_reconnaissance:
  - read repository authority and accepted WC0 source/evidence
  - read exact pinned official VST3 SDK and submodule sources
  - read official IAudioProcessor and FUnknown/queryInterface definitions
  - read AGain and AudioEffect interface implementation
  - inspect existing source/evidence schemas without mutation

permitted_external_mutation:
  - none during reconnaissance and design

accepted_infrastructure_reuse:
  - WC0 processor creation and minimal host-context lifecycle
  - WC0 object-quiescence gate and inherited shutdown
  - WF0 Windows/MSVC build plane
  - WF0 private Actions artifact custody
  - WF0 Mac-to-Deck source and artifact handoff
  - WF0 detached Deck execution worktree
  - WF0 Runtime 4 / Proton 11 supervisor
  - WF0 disposable environment ownership and cleanup
  - WF0 module and factory lifecycle

accepted_infrastructure_reproof:
  - exact identity preflight
  - one bounded regression only where WA0 directly extends the lifecycle
  - no full WF0 or WC0 proof-matrix replay
  - no infrastructure redesign

required_positive_order:
  - WC0 component initialized
  - query exact IAudioProcessor IID
  - acquire exactly one non-null interface lease
  - release that lease exactly once to the component-owner baseline
  - WC0 terminate
  - WC0 component release to zero
  - inherited factory and module shutdown

required_design_laws:
  - exact query result and output-pointer consistency
  - no pointer-address equality claim
  - exact logical IID and raw Windows TUID binding
  - exactly one acquired IAudioProcessor lease
  - exactly one release of that lease
  - no IAudioProcessor method call
  - no edit-controller creation
  - deterministic unmatched query/release attribution
  - object quiescence requires the interface lease to be absent
  - physical containment cannot manufacture clean interface retirement
  - material ownership or ordering discovery returns to design

protected_state:
  - accepted WC0 source, evidence, build, custody, object-lifecycle, and runtime identities
  - accepted WF0 source, evidence, build, custody, and runtime identities
  - accepted WR0 environment and receipts
  - exact Runtime 4 and Proton 11 assets
  - current Bitwig 6.1 installation and Flatpak configuration
  - historical SR0, HP0, HP1, WR0, WR0A, WF0, and WC0 evidence
  - Serum and all vendor material
  - Steam compatdata and .wine
  - SteamOS read-only posture
  - every repository path outside the bounded WA0 design authority

explicit_nonclaims:
  - no IAudioProcessor method invocation
  - no sample-size capability query
  - no bus arrangement query or mutation
  - no processing setup
  - no setProcessing or process call
  - no latency or tail query
  - no edit-controller instance
  - no IConnectionPoint pairing
  - no component bus enumeration or activation
  - no parameter enumeration
  - no component or controller state
  - no audio or event processing
  - no GUI or editor
  - no native Linux proxy
  - no C ABI or IPC
  - no Bitwig execution
  - no Serum execution or authorization
  - no packaging, signing, release, or runner selection
  - no general Windows VST3 or Linux compatibility claim

material_discovery_requires_stop: true
successor_selection_authorized: false

operator_approval_text: >-
  I explicitly approve selecting WA0 — Windows VST3 Audio-Processor Interface
  Admission and replacing the no-active-slice card with its bounded
  reconnaissance-and-design authority. This approval does not authorize
  implementation. Implementation requires a separate approved design revision.
approved_at: 2026-09-02T12:02:13-07:00
```

## Technical-lead interpretation

This approval activates design only. WA0 is successful only if it remains one balanced interface lease inside the already-accepted WC0 processor lifecycle:

```text
initialized AGain IComponent
    -> query IAudioProcessor
    -> own one reference
    -> release to component baseline
    -> continue WC0 retirement
```

No audio-processor method, controller object, bus, state, processing, audio, proxy, or IPC work belongs in WA0.
