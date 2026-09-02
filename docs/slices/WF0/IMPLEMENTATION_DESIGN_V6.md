# WF0 Implementation Design V6 — Supported Windows Build Plane and Deck Artifact Handoff

## 1. Identity and authority

```yaml
slice: WF0
title: Supervised Windows VST3 Factory Census Probe
design_revision: wf0-design-v6
design_status: proposed_for_adversarial_review
implementation_authorized: false
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
implementation_source_paths: 26
evidence_paths: 14
total_future_tracked_paths: 40
owners: 14
material_operations: 22
proof_matrix_rows: 53
blocked_results: 28
implementation_authorized: false
successor_selection_authorized: false
```

This is a design amendment, not an implementation authority. It supersedes
only V5's Deck-local MinGW build and artifact-control assumptions. V5's
scanner, factory lifecycle, census schema, Runtime 4 / Proton 11 execution
route, protected-state law, and claim ceiling remain binding unless this card
states an exact build-plane adaptation.

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
different build plane. V6 requires a fresh independent adversarial review, a
separate exact operator approval, authority merge, and exact `main` readback
before any implementation source, workflow, Windows build, or Deck execution.

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

The product claim is unchanged. V6 changes the build and artifact-control
planes only.

### End-to-end route

```text
one clean V6 implementation-source commit
  -> fixed GitHub Actions workflow on explicit `windows-2022`
  -> exact SDK acquisition and two clean MSVC 2022 x64 builds
  -> independently verified scanner/fault/AGain PE artifacts
  -> canonical extracted-file manifest + bounded build receipt
  -> outer payload archive + provenance attestation
  -> exact authenticated Mac run download and verification
  -> ordinary SSH byte transfer with no credential forwarding
  -> content-addressed read-only Deck artifact cache
  -> disposable WF0 environment
  -> accepted Runtime 4 / Proton 11 scanner route
  -> held-gate, negative-family, and positive AGain proofs
  -> bounded sanitized evidence handoff over SSH
  -> Mac verification and evidence-only Git commit
```

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
```

The installed MinGW extension may remain at its exact user-scope commit but is
not an input to V6. Its classification is
`installed_exact_but_not_selected_for_wf0_again`.

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
| `MacControlPlane` | canonical repository, branches/commits/PRs, GitHub authentication, run observation, exact artifact download, provenance verification, SSH transfer, evidence retrieval and Git publication | Windows compiler semantics, Proton execution, live Deck truth |
| `WindowsBuildPlane` | exact source checkout, SDK materialization, observed tool identities, CMake/MSVC builds, two-build comparison, PE checks, manifest/archive, workflow artifact, provenance attestation | Deck credentials, Proton, accepted runtime behavior, release signing or permanent build service |
| `SteamDeckExecutionPlane` | inbound hash/manifest verification, content-addressed cache, disposable environment, Runtime 4 / Proton 11 execution, negative/positive proofs, cleanup, evidence handoff | GitHub login/API, Git push/PR, compilation, incoming-binary alteration |

### Exact owner map

Exactly 14 owners exist. A script may implement more than one only while their
state, identities, and commit points remain distinct.

| # | Plane | Owner | Authoritative output | Permitted mutation | Must not own |
|---:|---|---|---|---|---|
| 1 | Mac | `ImplementationSource` | clean commit/tree and canonical 26-record source manifest | future V6 implementation branch only | artifacts, workflow-run truth, Deck state |
| 2 | Mac | `MacArtifactCustodian` | exact run/download/attestation/transfer receipt | new private Mac staging/handoff roots | compilation, Deck cache adoption |
| 3 | Mac | `EvidencePublisher` | exact verified 14-file evidence-only commit | canonical implementation worktree evidence paths | raw Deck state, primary fact generation |
| 4 | Windows | `WindowsBuildTransaction` | observed runner/tool/SDK receipts and two clean build results | ephemeral runner acquisition/build roots | source history, Proton, GitHub secrets |
| 5 | Windows | `WindowsArtifactBundle` | canonical extracted manifest, payload archive, outer hash, bounded receipt | ephemeral publication root and workflow artifact | runtime claims, proprietary payloads |
| 6 | Windows | `WindowsFactoryProbe` | bounded scanner events and raw factory census | build outputs; later process memory only on Deck | process cleanup, class instantiation |
| 7 | Windows | `FaultFixtureFamily` | exact approved negative DLL roster | build outputs only | positive compatibility claim |
| 8 | Deck | `DeckArtifactCache` | one manifest-digest-addressed read-only artifact set | exact staging root then exact cache root | compilation, repair, `latest` alias |
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
v6_authority_proposed
  -> v6_reviewed_clear
  -> v6_separately_approved
  -> v6_authority_merged_and_read_back
  -> implementation_branch_created_from_exact_merge
  -> implementation_source_committed_once
  -> implementation_source_manifest_frozen
  -> exact_source_commit_pushed_by_mac
  -> workflow_run_created_for_exact_push
```

No arrow through `v6_authority_merged_and_read_back` exists during this design
turn. Future implementation uses a new branch
`codex/wf0-windows-vst3-factory-census-v6` from the exact merged V6 authority;
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
  -> attestation_job_downloaded_exact_artifact
  -> provenance_attested
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
`WF0_BUILD_COMPARISON_BLOCKED`; V6 does not normalize arbitrary PE differences
into equality.

### Mac artifact-custody lifecycle

```text
run_selected_by_id
  -> run_metadata_verified
  -> new_empty_download_root
  -> exact_named_workflow_artifact_downloaded
  -> download_roster_verified
  -> build_receipt_verified
  -> payload_outer_hash_verified
  -> provenance_attestation_verified
  -> canonical_manifest_verified_without_extraction_escape
  -> exact_handoff_set_ready
  -> ssh_transfer_complete
  -> remote_transfer_hash_equal
```

The authenticated Mac uses the exact run ID, repository, workflow path, branch,
head SHA, attempt, event, conclusion, and artifact name. It never asks for the
latest successful run. A rerun is a distinct attempt and build identity even
when the source SHA is unchanged.

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
artifact_cache_reverified_for_execution
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

