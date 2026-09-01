# WF0 V5 Design Approval Receipt

```yaml
schema: linux-vst-bridge-design-approval/v1
repository: kasselvania/Linux-VST-bridge
slice_id: WF0
slice_title: Supervised Windows VST3 Factory Census Probe
design_revision: wf0-design-v5
design_card_path: docs/slices/WF0/IMPLEMENTATION_DESIGN_V5.md
design_card_git_blob: 74fd8dc5616bf6a5bca45753503947681b35cfaf
design_card_sha256: 7d6db4dde4f0e5795757699a46f4443a5cbc19ceb917b5dc789b29c931499cc5
design_commit: 6fc3632a04fdba9b8fc20794205e97881c4313f9
design_tree: 0e6b666b28edf6be510d9d29dfa501b5460312bc
adversarial_review_path: docs/slices/WF0/ADVERSARIAL_DESIGN_REVIEW_V5.md
adversarial_review_git_blob: 8abbbdaf60fc00e00eb0a036fe9463ab4da35984
adversarial_review_github_id: 5083590311
adversarial_review_result: DESIGN_CLEAR
reviewed_amendment_head: 38a5a87239ef91cb4537ec23aca1e72829ef5861
amendment_basis_commit: 15523c69567d24b256cb3c65cb6f06bfa07854be
amendment_basis_tree: a6e2fc8c7a564f50e9033094fde847387478de62
fixture_reconciliation_path: docs/slices/WF0/BITWIG_6_1_FIXTURE_RECONCILIATION.md
primary_claim: >-
  On the exact accepted Steam Deck fixture, a repository-owned supervised
  Windows x86_64 factory probe, built against the pinned official VST3 SDK and
  executed through the accepted Runtime 4 / Proton 11 lane in a disposable
  WF0-owned scan environment, loads the exact pinned AGain VST3 bundle,
  obtains its plug-in factory, retains deterministic factory metadata and the
  complete expected three-class census, unloads cleanly, and leaves the
  accepted WR0 environment and every protected fixture unchanged.
exact_fixture:
  host: Steam Deck Galileo / SteamOS 3.8.16 / x86_64 / KDE Wayland
  runner_runtime_digest: 2d64df1d36786ca2d0e955c553005423dc2b5bdd714bd0a17872622e33912547
  accepted_wr0_environment_identity: d3ed38d7ed53e9a5973cbf22fc504f2479dbdd15616fe1bfc1d5e1cd0bf5f4c4
  accepted_wr0_posture: protected_read_only
  current_bitwig_version: 6.1
  current_bitwig_ref: app/com.bitwig.BitwigStudio/x86_64/stable
  current_bitwig_commit: 8a048e733e74dda8b897339436153a2d5df952f29d362dfcaca9d8e0d6f6c231
  current_bitwig_scope: system
  current_bitwig_origin: flathub
  current_bitwig_user_shadow: absent
  current_bitwig_runtime_ref: org.freedesktop.Platform/x86_64/25.08
  current_bitwig_runtime_commit: bd44a6230581917d04f89812a4c21090c304d390edb73995af1c2f9fd8abf4e8
  current_bitwig_user_override_sha256: 1b4a6a6ed688f69dd3c36dcac8db008c5a41ed52170ea3e23dee984b0aae6a1e
  current_bitwig_system_override_sha256: e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
  current_bitwig_permission_output_sha256: c7f5a34dce104cc3d347dcaf89e135cc5ad73b891101d7587f78423858ad5c73
  freedesktop_sdk_commit: b90ed309cc1d505dea48b6a2121c5dcfac22868120eee643b0596d31f96b9bb8
  mingw_extension_ref: org.freedesktop.Sdk.Extension.mingw-w64/x86_64/25.08
  mingw_extension_commit: f15a5a88eb09557f645860bfcb3c4a6bc267683fce06cb68436bd76376fca694
  vst3_sdk_root_commit: 3cdf9ca5d1f5b1b21e0a86832aa4abe55607bd96
  positive_fixture: official pinned AGain Windows VST3
approved_owner_model: true
approved_state_machine: true
approved_mutation_fault_ledger: true
approved_identity_ledger: true
approved_proof_matrix: true
approved_changed_path_envelope: true
approved_external_mutation_envelope: true
approved_implementation_path_count: 40
approved_implementation_source_path_count: 26
approved_evidence_path_count: 14
approved_proof_matrix_rows: 35
approved_blocked_results: 23
approved_material_operations: 14
bitwig_6_1_risk_posture: >-
  Bitwig 6.1 is approved as current protected state without HP0 or HP1
  behavioral revalidation on 6.1. WF0 does not launch Bitwig and makes no
  Bitwig 6.1 discovery, scanning, instance, project, audio, GUI, state, or
  compatibility claim.
binding_implementation_laws:
  - >-
    Every ordinary return from all 15 closed call operations must be followed
    immediately by its paired, synchronously flushed call_completed record
    before any later lifecycle event or call attempt.
  - >-
    Required GetPluginFactory must be resolved and checked before optional
    InitDll. A missing required export must not execute InitDll. For a present
    export, successful resolution is retained internally, optional entry
    handling completes, the entry lifecycle result is published,
    factory_export_found is then published, and only an entry-absent or
    entry-succeeded branch may invoke the already-resolved factory export.
material_discovery_requires_stop: true
implementation_authorized: true
successor_selection_authorized: false
technical_lead_approval_text: >-
  I approve the immutable wf0-design-v5 card as DESIGN_CLEAR under GitHub
  review 5083590311. The current Bitwig 6.1 installation is protected state
  only, the complete historical 6.0.11 evidence remains unchanged, and the
  approved WF0 implementation remains limited to the exact factory-census
  claim and 40-path envelope.
operator_approval_text: |-
  I explicitly approve implementation design revision wf0-design-v5 for
  WF0 — Supervised Windows VST3 Factory Census Probe.

  Approved design path:
  docs/slices/WF0/IMPLEMENTATION\_DESIGN\_V5.md

  Approved design SHA-256 or Git blob:
  Git blob 74fd8dc5616bf6a5bca45753503947681b35cfaf and SHA-256 7d6db4dde4f0e5795757699a46f4443a5cbc19ceb917b5dc789b29c931499cc5

  I acknowledge and approve the current Bitwig 6.1 stable installation as
  protected state for WF0 without requiring HP0 or HP1 behavioral revalidation
  on Bitwig 6.1. No Bitwig 6.1 discovery, scanning, instance, project, audio,
  GUI, state, or compatibility claim is authorized by this approval.

  Implementation is authorized only for the exact primary claim, owner map,
  state machine, mutation/fault ledger, identity ledger, proof matrix,
  40-path changed-path envelope, fixture, and nonclaims contained in that
  revision.

  The following inherited implementation laws remain binding:

  1. Every ordinary return from all 15 closed call operations must be followed
     immediately by its paired, synchronously flushed call_completed record before
     any later lifecycle event or call attempt.

  2. Required GetPluginFactory must be resolved and checked before optional
     InitDll. A missing required export must not execute InitDll. For a present
     export, successful resolution is retained internally, optional entry handling
     completes, the entry lifecycle result is published, factory_export_found is
     then published, and only an entry-absent or entry-succeeded branch may invoke
     the already-resolved factory export.

  This approval binds technical-lead review 5083590311.

  A material design discovery requires implementation to stop and return to the
  design gate. No successor slice or additional design amendment is authorized
  by this approval.
approved_at: 2026-09-01T15:15:22-07:00
```

## Authority effect

This receipt approves the immutable `wf0-design-v5` card and the Bitwig 6.1 protected-fixture amendment. It does not itself make the WF0 technical claim true. Implementation begins only from the exact merged amendment-authority basis and remains limited to the approved 40 tracked paths and external mutation envelope.

The exact approved user-scope MinGW extension installation is permitted only as part of WF0 implementation. No other package, runner, SDK, system, Bitwig, Serum, accepted WR0 environment, `.wine`, Steam-compatdata, or vendor-state mutation is authorized.

```text
implementation_authorized=true
successor_selection_authorized=false
```
