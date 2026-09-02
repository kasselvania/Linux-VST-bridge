# WA0 Design Approval Receipt

```yaml
schema: linux-vst-bridge-design-approval/v1
repository: kasselvania/Linux-VST-bridge
slice_id: WA0
slice_title: Windows VST3 Audio-Processor Interface Admission
design_revision: wa0-design-v1
design_card_path: docs/slices/WA0/IMPLEMENTATION_DESIGN.md
design_card_git_blob: d927c6dae430ffe7a811193e3d9119e573cfe316
design_card_sha256: eb5bd8f3aa439944f5933ecc9f0c4bcb90d46b3657499b365d11a82c0e2858ac
design_commit: 0d5a41936c171f5d01d01b4c933875a1cfbe724a
design_tree: 787f1adf99e6083c3c90fa73996f6c151d2de50c
design_parent: 5f647b949d429403e3c6e5f2ff91a1eb0104da6a
accepted_design_basis_commit: 68f52e5b9678d87b13547d3bb37078a0e6a1255c
accepted_design_basis_tree: 776a01340eca9ec456b9c063cf9201d31bd9f3e3
selection_activation_commit: 5f647b949d429403e3c6e5f2ff91a1eb0104da6a
selection_activation_tree: 0fa2adda90dbbd68d6862240061604ddc0d1c396
adversarial_review_github_id: 5094205619
adversarial_review_url: https://github.com/kasselvania/Linux-VST-bridge/pull/31#pullrequestreview-5094205619
adversarial_review_result: DESIGN_CLEAR
primary_claim: >-
  On the exact accepted Steam Deck fixture, the existing initialized AGain
  processor component exposes the mandatory Steinberg::Vst::IAudioProcessor
  interface through the exact interface query, the repository-owned Windows
  host acquires and retires exactly one such interface reference without
  calling any audio-processor method, and the accepted WC0 component, factory,
  module, process, environment, and protected-state shutdown remains exact.
exact_fixture:
  accepted_wc0_implementation_merge: cb831c38e1be88f4bb6a0ab6f2fca2d94164891b
  accepted_wc0_source_commit: 9c0096930df86fc5b171cdebec40b306a198316a
  accepted_wc0_evidence_commit: 77bb40dbf35e19fd93b9f79b5286a43d56b9cc21
  accepted_wc0_source_manifest_sha256: 38699a1d2026cb1078a569dc1997122b0111c78e608f294777dcf4afc49c8b25
  accepted_wc0_scanner_sha256: 51b899b7936921b24265ead9ff12180f14249d49b532ead9a18414559f5f83f7
  accepted_wc0_artifact_manifest_sha256: 25bd47471f01ef57b06b3c8232bb6cc7e40c767f281c30186fa5426130f8ae82
  runner_runtime_digest: 2d64df1d36786ca2d0e955c553005423dc2b5bdd714bd0a17872622e33912547
  again_module_sha256: 60aa9ff6b9918d4330449e7b3ab34b588dd93cba09f37413a3cd91f6e7d2e18f
  processor_logical_cid: 84E8DE5F92554F5396FAE4133C935A18
  processor_raw_windows_tuid: 5FDEE8845592534F96FAE4133C935A18
  component_interface: Steinberg::Vst::IComponent
  component_logical_iid: E831FF31F2D54301928EBBEE25697802
  component_raw_windows_tuid: 31FF31E8D5F20143928EBBEE25697802
  admitted_interface: Steinberg::Vst::IAudioProcessor
  admitted_interface_logical_iid: 42043F99B7DA453CA569E79D9AAEC33D
  admitted_interface_raw_windows_tuid: 993F0442DAB73C45A569E79D9AAEC33D
  current_bitwig_version: "6.1"
  current_bitwig_posture: protected_unlaunched_state_only
approved_owner_model: true
approved_owner_count: 1
approved_owner: AudioProcessorInterfaceLease
approved_state_machine: true
approved_lifecycle_states: 8
approved_new_call_operations: 2
approved_query_result_output_cases: 4
approved_interface_quiescence_law: true
approved_mutation_fault_ledger: true
approved_focused_negative_fixtures: 8
approved_identity_ledger: true
approved_proof_matrix: true
approved_proof_matrix_rows: 20
approved_blocked_results: 12
approved_changed_path_envelope: true
approved_implementation_source_path_count: 17
approved_evidence_path_count: 14
approved_total_tracked_path_count: 31
approved_external_mutation_envelope: true
accepted_wf0_wc0_infrastructure_reuse: >-
  The accepted WF0 and WC0 Windows/MSVC build plane, private Actions artifact
  custody, source and artifact handoff, detached Steam Deck worktree,
  content-addressed artifact admission, Runtime 4 / Proton 11 supervision,
  disposable environment ownership, component lifecycle, factory/module
  shutdown, process cleanup, and protected-state comparison are reused without
  redesign or broad proof-matrix replay.
material_discovery_requires_stop: true
implementation_authorized: true
successor_selection_authorized: false
technical_lead_approval_text: >-
  GitHub review 5094205619 returned DESIGN_CLEAR for the immutable
  wa0-design-v1 card. It clears the exact one-owner, eight-state, two-call,
  four-case query ownership, interface-quiescence, eight-fixture, 20-row proof,
  12-result blocker, and 17-source/14-evidence design without authorizing any
  IAudioProcessor method or redesign of inherited infrastructure.
operator_approval_text: |-
  I explicitly approve implementation design revision wa0-design-v1 for
  WA0 — Windows VST3 Audio-Processor Interface Admission.

  Approved design path:
  docs/slices/WA0/IMPLEMENTATION_DESIGN.md

  Approved design SHA-256 or Git blob:
  Git blob d927c6dae430ffe7a811193e3d9119e573cfe316 and SHA-256 eb5bd8f3aa439944f5933ecc9f0c4bcb90d46b3657499b365d11a82c0e2858ac

  Implementation is authorized only for the exact primary claim, one-owner
  AudioProcessorInterfaceLease model, eight-state lifecycle, two closed call
  operations, four-case query result/output ownership matrix, interface-
  quiescence law, failure precedence, eight focused negative fixtures, 20-row
  proof matrix, 12-result blocked taxonomy, exact AGain and IAudioProcessor
  fixture, and exact 17-source/14-evidence, 31-path implementation envelope
  contained in wa0-design-v1.

  Technical-lead review 5094205619, with result DESIGN_CLEAR, is binding.

  Accepted WF0 and WC0 build, custody, source and artifact handoff,
  Runtime/Proton, supervisor, environment, component, factory, module,
  process-cleanup, and protected-state infrastructure must be reused without
  redesign or broad proof-matrix replay.

  No IAudioProcessor method may be called. No edit-controller object,
  IConnectionPoint, bus, parameter, state, processing setup, audio or event
  path, GUI or editor, native proxy, C ABI, IPC or shared memory, Bitwig,
  Serum, packaging, signing, runner selection, another plug-in or DAW, or
  general compatibility claim is authorized.

  A material design discovery requires implementation to stop and return to
  the design gate. No successor slice or additional design amendment is
  authorized by this approval.
approved_at: 2026-09-02T12:43:04-07:00
```

## Authority effect

This receipt approves the immutable `wa0-design-v1` card and authorizes
implementation only after this receipt, the exact design, review disposition,
and implementation-authority `CURRENT_SLICE.md` are merged into `main`.

The implementation branch must begin at that exact design-authority merge
commit and tree. Its source commit is limited to the approved 17-path roster;
its evidence-only commit is limited to the approved fourteen-file WA0 packet.

```text
implementation_authorized=true
successor_selection_authorized=false
```
