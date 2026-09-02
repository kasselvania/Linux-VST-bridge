# WF0 Implementation Design V7 — Private Actions Artifact Custody and Deck Source Handoff

## 1. Identity and authority

```yaml
slice: WF0
title: Supervised Windows VST3 Factory Census Probe
design_revision: wf0-design-v7
design_status: proposed_for_adversarial_review
implementation_authorized: false
superseded_design_revision: wf0-design-v6
superseded_design_commit: 1e14eb8c318263b7307fe8e07aae13628402e0c1
superseded_design_tree: 63f9ce1edfe663848e2a4832c7facad881b83aac
superseded_design_blob: e153943a302e3f9b38c74c9048f70a1d65c7b238
superseded_design_sha256: c6835b31ac8b2cf2f0fb1dee7634ef16f133387c0bffba8bf6671447175cd737
superseding_review: GitHub 5084539680 / DESIGN_REPAIR_REQUIRED
additional_live_reconnaissance_required: false
target_branch: main
accepted_basis_commit: 67026ad7160a584cbf9cdb4bf0db7b8dbfa60136
accepted_basis_tree: 2021dfcbf000d934a462d40efef6bbe7b54e0862
reconciliation_path: docs/slices/WF0/WINDOWS_BUILD_PLANE_RECONCILIATION.md
reconciliation_result: RETURN_TO_DESIGN_GATE_retained
provisional_source_commit: 5b166f4902af5e1e3b9c2287512f1f8f099c51d0
provisional_source_tree: fb6a14a78bcab6dd11b5720a725e9df841714743
provisional_source_status: preserved_salvage_material
provisional_bundle_sha256: c442d429ff20447d616c62c4273bbefaedbe4b28cd4631e3fc5683c4bb2d1143
selected_windows_runner_label: windows-2022
selected_visual_studio_generation: Visual Studio 2022
selected_platform_toolset: v143
selected_windows_sdk: 10.0.19041.0
selected_cmake_generator: Visual Studio 17 2022
selected_generator_platform: x64
repository_full_name: kasselvania/Linux-VST-bridge
repository_visibility: private
repository_owner_type: User
implementation_source_paths: 26
evidence_paths: 14
total_future_tracked_paths: 40
owners: 14
material_operations: 25
proof_matrix_rows: 62
blocked_results: 29
implementation_authorized: false
successor_selection_authorized: false
```

This is a design amendment, not an implementation authority. It supersedes
V6 only for two exact P1 custody repairs identified by GitHub technical-lead
review `5084539680`: the unavailable mandatory artifact-attestation dependency
and the missing implementation-source handoff to the Deck. Every other sound
V6 decision remains binding. V5's scanner, factory lifecycle, census schema,
Runtime 4 / Proton 11 execution route, protected-state law, and claim ceiling
remain unchanged.

After a reviewer binds an exact V7 commit, tree, card blob, and SHA-256, this
revision is immutable. Any later design repair requires a separately named
successor revision; reviewed V7 bytes are never amended in place.

### V5 lineage retained

```text
selection basis:
  commit 745ca63bdd8641ade85cb9a024c1dc842681192d
  tree   275521adff574f165bb2b3e883c2ec909bae4c66

amendment basis:
  commit 15523c69567d24b256cb3c65cb6f06bfa07854be
  tree   a6e2fc8c7a564f50e9033094fde847387478de62

V5 immutable design:
  commit 6fc3632a04fdba9b8fc20794205e97881c4313f9
  tree   0e6b666b28edf6be510d9d29dfa501b5460312bc
  blob   74fd8dc5616bf6a5bca45753503947681b35cfaf
  sha256 7d6db4dde4f0e5795757699a46f4443a5cbc19ceb917b5dc789b29c931499cc5

V5 review:
  blob   8abbbdaf60fc00e00eb0a036fe9463ab4da35984
  result DESIGN_CLEAR

V5 approval:
  blob   aabf50ae545a4a4a3d7efddc4abec00d04dfc452

accepted V5 authority/readback basis for stopped implementation:
  commit 67026ad7160a584cbf9cdb4bf0db7b8dbfa60136
  tree   2021dfcbf000d934a462d40efef6bbe7b54e0862
```

The V5 approval authorized the attempt that stopped. It does not authorize a
different build plane. The immutable V6 card at commit
`1e14eb8c318263b7307fe8e07aae13628402e0c1`, tree
`63f9ce1edfe663848e2a4832c7facad881b83aac`, blob
`e153943a302e3f9b38c74c9048f70a1d65c7b238`, SHA-256
`c6835b31ac8b2cf2f0fb1dee7634ef16f133387c0bffba8bf6671447175cd737`
received `DESIGN_REPAIR_REQUIRED` in GitHub review `5084539680`, materialized
at `docs/slices/WF0/ADVERSARIAL_DESIGN_REVIEW_V6.md`. V7 requires a fresh
independent adversarial review, separate exact operator approval, authority
merge, and exact `main` readback before any implementation source, workflow,
Windows build, or Deck execution.

### Binding inherited implementation laws

1. Every ordinary return from all 15 closed call operations is followed
   immediately by its paired synchronously flushed `call_completed` record
   before any later lifecycle event or call attempt.
2. Required `GetPluginFactory` is resolved and checked before optional
   `InitDll`. A missing export never resolves or invokes `InitDll`.
3. With a present export, optional entry handling completes, its lifecycle
   result is published, then `factory_export_found` is published. Only an
   entry-absent or entry-succeeded branch invokes the retained factory export.
4. Cleanup failure remains secondary to the first primary stage failure.
5. No class-instantiation operation exists in the scanner command, event enum,
   or retained schema.

## 2. Primary claim and claim ceiling

### Primary claim

> On the exact accepted Steam Deck fixture, a repository-owned supervised
> Windows x86_64 factory probe, built against the pinned official VST3 SDK and
> executed through the accepted Runtime 4 / Proton 11 lane in a disposable
> WF0-owned scan environment, loads the exact pinned AGain VST3 bundle, obtains
> its plug-in factory, retains deterministic factory metadata and the complete
> expected three-class census, unloads cleanly, and leaves the accepted WR0
> environment and every protected fixture unchanged.

The product claim is unchanged. V7 repairs artifact and execution-source
custody only; it does not reopen the supported Windows build or Deck execution
design.

### End-to-end route

```text
Mac creates one clean exact V7 implementation-source commit
  -> Mac pushes that exact commit to the private GitHub repository
  -> fixed private GitHub Actions workflow on explicit `windows-2022`
  -> exact SDK acquisition and two clean MSVC 2022 x64 builds
  -> independently verified scanner/fault/AGain PE artifacts
  -> canonical extracted-file manifest + bounded build receipt
  -> one immutable Actions artifact with exact ID and digest
  -> Mac downloads the raw artifact wrapper by exact artifact ID
  -> Mac verifies its GitHub digest and exact three-file inner envelope
  -> Mac creates and verifies the exact implementation-source Git bundle
  -> Mac transfers source bundle, receipt, and Windows envelope over SSH
  -> Deck verifies/imports the exact source commit under a local handoff ref
  -> Deck creates one clean detached execution worktree
  -> content-addressed read-only Deck artifact cache
  -> disposable WF0 environment
  -> accepted Runtime 4 / Proton 11 scanner route
  -> held-gate, negative-family, and positive AGain proofs
  -> bounded sanitized evidence handoff over SSH
  -> Mac verification and evidence-only Git commit
```

No Deck GitHub credential, API, fetch, artifact-download, push, or PR
dependency exists. Artifact custody proves exact byte and identity joins only;
it makes no cryptographic provenance, trusted-builder, SLSA, release-signing,
or code-signing claim.

### Claim ceiling

WF0 ends after factory metadata and the ordered class census. It does not
establish class instantiation, component/controller lifecycle, connections,
host context, buses, parameters, events, state, processing, audio, timing,
automation, presets, GUI, native proxy publication, IPC, Bitwig behavior,
Serum behavior, authorization, packaging, release signing, distribution, or
general VST3/Windows/Linux compatibility. A Windows-hosted build is not a
Windows runtime compatibility proof and cannot substitute for the exact Deck
execution proof.

## 3. Exact fixture and accepted prerequisites

```yaml
deck:
  hardware: Steam Deck Galileo
  os: SteamOS 3.8.16
  architecture: x86_64
  session: KDE Wayland
  read_only_mode: enabled
runner_runtime:
  runtime: SteamLinuxRuntime_4
  proton: Proton 11.0
  digest: 2d64df1d36786ca2d0e955c553005423dc2b5bdd714bd0a17872622e33912547
accepted_wr0:
  posture: protected_read_only
  environment_identity: d3ed38d7ed53e9a5973cbf22fc504f2479dbdd15616fe1bfc1d5e1cd0bf5f4c4
  contract_source_sha256: c6543004fbcd70252393f0e0cab4f9ad7a31e85f433ce3e903e690244cc79541
bitwig_protected_state:
  version: 6.1
  ref: app/com.bitwig.BitwigStudio/x86_64/stable
  application_commit: 8a048e733e74dda8b897339436153a2d5df952f29d362dfcaca9d8e0d6f6c231
  runtime_commit: bd44a6230581917d04f89812a4c21090c304d390edb73995af1c2f9fd8abf4e8
  scope: system
  origin: flathub
  user_shadow: absent
  user_override_sha256: 1b4a6a6ed688f69dd3c36dcac8db008c5a41ed52170ea3e23dee984b0aae6a1e
  system_override_sha256: e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
  permission_output_sha256: c7f5a34dce104cc3d347dcaf89e135cc5ad73b891101d7587f78423858ad5c73
vst3_sdk:
  root_commit: 3cdf9ca5d1f5b1b21e0a86832aa4abe55607bd96
  root_tree: 38343890fd1a0cedd48b7ec80ef17da15231b6c8
  base: fcf9da0bd27a16f7f03773a3a39822f28f5c8477
  cmake: 054c9143cbb8d47fc4694e473f2ee3b4d951a8f5
  doc: 8bfca19d3b76a61d093951ba9297047f544caea1
  pluginterfaces: 4f547e8e102b47de4a8b8aaf343c73b700786372
  public_sdk: 586dc5e6c8012c3e4b01c79389375cbe96bdb1da
  tutorials: 33b73dfbb87f3fde3bce8c0a10cae934dc66ad34
  vstgui4: 5db272256172557818b6158cf0bb2c4410bddb25
positive_fixture: exact pinned official AGain Windows x86_64 VST3 bundle
repository:
  full_name: kasselvania/Linux-VST-bridge
  visibility: private
  owner_type: User
```

The installed MinGW extension may remain at its exact user-scope commit but is
not an input to V7. Its classification is
`installed_exact_but_not_selected_for_wf0_again`.

The Mac control plane and fixed workflow may authenticate to this private
repository. The Deck may not. GitHub artifact attestations, GitHub Enterprise
Cloud, a visibility change, organization migration, Sigstore or another
signing service, release signing, and code signing are not prerequisites and
are outside this design.

## 4. Reconciled design finding

Exact upstream source proves that the pinned SDK integration forces
`VSTGUI_STANDALONE=ON`, while the pinned standalone target selects Windows
sources only under `if(MSVC)`. MinGW matches no source-selection branch, and
CMake stops with `No SOURCES given to target: vstgui_standalone`. AGain cannot
be kept exact by disabling VSTGUI because its CMake file returns in that state
and links `vstgui_support` when enabled. Patching/copying the SDK, replacing
VSTGUI, or rewriting AGain would change the approved positive-fixture owner.
The exact diagnosis and source blobs are retained in the reconciliation.

The correction is to use the upstream-supported MSVC branch, not to patch
around the source gap. The stopped MinGW work remains preserved as salvage
material, never accepted implementation evidence.

## 5. Plane and owner model

### Plane boundaries

| Plane | Owns | Does not own |
|---|---|---|
| `MacControlPlane` | canonical private repository, branches/commits/PRs, GitHub authentication, exact run/artifact-ID download, Mac artifact-custody receipt, exact source Git bundle, SSH transfer, evidence retrieval and Git publication | Windows compiler semantics, Proton execution, live Deck truth, signing/provenance claims |
| `WindowsBuildPlane` | exact source checkout, SDK materialization, observed tool identities, CMake/MSVC builds, two-build comparison, PE checks, manifest/archive/receipt, one workflow artifact upload | Deck credentials, Proton, accepted runtime behavior, attestation, release signing or permanent build service |
| `SteamDeckExecutionPlane` | inbound artifact and source-bundle verification, local source-ref/worktree admission, content-addressed artifact cache, disposable environment, Runtime 4 / Proton 11 execution, negative/positive proofs, cleanup, evidence handoff | GitHub login/API, Git fetch/push/PR, compilation, incoming-binary or source alteration |

### Exact owner map

Exactly 14 owners exist. A script may implement more than one only while their
state, identities, and commit points remain distinct.

