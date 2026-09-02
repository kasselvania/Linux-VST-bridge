# WF0 V7 Design Approval Receipt

```yaml
schema: linux-vst-bridge-design-approval/v1
repository: kasselvania/Linux-VST-bridge
repository_visibility: private
repository_owner_type: User
slice_id: WF0
slice_title: Supervised Windows VST3 Factory Census Probe
design_revision: wf0-design-v7
design_card_path: docs/slices/WF0/IMPLEMENTATION_DESIGN_V7.md
design_card_git_blob: 3c8ac56cfaa8c95cb99fa327b392ed447453a87b
design_card_sha256: 844d646509933516ff60eeb2c6213cd1c2e89a8d22b5c986ac986fb104a19d77
design_commit: 1d13fefc60ad6c2c49e384cd30631f60be2a3de2
design_tree: 752b885643c730378ceefab99c7d7ec9277fdf56
design_parent: 1e14eb8c318263b7307fe8e07aae13628402e0c1
accepted_design_basis_commit: 67026ad7160a584cbf9cdb4bf0db7b8dbfa60136
accepted_design_basis_tree: 2021dfcbf000d934a462d40efef6bbe7b54e0862
adversarial_review_path: docs/slices/WF0/ADVERSARIAL_DESIGN_REVIEW_V7.md
adversarial_review_git_blob: 36f6f7579f3e31e2d52a8d8e86b2950f56eaec90
adversarial_review_github_id: 5084789559
adversarial_review_result: DESIGN_CLEAR
superseded_v6_review_github_id: 5084539680
superseded_v6_review_result: DESIGN_REPAIR_REQUIRED
windows_build_plane_reconciliation_path: docs/slices/WF0/WINDOWS_BUILD_PLANE_RECONCILIATION.md
provisional_source_commit: 5b166f4902af5e1e3b9c2287512f1f8f099c51d0
provisional_source_tree: fb6a14a78bcab6dd11b5720a725e9df841714743
provisional_source_bundle_sha256: c442d429ff20447d616c62c4273bbefaedbe4b28cd4631e3fc5683c4bb2d1143
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
  current_bitwig_commit: 8a048e733e74dda8b897339436153a2d5df952f29d362dfcaca9d8e0d6f6c231
  current_bitwig_runtime_commit: bd44a6230581917d04f89812a4c21090c304d390edb73995af1c2f9fd8abf4e8
  current_bitwig_user_override_sha256: 1b4a6a6ed688f69dd3c36dcac8db008c5a41ed52170ea3e23dee984b0aae6a1e
  current_bitwig_system_override_sha256: e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
  current_bitwig_permission_output_sha256: c7f5a34dce104cc3d347dcaf89e135cc5ad73b891101d7587f78423858ad5c73
  windows_runner_label: windows-2022
  visual_studio_generation: Visual Studio 2022
  platform_toolset: v143
  architecture: x64
  windows_sdk: 10.0.19041.0
  vst3_sdk_root_commit: 3cdf9ca5d1f5b1b21e0a86832aa4abe55607bd96
  positive_fixture: official pinned AGain Windows VST3
approved_three_plane_model:
  mac_control_plane: >-
    Owns Git, GitHub, exact private Actions artifact custody, the exact
    implementation-source Git bundle, SSH handoff, evidence retrieval, and
    Git publication.
  windows_build_plane: >-
    Owns the exact windows-2022 / Visual Studio 2022 / v143 / x64 / Windows
    SDK 10.0.19041.0 build of the scanner, fault fixtures, and pinned AGain.
  steam_deck_execution_plane: >-
    Owns only exact local source and artifact admission, Runtime 4 / Proton 11
    execution, cleanup, protected-state verification, and bounded evidence
    generation.
approved_owner_model: true
approved_state_machines: true
approved_mutation_fault_ledger: true
approved_identity_ledger: true
approved_proof_matrix: true
approved_changed_path_envelope: true
approved_security_posture: true
approved_external_mutation_envelope: true
approved_implementation_path_count: 40
approved_implementation_source_path_count: 26
approved_evidence_path_count: 14
approved_proof_matrix_rows: 62
approved_blocked_results: 29
approved_material_operations: 25
approved_owner_count: 14
private_artifact_custody: >-
  The exact private Actions artifact is bound through repository, workflow
  path and blob, implementation head SHA, run ID and attempt, artifact ID,
  artifact name, Actions artifact digest, exact-ID API download, three-file
  inner envelope, build receipt, payload digest, artifact-manifest digest,
  Mac custody receipt, and Deck byte readback. No cryptographic provenance,
  trusted-builder, SLSA, code-signing, or release-signing claim follows.
source_handoff_law: >-
  The exact implementation source reaches the Deck through the approved
  self-contained Git bundle and source-handoff receipt over ordinary SSH with
  no credential or agent forwarding. The Deck imports only under the fixed
  local handoff ref, executes from a clean detached worktree at the exact
  implementation commit, and reproduces the 26-record source manifest before
  every Deck run.
binding_implementation_clarifications:
  - >-
    The upload action's human-facing artifact URL and the REST artifact
    object's API URL are distinct typed values and must not be compared for
    literal string equality. They are joined through exact artifact ID,
    repository, run, head SHA, name, and digest. The Mac custody receipt's
    url field is the REST API artifact URL; the human-facing upload URL may be
    retained separately as bounded transport metadata.
  - >-
    Every ordinary return from all 15 closed call operations must be followed
    immediately by its paired synchronously flushed call_completed record
    before any later lifecycle event or call attempt.
  - >-
    Required GetPluginFactory must be resolved and checked before optional
    InitDll. A missing required export must not execute InitDll. For a present
    export, optional entry handling completes, its lifecycle result is
    published, factory_export_found is then published, and only an
    entry-absent or entry-succeeded branch may invoke the retained export.
material_discovery_requires_stop: true
implementation_authorized: true
successor_selection_authorized: false
technical_lead_approval_text: >-
  The immutable wf0-design-v7 card is DESIGN_CLEAR under GitHub review
  5084789559. It closes exact private Actions artifact custody and exact
  implementation-source custody to the Deck without adding a Deck GitHub
  dependency or widening the scanner, process, factory, census, or claim
  boundary.
operator_approval_text: |-
  I explicitly approve implementation design revision wf0-design-v7 for
  WF0 — Supervised Windows VST3 Factory Census Probe.

  Approved design path:
  docs/slices/WF0/IMPLEMENTATION_DESIGN_V7.md

  Approved design SHA-256 or Git blob:
  Git blob 3c8ac56cfaa8c95cb99fa327b392ed447453a87b and SHA-256 844d646509933516ff60eeb2c6213cd1c2e89a8d22b5c986ac986fb104a19d77

  I approve the exact three-plane implementation boundary:

  - the Mac control plane owns Git, GitHub, exact private Actions artifact
    custody, the exact implementation-source Git bundle, SSH handoff, evidence
    retrieval, and Git publication;

  - the Windows build plane owns the exact windows-2022 / Visual Studio 2022 /
    v143 / x64 / Windows SDK 10.0.19041.0 build of the scanner, fault fixtures,
    and pinned AGain module;

  - the Steam Deck execution plane owns only exact local source and artifact
    admission, Runtime 4 / Proton 11 execution, cleanup, protected-state
    verification, and bounded evidence generation.

  I acknowledge that GitHub artifact attestation, GitHub Enterprise Cloud,
  repository visibility or ownership changes, Sigstore, code signing, release
  signing, and any replacement signing system are not required or claimed.

  Private Actions artifact custody must bind the exact repository, workflow path
  and blob, implementation head SHA, run ID and attempt, artifact ID, artifact
  name, Actions artifact digest, exact-ID API download, three-file inner
  envelope, build receipt, payload digest, artifact-manifest digest, Mac custody
  receipt, and Deck byte readback.

  The upload action's human-facing artifact URL and the REST artifact object's
  API URL are distinct typed values and must not be compared for literal string
  equality. They must be joined through the exact artifact ID, repository, run,
  head SHA, name, and digest, as clarified by technical-lead review 5084789559.

  The exact implementation source must reach the Deck through the approved
  self-contained Git bundle and source-handoff receipt over ordinary SSH, with
  no credential or agent forwarding. The Deck must import that source only under
  the fixed local handoff ref, execute from a clean detached worktree at the
  exact implementation commit, and reproduce the 26-record implementation-source
  manifest before every Deck run.

  Implementation is authorized only for the exact primary claim, three-plane
  owner model, state machines, 25-operation mutation/fault ledger, identity
  ledger, 62-row proof matrix, 29-result blocked taxonomy, 40-path tracked
  implementation/evidence envelope, fixture, security posture, stop law,
  nonclaims, and inherited scanner/call-order laws contained in wf0-design-v7.

  A material design discovery requires implementation to stop and return to the
  design gate. No successor slice or additional design amendment is authorized
  by this approval.
approved_at: 2026-09-01T19:00:16-07:00
```

## Authority effect

This receipt approves the immutable `wf0-design-v7` card and its exact three-plane custody model. It does not make the WF0 technical claim true. Implementation begins only from the exact merged V7 authority basis and remains limited to the approved 40 tracked paths and external mutation envelope.

The Deck is not required or authorized to authenticate to GitHub for ordinary WF0 work. The Mac owns private repository operations, exact Actions artifact custody, source-bundle creation, SSH handoff, evidence retrieval, and publication. The Windows build plane owns supported MSVC build truth. The Steam Deck owns exact local source/artifact admission and Runtime 4 / Proton 11 execution truth.

```text
implementation_authorized=true
successor_selection_authorized=false
```
