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
adversarial_review_path: docs/slices/WF0/ADVERSARIAL_DESIGN_REVIEW_V7.md
adversarial_review_git_blob: 962ba4ba031bcec79cb57ae234985ee962bb16ce
adversarial_review_github_id: 5084789559
adversarial_review_result: DESIGN_CLEAR
accepted_basis_commit: 67026ad7160a584cbf9cdb4bf0db7b8dbfa60136
accepted_basis_tree: 2021dfcbf000d934a462d40efef6bbe7b54e0862
reconciliation_path: docs/slices/WF0/WINDOWS_BUILD_PLANE_RECONCILIATION.md
superseded_design_revision: wf0-design-v6
superseded_design_commit: 1e14eb8c318263b7307fe8e07aae13628402e0c1
superseded_design_tree: 63f9ce1edfe663848e2a4832c7facad881b83aac
superseded_design_review_github_id: 5084539680
superseded_design_review_result: DESIGN_REPAIR_REQUIRED
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
three_plane_boundary:
  mac_control_plane: >-
    Owns Git, private GitHub repository authority, exact Actions run and
    artifact custody, exact implementation-source Git bundle, SSH handoff,
    evidence retrieval, and Git publication.
  windows_build_plane: >-
    Owns the exact windows-2022 / Visual Studio 2022 / v143 / x64 / Windows
    SDK 10.0.19041.0 build of the scanner, fault fixtures, and pinned AGain
    module.
  steam_deck_execution_plane: >-
    Owns only exact local source and artifact admission, Runtime 4 / Proton 11
    execution, cleanup, protected-state verification, and bounded evidence
    generation.
exact_fixture:
  host: Steam Deck Galileo / SteamOS 3.8.16 / x86_64 / KDE Wayland
  steam_os_read_only: true
  runner_runtime_digest: 2d64df1d36786ca2d0e955c553005423dc2b5bdd714bd0a17872622e33912547
  accepted_wr0_environment_identity: d3ed38d7ed53e9a5973cbf22fc504f2479dbdd15616fe1bfc1d5e1cd0bf5f4c4
  accepted_wr0_posture: protected_read_only
  current_bitwig_version: 6.1
  current_bitwig_commit: 8a048e733e74dda8b897339436153a2d5df952f29d362dfcaca9d8e0d6f6c231
  current_bitwig_runtime_commit: bd44a6230581917d04f89812a4c21090c304d390edb73995af1c2f9fd8abf4e8
  current_bitwig_posture: protected_unlaunched
  vst3_sdk_root_commit: 3cdf9ca5d1f5b1b21e0a86832aa4abe55607bd96
  positive_fixture: official pinned AGain Windows x86_64 VST3
windows_build_plane:
  runner_label: windows-2022
  visual_studio_generation: Visual Studio 2022
  platform_toolset: v143
  architecture: x64
  windows_sdk: 10.0.19041.0
  cmake_generator: Visual Studio 17 2022
private_artifact_custody:
  artifact_attestation_required: false
  github_enterprise_cloud_required: false
  visibility_or_owner_change_authorized: false
  replacement_signing_authorized: false
  mac_custody_schema: linux-vst-bridge-wf0-mac-artifact-custody/v1
  required_join: >-
    Exact repository, workflow path and blob, implementation head SHA, run ID
    and attempt, artifact ID, artifact name, Actions artifact digest,
    exact-ID API download, three-file inner envelope, build receipt, payload
    digest, artifact-manifest digest, Mac custody receipt, and Deck byte
    readback.
source_handoff:
  schema: linux-vst-bridge-wf0-source-handoff/v1
  transport: ordinary Mac-to-Deck SSH without credential or agent forwarding
  deck_local_ref: refs/handoff/wf0-v7-source/<implementation-commit>
  execution_source: clean detached worktree at exact implementation commit
  per_run_source_requirement: exact 26-record implementation-source manifest
  deck_github_dependency: none
approved_owner_model: true
approved_state_machines: true
approved_mutation_fault_ledger: true
approved_identity_ledger: true
approved_proof_matrix: true
approved_changed_path_envelope: true
approved_security_posture: true
approved_material_stop_law: true
approved_nonclaims: true
approved_owner_count: 14
approved_material_operations: 25
approved_proof_matrix_rows: 62
approved_blocked_results: 29
approved_implementation_path_count: 40
approved_implementation_source_path_count: 26
approved_evidence_path_count: 14
binding_implementation_laws:
  - >-
    Every ordinary return from all 15 closed call operations must be followed
    immediately by its paired synchronously flushed call_completed record
    before any later lifecycle event or call attempt.
  - >-
    Required GetPluginFactory is resolved and checked before optional InitDll.
    A missing export never resolves or invokes InitDll. With a present export,
    optional entry handling completes, its lifecycle result is published,
    factory_export_found is then published, and only an entry-absent or
    entry-succeeded branch invokes the retained factory export.
  - >-
    Cleanup failure remains secondary to the first primary stage failure.
  - >-
    No class-instantiation operation exists in the scanner command, event enum,
    or retained schema.
  - >-
    The upload action human-facing artifact URL and the REST artifact object
    API URL are distinct typed values and must not be compared for literal
    string equality. They are joined by artifact ID, repository, run, head
    SHA, name, and digest. The Mac custody receipt URL is the REST API URL.
material_discovery_requires_stop: true
implementation_authorized: true
successor_selection_authorized: false
technical_lead_approval_text: >-
  I approve the immutable wf0-design-v7 card as DESIGN_CLEAR under GitHub
  review 5084789559. V7 truthfully binds the private user-owned repository,
  exact Actions artifact custody, exact implementation-source Git-bundle
  handoff, clean detached Deck execution worktree, no-Deck-GitHub boundary,
  unchanged scanner and process laws, and the exact 40-path claim envelope.
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
approved_at: 2026-09-01T17:53:00-07:00
```

## Authority effect

This receipt approves the immutable `wf0-design-v7` card and technical-lead review `5084789559`. It does not make the WF0 technical claim true. Implementation begins only from the exact merged V7 authority and subsequent exact `main` readback.

Implementation is limited to the approved 26 implementation/configuration paths, 14 evidence paths, private Actions artifact custody, private source-bundle handoff, bounded Mac/Deck SSH transfers, exact Windows/MSVC build plane, exact Runtime 4 / Proton 11 execution plane, and the declared external mutation envelope.

No GitHub artifact attestation, visibility or ownership change, replacement signing, Bitwig execution, Serum execution, class instantiation, IPC, proxy, audio, GUI, packaging, product-runner selection, successor slice, or implementation-agent merge is authorized.

```text
implementation_authorized=true
successor_selection_authorized=false
```