| # | Plane | Owner | Authoritative output | Permitted mutation | Must not own |
|---:|---|---|---|---|---|
| 1 | Mac | `ImplementationSource` | clean commit/tree and canonical 26-record source manifest | future V7 implementation branch only | artifacts, workflow-run truth, Deck state |
| 2 | Mac | `MacArtifactCustodian` | exact run/artifact custody receipt plus source-bundle and SSH custody receipts | new private Mac artifact/source staging and handoff roots | compilation, Deck cache/worktree adoption, signing claims |
| 3 | Mac | `EvidencePublisher` | exact verified 14-file evidence-only commit | canonical implementation worktree evidence paths | raw Deck state, primary fact generation |
| 4 | Windows | `WindowsBuildTransaction` | observed runner/tool/SDK receipts and two clean build results | ephemeral runner acquisition/build roots | source history, Proton, GitHub secrets |
| 5 | Windows | `WindowsArtifactBundle` | canonical extracted manifest, payload archive, outer hash, bounded receipt | ephemeral publication root and workflow artifact | runtime claims, proprietary payloads |
| 6 | Windows | `WindowsFactoryProbe` | bounded scanner events and raw factory census | build outputs; later process memory only on Deck | process cleanup, class instantiation |
| 7 | Windows | `FaultFixtureFamily` | exact approved negative DLL roster | build outputs only | positive compatibility claim |
| 8 | Deck | `DeckArtifactCache` | one manifest-digest-addressed read-only artifact set plus exact local source handoff ref and clean detached execution worktree | exact artifact/source handoff stages, local handoff ref, cache root, canonical execution-worktree root | compilation, repair, `latest` alias, GitHub access, dirty source adoption |
| 9 | Deck | `WF0ScanEnvironment` | marker-bound disposable stage and retirement proof | exact `.wf0-factory-census.stage-<run-id>` | WR0, `.wine`, Steam compatdata |
| 10 | Deck | `WF0Supervisor` | bounded launch/gate/topology/cleanup receipt | exact process group and handshake files | module/factory semantics, global kill |
| 11 | Deck | `ScannerProcessSession` | nonce, command/artifact binding, lifecycle/call timeline | session transients | artifact import, evidence publication |
| 12 | Deck | `FactoryCensusNormalizer` | unchanged canonical census v1 | transient then staged normalized file | repairing or inventing scanner facts |
| 13 | Deck | `ProtectedFixtureSnapshot` | labeled pre/post equality | none | Bitwig launch, protected repair, secret traversal |
| 14 | Deck | `CensusEvidencePacket` | bounded 14-file packet and evidence-handoff receipt | private staging/handoff root only | Git commit/push, compiled artifact retention |

Path names, workflow labels, artifact names, branch names, PIDs, and friendly
plug-in names are never identity on their own. Exact immutable objects,
canonical manifests, and fresh readback authorize every transition.

## 6. State machines

### Source and workflow lifecycle

```text
v7_authority_proposed
  -> v7_reviewed_clear
  -> v7_separately_approved
  -> v7_authority_merged_and_read_back
  -> implementation_branch_created_from_exact_merge
  -> implementation_source_committed_once
  -> implementation_source_manifest_frozen
  -> exact_source_commit_pushed_by_mac
  -> workflow_run_created_for_exact_push
```

No arrow through `v7_authority_merged_and_read_back` exists during this design
turn. Future implementation uses a new branch
`codex/wf0-windows-vst3-factory-census-v7` from the exact merged V7 authority;
it does not move the preserved archive ref, adopt the provisional worktree, or
silently fast-forward the old remote implementation branch.

The only workflow trigger is a `push` to that exact future branch, restricted
to the exact 26 source/configuration paths. The pushed `github.sha` is the only
checkout candidate. `pull_request_target`, tag triggers, schedule triggers,
unbounded branch patterns, and workflow execution of a different requested SHA
are absent. The Mac push is the trigger; the Deck never triggers a run.

### Windows build lifecycle

```text
run_identity_observed
  -> exact_source_checked_out_detached
  -> source_manifest_verified
  -> sdk_acquisition_open
  -> sdk_root_and_submodules_materialized
  -> sdk_identity_verified
  -> dependency_network_closed
  -> build_root_a_empty
  -> build_a_configured -> build_a_complete -> build_a_verified
  -> build_root_b_empty
  -> build_b_configured -> build_b_complete -> build_b_verified
  -> two_builds_compared
  -> canonical_artifact_manifest_published
  -> payload_archive_published
  -> build_receipt_published
  -> one_workflow_artifact_uploaded
  -> upload_result_id_name_url_digest_recorded
```

Only SDK acquisition may contact dependency remotes. Configure and build run
with CMake package registries disabled, FetchContent fully disconnected, no
package-manager step, no SDK updater, and fresh outbound-deny rules for the
exact CMake, MSBuild, compiler, linker, resource-compiler, Python, and Git
programs used in the build. Rule creation/readback precedes configure; removal
occurs only after both builds and artifact verification so GitHub publication
can resume. Failure to establish or read back that bounded offline posture is
`WF0_WINDOWS_BUILD_BLOCKED`; there is no ambient-network fallback.

The two roots have no shared generated state. Both use the same detached
source commit, SDK checkout, Visual Studio generator, platform/toolset, Windows
SDK, Release configuration, and target roster. Transfer artifacts must be
byte-identical across A and B. Both raw hashes are retained. With no product
signing and PDBs excluded, an unexplained difference is
`WF0_BUILD_COMPARISON_BLOCKED`; V7 does not normalize arbitrary PE differences
into equality.

### Mac artifact-custody lifecycle

```text
run_selected_by_id
  -> run_metadata_verified
  -> new_empty_download_root
  -> exact_artifact_api_metadata_verified
  -> raw_wrapper_downloaded_by_artifact_id
  -> raw_wrapper_sha256_equals_actions_digest
  -> wrapper_safely_extracted
  -> exact_three_file_inner_envelope_verified
  -> build_receipt_verified
  -> payload_outer_hash_verified
  -> canonical_manifest_verified_without_extraction_escape
  -> canonical_mac_artifact_custody_receipt_published
  -> exact_handoff_set_ready
```

The authenticated Mac uses the exact run ID, repository, workflow path/blob,
branch, head SHA, attempt, event, conclusion, artifact ID, name, URL, size,
digest, expiry state, and artifact workflow-run/head metadata. It never asks
for the latest successful run or latest/name-only artifact. A rerun is a
distinct attempt and build identity even when the source SHA is unchanged.

### Mac source-bundle and Deck source-admission lifecycle

```text
implementation_source_manifest_reverified_on_mac
  -> new_empty_marker_bound_mac_source_stage
  -> temporary_bare_handoff_repository_created
  -> authority_and_implementation_history_materialized
  -> fixed_handoff_ref_created_in_stage
  -> self_contained_source_bundle_created
  -> source_bundle_verified_complete
  -> source_handoff_receipt_published
  -> source_and_receipt_transferred_by_ssh
  -> remote_source_transfer_hashes_equal
  -> new_empty_marker_bound_deck_source_stage
  -> deck_source_bundle_and_receipt_reverified
  -> deck_git_bundle_verify_complete
  -> exact_local_handoff_ref_imported
  -> implementation_commit_tree_parent_delta_verified
  -> exact_26_record_manifest_reproduced
  -> clean_detached_execution_worktree_created
  -> source_execution_worktree_ready
```

The bundle has the fixed logical name
`wf0-v7-execution-source-<implementation-commit>.bundle` and advertises exactly
`refs/handoff/wf0-v7-source/<implementation-commit>`. The Mac constructs it in
a new private staging repository so the canonical repository receives no
temporary handoff ref. It is self-contained, has no prerequisite bundle, and
contains the implementation commit plus the exact merged V7 authority history
reachable through its required single parent.

The source bundle is at most 128 MiB, the canonical source-handoff receipt and
its hash sidecar are each at most 64 KiB, and the persistent handoff directory
contains exactly those three regular files. Any overflow or extra object is
`WF0_SOURCE_HANDOFF_BLOCKED`.

The canonical persistent Deck handoff destination is:

```text
<HOME>/.local/share/linux-vst-bridge/handoffs/wf0/source/by-commit/
  <implementation-commit>/
    wf0-v7-execution-source-<implementation-commit>.bundle
    WF0_SOURCE_HANDOFF_RECEIPT.json
    WF0_SOURCE_HANDOFF_RECEIPT.sha256
```

The local imported ref is exactly
`refs/handoff/wf0-v7-source/<implementation-commit>`. The clean detached
execution worktree is exactly:

```text
<HOME>/.local/share/linux-vst-bridge/worktrees/wf0/
  <implementation-commit>/
```

An existing unknown stage, ref, destination, or worktree is preserved and
blocks. The verified bundle, receipt, local ref, and worktree remain through
WF0 acceptance; later retirement requires a separate exact preview, no live
reference, and explicit authority. There is no partial-recovery requirement.

### Deck artifact-import lifecycle

```text
handoff_absent
  -> handoff_staging_created
  -> transfer_envelope_complete
  -> transfer_hashes_verified
  -> archive_census_safe
  -> extraction_stage_created
  -> every_extracted_file_verified
  -> canonical_manifest_digest_verified
  -> artifact_cache_root_published
  -> artifact_cache_read_only
  -> artifact_cache_reverified_for_execution
```

The final root is exactly:

```text
<HOME>/.local/share/linux-vst-bridge/artifacts/wf0/
  <artifact-set-sha256>/
```

`artifact-set-sha256` is the SHA-256 of the canonical extracted-file manifest,
not the mutable GitHub artifact name and not merely the ZIP hash. Publication
is a same-filesystem atomic rename from a nonce-bound stage after full readback.
An existing exact root is reverified and may be reused; any differing or unsafe
existing root blocks and is preserved. There is no `latest` symlink or ambient
selection.

### Deck execution and evidence-return lifecycle

```text
source_execution_worktree_reverified
  + artifact_cache_reverified_for_execution
  -> source_build_artifact_identity_join_equal
  -> protected_preflight_equal
  -> environment_absent
  -> environment_stage_created
  -> environment_marker_durable
  -> verified_payload_copied
  -> first_supervised_launch_initializing
  -> scanner_readiness_announced
  -> scanner_identified
  -> scanner_gated
  -> held_gate_or_negative_or_positive_terminal
  -> process_draining
  -> process_clean
  -> evidence_inputs_captured
  -> environment_retired
  -> protected_postflight_equal
  -> evidence_packet_staged
  -> evidence_handoff_manifest_published
  -> evidence_handoff_transferred_to_mac
  -> mac_packet_verified
  -> evidence_only_commit_published_by_mac
```

Each held-gate, negative, and positive exercise gets a new 32-hex run ID and
fresh marker-bound environment. The first exact scanner launch owns any prefix
initialization; no `wineboot`, validator, Bitwig, or bootstrap workload runs.
The artifact cache is an input and remains read-only. Environment retirement
requires zero owned descendants, durable parent update, and exact absence.
Every Deck command runs from the exact clean detached execution worktree.
Before every held-gate, fault, positive, normalization, or evidence run, the
Deck reproduces the 26-record path/mode/blob source manifest and requires an
empty worktree status. The source commit and manifest digest must equal the
Windows build receipt, artifact manifest, exercise receipts, and evidence
handoff.

### Inherited scanner lifecycle

V7 preserves the exact 23 lifecycle names, two typed call-event kinds, and
closed 15-operation enum from V5:

```text
scanner_started
readiness_announced
supervisor_gate_accepted
module_open_started
module_opened
module_entry_absent | module_entry_succeeded | module_entry_failed
factory_export_missing | factory_export_found
factory_get_started
factory_obtained | factory_get_failed
factory_info_obtained
factory_interface_versions_recorded
class_count_obtained
class_enumeration_in_progress
class_enumeration_complete
module_exit_absent | module_exit_succeeded | module_exit_failed
module_unloaded
scanner_completed
```

```text
load_library
init_dll
get_plugin_factory
get_factory_info
query_factory_2
query_factory_3
count_classes
get_class_info_unicode
get_class_info_2
get_class_info_1
release_factory_3
release_factory_2
release_factory_base
exit_dll
free_library
```

The V5 sequence, bounds, required-export-before-entry rule, paired-completion
law, tier fallback, release order, primary/secondary failure precedence, and
exit-code mapping are unchanged. Windows compilation may not redesign them.

### Retry and restart law

- A new hosted-runner image, run ID, or run attempt creates new build receipts
  and new artifacts; no fact is inherited from an older run.
- An identical already-imported Deck artifact root is rehashed before reuse.
  A partial or differing root is never repaired or adopted.
- A failed scan environment is retained when cleanup ownership is uncertain;
  it is never a future input.
- Source or workflow blob change invalidates all prior build and execution
  evidence. Evidence-only changes do not rewrite the frozen source manifest.
- A source-bundle, source-handoff receipt, imported ref, commit/tree/parent,
  execution worktree, or source-manifest mismatch is
  `WF0_SOURCE_HANDOFF_BLOCKED`; unknown or dirty state is preserved and never
  repaired into current source.
- A failed evidence transfer may retry into a fresh Mac staging root only after
  the Deck packet remains immutable and its exact hashes are re-read.

## 7. Fallible-operation and mutation ledger

Exactly 25 material operations exist. The source bundle, source transfer, and
Deck source/worktree admission are distinct commit points; ordinary temporary
files inside an already-owned stage remain covered by their enclosing
operation.

