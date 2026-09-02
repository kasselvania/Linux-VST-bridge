# WC0 Design Approval Receipt

```yaml
schema: linux-vst-bridge-design-approval/v1
repository: kasselvania/Linux-VST-bridge
slice_id: WC0
slice_title: Windows VST3 Processor Component Admission
design_revision: wc0-design-v2
design_card_path: docs/slices/WC0/IMPLEMENTATION_DESIGN.md
design_card_git_blob: df31b8467af9dcd97bc06b6afdf5b4b8d6be7018
design_card_sha256: ca68cde6f68b02320e3c950b445aca9db99fdac1301fa7dfb50d510f83c7d78c
design_commit: 064db624056f0fdf4daeda3b5ae394ab6210bee2
design_tree: 6d7baf83e29704e803a76106c36d346cdbf7be03
design_parent: e712956b14f7e216a1f83db5888701a5ee072eb6
accepted_design_basis_commit: 1f4b946178319887950bdb764114543a1a7b845b
accepted_design_basis_tree: 75b91e9f4d8b059aef76abeb9ff6518cf0c310bc
selection_activation_commit: b6292a505666db8fc646fba35406dff7382d3e67
selection_activation_tree: 63b09755f9870253d31487cce1311f786ddfa15f
v1_adversarial_review_path: docs/slices/WC0/ADVERSARIAL_DESIGN_REVIEW.md
v1_adversarial_review_github_id: 5091717689
v1_adversarial_review_result: DESIGN_REPAIR_REQUIRED
v2_adversarial_review_github_id: 5092052158
v2_adversarial_review_result: DESIGN_CLEAR
primary_claim: >-
  On the exact accepted Steam Deck fixture, the existing supervised Windows
  VST3 host path creates the exact AGain processor class as one IComponent,
  verifies its declared controller class ID, initializes it with a minimal
  repository-owned IHostApplication, terminates and releases it correctly,
  and then completes the already-proven factory/module shutdown with no
  remaining object, process, environment, or protected-state residue.
exact_fixture:
  accepted_wf0_source_commit: 8b76ab886fd75079c72e3f820781beb5d1b36ae9
  accepted_wf0_evidence_commit: 0096010a36ebf31a36149064d64059142ce7cfed
  accepted_wf0_source_manifest_sha256: 03c3c017f7d6eb357ae657e992ef3c3988932f6870ac3a18192dfd9e343ee05f
  runner_runtime_digest: 2d64df1d36786ca2d0e955c553005423dc2b5bdd714bd0a17872622e33912547
  again_module_sha256: 60aa9ff6b9918d4330449e7b3ab34b588dd93cba09f37413a3cd91f6e7d2e18f
  processor_logical_cid: 84E8DE5F92554F5396FAE4133C935A18
  processor_raw_windows_tuid: 5FDEE8845592534F96FAE4133C935A18
  requested_interface: Steinberg::Vst::IComponent
  requested_interface_logical_iid: E831FF31F2D54301928EBBEE25697802
  requested_interface_raw_windows_tuid: 31FF31E8D5F20143928EBBEE25697802
  expected_controller_logical_cid: D39D5B65D7AF42FA843F4AC841EB04F0
  expected_controller_raw_windows_tuid: 655B9DD3AFD7FA42843F4AC841EB04F0
  current_bitwig_version: "6.1"
  current_bitwig_posture: protected_unlaunched_state_only
approved_owner_model: true
approved_owner_count: 2
approved_state_machine: true
approved_component_lifecycle_states: 13
approved_object_quiescence_law: true
approved_new_call_operations: 5
approved_host_callback_contract: true
approved_mutation_fault_ledger: true
approved_identity_ledger: true
approved_proof_matrix: true
approved_proof_matrix_rows: 29
approved_blocked_results: 13
approved_changed_path_envelope: true
approved_implementation_source_path_count: 19
approved_evidence_path_count: 14
approved_total_tracked_path_count: 33
approved_external_mutation_envelope: true
accepted_wf0_infrastructure_reuse: >-
  The accepted Windows/MSVC build plane, private Actions artifact custody,
  source and artifact handoff, detached Steam Deck worktree, content-addressed
  artifact admission, Runtime 4 / Proton 11 supervision, disposable
  environment ownership, factory census and release, ExitDll, FreeLibrary,
  process cleanup, and protected-state comparison are reused without redesign
  or complete proof-matrix replay.
binding_implementation_clarifications:
  - >-
    The complete controller TUID is zero-initialized before
    getControllerClassId and is consumed or retained only after an ordinary
    successful result.
  - >-
    Callback and reference-count evidence storage outlives the self-deleting
    MinimalHostApplication object. No field is read through the host pointer
    after its final release returns zero.
  - >-
    Deterministic unmatched-call attribution covers all five new operations,
    including create_component and get_controller_class_id, without expanding
    the live fault family.
material_discovery_requires_stop: true
implementation_authorized: true
successor_selection_authorized: false
technical_lead_approval_text: >-
  The immutable wc0-design-v2 card is DESIGN_CLEAR under GitHub review
  5092052158. It closes object-quiescence and exact source/build/handoff
  identity while preserving the selected one-component product boundary and
  reusing accepted WF0 infrastructure without redesign.
operator_approval_text: |-
  I explicitly approve implementation design revision wc0-design-v2 for
  WC0 — Windows VST3 Processor Component Admission.

  Approved design path:
  docs/slices/WC0/IMPLEMENTATION_DESIGN.md

  Approved design SHA-256 or Git blob:
  Git blob df31b8467af9dcd97bc06b6afdf5b4b8d6be7018 and SHA-256 ca68cde6f68b02320e3c950b445aca9db99fdac1301fa7dfb50d510f83c7d78c

  Implementation is authorized only for the exact primary claim, two-owner
  model, 13-state component lifecycle, object-quiescence law, five new call
  operations, host-callback contract, 29-row focused proof matrix, 13-result
  blocked taxonomy, exact AGain fixture, and 33-path tracked implementation and
  evidence envelope contained in wc0-design-v2.

  The following technical-lead clarifications retained by GitHub review
  5092052158 are binding:

  1. The complete controller TUID must be zero-initialized before
     getControllerClassId and may be consumed only after an ordinary successful
     result.

  2. Callback and reference-count evidence storage must outlive the
     self-deleting MinimalHostApplication object; no field may be read through
     the host pointer after its final release returns zero.

  3. Deterministic unmatched-call attribution must cover all five new
     operations, including create_component and get_controller_class_id, without
     expanding the live fault family.

  Accepted WF0 build, custody, source and artifact handoff, Runtime/Proton,
  environment, process-supervision, and cleanup infrastructure must be reused
  without redesign or complete proof-matrix replay.

  A material design discovery requires implementation to stop and return to the
  design gate. No successor slice or additional design amendment is authorized
  by this approval.
approved_at: 2026-09-02T09:08:29-07:00
```

## Authority effect

This receipt approves the immutable `wc0-design-v2` card and authorizes implementation only after this receipt, the approved design, the clear review, and implementation-authority `CURRENT_SLICE.md` are merged into `main`.

The implementation branch must begin from that exact design-authority merge commit and tree. Its source commit is limited to the approved 19-path source/configuration roster, and its evidence-only commit is limited to the approved fourteen-file WC0 evidence packet.

```text
implementation_authorized=true
successor_selection_authorized=false
```