### Inherited scanner lifecycle

V6 preserves the exact 23 lifecycle names, two typed call-event kinds, and
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
- A failed evidence transfer may retry into a fresh Mac staging root only after
  the Deck packet remains immutable and its exact hashes are re-read.

## 7. Fallible-operation and mutation ledger

Exactly 22 material operations exist:

| # | Operation | Plane/owner | Preconditions | Commit point | Primary failure |
|---:|---|---|---|---|---|
| 1 | freeze implementation source | Mac / `ImplementationSource` | merged V6 authority exact; clean new branch | one clean 26-path commit and manifest readback | `WF0_WORKFLOW_SOURCE_BLOCKED` |
| 2 | push exact source | Mac / `ImplementationSource` | commit/tree/branch reverified | remote ref equals exact commit | `WF0_WORKFLOW_SOURCE_BLOCKED` |
| 3 | create and bind workflow run | Mac/GitHub | fixed push trigger | run ID/attempt/head/path receipt | `WF0_WORKFLOW_SOURCE_BLOCKED` |
| 4 | check out exact source | Windows / `WindowsBuildTransaction` | `github.sha` exact | detached HEAD/tree/manifest equal | `WF0_WORKFLOW_SOURCE_BLOCKED` |
| 5 | acquire exact SDK | Windows / `WindowsBuildTransaction` | acquisition network open; allowlisted remotes | root plus seven submodules exact/clean | `WF0_DEPENDENCY_ACQUISITION_BLOCKED` |
| 6 | configure/build A | Windows / `WindowsBuildTransaction` | offline posture; empty root; tool receipts | requested targets complete | `WF0_WINDOWS_BUILD_BLOCKED` |
| 7 | verify A | Windows / `WindowsArtifactBundle` | build A complete | exact PE/roster/import/export receipt | `WF0_ARTIFACT_BUNDLE_BLOCKED` |
| 8 | configure/build B | Windows / `WindowsBuildTransaction` | same identities; distinct empty root | requested targets complete | `WF0_WINDOWS_BUILD_BLOCKED` |
| 9 | verify B and compare | Windows / `WindowsArtifactBundle` | both roots complete | byte-equal transfer roster | `WF0_BUILD_COMPARISON_BLOCKED` |
| 10 | publish manifest/archive/receipt | Windows / `WindowsArtifactBundle` | comparison accepted | canonical files and hashes read back | `WF0_ARTIFACT_BUNDLE_BLOCKED` |
| 11 | upload once and attest | Windows / `WindowsArtifactBundle` | exact payload and receipt | one workflow artifact; exact same-run download; payload attestation | `WF0_BUILD_PROVENANCE_BLOCKED` |
| 12 | download exact run artifact | Mac / `MacArtifactCustodian` | authenticated exact run readback | new root has exact artifact roster | `WF0_MAC_ARTIFACT_BLOCKED` |
| 13 | verify provenance and payload | Mac / `MacArtifactCustodian` | downloaded bytes bounded | attestation/receipt/archive/manifest closure | `WF0_MAC_ARTIFACT_BLOCKED` |
| 14 | transfer handoff to Deck | Mac / `MacArtifactCustodian` | no credential forwarding; exact local hashes | remote hashes equal local | `WF0_ARTIFACT_TRANSFER_BLOCKED` |
| 15 | import artifact set | Deck / `DeckArtifactCache` | safe new staging root | atomic content-addressed read-only root | `WF0_ARTIFACT_IMPORT_BLOCKED` |
| 16 | create execution environment | Deck / `WF0ScanEnvironment` | artifact reverified; protected preflight | marker and payload readback | `WF0_SCAN_ENVIRONMENT_BLOCKED` |
| 17 | spawn and identify scanner | Deck / `WF0Supervisor` | exact command/session; gate absent | readiness, ancestry, role census | `WF0_SCANNER_LAUNCH_BLOCKED` |
| 18 | publish gate and run census | Deck / `ScannerProcessSession` | fresh protected equality and artifact hashes | terminal bounded scanner receipt | exact module/factory blocker |
| 19 | drain and retire | Deck / supervisor/environment | terminal stage | zero descendants and root absence | `WF0_PROCESS_CLEANUP_BLOCKED` |
| 20 | normalize and stage evidence | Deck / normalizer/evidence | complete receipts; protected postflight equal | exact 14-file packet/hash closure | `WF0_EVIDENCE_BLOCKED` |
| 21 | return and verify evidence | Deck/Mac / evidence owners | immutable packet and manifest | Mac bytes/roster/hashes equal | `WF0_EVIDENCE_BLOCKED` |
| 22 | publish evidence-only commit | Mac / `EvidencePublisher` | exact source manifest still equal | exact 14 evidence paths committed | `WF0_EVIDENCE_BLOCKED` |

No failure authorizes mutation outside its owner. Unknown roots, wrong refs,
unexpected artifacts, or protected drift are preserved and block. Cleanup may
delete only nonce/marker-bound stages created by the same transaction.

## 8. Process, thread, topology, and bounds

### Windows build topology