| # | Operation | Plane/owner | Preconditions | Commit point | Primary failure |
|---:|---|---|---|---|---|
| 1 | freeze implementation source | Mac / `ImplementationSource` | merged V7 authority exact; clean new branch | one clean 26-path commit and manifest readback | `WF0_WORKFLOW_SOURCE_BLOCKED` |
| 2 | push exact source | Mac / `ImplementationSource` | commit/tree/branch reverified | remote ref equals exact commit | `WF0_WORKFLOW_SOURCE_BLOCKED` |
| 3 | create and bind workflow run | Mac/GitHub | fixed push trigger | run ID/attempt/head/path receipt | `WF0_WORKFLOW_SOURCE_BLOCKED` |
| 4 | check out exact source | Windows / `WindowsBuildTransaction` | `github.sha` exact | detached HEAD/tree/manifest equal | `WF0_WORKFLOW_SOURCE_BLOCKED` |
| 5 | acquire exact SDK | Windows / `WindowsBuildTransaction` | acquisition network open; allowlisted remotes | root plus seven submodules exact/clean | `WF0_DEPENDENCY_ACQUISITION_BLOCKED` |
| 6 | configure/build A | Windows / `WindowsBuildTransaction` | offline posture; empty root; tool receipts | requested targets complete | `WF0_WINDOWS_BUILD_BLOCKED` |
| 7 | verify A | Windows / `WindowsArtifactBundle` | build A complete | exact PE/roster/import/export receipt | `WF0_ARTIFACT_BUNDLE_BLOCKED` |
| 8 | configure/build B | Windows / `WindowsBuildTransaction` | same identities; distinct empty root | requested targets complete | `WF0_WINDOWS_BUILD_BLOCKED` |
| 9 | verify B and compare | Windows / `WindowsArtifactBundle` | both roots complete | byte-equal transfer roster | `WF0_BUILD_COMPARISON_BLOCKED` |
| 10 | publish manifest/archive/receipt | Windows / `WindowsArtifactBundle` | comparison accepted | canonical files and hashes read back | `WF0_ARTIFACT_BUNDLE_BLOCKED` |
| 11 | upload exact three-file envelope once | Windows / `WindowsArtifactBundle` | exact payload and receipt; same build/upload job | one fixed receipt-bound Actions artifact; upload ID/name/URL/digest captured | `WF0_BUILD_CUSTODY_BLOCKED` |
| 12 | select run/artifact and download raw wrapper by exact artifact ID | Mac / `MacArtifactCustodian` | authenticated private-repository API readback | new private stage contains exact raw wrapper bytes | `WF0_MAC_ARTIFACT_BLOCKED` |
| 13 | verify wrapper/inner envelope and publish Mac custody receipt | Mac / `MacArtifactCustodian` | API digest and downloaded bytes bounded | raw digest, exact three-file roster, receipt/payload/manifest closure | `WF0_BUILD_CUSTODY_BLOCKED` |
| 14 | create and verify exact source bundle | Mac / `MacArtifactCustodian` | source commit/tree/parent/manifest and merged V7 authority exact | self-contained bundle, fixed advertised ref, source-handoff receipt read back | `WF0_SOURCE_HANDOFF_BLOCKED` |
| 15 | transfer source bundle and receipt | Mac / `MacArtifactCustodian` | no agent/credential forwarding; exact local hashes | new Deck source stage hashes equal Mac | `WF0_SOURCE_HANDOFF_BLOCKED` |
| 16 | transfer Windows artifact envelope and Mac custody receipt | Mac / `MacArtifactCustodian` | exact local custody closure; source transfer complete | new Deck artifact stage hashes equal Mac | `WF0_ARTIFACT_TRANSFER_BLOCKED` |
| 17 | verify/import source and create execution worktree | Deck / `DeckArtifactCache` | new marker-bound source stage; existing repo exact | fixed local ref plus clean detached exact-commit worktree and manifest | `WF0_SOURCE_HANDOFF_BLOCKED` |
| 18 | import artifact set | Deck / `DeckArtifactCache` | safe new artifact staging root; source worktree exact | atomic content-addressed read-only root | `WF0_ARTIFACT_IMPORT_BLOCKED` |
| 19 | create execution environment | Deck / `WF0ScanEnvironment` | source/artifact join reverified; protected preflight | marker and payload readback | `WF0_SCAN_ENVIRONMENT_BLOCKED` |
| 20 | spawn and identify scanner | Deck / `WF0Supervisor` | exact worktree/manifest/command/session; gate absent | readiness, ancestry, role census | `WF0_SCANNER_LAUNCH_BLOCKED` |
| 21 | publish gate and run census | Deck / `ScannerProcessSession` | fresh source/artifact/protected equality | terminal bounded scanner receipt | exact module/factory blocker |
| 22 | drain and retire environment | Deck / supervisor/environment | terminal stage | zero descendants and environment-root absence | `WF0_PROCESS_CLEANUP_BLOCKED` |
| 23 | normalize and stage evidence | Deck / normalizer/evidence | exact source manifest repeated; complete receipts; protected postflight equal | exact 14-file packet/hash closure | `WF0_EVIDENCE_BLOCKED` |
| 24 | return and verify evidence | Deck/Mac / evidence owners | immutable packet and manifest | Mac bytes/roster/hashes and source/artifact join equal | `WF0_EVIDENCE_BLOCKED` |
| 25 | publish evidence-only commit | Mac / `EvidencePublisher` | original 26-record source manifest still equal | exact 14 evidence paths committed | `WF0_EVIDENCE_BLOCKED` |

No failure authorizes mutation outside its owner. Unknown roots, wrong refs,
unexpected artifacts, or protected drift are preserved and block. Cleanup may
delete only nonce/marker-bound stages created by the same transaction.

## 8. Process, thread, topology, and bounds

### Windows build topology

The hosted build contains only GitHub runner orchestration, exact checkout and
SDK acquisition tools, the VS 2022 x64 developer environment, CMake/MSBuild,
MSVC compiler/linker/resource tools, repository-owned verification scripts,
and one archive upload. It does not execute any produced PE,
start Wine/Proton, instantiate VST3 classes, or launch a host application.

The workflow records `ImageOS`, `ImageVersion`, OS build, runner architecture,
`vswhere` JSON, selected VS installation, `VSCMD_VER`, `cl /Bv`, `link /?`
identity line, `WindowsSDKVersion`, installed SDK directory, `cmake --version`,
and generator/platform/toolset. The mutable label is never substituted for
these observations.

### Deck runtime topology

The unchanged expected execution tree is:

```text
repository Python supervisor
  -> exact SteamLinuxRuntime_4 entry point
     -> exact Proton 11 `runinprefix`
        -> Wine-hosted `wf0-factory-probe.exe`
```

Steam game-launch ancestry, UMU, yabridge, Bitwig, validator, and an alternate
Wine/Proton lane are prohibited. Observation follows full descendant ancestry
from the exact Runtime root PID/start identity; group/session membership does
not replace ancestry. Signals target only revalidated owned identities. A
same-family sentinel outside the tree must survive.

### Unchanged Deck execution bounds

| Boundary | Bound |
|---|---:|
| process identities per session | 256 |
| first spawn to readiness | 180 seconds |
| held gate | 15 seconds |
| each individual synchronous call after `call_started` | 15 seconds |
| class enumeration | 30 seconds |
| total post-gate scanner time | 120 seconds |
| ordinary drain | 20 seconds |
| TERM/KILL cleanup | 10 seconds |
| stdout | 1,048,576 bytes |
| stderr | 65,536 bytes |
| scanner events | 2,048 |
| ordinary event | 4,096 bytes |
| final raw census record | 786,432 bytes |

Artifact import additionally caps the ZIP at 256 MiB, the extracted payload at
512 MiB, a single entry at 128 MiB, the entry count at 256, and a UTF-8
safe-relative path at 240 bytes. These are refusal bounds, not estimates of
expected output.

## 9. Identity and authorization ledger

| Operation | Required identity | Fresh readback | Refusal |
|---|---|---|---|
| accept authority | exact V7 design/review/approval/merge/main readback plus immutable V6/review lineage | before branch creation | missing or mismatched authority |
| accept repository posture | `kasselvania/Linux-VST-bridge`, private, owner type `User` | before workflow and Mac API use | wrong repository/visibility/owner; never repair posture |
| accept source | clean commit/tree + exact 26-record manifest | before push, workflow checkout, each build, each Deck run, final evidence commit | dirty/missing/extra/mode/blob change |
| accept workflow | fixed path, blob, source commit, exact branch trigger | run creation and receipt | wrong event/ref/path/blob/SHA |
| accept hosted runner | label plus observed image/OS/architecture | workflow start | wrong label or non-x64/self-hosted runner |
| accept toolchain | VS 2022 `v143`, x64 developer environment, complete `cl /Bv`, Windows SDK `10.0.19041.0`, CMake/generator | before each configure | ambient/different VS, compiler, SDK, generator |
| accept SDK | root commit/tree + seven gitlink/submodule commits + clean status | after acquisition and before both configures | wrong/dirty/missing/unexpected submodule or remote |
| accept build | source/workflow/toolchain/SDK identities + distinct empty root | each configure/build | reused root, network-open posture, target drift |
| accept artifact | canonical manifest digest + exact receipt + archive hash + PE closure + A/B equality | Windows publication, Mac download, Deck import, pre-execution | missing/extra/different/unsafe file or stale run |
| accept Actions artifact | exact run/attempt/head/workflow plus artifact ID/name/URL/size/digest/expiry and artifact workflow metadata | upload result and Mac API readback | latest/name-only/stale/expired/mismatched artifact |
| accept raw download | exact artifact-ID API route; raw wrapper SHA-256 equals Actions digest; exact three-file inner envelope | Mac before custody receipt | redirect/selection/digest/roster mismatch |
| accept source bundle | exact self-contained bundle/ref/hash/size/verify plus source-handoff receipt | Mac creation, SSH, Deck import | prerequisite/multiple ref, partial, wrong history or digest |
| accept source worktree | fixed local ref; exact commit/tree/parent/delta; clean detached worktree; 26-record manifest | admission and before every Deck command | GitHub access, provisional/dirty/wrong source |
| accept transfer | local/remote SHA-256 of exact source and artifact handoff files | before and after SSH | mismatch, partial, credential or agent forwarding |
| accept cache root | manifest digest equals directory name; exact bytes/modes; no links | import and before each environment copy | unknown root, drift, writable or unsafe entry |
| spawn scanner | exact worktree/source manifest + artifact/build/custody receipt, environment marker, runner digest, nonce, command | immediately before spawn and readiness | stale/dirty source, alternate vector, wrong artifact |
| publish gate | ready bytes + scanner/module hashes + live PID/start/ancestry + protected equality | one fresh census before atomic gate | changed/disappeared identity or preexisting gate |
| publish census | schema/timeline/source/build/artifact equality; `create_instance_called=false` | normalization and packet readback | malformed/oversized/incomplete/identity mismatch |
| publish evidence | exact 14-file roster, hashes, evidence-handoff receipt, unchanged source manifest | Deck handoff, Mac import, Git commit | private data, binary, extra/missing path, source drift |

## 10. Data, durability, recovery, and retention

| Data | Owner/location | Durability and authority | Recovery/retention law |
|---|---|---|---|
| preserved provisional source | local archive refs plus verified Deck/Mac Git bundle | exact commit/ref/bundle SHA | never pushed or moved without separate instruction |
| implementation source manifest | untracked build receipt, then `BUILD_MANIFEST.json` | canonical JSON and commit/tree/path/mode/blob closure | any source/workflow change rebuilds and reruns |
| SDK checkout | ephemeral Windows acquisition root | exact clean root/tree/submodules; no archive in payload | reacquire exactly; never vendor into repo |
| Windows build roots A/B | ephemeral runner | distinct disposable roots | discarded with runner after receipts; never evidence |
| workflow artifact | private GitHub Actions run | one upload bound by exact run/attempt/head plus artifact ID/name/URL/size/digest | expiry blocks new download but does not invalidate already verified local custody |
| Mac artifact custody | new private staging root | raw wrapper digest, exact inner envelope, canonical custody receipt | retain until Deck import and evidence completion; no Git tracking or credential retention |
| source handoff | private Mac stage and Deck `handoffs/wf0/source/by-commit/<commit>` | self-contained bundle, fixed ref, receipt, byte-equal SSH transfer | private/untracked; retain through WF0 acceptance |
| Deck execution source | fixed local handoff ref plus `worktrees/wf0/<commit>` | detached clean exact commit, parent/tree/delta and 26-record manifest | no GitHub; preserve unknown/dirty state; retain through WF0 acceptance |
| Deck artifact cache | content-addressed root under declared user data | canonical manifest digest, atomic publish, files `0444`, directories `0555`, rehash before use | no automatic eviction in WF0; explicit preview and no live references required for later removal |
| scan environment | exact marker-bound stage | disposable; first launch owns initialization | remove only after zero descendants; never migrate/adopt |
| raw scanner/process data | bounded transient memory/private staging | raw hashes only after safe normalization | discard; no raw PID/path/cmdline retention |
| evidence handoff | Deck private handoff stage and Mac private staging | exact roster/manifest/packet hashes | retain through evidence commit; never contains compiled binaries |
| canonical evidence | fixed 14 tracked paths | atomic writes, hash manifest, Git blobs | evidence-only commit; schema change requires review |

Atomic local publication uses same-filesystem temporary files, file flush,
rename, parent-directory flush, and exact readback. A directory is authority
only through its exact manifest/marker. Existing unknown state is never deleted
or converted into success.

## 11. Versioned schemas and canonical identity

### Implementation-source manifest

V7 retains `linux-vst-bridge-wf0-implementation-source/v1`. Canonical JSON uses
UTF-8, sorted object keys, no insignificant whitespace, and one trailing LF.
It contains the exact clean implementation commit plus 26 raw-UTF-8-sorted
records of `path`, six-digit `git_mode`, and 40-lowercase-hex `git_blob`.
Missing, extra, duplicate, reordered, non-blob, or mode/blob-different records
block. The workflow file is one of those 26 records.

### Workflow-source binding

The build receipt repeats, rather than infers, this binding:

```json
{
  "workflow_path": ".github/workflows/wf0-windows-msvc-build.yml",
  "workflow_git_blob": "<40-lowercase-hex>",
  "workflow_commit": "<exact-implementation-commit>",
  "workflow_ref": "refs/heads/codex/wf0-windows-vst3-factory-census-v7",
  "workflow_event": "push",
  "workflow_run_id": "<decimal-string>",
  "workflow_run_attempt": "<decimal-string>"
}
```

The workflow validates its own blob from the detached checkout. The Mac checks
the same fields against GitHub run metadata and its local Git object. A source
commit cannot name a workflow from another commit.

### Windows source-to-artifact receipt

The canonical schema is:

```text
linux-vst-bridge-wf0-windows-build/v1
```

It binds at least this closed shape; implementations may add only fields
explicitly required by this card, never free-form environment dumps:

```json
{
  "schema": "linux-vst-bridge-wf0-windows-build/v1",
  "repository": {
    "full_name": "kasselvania/Linux-VST-bridge",
    "visibility": "private",
    "owner_type": "User"
  },
  "source": {
    "commit": "<40-lowercase-hex>",
    "tree": "<40-lowercase-hex>",
    "implementation_source_manifest": {
      "schema": "linux-vst-bridge-wf0-implementation-source/v1",
      "record_count": 26,
      "sha256": "<64-lowercase-hex>"
    }
  },
  "workflow": {
    "path": ".github/workflows/wf0-windows-msvc-build.yml",
    "git_blob": "<40-lowercase-hex>",
    "commit": "<same-source-commit>",
    "run_id": "<decimal-string>",
    "run_attempt": "<decimal-string>",
    "event": "push",
    "ref": "refs/heads/codex/wf0-windows-vst3-factory-census-v7"
  },
  "runner": {
    "requested_label": "windows-2022",
    "architecture": "X64",
    "image_os": "<observed>",
    "image_version": "<observed>",
    "windows_product_name": "<observed>",
    "windows_version": "<observed>",
    "windows_build": "<observed>"
  },
  "toolchain": {
    "visual_studio_edition": "Enterprise",
    "visual_studio_version": "<observed-full-version>",
    "visual_studio_installation_sha256": "<bounded-inventory-digest>",
    "platform_toolset": "v143",
    "host_architecture": "x64",
    "target_architecture": "x64",
    "cl_bv_sha256": "<full-cl-Bv-output-sha256>",
    "cl_version": "<observed>",
    "link_version": "<observed>",
    "windows_sdk_version": "10.0.19041.0",
    "cmake_version": "<observed>",
    "generator": "Visual Studio 17 2022",
    "generator_platform": "x64",
    "generator_toolset": "v143"
  },
  "vst3_sdk": {
    "root_commit": "3cdf9ca5d1f5b1b21e0a86832aa4abe55607bd96",
    "root_tree": "38343890fd1a0cedd48b7ec80ef17da15231b6c8",
    "submodules": [{"path": "<closed-path>", "commit": "<locked-commit>"}],
    "source_patch_count": 0,
    "dirty_path_count": 0
  },
  "build": {
    "configuration": "Release",
    "cmake_options_sha256": "<canonical-option-list-sha256>",
    "targets": ["wf0-factory-probe", "wf0-loader-adapter-tests", "wf0-fault-fixtures", "again"],
    "msvc_runtime": "MultiThreaded",
    "dependency_network_during_configure_build": false,
    "build_a": {"artifact_manifest_sha256": "<64-lowercase-hex>"},
    "build_b": {"artifact_manifest_sha256": "<same-hex>"},
    "comparison": {"level": "byte_identical", "path_count": "<integer>"}
  },
  "artifacts": {
    "scanner_and_fault_roster": [{"path": "<safe-relative>", "size": "<integer>", "sha256": "<hex>"}],
    "again_bundle_roster": [{"path": "<safe-relative>", "size": "<integer>", "sha256": "<hex>"}],
    "pe_receipts": [{"path": "<safe-relative>", "machine": "AMD64", "imports": ["<closed-name>"], "exports": ["<closed-name>"]}],
    "copied_runtime_dependencies": [],
    "artifact_manifest_schema": "linux-vst-bridge-wf0-artifact-manifest/v1",
    "artifact_manifest_sha256": "<artifact-set-sha256>",
    "payload_archive_sha256": "<outer-payload-archive-sha256>"
  },
  "explicit_nonclaims": [
    "no_windows_runtime_proof", "no_class_instantiation", "no_proton",
    "no_bitwig", "no_serum", "no_release_build", "no_reproducible_runner_image_claim"
  ]
}
```

Numeric run IDs, attempts, sizes, and counts are canonical decimal strings or
bounded integers exactly as the schema declares; implementations do not mix
representations. Full `cl /Bv` output is hashed and retained in a bounded
sanitized tool receipt, not copied into every evidence field.

The Windows build receipt ends before upload and therefore contains no
post-upload artifact ID, URL, size, or GitHub artifact digest. Adding those
fields would create an impossible recursive dependency. The authenticated Mac
creates a distinct artifact-custody receipt that joins the byte-identical build
receipt to the later GitHub Actions upload/API facts; it never rewrites the
producer's receipt.

### Mac artifact-custody receipt

The canonical private-repository download receipt is:

```text
linux-vst-bridge-wf0-mac-artifact-custody/v1
```

It is credential-free canonical JSON with one LF and binds this closed shape:

```json
{
  "schema": "linux-vst-bridge-wf0-mac-artifact-custody/v1",
  "repository": {
    "full_name": "kasselvania/Linux-VST-bridge",
    "visibility": "private",
    "owner_type": "User"
  },
  "workflow": {
    "path": ".github/workflows/wf0-windows-msvc-build.yml",
    "git_blob": "<40-lowercase-hex>",
    "event": "push",
    "head_branch": "codex/wf0-windows-vst3-factory-census-v7",
    "head_sha": "<implementation-commit>",
    "run_id": "<decimal-string>",
    "run_attempt": "<decimal-string>",
    "conclusion": "success"
  },
  "artifact": {
    "id": "<decimal-string>",
    "name": "wf0-windows-build-<implementation-commit>-run-<run-id>-attempt-<attempt>",
    "url": "<exact-api-url>",
    "size_in_bytes": "<decimal-string>",
    "digest": "sha256:<64-lowercase-hex>",
    "expired": false,
    "workflow_run_id": "<same-run-id>",
    "workflow_head_branch": "codex/wf0-windows-vst3-factory-census-v7",
    "workflow_head_sha": "<same-implementation-commit>",
    "raw_download_route": "/repos/kasselvania/Linux-VST-bridge/actions/artifacts/<artifact-id>/zip",
    "raw_github_artifact_zip_sha256": "<same-64-lowercase-hex>"
  },
  "inner_envelope": {
    "record_count": 3,
    "records": [
      {"path": "wf0-payload.zip", "size": "<integer>", "sha256": "<hex>"},
      {"path": "WF0_WINDOWS_BUILD_RECEIPT.json", "size": "<integer>", "sha256": "<hex>"},
      {"path": "WF0_WINDOWS_BUILD_RECEIPT.sha256", "size": "<integer>", "sha256": "<hex>"}
    ],
    "build_receipt_sha256": "<hex>",
    "payload_archive_sha256": "<hex>",
    "artifact_manifest_sha256": "<hex>"
  },
  "authenticated_private_repository_download": true,
  "credentials_retained": false,
  "explicit_nonclaims": [
    "no_cryptographic_provenance", "no_trusted_builder", "no_slsa",
    "no_code_signing", "no_release_signing"
  ]
}
```

The run and artifact APIs are authoritative for selection metadata; the raw
downloaded wrapper bytes must hash to the Actions artifact digest. A digest
format or API posture that does not permit that exact equality is
`WF0_BUILD_CUSTODY_BLOCKED`, not a reason to omit the check or substitute a
different trust claim.

### Implementation-source handoff receipt

The canonical private source-handoff schema is:

```text
linux-vst-bridge-wf0-source-handoff/v1
```

It is canonical JSON with one LF and binds:

```json
{
  "schema": "linux-vst-bridge-wf0-source-handoff/v1",
  "repository": "kasselvania/Linux-VST-bridge",
  "v7_authority": {
    "merge_commit": "<exact-merged-v7-authority-commit>",
    "merge_tree": "<exact-merged-v7-authority-tree>"
  },
  "implementation_source": {
    "commit": "<40-lowercase-hex>",
    "tree": "<40-lowercase-hex>",
    "parent": "<same-v7-authority-merge-commit>",
    "branch": "codex/wf0-windows-vst3-factory-census-v7",
    "ref": "refs/heads/codex/wf0-windows-vst3-factory-census-v7",
    "manifest_schema": "linux-vst-bridge-wf0-implementation-source/v1",
    "manifest_record_count": 26,
    "manifest_sha256": "<64-lowercase-hex>"
  },
  "bundle": {
    "name": "wf0-v7-execution-source-<implementation-commit>.bundle",
    "advertised_ref": "refs/handoff/wf0-v7-source/<implementation-commit>",
    "sha256": "<64-lowercase-hex>",
    "size": "<bounded-integer>",
    "max_size_bytes": 134217728,
    "git_bundle_verify": "passed",
    "self_contained": true,
    "prerequisite_count": 0
  }
}
```

The receipt and bundle are private, untracked handoff material. The receipt
contains no credential, SSH key, agent socket, token, cookie, hostname, IP
address, or private absolute path.

### Canonical artifact manifest

The extracted payload authority is:

```text
linux-vst-bridge-wf0-artifact-manifest/v1
```

It is canonical JSON with one LF and contains the source-manifest digest,
build-receipt core digest, and a raw-UTF-8-sorted array of exact file records:

```json
{
  "path": "again.vst3/Contents/x86_64-win/again.vst3",
  "type": "file",
  "size": 123,
  "sha256": "<64-lowercase-hex>",
  "portable_mode": "0444",
  "role": "positive_fixture_module"
}
```

Allowed roles are closed: scanner executable, loader-adapter executable,
approved fault module, AGain module/resource, required runtime dependency,
build-identity core, and required license/notice. Directories are implicit and
must contain only declared files. No symlink, hard link, device, alternate data
stream, absolute path, drive/UNC path, empty component, `.`/`..`, backslash,
control character, case-fold collision, Unicode-normalization collision, or
duplicate record is permitted.

The record array covers every extracted payload file except
`ARTIFACT_MANIFEST.json` and `ARTIFACT_MANIFEST.sha256`; those two mandatory
metadata files are admitted separately and no other unrecorded entry is
allowed. `ARTIFACT_MANIFEST.json` therefore does not contain its own hash. Its
exact byte SHA-256, stored in `ARTIFACT_MANIFEST.sha256`, is the artifact-set
identity and final Deck directory name. `BUILD_IDENTITY_CORE.json` is a normal
hashed record and binds source, workflow run/attempt, observed toolchain, SDK,
and comparison facts except the not-yet-created outer ZIP hash. Thus a new run
or tool identity produces a different artifact-set identity even if PE bytes
happen to match. This avoids recursive identity while retaining exact custody
and source/build identity closure.

### Payload archive and sidecar law

`wf0-payload.zip` contains the binaries/resources, required notices, a bounded
`BUILD_IDENTITY_CORE.json`, and the canonical artifact manifest/hash. The full
Windows build receipt is a mandatory sidecar created after the ZIP so it can
bind the outer ZIP SHA-256 without self-reference. A hash file binds that
receipt. The one GitHub Actions artifact and the Mac-to-Deck handoff envelope
therefore contain exactly:

```text
wf0-payload.zip
WF0_WINDOWS_BUILD_RECEIPT.json
WF0_WINDOWS_BUILD_RECEIPT.sha256
```

The single Actions artifact name is exactly
`wf0-windows-build-<implementation-commit>-run-<run-id>-attempt-<attempt>`.
It is receipt-bound transport metadata, not artifact identity. GitHub's raw
wrapper ZIP SHA-256 must equal the exact digest returned by the upload result
and artifact API. The canonical extracted manifest is artifact identity; the
inner payload ZIP hash remains a mandatory transport binding. Deterministic
inner ZIP bytes are preferred and recorded, but the accepted equality claim is
made over the A/B extracted file manifest and exact transfer files. No
cryptographic provenance, trusted-builder, SLSA, or signing claim follows.

The published Deck cache root contains the extracted payload plus the two
manifest metadata files, the exact full build-receipt sidecars, a bounded Mac
artifact-custody receipt, and one owner marker that hashes those
sidecars. The payload manifest digest still names the root because its hashed
build-identity core differentiates runs. The importer validates this complete
cache-root roster separately; execution copies only manifest-declared runtime
payload files.

### Evidence handoff schema

The Deck-to-Mac private handoff uses:

```text
linux-vst-bridge-wf0-evidence-handoff/v1
```

It binds the implementation commit/tree/source-manifest digest, source-handoff
receipt/bundle/ref/worktree identity, workflow run/attempt/build-receipt and
Mac artifact-custody receipt digests, artifact-set and payload-archive digests,
runner/runtime digest, ordered exercise receipt digests, exact 14-file evidence
roster, each size/SHA-256, and an aggregate manifest SHA-256. It contains no
raw PID, hostname, network address, private absolute path, credential, binary,
compatdata, or proprietary state. The evidence packet schema and census schema
remain V5 v1; the build fields are adapted to reference the Windows receipt.

## 12. Windows/MSVC build-plane contract

### Runner and toolchain selection

The workflow uses exactly:

```yaml
runs-on: windows-2022
generator: Visual Studio 17 2022
architecture: x64
platform_toolset: v143
windows_sdk: 10.0.19041.0
configuration: Release
cmake_minimum: 3.25.0
```

The versioned label is currently official and x64. The current inventory shows
Visual Studio Enterprise 2022, CMake, Ninja, and the selected SDK. The pinned
SDK's own README selects `Visual Studio 17 2022 -A x64`, and Steinberg lists
MSVC 2022 as supported for x86_64 Windows. V7 chooses the Visual Studio
generator rather than Ninja because it is the exact upstream documented route
and directly activates the pinned `MSVC` VSTGUI branch.

The label remains mutable. An accepted run retains and binds actual image,
Windows, VS, compiler, linker, SDK, CMake, and generator identities. A changed
image is new evidence, not the same build.

### Workflow location, trigger, and permissions

The only workflow path is:

```text
.github/workflows/wf0-windows-msvc-build.yml
```