The hosted build contains only GitHub runner orchestration, exact checkout and
SDK acquisition tools, the VS 2022 x64 developer environment, CMake/MSBuild,
MSVC compiler/linker/resource tools, repository-owned verification scripts,
and archive/attestation publication. It does not execute any produced PE,
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
| accept authority | exact V6 design/review/approval/merge/main readback | before branch creation | missing or mismatched authority |
| accept source | clean commit/tree + exact 26-record manifest | before push, workflow checkout, each build, each Deck run, final evidence commit | dirty/missing/extra/mode/blob change |
| accept workflow | fixed path, blob, source commit, exact branch trigger | run creation and receipt | wrong event/ref/path/blob/SHA |
| accept hosted runner | label plus observed image/OS/architecture | workflow start | wrong label or non-x64/self-hosted runner |
| accept toolchain | VS 2022 `v143`, x64 developer environment, complete `cl /Bv`, Windows SDK `10.0.19041.0`, CMake/generator | before each configure | ambient/different VS, compiler, SDK, generator |
| accept SDK | root commit/tree + seven gitlink/submodule commits + clean status | after acquisition and before both configures | wrong/dirty/missing/unexpected submodule or remote |
| accept build | source/workflow/toolchain/SDK identities + distinct empty root | each configure/build | reused root, network-open posture, target drift |
| accept artifact | canonical manifest digest + exact receipt + archive hash + PE closure + A/B equality | Windows publication, Mac download, Deck import, pre-execution | missing/extra/different/unsafe file or stale run |
| accept attestation | exact repository, signer workflow, source digest/ref, payload subject digest, hosted runner | Mac before transfer | absent/failed/wrong signer or source |
| accept transfer | local/remote SHA-256 of exact envelope files | before and after SSH | mismatch, partial, credential forwarding |
| accept cache root | manifest digest equals directory name; exact bytes/modes; no links | import and before each environment copy | unknown root, drift, writable or unsafe entry |
| spawn scanner | exact artifact/source/build receipt, environment marker, runner digest, nonce, command | immediately before spawn and readiness | stale session, alternate vector, wrong artifact |
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
| workflow artifact | GitHub Actions run | convenience transport bound by exact run/attempt and payload hash | retention expiry does not invalidate already verified local custody |
| Mac handoff set | new private staging root | exact receipt/archive/attestation verification bytes | retain until Deck import and evidence completion; no Git tracking |
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

V6 retains `linux-vst-bridge-wf0-implementation-source/v1`. Canonical JSON uses
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
  "workflow_ref": "refs/heads/codex/wf0-windows-vst3-factory-census-v6",
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
  "repository": "kasselvania/Linux-VST-bridge",
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
    "ref": "refs/heads/codex/wf0-windows-vst3-factory-census-v6"
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
  "attestation": {
    "requested": true,
    "subject": "wf0-payload.zip",
    "subject_sha256": "<same-outer-hash>",
    "identity": "<bounded-verification-identity>",
    "mac_verification": "pending_until_mac_custody"
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

The Windows producer writes `mac_verification` as pending. The authenticated
Mac creates a distinct custody receipt that binds the byte-identical build
receipt and records attestation success; it never rewrites the producer's
receipt.

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
happen to match. This avoids recursive identity without losing provenance.

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

Attestation is associated with `wf0-payload.zip` and is verified by the Mac.
The GitHub artifact container name is transport metadata only. The canonical
extracted manifest is artifact identity; the ZIP hash remains a mandatory
transport and provenance binding. Deterministic ZIP bytes are preferred and
recorded, but the accepted equality claim is made over the A/B extracted file
manifest and transfer files; it never treats GitHub's wrapping archive as
stable identity.

The published Deck cache root contains the extracted payload plus the two
manifest metadata files, the exact full build-receipt sidecars, a bounded Mac
provenance-verification receipt, and one owner marker that hashes those
sidecars. The payload manifest digest still names the root because its hashed
build-identity core differentiates runs. The importer validates this complete
cache-root roster separately; execution copies only manifest-declared runtime
payload files.

### Evidence handoff schema

The Deck-to-Mac private handoff uses:

```text
linux-vst-bridge-wf0-evidence-handoff/v1
```

It binds the implementation commit/tree/source-manifest digest, workflow
run/attempt/build-receipt digest, artifact-set and payload-archive digests,
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
MSVC 2022 as supported for x86_64 Windows. V6 chooses the Visual Studio
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
`codex/wf0-windows-vst3-factory-census-v6` touching one of the exact 26 source
paths. It checks out only `github.sha`, verifies the exact expected branch/ref,
and rejects merge commits or a source commit with the wrong authority parent
shape. `pull_request_target` is prohibited. No `workflow_run`, issue comment,
schedule, tag, wildcard branch, or untrusted requested-ref execution exists.

Every `uses:` value is a full 40-character commit SHA verified as belonging to
the named action repository. Tags and moving branches are comments only, never
the executable ref. Checkout uses `persist-credentials: false`. No repository,
organization, environment, or inherited secret is exposed to build steps.

Permissions are job-scoped:

```yaml
build:
  contents: read
attest_and_publish:
  contents: read
  id-token: write
  attestations: write
  artifact-metadata: write
```

`artifact-metadata: write` is limited to recording the official artifact
attestation. No packages, actions, deployments, pull requests, issues, checks,
or contents write permission is allowed. Build steps receive no secret. The
attestation job downloads only the exact prior-job artifact and executes no
produced PE.

GitHub documents full-SHA action pins as the immutable action reference,
requires `id-token: write`, `contents: read`, and `attestations: write` for
binary provenance, and warns against executing untrusted code with
`pull_request_target`:

- [Secure use reference](https://docs.github.com/en/actions/reference/security/secure-use)
- [Secure use of pull_request_target](https://docs.github.com/en/actions/reference/security/securely-using-pull_request_target)
- [Artifact attestations](https://docs.github.com/en/actions/how-tos/secure-your-work/use-artifact-attestations/use-artifact-attestations)

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
imports are recorded within V6's existing artifact role; otherwise the build
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

### Attestation posture

For this public repository, the workflow uses the current official
`actions/attest` action pinned by full SHA to attest the exact
`wf0-payload.zip` subject digest. The Mac verifies repository, signer workflow,
source digest/ref, SLSA provenance predicate, subject digest, and hosted-runner
posture. Failure or current unavailability is
`WF0_BUILD_PROVENANCE_BLOCKED`; it is not silently omitted. The Deck receives
the bounded Mac verification receipt and build receipt but need not authenticate
to GitHub or verify the attestation online.

## 13. Mac/Deck handoff and evidence-return contracts

### Exact Mac download and verification

The authenticated Mac first reads the run by exact numeric ID and verifies
repository, workflow path, event, branch, head SHA, run attempt, status, and
successful conclusion. It then uses the exact equivalent of:

```text
gh run download <RUN_ID> \
  --repo kasselvania/Linux-VST-bridge \
  --name <EXACT_RECEIPT_BOUND_ARTIFACT_NAME> \
  --dir <NEW_EMPTY_PRIVATE_STAGE>
```

An omitted run ID or an interactive/latest selection is prohibited. GitHub's
CLI documentation explicitly distinguishes exact run-ID download from latest
artifact selection: [gh run download](https://cli.github.com/manual/gh_run_download).

The Mac rejects any extra/missing envelope file, verifies receipt/hash
canonicalization and source/workflow/run identities, verifies the payload
outer SHA, safely inventories the ZIP against the canonical manifest, and runs
attestation verification with exact repository, signer workflow, source digest
and ref, and `--deny-self-hosted-runners`. It stores only a bounded verification
receipt; tokens and raw credential configuration never enter the handoff.

### Mac-to-Deck transfer

The Mac transfers the exact three-file envelope through the already-established
ordinary local SSH route with agent forwarding disabled. It does not copy a
GitHub key, token, cookie, environment, credential helper, or SSH agent socket.
Both ends hash every transfer file; equality is required before import. A
partial or existing unknown remote stage blocks and is not overwritten.

### Safe Deck extraction and import

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
has no default.

### Deck-to-Mac evidence return

The Deck renders the unchanged 14-file canonical evidence packet into a new
private stage, validates sanitization and hashes, then creates an exact
evidence-handoff manifest. The Mac pulls that handoff through SSH into a new
empty stage and requires transfer byte equality, schema/roster/hash closure,
source/build/artifact identity equality, and safe regular files.

Only the Mac copies the exact 14 files into the canonical implementation
worktree and creates the evidence-only commit. The Deck never commits, pushes,
opens a PR, or needs GitHub access. Handoff material excludes credentials,
hostnames, network addresses, raw process identifiers, private absolute paths,
proprietary state, compatdata, and compiled binaries.

## 14. Security, privacy, licensing, and threat model

- Workflow source is executable supply-chain input and is bound to the exact
  source commit/blob. No secret is exposed to executable implementation code.
- Full-SHA actions, least permissions, fixed trigger/ref, detached exact
  checkout, and no `pull_request_target` limit workflow authority.
- Hosted-runner labels are mutable service selectors, never durable build
  identity. Observed image/tool facts and artifact hashes are retained.
- SDK and VSTGUI are untrusted build inputs even when official. They execute no
  produced PE in the Windows plane and cannot write repository history.
- ZIP and evidence archives are untrusted until bounded safe-path extraction,
  exact manifest closure, and hash readback complete.
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
| `.github/workflows/wf0-windows-msvc-build.yml` | fixed source checkout, acquisition/build/attest orchestration and job permissions | runtime proof, release pipeline, secrets |
| `cmake/WF0DependencyLock.cmake` | cross-platform exact SDK/submodule/options assertions | network acquisition, SDK edits |
| `windows-factory-probe/` | unchanged bounded Win32 load/factory/census contract | class instantiation, process kill, evidence |
| `windows-fixtures/wf0/` | exact approved negative family | positive compatibility claims |
| `build.py` | WindowsBuildPlane receipt and two-root build coordination only | Deck execution or GitHub authentication |
| `verify.py` | independent PE/parser/manifest/A-B checks | process execution or data repair |
| `artifacts.py` | offline Mac verification, safe Deck import, evidence-handoff verification | network login, compiling, Git commit |
| `environment.py` | exact cache-to-stage copy and marker-bound retirement | artifact download, WR0 mutation |
| `supervise.py` | unchanged Runtime/Proton launch/gate/cleanup | factory semantics or global cleanup |
| `normalize.py` | unchanged census truth plus exact build/artifact binding | inventing or repairing output |
| `evidence.py` | fixed packet, handoff manifest, sanitizer and final source equality | primary fact generation or GitHub access |
| `run.py` | Deck-only dependency-ordered import/execution/evidence commands | Windows build, Mac Git, product manager |

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
the Windows/MSVC/image/workflow/provenance facts. No evidence path is added.

### V5-to-V6 substitution justification

| V5 path/action | V6 disposition | Reason |
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
V5 paths are replaced by three necessary V6 paths while the justified 14-file
evidence roster remains. The counts were derived from owner needs, not preserved
as a cosmetic target.

### Path-by-path provisional salvage classification

Every row remains untrusted until future exact MSVC build and Deck runtime
proof. “Without semantic change” permits reconstructing the exact blob from the
archive into a new implementation branch only after V6 authority; it is not
current adoption.

| # | Provisional path | Primary classification | V6 law |
|---:|---|---|---|
| 1 | `.gitignore` | reusable after V6 build-plane adaptation | replace Deck build roots with handoff/cache exclusions |
| 2 | `CMakeLists.txt` | reusable after V6 build-plane adaptation | retain HP0 separation; select supported MSVC WF0 shape |
| 3 | `cmake/WF0DependencyLock.cmake` | reusable after V6 build-plane adaptation | remove Unix-only Git path; keep exact root/submodule checks |
| 4 | `cmake/WF0Toolchain.cmake` | MinGW-specific and superseded; prohibited direct adoption | path is retired |
| 5 | `docs/WF0_WINDOWS_TOOLCHAIN_LOCK.md` | MinGW-specific and superseded | replace with build-plane lock, do not rename/copy claims |
| 6 | `tools/wf0-factory-census/README.md` | reusable after V6 build-plane adaptation | rewrite entry points and plane ownership |
| 7 | `tools/wf0-factory-census/build.py` | incomplete; reusable after V6 adaptation | delete Flatpak/MinGW/Deck semantics; Windows-only build receipts |
| 8 | `tools/wf0-factory-census/common.py` | incomplete; reusable after V6 adaptation | replace identities/roster/roots, preserve fail-closed primitives |
| 9 | `tools/wf0-factory-census/environment.py` | reusable after V6 adaptation | require explicit content-addressed cache identity |
| 10 | `tools/wf0-factory-census/evidence.py` | reusable after V6 adaptation | bind Windows receipt and evidence return; keep sanitizer/14 paths |
| 11 | `tools/wf0-factory-census/host-shims/attrib` | MinGW-specific and superseded; prohibited direct adoption | path is retired; no replacement shim |
| 12 | `tools/wf0-factory-census/negative_tests.py` | reusable without semantic change | exact fault expectations and tripwire remain; prove afresh |
| 13 | `tools/wf0-factory-census/normalize.py` | reusable after V6 adaptation | census semantics unchanged; replace toolchain identity fields |
| 14 | `tools/wf0-factory-census/run.py` | incomplete; reusable after V6 adaptation | remove `build` on Deck; add exact imported-artifact selection |
| 15 | `tools/wf0-factory-census/supervise.py` | reusable without semantic change | exact Runtime/Proton process contract remains; prove afresh |
| 16 | `tools/wf0-factory-census/verify.py` | incomplete; reusable after V6 adaptation | replace MinGW parser driver and add manifest/envelope checks |
| 17 | `windows-factory-probe/CMakeLists.txt` | reusable after V6 adaptation | express MSVC warnings/runtime; no scanner semantic change |
| 18 | `windows-factory-probe/include/linux_vst_bridge/wf0_probe/census.h` | reusable without semantic change | exact data boundary; prove MSVC ABI/build |
| 19 | `windows-factory-probe/include/linux_vst_bridge/wf0_probe/events.h` | reusable without semantic change | exact flush/event contract; prove MSVC build/runtime |
| 20 | `windows-factory-probe/source/factory_census.cpp` | reusable without semantic change | exact factory census; prove MSVC and Deck behavior |
| 21 | `windows-factory-probe/source/factory_census.h` | reusable without semantic change | exact internal contract; prove afresh |
| 22 | `windows-factory-probe/source/main.cpp` | reusable without semantic change | exact command/lifecycle/no-instantiation contract; prove afresh |
| 23 | `windows-factory-probe/source/win32_module.cpp` | reusable without semantic change | exact Win32 load/export/entry/unload semantics; prove afresh |
| 24 | `windows-factory-probe/source/win32_module.h` | reusable without semantic change | exact binding types; prove afresh |
| 25 | `windows-fixtures/wf0/CMakeLists.txt` | reusable after V6 adaptation | express MSVC flags/runtime; preserve exact target roster |
| 26 | `windows-fixtures/wf0/source/fault_fixture.cpp` | reusable without semantic change | exact fault behavior/tripwire; prove MSVC and Deck behavior |

No whole-commit merge, rebase, or cherry-pick is permitted. Future
implementation begins from the exact merged V6 authority, reconstructs each
approved blob/path after audit, creates the new V6 paths, and commits one clean
implementation-source identity. The archive is a source reference, not an
authority shortcut.

### Design and future mutation envelopes

This design commit may differ from basis
`67026ad7160a584cbf9cdb4bf0db7b8dbfa60136` at exactly:

```text
CURRENT_SLICE.md
docs/slices/WF0/WINDOWS_BUILD_PLANE_RECONCILIATION.md
docs/slices/WF0/IMPLEMENTATION_DESIGN_V6.md
```

No implementation or evidence path is changed now. A later V6 review may add
only `docs/slices/WF0/ADVERSARIAL_DESIGN_REVIEW_V6.md`; a later approval receipt
requires separate exact operator approval and may add only its own path.

Future implementation may change only the 26 source/configuration paths and,
in one later evidence-only commit, the 14 evidence paths. Generated artifacts,
Git bundles, SDK source, and private handoffs never enter Git.

Permitted future external mutations are limited to ephemeral GitHub-hosted
acquisition/build/publication state, new private Mac download/handoff stages,
the exact content-addressed Deck artifact cache, marker-bound disposable WF0
environments, owned runtime transients, and private bounded evidence handoffs.
The accepted WR0 environment and all other protected state remain read-only.

## 16. Amended proof matrix

Exactly 53 rows exist: 25 supported build/handoff proofs replace V5's seven
MinGW-local build rows, and 28 applicable execution/evidence rows are retained
from V5. One run may satisfy multiple rows only when its receipts map them
explicitly. A green workflow is not a Deck proof.

| # | Claim | Positive proof | Negative/refusal proof | Retained evidence and ceiling |
|---:|---|---|---|---|
| 1 | exact Windows runner selector and observed image | `windows-2022`, X64, exact `ImageOS`/`ImageVersion`/OS receipt | wrong label, self-hosted, missing image fact blocks | `TOOLCHAIN.md`; label is not immutable identity |
| 2 | supported VS/MSVC | VS 2022 Enterprise, `v143`, x64 developer environment, full `cl /Bv` receipt | alternate VS/toolset/host/target or incomplete identity blocks | build receipt; no general compiler claim |
| 3 | exact Windows SDK | installed and selected `10.0.19041.0` | default/different/missing SDK blocks | build receipt; this build only |
| 4 | exact source/tree/workflow | detached head, tree, 26-record manifest, workflow path/blob/commit equal | wrong ref/event/SHA/blob, dirty or extra source blocks | `BASIS.md`, `BUILD_MANIFEST.json`; source identity only |
| 5 | exact pinned SDK recursively | root/tree plus seven exact clean submodules | wrong/dirty/missing/extra gitlink/submodule blocks | `BUILD.md`; no generated binary claim |
| 6 | no SDK/VSTGUI patch | zero diff/untracked and five source blobs exact before both configures | injected edit/copy/shadow target blocks | `BUILD.md`; source custody only |
| 7 | two clean MSVC builds | distinct empty roots and byte-identical transfer rosters | root reuse or unexplained byte difference blocks | build manifest; no hosted-image reproducibility claim |
| 8 | scanner/fault PE identity | all exact files parse PE32+ AMD64 and match cross-check receipts | wrong machine/header/roster blocks | `BUILD_MANIFEST.json`; no runtime claim |
| 9 | exact AGain bundle | exact pinned target, resource/module roster and manifest | missing/extra/different bundle object blocks | build manifest; no factory-call claim |
| 10 | export/import/dependency closure | required/forbidden exports and closed recursive imports match | unknown import/export/runtime source blocks | build receipt; no product packaging claim |
| 11 | canonical artifact manifest | canonical bytes, sorted exact file roster and aggregate digest | duplicate/extra/missing/unsafe/noncanonical record blocks | `BUILD_MANIFEST.json`; extracted identity only |
| 12 | payload archive SHA-256 | outer ZIP hash equals build receipt and Mac/Deck readings | any byte mismatch blocks before extraction | build/custody receipts; GitHub wrapper not identity |
| 13 | provenance/attestation | exact payload subject attested; Mac verifies repo/signer/source/ref/hosted runner | absent, unverifiable, wrong signer/source/subject blocks | `BUILD.md`; not code signing |
| 14 | exact Mac download | exact run ID/attempt/head/path/artifact name and bounded roster | latest/interactive/wrong run or extra file refused | custody receipt; Mac auth not transferred |
| 15 | exact Mac-to-Deck equality | per-file local/remote hashes equal | partial or changed transfer blocks | handoff receipt; transport only |
| 16 | safe Deck import | bounded archive census, exclusive extraction, manifest closure, atomic publish | traversal/link/collision/overflow/unsafe type blocks | import receipt; no execution claim |
| 17 | one exact artifact-set selection | full manifest digest required and reverified before copy | omitted, stale, altered, or `latest` selection refused | environment receipt; no ambient cache trust |
| 18 | no Deck GitHub dependency | execution completes with only local cache and SSH handoff | any required `gh`, API, token, key, or workflow download on Deck blocks | custody/process receipt; maintenance identity out of scope |
| 19 | execution binds Windows receipt | environment/session/census carry exact source, workflow run/attempt, build receipt and artifact digests | any cross-object mismatch blocks before gate/evidence | environment/census; no build inference from path |
| 20 | Deck-to-Mac evidence equality | packet and handoff manifest hashes equal on both ends | partial, changed, unsafe, extra/missing file blocks | evidence custody receipt; no raw state |
| 21 | Mac commits only exact sanitized packet | diff contains exactly 14 validated evidence paths | binary/private token/path or another tracked path blocks | final Git diff/hashes; publication only |
| 22 | source/workflow change invalidates proof | final source manifest equals build/live manifest | any mode/blob/workflow change forces rebuild and rerun | `BUILD_MANIFEST.json`; no stale inheritance |
| 23 | wrong/stale artifact refused | exact source/run/manifest/archive closure accepted | old run, wrong receipt, wrong digest, altered cache blocks | negative import receipt |
| 24 | wrong workflow run/source refused | run metadata and local Git objects equal | wrong head/ref/event/path/attempt blocks | Mac custody receipt |
| 25 | missing/extra artifact refused | exact canonical roster accepted in Windows, Mac, Deck stages | delete/add/rename/case-collide entry blocks | bundle/import negatives |
| 26 | missing fixture refused | exact declared fixture path accepted | absent/renamed module in disposable stage blocks | `NEGATIVE_TESTS.md`; no module open |
| 27 | wrong PE architecture refused | x64 module admitted | repository-owned wrong-machine header fixture refused before launch | negative receipt; admission only |
| 28 | wrong scanner/module hash refused | exact hashes pass import/copy/spawn | one-byte mutation blocks | negative receipt; no semantic claim |
| 29 | DLL search authority exact | System32-only default, absolute module, null `hFile`, DLL-load-dir + System32 flags | relative/default-widening/non-null/application/PATH search rejected | timeline/adapter; dependency search only |
| 30 | first launch owns initialization | payload-only prestate and exact Runtime/Proton/scanner readiness | separate bootstrap command or prefix mutation refused | environment/process evidence; disposable only |
| 31 | gate causally prevents load | scanner waits 15 seconds with no load attempt/map/factory event | early load/map/event/departure blocks and cleans | timeline/preservation; no post-gate claim |
| 32 | complete AGain factory census | exact lifecycle/calls, metadata and ordered three classes, release/exit/unload | expected-field/order/completion mismatch blocks | census/timeline; factory boundary only |
| 33 | required export ownership | AGain export found after earlier resolution/check | `wf0-missing-factory`; proves no `InitDll` invocation | negative evidence; no factory obtained |
| 34 | null factory ownership | AGain returns non-null | `wf0-null-factory` | negative evidence; no class query |
| 35 | absent optional entry is observation | AGain entry retained | `wf0-no-entry` completes factory path | negative evidence; absence not generalized |
| 36 | entry false is stage failure | AGain entry true | `wf0-init-false` with cleanup | negative evidence; no compatibility claim |
| 37 | factory-info failure owned | exact AGain factory metadata | `wf0-factory-info-false` | negative evidence; no classes |
| 38 | factory version/tier fallback exact | support booleans and selected tier retained | factory3/2 unsupported/error branches fall back or block exactly | census/negative; only factory 1/2/3 |
| 39 | invalid/excessive count refused | AGain count is three | negative and 257 fixtures | negative evidence; no allocation/enumeration |
| 40 | ordinal class failure owned | all three AGain ordinals succeed | fixed class-info failure | negative evidence; no partial success |
| 41 | duplicate class ID refused | three unique raw TUIDs | duplicate-ID fixture | negative evidence; no dedup repair |
| 42 | malformed/oversized output refused | bounded canonical JSONL accepted | truncation, duplicate final, invalid UTF, cap+1 rejected | negative evidence; no data recovery |
| 43 | scanner crash call-attributed | every ordinary return has completion | entry/factory/class/release crash leaves exact final in-flight call | timeline/negative; containment only |
| 44 | scanner timeout call-attributed | calls complete inside bounds | entry/factory/class/release hangs map from final attempt | timeline/negative; no real-time claim |
| 45 | reverse factory release attributed | one reverse-order completion per acquired interface | hang/crash/ledger mismatch maps release blocker without erasing primary | timeline/negative; cleanup only |
| 46 | absent optional exit is observation | AGain exit retained | no-exit variant unloads | negative evidence; absence not generalized |
| 47 | exit false owned | AGain exit true | `wf0-exit-false`, unload still attempted | negative evidence |
| 48 | unload result mapping | AGain `FreeLibrary` true | loader-adapter false/in-flight mapping | timeline/unit receipt; no inducible valid-handle failure claim |
| 49 | no class instantiation | source token/AST check, operation/schema absence, tripwire marker absent, final false | injected marker or invocation token blocks | negative/census; no class lifecycle |
| 50 | exact cleanup and unrelated survival | zero owned descendants after success/failure | held-gate/stage failure plus same-family sentinel | launch/process evidence; no global cleanup |
| 51 | environment ownership and retirement | exact marker accepted, each stage reaches durable absence | forged/missing marker preserved; failed retirement blocks | environment/negative; no adoption/recovery |
| 52 | WR0/Bitwig/protected equality | labeled before/after exact V5 identities and mutation-root audit | deliberate mismatch blocks packet and no repair | `PRESERVATION.md`; no Bitwig launch or behavior claim |
| 53 | evidence bounded/sanitized/hash-closed | fixed roster, sanitizer, handoff equality, hashes and claim ceiling | forbidden token/path/binary/extra file blocks | `SANITIZATION.md`, hashes; no broader compatibility |

Rows 32 through 49 retain V5's exact lifecycle/call completion and primary
failure precedence. Row 33 specifically proves required-export check before
entry and no entry call when absent. Rows 43 through 48 reject any timeline
that publishes a later lifecycle event before an ordinary call's paired
completion is flushed.

## 17. Failure and blocked-result taxonomy

Exactly 28 primary results exist. Secondary cleanup failures remain beside the
first primary result and never overwrite it.

| # | Result | Exact owner/stage | Preserved facts | Next lawful action |
|---:|---|---|---|---|
| 1 | `WF0_V6_PREFLIGHT_BLOCKED` | authority/fixture/custody preflight | read-only mismatch | stop; external authority resolution only |
| 2 | `WF0_WINDOWS_BUILD_PLANE_DESIGN_BLOCKED` | unsupported/unavailable selected build plane | official/source findings | return design gate |
| 3 | `WF0_REFERENCE_FIXTURE_DESIGN_BLOCKED` | exact AGain cannot remain selected | source/license findings | return design gate; no substitute |
| 4 | `WF0_WORKFLOW_SOURCE_BLOCKED` | source/branch/trigger/workflow identity | exact mismatch receipt | repair only inside approved source envelope |
| 5 | `WF0_DEPENDENCY_ACQUISITION_BLOCKED` | SDK root/submodule/remotes | bounded acquisition facts | reacquire exact dependency or return gate |
| 6 | `WF0_WINDOWS_BUILD_BLOCKED` | tool identity/offline/configure/compile/link | bounded logs and receipts | repair exact build inside envelope |
| 7 | `WF0_BUILD_COMPARISON_BLOCKED` | A/B identity or bytes differ | both manifests and differing safe paths | repair determinism; no normalization invention |
| 8 | `WF0_BUILD_PROVENANCE_BLOCKED` | attestation generation/identity | exact subject/source facts | repair supported attestation or return gate |
| 9 | `WF0_ARTIFACT_BUNDLE_BLOCKED` | PE/roster/import/export/manifest/archive | bounded valid build facts | repair packaging/verification only |
| 10 | `WF0_MAC_ARTIFACT_BLOCKED` | exact run download/provenance/custody | run and downloaded hashes | choose/verify exact run; no Deck fallback |
| 11 | `WF0_ARTIFACT_TRANSFER_BLOCKED` | Mac-to-Deck byte equality | local/remote hash prefix | retry fresh exact staging; no credential forwarding |
| 12 | `WF0_ARTIFACT_IMPORT_BLOCKED` | safe extraction/content-addressed publish | transfer/archive/manifest facts | preserve unknown root; retry new stage |
| 13 | `WF0_SCAN_ENVIRONMENT_BLOCKED` | create/copy/retire stage | marker/root facts | mutate only exact owned stage |
| 14 | `WF0_SCANNER_LAUNCH_BLOCKED` | spawn or pre-readiness failure | launch/last-stage receipt | scoped cleanup; repair launch contract |
| 15 | `WF0_PROCESS_IDENTITY_BLOCKED` | topology/gate/ancestry mismatch | sanitized session mismatch | signal only revalidated owned identities |
| 16 | `WF0_MODULE_OPEN_BLOCKED` | `LoadLibraryExW` | artifact IDs and error class | applicable exit/unload/drain |
| 17 | `WF0_MODULE_ENTRY_BLOCKED` | present entry false/in-flight | entry presence/attempt/result | applicable exit/unload/drain |
| 18 | `WF0_FACTORY_GET_BLOCKED` | missing export/null/in-flight call | exact export/factory prefix | release if acquired; exit/unload/drain |
| 19 | `WF0_FACTORY_INFO_BLOCKED` | metadata/query inconsistency | bounded factory prefix | release/exit/unload/drain |
| 20 | `WF0_CLASS_ENUMERATION_BLOCKED` | count/tier/ordinal/duplicate | valid prefix, never partial success | release/exit/unload/drain |
| 21 | `WF0_FACTORY_RELEASE_BLOCKED` | reverse-release in flight/ledger mismatch | earlier primary plus release prefix | continue reachable cleanup; never erase primary |
| 22 | `WF0_MODULE_EXIT_BLOCKED` | present exit false/in-flight | prior result plus exit prefix | still attempt unload/drain |
| 23 | `WF0_MODULE_UNLOAD_BLOCKED` | unload false/in-flight | prior stages plus unload prefix | process containment/drain |
| 24 | `WF0_OUTPUT_NORMALIZATION_BLOCKED` | malformed/oversized/incomplete output | raw hash and last valid stage/call | discard partial normalized output; drain |
| 25 | `WF0_PROCESS_CLEANUP_BLOCKED` | owned descendant survives | exact transient identity; sanitized role | preserve stage; no broad kill |
| 26 | `WF0_PROTECTED_FIXTURE_DRIFT` | any labeled protected mismatch | content-free mismatch | stop; never repair protected state |
| 27 | `WF0_EVIDENCE_BLOCKED` | Deck packet/return/Mac verification/final diff | bounded valid inputs and hashes | regenerate/return exact packet or rebuild/rerun if source-bound |
| 28 | `RETURN_TO_DESIGN_GATE` | material owner/envelope/claim discovery | bounded read-only finding | fresh design/review/approval |

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
- Mac cannot verify the exact run, receipt, payload hash, and attestation before
  Deck transfer;
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
5. successful public-repository provenance generation and exact Mac verifier
   output under the future run;
6. the final content-addressed artifact-set digest;
7. live scanner-specific Runtime/Proton topology and exact AGain factory data;
8. successful unload and all bounded fault results on the Deck; and
9. exact Deck-to-Mac evidence-handoff digest.

Every unknown has a proof row and blocker. None grants permission to implement
or change authority during this design phase.

## 19. Future implementation sequence

This dependency order is not current authorization:

```text
1. Obtain fresh independent V6 review, separate exact operator approval,
   approval receipt, merge, and exact `main` commit/tree readback.
2. Reverify protected Deck state, archive custody, Mac control-plane custody,
   and the current official availability of `windows-2022` without changing
   the selected label silently.
3. Create `codex/wf0-windows-vst3-factory-census-v6` from the exact merged V6
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
   identity, package, attest, and upload one exact artifact envelope.
8. On the Mac, select the exact run ID/attempt, download the exact artifact,
   verify source/workflow/toolchain/SDK/manifest/archive/provenance closure,
   and transfer exact bytes through SSH without credential forwarding.
9. On the Deck, safely import to the exact manifest-digest root, make it
   read-only, and reverify it. Do not build or contact GitHub.
10. Execute all bounded import negatives, deterministic adapters, live fault
    family, held-gate proof, and positive AGain census in fresh disposable
    environments through Runtime 4 / Proton 11.
11. Drain every owned process, retire every environment, and prove exact
    Bitwig/WR0/protected equality.
12. Render the exact 14-file sanitized packet and evidence-handoff manifest;
    return it by SSH; verify it on the Mac.
13. Add only the 14 evidence paths in a later evidence-only commit. Regenerate
    and require equality of the original 26-record source manifest.
14. Push from the Mac and leave the implementation PR unmerged for independent
    audit. The Deck never commits or pushes.
```

No whole provisional commit is merged, rebased, or cherry-picked. Useful work
is reconstructed; obsolete build ownership is not carried forward.

## 20. Adversarial design-review disposition

```yaml
review_path: docs/slices/WF0/ADVERSARIAL_DESIGN_REVIEW_V6.md
review_status: required_not_created
reviewed_revision: wf0-design-v6
reviewed_commit: external_identity_from_design_commit
reviewed_design_blob: external_identity_from_design_commit
reviewed_design_sha256: external_identity_from_design_commit
prior_v5_review_path: docs/slices/WF0/ADVERSARIAL_DESIGN_REVIEW_V5.md
prior_v5_review_blob: 8abbbdaf60fc00e00eb0a036fe9463ab4da35984
prior_v5_review_result: DESIGN_CLEAR
unresolved_findings: fresh_independent_v6_review_required
implementation_authorized: false
```

The design-authoring context does not review its own work. A fresh independent
reviewer must inspect the exact immutable V6 commit/tree/blob/SHA-256 and at
least determine whether:

1. the exact MinGW/VSTGUI failure and correct stop are fully supported by the
   pinned source objects;
2. the three planes have non-overlapping authority and the Deck has no ordinary
   GitHub dependency;
3. mutable runner label and observed build identity are kept distinct;
4. workflow trigger, permissions, secret posture, checkout, and action pins
   exclude `pull_request_target` and unbound code;
5. SDK acquisition/network closure/MSVC procedure preserve the exact fixture;
6. schemas close the source-to-workflow-to-build-to-archive-to-import chain
   without recursive or path-name-only identity;
7. safe extraction, content-addressed cache, and evidence return fail closed;
8. all 26 provisional paths have truthful salvage classifications;
9. the 26+14 path envelope, 53 proof rows, 28 results, 14 owners, and 22
   material operations agree across the card; and
10. the unchanged scanner/call lifecycle, protected-state law, and claim
    ceiling remain internally consistent.

No review or approval record is created by this commit.

## 21. Proposed design identity

The card cannot embed its own Git blob or SHA-256 without changing those
identities. They are computed externally from the final one-commit design and
must be copied verbatim into later review and approval records.

```yaml
card_path: docs/slices/WF0/IMPLEMENTATION_DESIGN_V6.md
card_git_blob: external_identity_from_design_commit
card_sha256: external_identity_from_design_commit
design_revision: wf0-design-v6
design_status: proposed_for_adversarial_review
design_basis_commit: 67026ad7160a584cbf9cdb4bf0db7b8dbfa60136
design_basis_tree: 2021dfcbf000d934a462d40efef6bbe7b54e0862
reconciliation_path: docs/slices/WF0/WINDOWS_BUILD_PLANE_RECONCILIATION.md
approval_receipt_path: docs/slices/WF0/DESIGN_APPROVAL_V6.md
approval_receipt_identity: null
implementation_authorized: false
```

A later approval must bind the exact V6 card blob/SHA-256, reconciliation,
unchanged primary claim, three-plane owner model, runner/toolchain/SDK policy,
workflow security, schemas, source salvage law, exact path roster, 53-row
matrix, 28-result taxonomy, stop law, nonclaims, and inherited scanner laws.
`CURRENT_SLICE.md` must separately transition authority only after merge and
exact `main` readback.

## 22. Implementation handoff

No implementation handoff or implementation prompt is produced in this phase.
After the required review, approval, merge, and readback, a separate
authority-owned handoff must bind the new basis, exact V6 external identities,
approval receipt, one primary claim, exact fixture, 26+14 path envelope,
Windows workflow/tool identities, artifact/evidence custody chain, proof matrix,
stop law, nonclaims, and all inherited scanner/call-order laws. It must not use
the full dossier as an implementation task contract or treat the preserved
provisional commit as accepted authority.

```text
design_status=proposed_for_adversarial_review
implementation_authorized=false
successor_selection_authorized=false
```