It is triggered only by a push to
`codex/wf0-windows-vst3-factory-census-v7` touching one of the exact 26 source
paths. It checks out only `github.sha`, verifies the exact expected branch/ref,
and rejects merge commits or a source commit with the wrong authority parent
shape. `pull_request_target` is prohibited. No `workflow_run`, issue comment,
schedule, tag, wildcard branch, or untrusted requested-ref execution exists.

The workflow has one `build_and_upload` job. At minimum it uses only pinned
`actions/checkout` and `actions/upload-artifact`; every `uses:` value is a full
40-character commit SHA verified as belonging to the named action repository.
Tags and moving branches are comments only, never the executable ref. Checkout
uses `persist-credentials: false` and the exact `github.sha`, followed by exact
branch/ref, commit, tree, parent, workflow-blob, and 26-record manifest
verification. No repository, organization, environment, or inherited secret is
exposed to build steps.

The complete workflow permission is:

```yaml
permissions:
  contents: read
```

No ID-token, attestation, artifact-metadata, packages, actions, deployments,
pull-request, issue, check, or contents-write permission exists. No attestation
job or second publication job exists. The same build job uploads the exact
three-file inner envelope exactly once under the fixed receipt-bound name and
captures the upload action's artifact ID, name, URL, and SHA-256 digest together
with run ID, run attempt, workflow run, head branch, and head SHA. These
post-upload facts do not enter or rewrite the inner build receipt; the Mac
custody receipt owns their join.

GitHub documents full-SHA action pins as the immutable action reference and
warns against executing untrusted code with `pull_request_target`. The Actions
artifact API owns later exact-ID metadata and raw download:

- [Secure use reference](https://docs.github.com/en/actions/reference/security/secure-use)
- [Secure use of pull_request_target](https://docs.github.com/en/actions/reference/security/securely-using-pull_request_target)
- [Actions artifacts REST API](https://docs.github.com/en/rest/actions/artifacts)
- [Pinned upload-artifact action](https://github.com/actions/upload-artifact)

### SDK acquisition and offline boundary

The dependency-acquisition step uses only official allowlisted VST3 SDK and
submodule remotes. It materializes the root at exact commit
`3cdf9ca5d1f5b1b21e0a86832aa4abe55607bd96`, checks its tree, reads every
gitlink, initializes all seven recursive submodules, and checks their exact
commits. It then requires clean roots, no extra submodule, no source patch, and
the exact five failure-source blobs recorded by the reconciliation.

Acquisition output is a local source tree, not a vendored or uploaded SDK
source archive. Once verified, dependency network closes. Configure/build
cannot fetch packages or sources, and the produced bundle excludes SDK source,
headers, build trees, and Git metadata.

### Exact build shape

Build A and B use separate new roots. Repository configure builds only:

```text
wf0-factory-probe
wf0-loader-adapter-tests
wf0-fault-fixtures
```

The separate exact SDK configure enables plug-in examples and VSTGUI only as
required for the single `again` target, disables hosting examples, validator,
module-info generation, plug-in links, and unrelated examples/targets, and
builds only:

```text
again
```

Both use Release, `-A x64`, `-T v143`, Windows SDK `10.0.19041.0`, static MSVC
runtime through `CMAKE_MSVC_RUNTIME_LIBRARY=MultiThreaded`, and deterministic
compiler/linker options supported by the selected tools. The native Windows
`attrib` invoked by the official SDK is used as-is; the Deck host shim is gone.
No upstream file is patched, copied into repository source, or shadowed by a
replacement target.

The future implementation may repair ordinary CMake/compiler defects inside
the exact 26-path envelope. If exact AGain cannot build on this supported route
without source modification, implementation returns to the design gate.

### PE and dependency verification

Repository-owned `verify.py` parses PE headers/import/export directories from
bytes and cross-checks the same outputs with exact selected `dumpbin` tool
receipts. Every executable/module is PE32+ AMD64. The scanner has the expected
entry point; the missing-factory negative lacks `GetPluginFactory`; every other
fault fixture has its declared exports; AGain has `GetPluginFactory` and its
applicable Windows entry/exit exports.

Imports are closed transitively. Windows system DLLs are named in a closed
case-insensitive roster. Static `/MT` is selected so the expected copied runtime
dependency roster is empty. If a non-system dependency remains, it is admitted
only after exact source, redistributable license, file identity, and recursive
imports are recorded within V7's existing artifact role; otherwise the build
blocks. No DLL is copied merely because it exists on `PATH`.

PDBs, import libraries, object files, source maps, compiler caches, and build
trees are excluded. PE checks do not execute the scanner or AGain.

### Artifact contents

The payload contains only:

- `wf0-factory-probe.exe` and the loader-adapter executable;
- the exact 22 approved fault-fixture DLLs;
- the exact AGain VST3 bundle and declared resources;
- only dependency DLLs proven necessary by the closed import traversal;
- `ARTIFACT_MANIFEST.json` and its SHA-256 file;
- bounded `BUILD_IDENTITY_CORE.json`; and
- required MIT/license/notice texts.

It excludes VST3 SDK source, full build roots, compilers, Git metadata,
credentials, PDBs/private paths, paid or proprietary plug-ins, Serum, Bitwig
content, installers, license state, and unrelated workflow files.

### Actual private-repository artifact custody

The exact repository posture is private and user-owned. Mandatory GitHub
artifact attestation is unavailable within the accepted posture and is absent
from the workflow, permissions, schemas, proofs, and blockers. WF0 does not
change repository visibility or ownership, require GitHub Enterprise Cloud, or
add another signing service. Exact Actions run/artifact metadata and byte hashes
establish custody only. They do not establish cryptographic provenance,
trusted-builder identity, SLSA, release signing, or code signing.

The one build-and-upload job publishes exactly one Actions artifact and records
the upload outputs. The authenticated Mac independently re-reads the same
artifact through the run and artifact APIs. A missing, expired, digest-less, or
identity-mismatched artifact is `WF0_BUILD_CUSTODY_BLOCKED`; there is no
attestation fallback and no alternative artifact selector.

## 13. Mac/Deck handoff and evidence-return contracts

### Exact private-repository Mac artifact download

The authenticated Mac selects one workflow run only by exact numeric run ID.
It reads the GitHub Actions run API and verifies:

- repository `kasselvania/Linux-VST-bridge`;
- workflow path and local exact workflow Git blob;
- event `push`;
- head branch `codex/wf0-windows-vst3-factory-census-v7`;
- exact implementation head SHA;
- run ID and run attempt;
- completed status and successful conclusion.

It then reads both the exact run's artifact list and the exact artifact-ID API
object and requires one matching artifact with exact ID, fixed receipt-bound
name, URL, size, SHA-256 digest, `expired=false`, workflow-run ID, workflow head
branch, and workflow head SHA. It never uses latest successful run, latest
artifact, name-only selection, an interactive selector, or an unverified
mutable branch head.

The Mac downloads the raw GitHub Actions artifact ZIP only through:

```text
GET /repos/kasselvania/Linux-VST-bridge/actions/artifacts/<artifact-id>/zip
```

into a new empty private marker-bound stage. The raw downloaded ZIP SHA-256
must equal the exact `sha256:<hex>` digest exposed by GitHub Actions. The Mac
then applies the same bounded safe-archive rules to the GitHub wrapper and
requires exactly these regular files, with no directory, link, or extra entry:

```text
wf0-payload.zip
WF0_WINDOWS_BUILD_RECEIPT.json
WF0_WINDOWS_BUILD_RECEIPT.sha256
```

The raw wrapper is capped at 272 MiB, has exactly three entries, and each
sidecar is capped at 16 MiB; the inner payload retains the existing 256 MiB
archive and 512 MiB extracted-payload caps.

It verifies the build-receipt hash, payload ZIP outer hash,
source/workflow/run identities, complete inner roster, and canonical artifact
manifest. It finally writes and reads back
`linux-vst-bridge-wf0-mac-artifact-custody/v1`. Authentication material and raw
credential configuration never enter that receipt or either handoff.

### Exact Mac implementation-source bundle

After the clean 26-path implementation-source commit and manifest are exact,
and after the artifact custody receipt is complete, the Mac re-verifies the
commit/tree/single parent, merged V7 authority commit/tree, branch/ref, exact
26-path delta, no evidence paths, no extra tracked implementation path, and the
26-record path/mode/blob manifest.

In one new private marker-bound source stage, the Mac creates a temporary bare
repository, materializes only the required exact authority and implementation
history from the canonical local repository, and creates exactly one ref:

```text
refs/handoff/wf0-v7-source/<implementation-commit>
```

It creates the self-contained bundle
`wf0-v7-execution-source-<implementation-commit>.bundle`, runs `git bundle
verify`, requires zero prerequisites and exactly the one advertised ref, and
records bundle SHA-256 and bounded size in
`linux-vst-bridge-wf0-source-handoff/v1`. The temporary staging repository is
not authority and may be removed only after bundle/receipt readback; the
canonical repository receives no handoff ref. The bundle and receipt remain
private and untracked.

### Mac-to-Deck source and artifact transfer

The Mac transfers the exact source bundle, source-handoff receipt and hash,
three-file Windows envelope, and Mac artifact-custody receipt through the
already-established ordinary SSH lane with agent forwarding disabled. It does
not copy a GitHub token/key/cookie, credential helper, SSH key or agent socket.
Source and artifact sets use separate new empty marker-bound Deck stages. Both
ends hash every transfer file; exact equality is required before import. A
partial or unknown preexisting stage blocks and is preserved.

The persistent source handoff is keyed by exact source commit at
`<HOME>/.local/share/linux-vst-bridge/handoffs/wf0/source/by-commit/<commit>/`.
The artifact handoff remains distinct and is keyed by its Mac custody and
payload digests. No Deck GitHub operation occurs.

### Exact Deck source import and execution worktree

The Deck re-verifies source-bundle and receipt hashes, runs `git bundle verify`,
and requires the exact self-contained status, advertised ref, implementation
commit, authority history, and zero prerequisites. It imports into the existing
Deck repository only as:

```text
refs/handoff/wf0-v7-source/<implementation-commit>
```

It then verifies the implementation commit, tree, single parent equal to the
merged V7 authority commit, exact 26-path delta, exact 26-record path/mode/blob
manifest, absence of all 14 evidence paths, and no additional tracked
implementation path. It creates one new clean detached worktree at the exact
commit under
`<HOME>/.local/share/linux-vst-bridge/worktrees/wf0/<implementation-commit>/`.
Unknown existing refs, destinations, worktrees, extra paths, or dirty state are
preserved and produce `WF0_SOURCE_HANDOFF_BLOCKED`.

Every Deck command executes from that worktree. Before every held-gate, fault,
positive, normalization, or evidence run it rechecks detached HEAD, clean
status, tree, parent, and reproduces the exact 26-record source manifest. The
Windows build receipt, artifact manifest, Mac custody receipt, source handoff,
worktree, execution receipts, and evidence must all name the same implementation
commit and source-manifest digest. The provisional MinGW worktree and copied
individual scripts are never execution source.

The verified bundle, handoff receipt, local ref, and worktree remain through
evidence return, final source re-verification, and WF0 acceptance. V7 defines no
automatic retirement and requires no clever partial recovery.

### Safe Deck artifact extraction and import

Before extraction, the Deck rejects an oversized ZIP, entry-count overflow,
duplicate/colliding paths, encryption, unsupported compression, non-regular
types, symlink mode bits, absolute/drive/UNC/backslash/traversal paths, unsafe
Unicode, and any entry absent from the canonical manifest. Extraction uses a
new owner/nonce-bound stage, exclusive file creation, per-component `lstat`
checks, declared byte bounds, streaming SHA-256, and final roster equality.

After exact readback, files become `0444`, directories `0555`, and the stage is
atomically renamed to the manifest-digest root. Read-only mode is defense in
depth, not identity; every future environment copy rehashes the manifest and
files. The execution command requires the full 64-hex artifact-set digest and
has no default. Artifact import completes only after the exact source execution
worktree is admitted, and execution begins only after the source/build/artifact
identity join is equal.

### Deck-to-Mac evidence return

The Deck renders the unchanged 14-file canonical evidence packet into a new
private stage, validates sanitization and hashes, then creates an exact
evidence-handoff manifest. The Mac pulls that handoff through SSH into a new
empty stage and requires transfer byte equality, schema/roster/hash closure,
source-handoff/worktree/build/artifact-custody identity equality, and safe
regular files.

Only the Mac copies the exact 14 files into the canonical implementation
worktree and creates the evidence-only commit. The Deck never authenticates to,
fetches from, pushes to, opens a PR in, or downloads a workflow artifact from
GitHub. Handoff material excludes credentials, hostnames, network addresses,
raw process identifiers, private absolute paths, proprietary state, compatdata,
and compiled binaries outside the exact artifact handoff.

## 14. Security, privacy, licensing, and threat model

- Workflow source is executable supply-chain input and is bound to the exact
  source commit/blob. No secret is exposed to executable implementation code.
- Full-SHA actions, least permissions, fixed trigger/ref, detached exact
  checkout, and no `pull_request_target` limit workflow authority.
- The repository is private and user-owned. WF0 neither changes that posture
  nor depends on Enterprise Cloud, artifact attestation, a signing service, or
  another repository owner.
- Actions run/artifact metadata and hashes prove bounded custody joins, not a
  trusted builder, cryptographic provenance, SLSA, or signed code.
- Hosted-runner labels are mutable service selectors, never durable build
  identity. Observed image/tool facts and artifact hashes are retained.
- SDK and VSTGUI are untrusted build inputs even when official. They execute no
  produced PE in the Windows plane and cannot write repository history.
- ZIP and evidence archives are untrusted until bounded safe-path extraction,
  exact manifest closure, and hash readback complete.
- The source Git bundle is untrusted until `git bundle verify`, exact
  ref/commit/tree/parent/delta/manifest closure, and a clean detached execution
  worktree all pass. Individually copied scripts have no source authority.
- SSH transports bytes only. It forwards no credential or agent and grants the
  Deck no GitHub capability.
- Runtime prefixes can contain sensitive machine/browser/license state. WF0
  uses only new disposable stages and never exports complete compatdata.
- Diagnostic/evidence output is allow-list based. Raw PIDs, command lines,
  paths, host/network identifiers, credentials, tokens, cookies, serials,
  vendor state, and generated binaries are excluded from Git.
- The pinned VST3 SDK/VSTGUI and repository fixtures are open source; required
  license/notice material accompanies transferred binaries. No paid vendor
  module, installer, preset, sample library, or activation data is built,
  transferred, or retained.
- This proof artifact is not a signed release, product distribution, or claim
  that Wine/Proton/Flatpak/GitHub-hosted runners provide a security sandbox.

## 15. Code topology, salvage, and tracked-path envelope

### Component responsibilities

| Component | Responsibility | Must not own |
|---|---|---|
| `.github/workflows/wf0-windows-msvc-build.yml` | fixed source checkout, acquisition/two-build/verification and one exact upload under `contents: read` | attestation, runtime proof, release pipeline, secrets |
| `cmake/WF0DependencyLock.cmake` | cross-platform exact SDK/submodule/options assertions | network acquisition, SDK edits |
| `windows-factory-probe/` | unchanged bounded Win32 load/factory/census contract | class instantiation, process kill, evidence |
| `windows-fixtures/wf0/` | exact approved negative family | positive compatibility claims |
| `build.py` | WindowsBuildPlane receipt and two-root build coordination only | Deck execution or GitHub authentication |
| `common.py` | exact repository/source/build/artifact/custody/handoff schemas and bounded roots | credential storage, network selection, process execution |
| `verify.py` | independent PE/parser/manifest/A-B and source-manifest checks | process execution or data repair |
| `artifacts.py` | authenticated Mac exact-ID artifact custody, source-bundle/SSH receipt verification, safe Deck source/artifact import, evidence-handoff verification | credential persistence, compiling, Git commit |
| `environment.py` | exact cache-to-stage copy and marker-bound retirement | artifact download, WR0 mutation |
| `supervise.py` | unchanged Runtime/Proton launch/gate/cleanup | factory semantics or global cleanup |
| `normalize.py` | unchanged census truth plus exact build/artifact binding | inventing or repairing output |
| `evidence.py` | fixed packet, handoff manifest, sanitizer and final source/worktree/build/artifact-custody equality | primary fact generation or GitHub access |
| `run.py` | Deck-only dependency-ordered source/worktree admission, artifact import, execution and evidence commands | Windows build, Mac Git, GitHub, product manager |

### Exact future source/configuration roster: 26

The raw-UTF-8 lexically sorted roster is:

```text
.github/workflows/wf0-windows-msvc-build.yml
.gitignore
CMakeLists.txt
cmake/WF0DependencyLock.cmake
docs/WF0_WINDOWS_BUILD_PLANE_LOCK.md
tools/wf0-factory-census/README.md
tools/wf0-factory-census/artifacts.py
tools/wf0-factory-census/build.py
tools/wf0-factory-census/common.py
tools/wf0-factory-census/environment.py
tools/wf0-factory-census/evidence.py
tools/wf0-factory-census/negative_tests.py
tools/wf0-factory-census/normalize.py
tools/wf0-factory-census/run.py
tools/wf0-factory-census/supervise.py
tools/wf0-factory-census/verify.py
windows-factory-probe/CMakeLists.txt
windows-factory-probe/include/linux_vst_bridge/wf0_probe/census.h
windows-factory-probe/include/linux_vst_bridge/wf0_probe/events.h
windows-factory-probe/source/factory_census.cpp
windows-factory-probe/source/factory_census.h
windows-factory-probe/source/main.cpp
windows-factory-probe/source/win32_module.cpp
windows-factory-probe/source/win32_module.h
windows-fixtures/wf0/CMakeLists.txt
windows-fixtures/wf0/source/fault_fixture.cpp
```

These and only these records form
`linux-vst-bridge-wf0-implementation-source/v1`. The workflow source is inside
the same manifest and cannot change independently of artifacts/evidence.

### Exact evidence roster: 14

```text
evidence/wf0-windows-vst3-factory-census/BASIS.md
evidence/wf0-windows-vst3-factory-census/TOOLCHAIN.md
evidence/wf0-windows-vst3-factory-census/BUILD.md
evidence/wf0-windows-vst3-factory-census/BUILD_MANIFEST.json
evidence/wf0-windows-vst3-factory-census/ENVIRONMENT.md
evidence/wf0-windows-vst3-factory-census/LAUNCH_AND_PROCESS.md
evidence/wf0-windows-vst3-factory-census/STAGE_TIMELINE.json
evidence/wf0-windows-vst3-factory-census/CENSUS.json
evidence/wf0-windows-vst3-factory-census/NEGATIVE_TESTS.md
evidence/wf0-windows-vst3-factory-census/PRESERVATION.md
evidence/wf0-windows-vst3-factory-census/FINDINGS.md
evidence/wf0-windows-vst3-factory-census/SANITIZATION.md
evidence/wf0-windows-vst3-factory-census/fixture.json
evidence/wf0-windows-vst3-factory-census/hashes.sha256
```

`TOOLCHAIN.md`, `BUILD.md`, and `BUILD_MANIFEST.json` are repurposed to retain
the Windows/MSVC/image/workflow, private artifact-custody, and exact
source-handoff facts. No evidence path is added.

### V5-to-V7 substitution justification

| V5 path/action | V7 disposition | Reason |
|---|---|---|
| add no workflow | add `.github/workflows/wf0-windows-msvc-build.yml` | the supported Windows build plane must be tracked and source-bound |
| `cmake/WF0Toolchain.cmake` | remove | exact MinGW cross-toolchain selection is superseded; VS developer environment and generator are observed/bound instead |
| `docs/WF0_WINDOWS_TOOLCHAIN_LOCK.md` | replace with `docs/WF0_WINDOWS_BUILD_PLANE_LOCK.md` | document mutable-label versus observed-image/tool identity and the supported MSVC route |
| `tools/wf0-factory-census/host-shims/attrib` | remove | native Windows `attrib` exists on the selected plane; a Linux no-op shim is wrong ownership |
| no cross-plane importer | add `tools/wf0-factory-census/artifacts.py` | smallest offline safe archive/manifest/import/evidence-handoff verification surface |
| `build.py` | repurpose | remove Flatpak/MinGW/Deck building; coordinate Windows-only exact builds and receipts |
| `common.py` | repurpose | replace MinGW identities with workflow/build/artifact schemas and plane-safe roots |
| `environment.py` | repurpose | copy only from one explicit content-addressed verified cache root |
| `verify.py` | repurpose | parse/cross-check MSVC PE outputs and canonical bundle rather than MinGW `objdump` |
| `run.py` | repurpose | remove Deck build phase; accept exact imported artifact identity only |
| scanner/supervisor/census sources | retain | no concrete build-plane contradiction was found; semantics stay bounded and require fresh proof |

The source count remains 26 and total count remains 40 because three obsolete
V5 paths are replaced by three necessary V7 paths while the justified 14-file
evidence roster remains. The counts were derived from owner needs, not preserved
as a cosmetic target.

### Path-by-path provisional salvage classification

Every row remains untrusted until future exact MSVC build and Deck runtime
proof. “Without semantic change” permits reconstructing the exact blob from the
archive into a new implementation branch only after V7 authority; it is not
current adoption.

| # | Provisional path | Primary classification | V7 law |
|---:|---|---|---|
| 1 | `.gitignore` | reusable after V7 build-plane/custody adaptation | replace Deck build roots with source/artifact handoff and cache/worktree exclusions |
| 2 | `CMakeLists.txt` | reusable after V7 build-plane adaptation | retain HP0 separation; select supported MSVC WF0 shape |
| 3 | `cmake/WF0DependencyLock.cmake` | reusable after V7 build-plane adaptation | remove Unix-only Git path; keep exact root/submodule checks |
| 4 | `cmake/WF0Toolchain.cmake` | MinGW-specific and superseded; prohibited direct adoption | path is retired |
| 5 | `docs/WF0_WINDOWS_TOOLCHAIN_LOCK.md` | MinGW-specific and superseded | replace with build-plane lock, do not rename/copy claims |
| 6 | `tools/wf0-factory-census/README.md` | reusable after V7 build-plane/custody adaptation | rewrite entry points and plane ownership |
| 7 | `tools/wf0-factory-census/build.py` | incomplete; reusable after V7 adaptation | delete Flatpak/MinGW/Deck semantics; Windows-only build receipts |
| 8 | `tools/wf0-factory-census/common.py` | incomplete; reusable after V7 adaptation | replace identities/roster/roots, preserve fail-closed primitives |
| 9 | `tools/wf0-factory-census/environment.py` | reusable after V7 adaptation | require explicit content-addressed cache identity |
| 10 | `tools/wf0-factory-census/evidence.py` | reusable after V7 adaptation | bind source handoff, Windows/Mac custody, and evidence return; keep sanitizer/14 paths |
| 11 | `tools/wf0-factory-census/host-shims/attrib` | MinGW-specific and superseded; prohibited direct adoption | path is retired; no replacement shim |
| 12 | `tools/wf0-factory-census/negative_tests.py` | reusable without semantic change | exact fault expectations and tripwire remain; prove afresh |
| 13 | `tools/wf0-factory-census/normalize.py` | reusable after V7 adaptation | census semantics unchanged; replace toolchain identity fields |
| 14 | `tools/wf0-factory-census/run.py` | incomplete; reusable after V7 adaptation | remove `build` on Deck; require exact imported source worktree and artifact selection |
| 15 | `tools/wf0-factory-census/supervise.py` | reusable without semantic change | exact Runtime/Proton process contract remains; prove afresh |
| 16 | `tools/wf0-factory-census/verify.py` | incomplete; reusable after V7 adaptation | replace MinGW parser driver and add source/artifact manifest/envelope checks |
| 17 | `windows-factory-probe/CMakeLists.txt` | reusable after V7 adaptation | express MSVC warnings/runtime; no scanner semantic change |
| 18 | `windows-factory-probe/include/linux_vst_bridge/wf0_probe/census.h` | reusable without semantic change | exact data boundary; prove MSVC ABI/build |
| 19 | `windows-factory-probe/include/linux_vst_bridge/wf0_probe/events.h` | reusable without semantic change | exact flush/event contract; prove MSVC build/runtime |
| 20 | `windows-factory-probe/source/factory_census.cpp` | reusable without semantic change | exact factory census; prove MSVC and Deck behavior |
| 21 | `windows-factory-probe/source/factory_census.h` | reusable without semantic change | exact internal contract; prove afresh |
| 22 | `windows-factory-probe/source/main.cpp` | reusable without semantic change | exact command/lifecycle/no-instantiation contract; prove afresh |
| 23 | `windows-factory-probe/source/win32_module.cpp` | reusable without semantic change | exact Win32 load/export/entry/unload semantics; prove afresh |
| 24 | `windows-factory-probe/source/win32_module.h` | reusable without semantic change | exact binding types; prove afresh |
| 25 | `windows-fixtures/wf0/CMakeLists.txt` | reusable after V7 adaptation | express MSVC flags/runtime; preserve exact target roster |
| 26 | `windows-fixtures/wf0/source/fault_fixture.cpp` | reusable without semantic change | exact fault behavior/tripwire; prove MSVC and Deck behavior |

No whole-commit merge, rebase, or cherry-pick is permitted. Future
implementation begins from the exact merged V7 authority, reconstructs each
approved blob/path after audit, creates the new V7 paths, and commits one clean
implementation-source identity. The archive is a source reference, not an
authority shortcut.

### Design and future mutation envelopes

This V7 repair commit differs from its exact V6 parent only at:

```text
CURRENT_SLICE.md
docs/slices/WF0/ADVERSARIAL_DESIGN_REVIEW_V6.md
docs/slices/WF0/IMPLEMENTATION_DESIGN_V7.md
```

The cumulative V7 design branch differs from basis
`67026ad7160a584cbf9cdb4bf0db7b8dbfa60136` at exactly:

```text
CURRENT_SLICE.md
docs/slices/WF0/WINDOWS_BUILD_PLANE_RECONCILIATION.md
docs/slices/WF0/IMPLEMENTATION_DESIGN_V6.md
docs/slices/WF0/ADVERSARIAL_DESIGN_REVIEW_V6.md
docs/slices/WF0/IMPLEMENTATION_DESIGN_V7.md
```

No implementation or evidence path is changed now. The V6 review is the exact
supplied technical-lead review. No V7 approval receipt exists; creating one
requires later separate exact operator approval and its own bounded change.

Future implementation may change only the 26 source/configuration paths and,
in one later evidence-only commit, the 14 evidence paths. Generated artifacts,
Git bundles, SDK source, and private handoffs never enter Git.

Permitted future external mutations are limited to ephemeral GitHub-hosted
acquisition/build/upload state, new private Mac artifact/source handoff stages,
the exact persistent Deck source bundle/receipt/local handoff ref/detached
worktree retained through acceptance, the exact content-addressed Deck artifact
cache, marker-bound disposable WF0 environments, owned runtime transients, and
private bounded evidence handoffs. The accepted WR0 environment and all other
protected state remain read-only.

## 16. Amended proof matrix

Exactly 62 rows exist: 34 supported build, private artifact-custody, source-
handoff, and cross-plane identity proofs plus 28 unchanged applicable
execution/evidence proofs retained from V5/V6. The count grew only where the
two reviewed custody gaps require distinct claims and refusal points. One run
may satisfy multiple rows only when its receipts map them explicitly. A green
workflow is not a Deck proof.

| # | Claim | Positive proof | Negative/refusal proof | Retained evidence and ceiling |
|---:|---|---|---|---|
| 1 | exact repository posture | API reads `kasselvania/Linux-VST-bridge`, private, owner type `User` | wrong visibility/owner/repository blocks; no posture repair | Mac basis/custody receipt; repository posture only |
| 2 | exact Windows runner selector and observed image | `windows-2022`, X64, exact `ImageOS`/`ImageVersion`/OS receipt | wrong label, self-hosted, missing image fact blocks | `TOOLCHAIN.md`; label is not immutable identity |
| 3 | supported VS/MSVC | VS 2022 Enterprise, `v143`, x64 developer environment, full `cl /Bv` receipt | alternate VS/toolset/host/target or incomplete identity blocks | build receipt; no general compiler claim |
| 4 | exact Windows SDK | installed and selected `10.0.19041.0` | default/different/missing SDK blocks | build receipt; this build only |
| 5 | exact source/tree/workflow | detached head, tree, 26-record manifest, workflow path/blob/commit equal | wrong ref/event/SHA/blob, dirty or extra source blocks | `BASIS.md`, `BUILD_MANIFEST.json`; source identity only |
| 6 | exact pinned SDK recursively | root/tree plus seven exact clean submodules | wrong/dirty/missing/extra gitlink/submodule blocks | `BUILD.md`; no generated binary claim |
| 7 | no SDK/VSTGUI patch | zero diff/untracked and five source blobs exact before both configures | injected edit/copy/shadow target blocks | `BUILD.md`; source custody only |
| 8 | two clean MSVC builds | distinct empty roots and byte-identical transfer rosters | root reuse or unexplained byte difference blocks | build manifest; no hosted-image reproducibility claim |
| 9 | scanner/fault PE identity | all exact files parse PE32+ AMD64 and match cross-check receipts | wrong machine/header/roster blocks | `BUILD_MANIFEST.json`; no runtime claim |
| 10 | exact AGain bundle | exact pinned target, resource/module roster and manifest | missing/extra/different bundle object blocks | build manifest; no factory-call claim |
| 11 | export/import/dependency closure | required/forbidden exports and closed recursive imports match | unknown import/export/runtime source blocks | build receipt; no product packaging claim |
| 12 | canonical artifact manifest | canonical bytes, sorted exact file roster and aggregate digest | duplicate/extra/missing/unsafe/noncanonical record blocks | `BUILD_MANIFEST.json`; extracted identity only |
| 13 | payload archive SHA-256 | inner payload ZIP hash equals build receipt and Mac/Deck readings | any byte mismatch blocks before extraction | build/custody receipts; wrapper is separate |
| 14 | exact single Actions artifact ID/digest | one upload output and APIs agree on ID/name/URL/size/digest/run/head | duplicate upload, missing digest, wrong ID/run/head or expiry blocks | Mac custody receipt; transport metadata only |
| 15 | mandatory attestation absent | workflow has one job, `contents: read`, pinned checkout/upload only; schemas contain no attestation | prohibited action/job/permission/field/blocker token scan rejects source | workflow/source receipt; no provenance or signing claim |
| 16 | raw exact-ID artifact download | exact numeric run and artifact APIs plus `/artifacts/<id>/zip`; raw ZIP hash equals Actions digest | latest/name-only/interactive selector or digest mismatch refused | Mac custody receipt; authenticated private download only |
| 17 | exact three-file wrapper envelope | bounded wrapper extraction yields only payload and two build-receipt files | extra/missing/link/unsafe/colliding entry blocks | Mac custody receipt; wrapper not artifact identity |
| 18 | canonical Mac artifact custody | v1 receipt joins repository/workflow/run/artifact/raw/inner/build/payload/manifest digests | recursive producer rewrite, credential field, or cross-object mismatch blocks | custody receipt; no trusted-builder claim |
| 19 | exact Mac-to-Deck artifact equality | artifact envelope and custody receipt hashes equal at both ends | partial, changed, unknown-stage or credential forwarding blocks | artifact transfer receipt; transport only |
| 20 | safe Deck artifact import | bounded archive census, exclusive extraction, manifest closure, atomic publish | traversal/link/collision/overflow/unsafe type blocks | import receipt; no execution claim |
| 21 | one exact artifact-set selection | full manifest digest required and reverified before copy | omitted, stale, altered, or `latest` selection refused | environment receipt; no ambient cache trust |
| 22 | exact Mac source bundle | fixed one-ref self-contained bundle plus v1 receipt binds authority/source/manifest/hash/size/verify | prerequisite, wrong/multiple ref, wrong commit/history or bundle mismatch blocks | source-handoff receipt; private/untracked only |
| 23 | exact Mac-to-Deck source transfer | bundle/receipt/hash bytes equal in new marker-bound stages | partial, changed, unknown-stage, token/key/agent forwarding blocks | source transfer receipt; no GitHub grant |
| 24 | exact Deck bundle import | `git bundle verify`, zero prerequisites, exact ref imported under fixed local handoff ref | wrong/multiple ref, commit/tree/parent/delta/path/evidence mismatch blocks | source admission receipt; local ref only |
| 25 | exact detached execution worktree | fixed user root is detached at exact commit, clean, exact tree/parent/delta | existing/dirty/attached/provisional/copied-script source refused | source admission receipt; no source mutation |
| 26 | source manifest before every Deck run | 26 path/mode/blob records reproduced before held-gate, each fault, positive, normalization and evidence | any dirty/missing/extra/mode/blob/HEAD change blocks before command | per-run receipt; source identity only |
| 27 | source/build/artifact/evidence agreement | all receipts name one implementation commit and source-manifest digest | any cross-object commit/digest mismatch blocks before execution/publication | build, custody, session and evidence receipts |
| 28 | no Deck GitHub dependency | source and artifacts arrive over SSH; all Deck work completes from local ref/worktree/cache | any fetch/push/API/token/key/artifact-download/PR requirement blocks | custody/process receipt; maintenance identity out of scope |
| 29 | Deck-to-Mac evidence equality | packet and handoff manifest hashes equal on both ends | partial, changed, unsafe, extra/missing file blocks | evidence custody receipt; no raw state |
| 30 | Mac commits only exact sanitized packet | diff contains exactly 14 validated evidence paths | binary/private token/path or another tracked path blocks | final Git diff/hashes; publication only |
| 31 | source/workflow change invalidates proof | final source manifest equals build/live manifest | any mode/blob/workflow change forces rebuild and rerun | `BUILD_MANIFEST.json`; no stale inheritance |
| 32 | wrong/stale artifact refused | exact source/run/manifest/archive/custody closure accepted | old run, wrong receipt, wrong digest, altered cache blocks | negative import receipt |
| 33 | wrong workflow run/source refused | run/artifact API metadata and local Git objects equal | wrong head/ref/event/path/blob/attempt/artifact ID blocks | Mac custody receipt |
| 34 | missing/extra artifact refused | exact canonical rosters accepted in Windows, Mac, and Deck stages | delete/add/rename/case-collide entry blocks | wrapper/bundle/import negatives |
| 35 | missing fixture refused | exact declared fixture path accepted | absent/renamed module in disposable stage blocks | `NEGATIVE_TESTS.md`; no module open |
| 36 | wrong PE architecture refused | x64 module admitted | repository-owned wrong-machine header fixture refused before launch | negative receipt; admission only |
| 37 | wrong scanner/module hash refused | exact hashes pass import/copy/spawn | one-byte mutation blocks | negative receipt; no semantic claim |
| 38 | DLL search authority exact | System32-only default, absolute module, null `hFile`, DLL-load-dir + System32 flags | relative/default-widening/non-null/application/PATH search rejected | timeline/adapter; dependency search only |
| 39 | first launch owns initialization | payload-only prestate and exact Runtime/Proton/scanner readiness | separate bootstrap command or prefix mutation refused | environment/process evidence; disposable only |
| 40 | gate causally prevents load | scanner waits 15 seconds with no load attempt/map/factory event | early load/map/event/departure blocks and cleans | timeline/preservation; no post-gate claim |
| 41 | complete AGain factory census | exact lifecycle/calls, metadata and ordered three classes, release/exit/unload | expected-field/order/completion mismatch blocks | census/timeline; factory boundary only |
| 42 | required export ownership | AGain export found after earlier resolution/check | `wf0-missing-factory`; proves no `InitDll` invocation | negative evidence; no factory obtained |
| 43 | null factory ownership | AGain returns non-null | `wf0-null-factory` | negative evidence; no class query |
| 44 | absent optional entry is observation | AGain entry retained | `wf0-no-entry` completes factory path | negative evidence; absence not generalized |
| 45 | entry false is stage failure | AGain entry true | `wf0-init-false` with cleanup | negative evidence; no compatibility claim |
| 46 | factory-info failure owned | exact AGain factory metadata | `wf0-factory-info-false` | negative evidence; no classes |
| 47 | factory version/tier fallback exact | support booleans and selected tier retained | factory3/2 unsupported/error branches fall back or block exactly | census/negative; only factory 1/2/3 |
| 48 | invalid/excessive count refused | AGain count is three | negative and 257 fixtures | negative evidence; no allocation/enumeration |
| 49 | ordinal class failure owned | all three AGain ordinals succeed | fixed class-info failure | negative evidence; no partial success |
| 50 | duplicate class ID refused | three unique raw TUIDs | duplicate-ID fixture | negative evidence; no dedup repair |
| 51 | malformed/oversized output refused | bounded canonical JSONL accepted | truncation, duplicate final, invalid UTF, cap+1 rejected | negative evidence; no data recovery |
| 52 | scanner crash call-attributed | every ordinary return has completion | entry/factory/class/release crash leaves exact final in-flight call | timeline/negative; containment only |
| 53 | scanner timeout call-attributed | calls complete inside bounds | entry/factory/class/release hangs map from final attempt | timeline/negative; no real-time claim |
| 54 | reverse factory release attributed | one reverse-order completion per acquired interface | hang/crash/ledger mismatch maps release blocker without erasing primary | timeline/negative; cleanup only |
| 55 | absent optional exit is observation | AGain exit retained | no-exit variant unloads | negative evidence; absence not generalized |
| 56 | exit false owned | AGain exit true | `wf0-exit-false`, unload still attempted | negative evidence |
| 57 | unload result mapping | AGain `FreeLibrary` true | loader-adapter false/in-flight mapping | timeline/unit receipt; no inducible valid-handle failure claim |
| 58 | no class instantiation | source token/AST check, operation/schema absence, tripwire marker absent, final false | injected marker or invocation token blocks | negative/census; no class lifecycle |
| 59 | exact cleanup and unrelated survival | zero owned descendants after success/failure | held-gate/stage failure plus same-family sentinel | launch/process evidence; no global cleanup |
| 60 | environment ownership and retirement | exact marker accepted, each stage reaches durable absence | forged/missing marker preserved; failed retirement blocks | environment/negative; no adoption/recovery |
| 61 | WR0/Bitwig/protected equality | labeled before/after exact V5 identities and mutation-root audit | deliberate mismatch blocks packet and no repair | `PRESERVATION.md`; no Bitwig launch or behavior claim |
| 62 | evidence bounded/sanitized/hash-closed | fixed roster, sanitizer, handoff equality, hashes and claim ceiling | forbidden token/path/binary/extra file blocks | `SANITIZATION.md`, hashes; no broader compatibility |

Rows 41 through 58 retain V5's exact lifecycle/call completion and primary
failure precedence. Row 42 specifically proves required-export check before
entry and no entry call when absent. Rows 52 through 57 reject any timeline
that publishes a later lifecycle event before an ordinary call's paired
completion is flushed.

## 17. Failure and blocked-result taxonomy

Exactly 29 primary results exist. Secondary cleanup failures remain beside the
first primary result and never overwrite it.

| # | Result | Exact owner/stage | Preserved facts | Next lawful action |
|---:|---|---|---|---|
| 1 | `WF0_V7_PREFLIGHT_BLOCKED` | authority/fixture/custody preflight | read-only mismatch | stop; external authority resolution only |
| 2 | `WF0_WINDOWS_BUILD_PLANE_DESIGN_BLOCKED` | unsupported/unavailable selected build plane | official/source findings | return design gate |
| 3 | `WF0_REFERENCE_FIXTURE_DESIGN_BLOCKED` | exact AGain cannot remain selected | source/license findings | return design gate; no substitute |
| 4 | `WF0_WORKFLOW_SOURCE_BLOCKED` | source/branch/trigger/workflow identity | exact mismatch receipt | repair only inside approved source envelope |
| 5 | `WF0_DEPENDENCY_ACQUISITION_BLOCKED` | SDK root/submodule/remotes | bounded acquisition facts | reacquire exact dependency or return gate |
| 6 | `WF0_WINDOWS_BUILD_BLOCKED` | tool identity/offline/configure/compile/link | bounded logs and receipts | repair exact build inside envelope |
| 7 | `WF0_BUILD_COMPARISON_BLOCKED` | A/B identity or bytes differ | both manifests and differing safe paths | repair determinism; no normalization invention |
| 8 | `WF0_BUILD_CUSTODY_BLOCKED` | one upload plus artifact ID/digest and Mac custody join | exact valid build/upload/API/raw/inner facts | repair exact custody inside envelope or return gate |
| 9 | `WF0_ARTIFACT_BUNDLE_BLOCKED` | PE/roster/import/export/manifest/archive | bounded valid build facts | repair packaging/verification only |
| 10 | `WF0_MAC_ARTIFACT_BLOCKED` | private run/artifact API selection, raw exact-ID download, wrapper/inner verification | run/artifact metadata and downloaded hashes | choose/verify exact run and artifact; no Deck fallback |
| 11 | `WF0_ARTIFACT_TRANSFER_BLOCKED` | Mac-to-Deck artifact/custody byte equality | local/remote hash prefix | retry fresh exact staging; no credential forwarding |
| 12 | `WF0_SOURCE_HANDOFF_BLOCKED` | source bundle/receipt/SSH/import/local ref/detached worktree/manifest | exact valid source and transfer prefix | preserve unknown/dirty state; retry only a new exact stage |
| 13 | `WF0_ARTIFACT_IMPORT_BLOCKED` | safe extraction/content-addressed publish | transfer/archive/manifest facts | preserve unknown root; retry new stage |
| 14 | `WF0_SCAN_ENVIRONMENT_BLOCKED` | create/copy/retire stage | marker/root facts | mutate only exact owned stage |
| 15 | `WF0_SCANNER_LAUNCH_BLOCKED` | spawn or pre-readiness failure | launch/last-stage receipt | scoped cleanup; repair launch contract |
| 16 | `WF0_PROCESS_IDENTITY_BLOCKED` | topology/gate/ancestry mismatch | sanitized session mismatch | signal only revalidated owned identities |
| 17 | `WF0_MODULE_OPEN_BLOCKED` | `LoadLibraryExW` | artifact IDs and error class | applicable exit/unload/drain |
| 18 | `WF0_MODULE_ENTRY_BLOCKED` | present entry false/in-flight | entry presence/attempt/result | applicable exit/unload/drain |
| 19 | `WF0_FACTORY_GET_BLOCKED` | missing export/null/in-flight call | exact export/factory prefix | release if acquired; exit/unload/drain |
| 20 | `WF0_FACTORY_INFO_BLOCKED` | metadata/query inconsistency | bounded factory prefix | release/exit/unload/drain |
| 21 | `WF0_CLASS_ENUMERATION_BLOCKED` | count/tier/ordinal/duplicate | valid prefix, never partial success | release/exit/unload/drain |
| 22 | `WF0_FACTORY_RELEASE_BLOCKED` | reverse-release in flight/ledger mismatch | earlier primary plus release prefix | continue reachable cleanup; never erase primary |
| 23 | `WF0_MODULE_EXIT_BLOCKED` | present exit false/in-flight | prior result plus exit prefix | still attempt unload/drain |
| 24 | `WF0_MODULE_UNLOAD_BLOCKED` | unload false/in-flight | prior stages plus unload prefix | process containment/drain |
| 25 | `WF0_OUTPUT_NORMALIZATION_BLOCKED` | malformed/oversized/incomplete output | raw hash and last valid stage/call | discard partial normalized output; drain |
| 26 | `WF0_PROCESS_CLEANUP_BLOCKED` | owned descendant survives | exact transient identity; sanitized role | preserve stage; no broad kill |
| 27 | `WF0_PROTECTED_FIXTURE_DRIFT` | any labeled protected mismatch | content-free mismatch | stop; never repair protected state |
| 28 | `WF0_EVIDENCE_BLOCKED` | Deck packet/return/Mac verification/final diff | bounded valid inputs and hashes | regenerate/return exact packet or rebuild/rerun if source-bound |
| 29 | `RETURN_TO_DESIGN_GATE` | material owner/envelope/claim discovery | bounded read-only finding | fresh design/review/approval |

Optional entry/exit absence is an observation, not a blocker. Wrong or stale
artifacts map to custody/import blockers before execution rather than being
misreported as module or factory behavior.

## 18. Material-discovery stop conditions

Future implementation returns `RETURN_TO_DESIGN_GATE` immediately if:

- no explicit supported GitHub-hosted Windows/MSVC route can build pinned
  AGain without SDK source modification;
- the supported route requires a different positive fixture;
- any VST3 SDK or VSTGUI source must be patched, copied, shadowed, or replaced;
- build artifacts cannot bind one exact source/workflow/toolchain identity;
- the fixed workflow trigger, full-SHA action policy, or least-permission model
  cannot be maintained;
- the mutable runner label cannot yield a complete observed image/tool receipt;
- Mac cannot verify exact private-repository run/artifact API metadata, raw
  exact-ID wrapper digest, inner receipt, payload, and manifest before Deck
  transfer;
- the exact implementation source cannot be bundled self-contained, transferred
  over ordinary SSH, imported under the fixed local ref, or executed from one
  clean detached exact-commit worktree;
- any Deck run cannot reproduce the 26-record source manifest and join the same
  commit/digest to build, artifact, session, and evidence receipts;
- the Deck must authenticate to GitHub or use its API for ordinary WF0 work;
- safe bounded import requires extraction outside the content-addressed root or
  alteration of incoming binaries;
- a permanent product service, self-hosted runner, or build farm is required;
- the accepted Runtime 4 / Proton 11 route must change;
- the accepted WR0 environment must be mutated or reused;
- Bitwig must be launched, changed, or behaviorally revalidated;
- a persistent product environment or separate bootstrap workload is required;
- class instantiation, a new call operation, IPC, native proxy, audio, GUI,
  Serum, authorization, product packaging, release signing, or distribution is
  required;
- scanner lifecycle/call events cannot remain independently observable under
  the inherited bounds and immediate-completion law;
- process topology materially changes or cleanup cannot remain exactly owned;
- the revised path envelope materially exceeds the exact 26 source plus 14
  evidence paths;
- a source/workflow change cannot invalidate old artifacts and execution
  evidence;
- evidence requires private/proprietary content, compiled binaries, raw process
  identity, or complete compatdata; or
- the primary claim, claim ceiling, owner map, proof ownership, security/privacy
  posture, or fixture identity changes.

A normal workflow, CMake, build-script, transfer-script, scanner, or evidence
defect inside these owners, schemas, paths, and claims is an implementation
repair. It does not authorize widening the design or silently weakening a gate.

### Open implementation observations

The following are intentionally unclaimed until an authorized run:

1. the actual hosted image version and complete observed VS/MSVC tool identity;
2. the exact MSVC PE sizes/hashes and A/B byte identity;
3. the closed system-import roster and whether any redistributable runtime DLL
   remains after `/MT`;
4. the exact AGain Windows bundle/resource roster produced by pinned SDK/MSVC;
5. exact private Actions artifact ID/name/URL/size/digest and raw exact-ID
   wrapper equality under the future run;
6. exact Mac source-bundle SHA/size and Deck local-ref/worktree identities;
7. the final content-addressed artifact-set digest;
8. live scanner-specific Runtime/Proton topology and exact AGain factory data;
9. successful unload and all bounded fault results on the Deck; and
10. exact Deck-to-Mac evidence-handoff digest.

Every unknown has a proof row and blocker. None grants permission to implement
or change authority during this design phase.

## 19. Future implementation sequence

This dependency order is not current authorization:

```text
1. Obtain fresh independent V7 review, separate exact operator approval,
   approval receipt, merge, and exact `main` commit/tree readback.
2. Reverify protected Deck state, archive custody, Mac control-plane custody,
   and the current official availability of `windows-2022` without changing
   the selected label silently.
3. Create `codex/wf0-windows-vst3-factory-census-v7` from the exact merged V7
   authority basis.
4. Reconstruct audited reusable provisional paths individually; retire the
   MinGW toolchain/doc/shim; add the exact workflow/build-plane doc/artifact
   verifier; change only the 26 source/configuration paths.
5. Commit one clean implementation-source identity and regenerate the exact
   26-record manifest.
6. Push that exact commit from the Mac. Allow only its fixed push trigger to
   create one `windows-2022` workflow run.
7. In the Windows plane, check out exactly, acquire and verify the pinned SDK,
   close dependency network, perform two clean MSVC builds, verify PE and A/B
   identity, package, and upload the exact three-file envelope once.
8. On the Mac, select the exact run ID and artifact ID through the private
   Actions APIs, download the raw wrapper by exact ID, verify its API digest,
   safely require the exact inner envelope, and publish the Mac custody receipt.
9. On the Mac, create and verify the exact self-contained one-ref source Git
   bundle and source-handoff receipt from the frozen implementation commit.
10. Transfer the exact source and artifact handoff sets over ordinary SSH with
    no credential or agent forwarding and require byte equality on the Deck.
11. On the Deck, verify/import the source bundle under the fixed local ref,
    create the clean detached exact-commit execution worktree, and reproduce
    the 26-record manifest. Do not contact GitHub.
12. On the Deck, safely import the Windows payload to the exact manifest-digest
    root, make it read-only, and require source/build/artifact identity equality.
13. Execute all bounded import negatives, deterministic adapters, live fault
    family, held-gate proof, and positive AGain census in fresh disposable
    environments through Runtime 4 / Proton 11 only from the exact worktree.
14. Drain every owned process, retire every environment, and prove exact
    Bitwig/WR0/protected equality.
15. Render the exact 14-file sanitized packet and evidence-handoff manifest;
    return it by SSH; verify it on the Mac.
16. Add only the 14 evidence paths in a later evidence-only commit. Regenerate
    and require equality of the original 26-record source manifest.
17. Push from the Mac and leave the implementation PR unmerged for independent
    audit. The Deck never commits or pushes.
```

No whole provisional commit is merged, rebased, or cherry-picked. Useful work
is reconstructed; obsolete build ownership is not carried forward.

## 20. Adversarial design-review disposition

```yaml
review_path: docs/slices/WF0/ADVERSARIAL_DESIGN_REVIEW_V7.md
review_status: required_not_created
reviewed_revision: wf0-design-v7
reviewed_commit: external_identity_from_design_commit
reviewed_design_blob: external_identity_from_design_commit
reviewed_design_sha256: external_identity_from_design_commit
superseded_review_path: docs/slices/WF0/ADVERSARIAL_DESIGN_REVIEW_V6.md
superseded_review_github_id: 5084539680
superseded_review_result: DESIGN_REPAIR_REQUIRED
prior_v5_review_path: docs/slices/WF0/ADVERSARIAL_DESIGN_REVIEW_V5.md
prior_v5_review_blob: 8abbbdaf60fc00e00eb0a036fe9463ab4da35984
prior_v5_review_result: DESIGN_CLEAR
unresolved_findings: fresh_independent_v7_review_required
implementation_authorized: false
```

The design-authoring context does not review its own work. A fresh independent
reviewer must inspect the exact immutable V7 commit/tree/blob/SHA-256 and at
least determine whether:

1. the exact MinGW/VSTGUI failure and correct stop are fully supported by the
   pinned source objects;
2. the exact private/user repository posture is bound without visibility,
   ownership, plan, attestation, or replacement-signing dependency;
3. mutable runner label and observed build identity are kept distinct;
4. the one-job workflow trigger, `contents: read`, secret posture, checkout,
   and full-SHA action pins exclude `pull_request_target` and unbound code;
5. SDK acquisition/network closure/MSVC procedure preserve the exact fixture;
6. exact run/artifact ID/digest, raw exact-ID download, inner envelope and Mac
   custody receipt close artifact transport without recursive producer fields;
7. source bundle, SSH equality, Deck local ref, detached worktree, per-run
   source manifest and source/build/artifact/evidence agreement fail closed;
8. all 26 provisional paths have truthful salvage classifications;
9. the 26+14 path envelope, 62 proof rows, 29 results, 14 owners, and 25
   material operations agree across the card; and
10. the unchanged scanner/call lifecycle, protected-state law, and claim
    ceiling remain internally consistent.

No V7 review or approval record is created by this commit. The V6 review file
only materializes supplied GitHub review `5084539680`.

## 21. Proposed design identity

The card cannot embed its own Git blob or SHA-256 without changing those
identities. They are computed externally from the final one-commit design and
must be copied verbatim into later review and approval records.

```yaml
card_path: docs/slices/WF0/IMPLEMENTATION_DESIGN_V7.md
card_git_blob: external_identity_from_design_commit
card_sha256: external_identity_from_design_commit
design_revision: wf0-design-v7
design_status: proposed_for_adversarial_review
design_repair_parent_commit: 1e14eb8c318263b7307fe8e07aae13628402e0c1
design_repair_parent_tree: 63f9ce1edfe663848e2a4832c7facad881b83aac
accepted_basis_commit: 67026ad7160a584cbf9cdb4bf0db7b8dbfa60136
accepted_basis_tree: 2021dfcbf000d934a462d40efef6bbe7b54e0862
reconciliation_path: docs/slices/WF0/WINDOWS_BUILD_PLANE_RECONCILIATION.md
superseding_review_path: docs/slices/WF0/ADVERSARIAL_DESIGN_REVIEW_V6.md
superseding_review_github_id: 5084539680
approval_receipt_path: docs/slices/WF0/DESIGN_APPROVAL_V7.md
approval_receipt_identity: null
implementation_authorized: false
```

A later approval must bind the exact V7 card blob/SHA-256, V6 review and
reconciliation,
unchanged primary claim, three-plane owner model, runner/toolchain/SDK policy,
private-repository workflow security, artifact/source custody schemas, source
salvage law, exact path roster, 62-row matrix, 29-result taxonomy, stop law,
nonclaims, and inherited scanner laws.
`CURRENT_SLICE.md` must separately transition authority only after merge and
exact `main` readback.

## 22. Implementation handoff

No implementation handoff or implementation prompt is produced in this phase.
After the required review, approval, merge, and readback, a separate
authority-owned handoff must bind the new basis, exact V7 external identities,
approval receipt, one primary claim, exact fixture, 26+14 path envelope,
Windows workflow/tool identities, exact private artifact custody, exact source
bundle/local-ref/worktree identity, artifact/evidence custody chain, proof
matrix, stop law, nonclaims, and all inherited scanner/call-order laws. It must not use
the full dossier as an implementation task contract or treat the preserved
provisional commit as accepted authority.

```text
design_status=proposed_for_adversarial_review
implementation_authorized=false
successor_selection_authorized=false
```
