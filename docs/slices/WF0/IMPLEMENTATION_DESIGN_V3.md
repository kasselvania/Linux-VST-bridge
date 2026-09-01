# Implementation Design Card — WF0 V3

## 1. Identity and authority

```yaml
slice: WF0
slice_id: WF0
title: Supervised Windows VST3 Factory Census Probe
design_revision: wf0-design-v3
design_status: proposed_for_adversarial_review
repository: kasselvania/Linux-VST-bridge
selection_basis_commit: 745ca63bdd8641ade85cb9a024c1dc842681192d
selection_basis_tree: 275521adff574f165bb2b3e883c2ec909bae4c66
amendment_basis_commit: 15523c69567d24b256cb3c65cb6f06bfa07854be
amendment_basis_tree: a6e2fc8c7a564f50e9033094fde847387478de62
selection_receipt_path: docs/slices/WF0/SLICE_SELECTION.md
selection_receipt_git_blob: 64721fbe3697f50f87cd16ae5fbf636f921143fc
fixture_reconciliation_path: docs/slices/WF0/BITWIG_6_1_FIXTURE_RECONCILIATION.md
current_slice_path: CURRENT_SLICE.md
current_slice_git_blob: external_identity_from_design_commit
authority_phase: reconnaissance_and_design
implementation_authorized: false
prepared_by: Codex design agent
prepared_at: 2026-09-01
```

The exact operator selection receipt says:

> I explicitly approve selecting WF0 — Supervised Windows VST3 Factory Census Probe and replacing the no-active-slice card with its bounded reconnaissance-and-design authority. This approval does not authorize implementation. Implementation requires a separate approved design revision.

V3 preserves the immutable approved V2 card and supersedes its current-fixture
binding only:

```yaml
superseded_design_revision: wf0-design-v2
superseded_design_commit: 4a04d5b52d1fa8e2d309ed1e2883ff96a7963ca6
superseded_design_tree: b14b216ad5964ec68a1cf32d33c4201670d3ffd7
superseded_design_blob: d618cbf6b397f10947d50fd4824cd3e06ef55726
superseded_design_sha256: f323e2b429c4f91c1821488d980b249901b8d82c3cceaa7bf9be9d874a455e24
superseded_review: GitHub 5082923871 / DESIGN_CLEAR
superseded_approval_blob: a84df2044b8a78b44b7c004b53b620d474994221
superseding_trigger: WF0_PROTECTED_FIXTURE_DRIFT
```

The V2 card in `docs/slices/WF0/IMPLEMENTATION_DESIGN.md`, its review, and its
approval receipt remain immutable historical authority. V3 does not edit or
reinterpret them. It incorporates the exact Bitwig `6.1` stable-fixture
reconciliation while preserving all other approved V2 laws.

V2 previously superseded the exact reviewed V1 card:

```yaml
superseded_design_commit: aa07e63368dddc2fdb51242326a0de127c6ffbee
superseded_design_tree: 8583a0a4846cae953af515aee350d8d3a7cb7c30
superseded_design_blob: acfb73f962fcd8423b936c67f8011913136dd701
superseded_design_sha256: f8e4d92602810ce78f0817ca55b06fc435d21befb933290511e26f996eac98b3
superseded_revision: wf0-design-v1
superseding_review: GitHub 5082639684 / DESIGN_REPAIR_REQUIRED
```

After a future independent review, this V3 card remains immutable. Review
disposition and approval belong to new separate records; neither is folded
into or used to mutate this card.

This card proposes a design amendment. It does not approve itself. The
operator's amendment authorization permits this draft and its independent
review only. No implementation, toolchain installation, build, Windows
execution, or scan-environment mutation is authorized until a fresh review and
exact V3 design-approval receipt are accepted and `CURRENT_SLICE.md` is changed
by that later authority.

### Design-gate triggers

```text
[x] owner boundary
[x] state machine/lifecycle
[x] durable mutation
[x] transaction/rollback/recovery
[x] process supervision/termination
[x] cross-process and cross-language record contract
[ ] real-time/deadline behavior
[x] thread affinity
[x] identity/authorization
[x] security/privacy
[x] licensing
[x] third-party runtime
[ ] persistent user content
[x] compatibility claim
```

No audio callback or real-time path exists in WF0. Deadlines below are process-supervision bounds, not real-time guarantees.

## 2. Primary claim and claim ceiling

### Primary claim

> On the exact accepted Steam Deck fixture, a repository-owned supervised Windows x86_64 factory probe, built against the pinned official VST3 SDK and executed through the accepted Runtime 4/Proton 11 lane in a disposable WF0-owned scan environment, loads the exact pinned AGain VST3 bundle, obtains its plug-in factory, retains deterministic factory metadata and the complete expected three-class census, unloads cleanly, and leaves the accepted WR0 environment and every protected fixture unchanged.

### End-to-end route

```text
exact user-scope Freedesktop SDK/MinGW lock + clean pinned VST3 SDK
    -> two clean offline Windows builds and independent PE/bundle verification
    -> WF0-owned disposable compatdata root containing copied verified artifacts
    -> accepted Runtime 4 / Proton 11 launch root
    -> nonce-bound scanner readiness before module load
    -> exact process/topology gate
    -> explicit Win32 module/factory lifecycle on one scanner thread
    -> bounded raw scanner records
    -> Linux-side strict normalization
    -> sanitized retained census/evidence
    -> owned process drain and exact environment retirement
    -> protected-state equality
```

### Claim ceiling

WF0 may prove only exact Windows x86_64 artifact builds, module opening, required `GetPluginFactory`, optional `InitDll`/`ExitDll` presence and boolean result, `IPluginFactory` retrieval, factory metadata, support for exact factory interface versions 2 and 3, ordered class metadata, reference release, module unload, and supervised cleanup for the declared AGain fixture.

WF0 never calls `createInstance`. It does not establish component/controller lifecycle, connection points, instantiated-interface census, host contexts, buses, parameters, MIDI/events, state, process setup, audio, editor/GUI behavior, a native Linux proxy, C ABI, IPC, shared memory, Rust service, Bitwig execution, Serum execution or compatibility, installation/authorization, a product runner, Steam-independent distribution, another plug-in, another format, general Windows VST3 support, or general Linux compatibility. Successful AGain factory enumeration must not be promoted to any later claim.

## 3. Exact fixture and accepted prerequisites

| Fixture/prerequisite | Exact identity | Accepted source/evidence | Mutation permitted? |
|---|---|---|---|
| Selection basis | commit `745ca63bdd8641ade85cb9a024c1dc842681192d`; tree `275521adff574f165bb2b3e883c2ec909bae4c66` | selection receipt and fetched `main` | no |
| Host | Steam Deck Galileo; SteamOS 3.8.16; `x86_64`; KDE Wayland | live WF0 reconnaissance | no system mutation |
| SteamOS posture | read-only enabled | live preflight | no |
| Linux harness interpreter | `/usr/bin/python3`, Python `3.13.5`, standard library only | live WF0 reconnaissance; reverify before use | read-only |
| Runtime/runner | Runtime 4 `4.0.20260805.254769`; pressure-vessel `0.20260805.0`; Proton `1787334450 proton-11.0-2-x86_64`; digest `2d64df1d36786ca2d0e955c553005423dc2b5bdd714bd0a17872622e33912547` | `docs/WR0_RUNNER_LOCK.md`, canonical WR0 evidence, live identity readback | read-only |
| WR0 contract | source digest `c6543004fbcd70252393f0e0cab4f9ad7a31e85f433ce3e903e690244cc79541` | canonical WR0 evidence | read-only |
| Accepted WR0 environment | identity `d3ed38d7ed53e9a5973cbf22fc504f2479dbdd15616fe1bfc1d5e1cd0bf5f4c4` | canonical WR0/WR0A evidence and live marker | prohibited |
| Historical Bitwig-bound proof layers | SR0, HP0, HP1, WR0, and WR0A tracked bytes at amendment basis `15523c69567d24b256cb3c65cb6f06bfa07854be`; original `hashes.sha256` packets exact | accepted historical evidence; HP1 remains a `6.0.11`-only claim | prohibited; no rewriting to `6.1` |
| Current protected Bitwig installation | system-scope `app/com.bitwig.BitwigStudio/x86_64/stable`; version `6.1`; commit `8a048e733e74dda8b897339436153a2d5df952f29d362dfcaca9d8e0d6f6c231`; origin `flathub`; no user shadow | bounded live fixture reconciliation | prohibited; never launched by WF0 |
| Current Bitwig runtime/configuration projection | runtime `org.freedesktop.Platform/x86_64/25.08` at `bd44a6230581917d04f89812a4c21090c304d390edb73995af1c2f9fd8abf4e8`; user override SHA-256 `1b4a6a6ed688f69dd3c36dcac8db008c5a41ed52170ea3e23dee984b0aae6a1e`; system override SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`; permission-output SHA-256 `c7f5a34dce104cc3d347dcaf89e135cc5ad73b891101d7587f78423858ad5c73` | bounded live fixture reconciliation | prohibited |
| Freedesktop SDK | `org.freedesktop.Sdk/x86_64/25.08`, version `freedesktop-sdk-25.08.16`, commit `b90ed309cc1d505dea48b6a2121c5dcfac22868120eee643b0596d31f96b9bb8` | live Flatpak readback | no; selected installed object is read-only |
| MinGW extension | `org.freedesktop.Sdk.Extension.mingw-w64/x86_64/25.08`, Flathub commit `f15a5a88eb09557f645860bfcb3c4a6bc267683fce06cb68436bd76376fca694`; absent at design time | Flathub remote metadata; source commit `345e5766c6cf8016cc62cb7755bd1711a7d739aa` | user-scope install only after separate approval |
| VST3 SDK | root `3cdf9ca5d1f5b1b21e0a86832aa4abe55607bd96`, tree `38343890fd1a0cedd48b7ec80ef17da15231b6c8`, recursive submodules in the identity ledger | HP0 dependency lock and clean live checkout | read-only |
| Positive fixture source | official AGain target; entry blob `13b920b4b7a74137301bf213cf048e96e82861d4`; CID blob `d32d1640ea187e718baba9cbb039d3a436d53cce`; target blob `f2616195f4f0b92b55ade45ac2fa448a4674ec79` | pinned `public.sdk` commit | source read-only; generated artifact only |
| Expected factory | vendor `Steinberg Media Technologies`; URL `http://www.steinberg.net`; email `mailto:info@steinberg.de`; flags `16`; three classes | pinned factory/version source | no source mutation |
| WF0 scan environment | initially absent `.wf0-factory-census.stage-<run-id>` below the declared environment root | new WF0 owner | only exact transaction root; disposable |

Immediately before implementation, build, and each live exercise, all identities above must be re-read. The implementation must also re-prove a clean repository/SDK, forbidden-process absence, no WR0 transaction sibling, exact Flatpak commits, exact runner files, scan-root absence, and a separate accepted record that grants implementation authority. This proposed card continues to state `implementation_authorized: false`. A drift is a blocker, not a repair invitation.

The current Bitwig installation is protected state, not a WF0 execution
fixture. WF0 does not launch Bitwig and V3 does not claim that HP0 or HP1 has
been reaccepted on `6.1`. The historical `6.0.11` evidence remains true for its
declared matrix. `ProtectedFixtureSnapshot` separately binds the current `6.1`
application/runtime/configuration projection before and after WF0 work.

WF0 must not call the historical WR0 `capture_fixture()` helper as current
Bitwig authority. That helper correctly remains bound to the accepted WR0
`6.0.11` evidence. The current snapshot is implemented only inside the existing
WF0 tooling paths and does not add or modify an HP0, HP1, WR0, WR0A, Bitwig, or
Flatpak path.

## 4. Reconnaissance findings

The original complete record is
[`docs/slices/WF0/RECONNAISSANCE.md`](RECONNAISSANCE.md). The bounded amendment
readback is
[`docs/slices/WF0/BITWIG_6_1_FIXTURE_RECONCILIATION.md`](BITWIG_6_1_FIXTURE_RECONCILIATION.md).

| Question | Observed answer | Evidence identity | Design consequence | Still unknown |
|---|---|---|---|---|
| Is the 25.08 build SDK installed? | yes, user-scope, exact commit | `b90ed309cc1d505dea48b6a2121c5dcfac22868120eee643b0596d31f96b9bb8` | build only inside this SDK | no |
| Is matching MinGW installed? | no | user/system Flatpak inventories | installation is a separately authorized operation | actual deployed tools/hashes |
| Is exact MinGW available? | yes, user-scope ref and remote commit | Flathub `f15a5a88eb09557f645860bfcb3c4a6bc267683fce06cb68436bd76376fca694`; source `345e5766c6cf8016cc62cb7755bd1711a7d739aa` | select exact 25.08 extension; no host package manager | actual compile/link behavior |
| Does the extension ship a CMake project toolchain? | no project-specific toolchain found | manifest blob `f9fa5d5261516151412ff294b978594172d971af` | repository owns full-path CMake toolchain | actual CMake/compiler version output |
| Is pinned SDK clean and complete? | yes, all seven submodules present/clean | root and submodule commits below | no source patch/copy allowed | future build result |
| Does SDK contain MinGW/Windows bundle handling? | yes | blobs `d1d0db26f8a4be1fe87146b50de5baf2881464ed`, `61ce003169b2ad1db892eb6578f5c670b1eec52a` | upstream target can be driven directly | current VSTGUI link closure |
| Should scanner use SDK `Win32Module`? | no | blob `7616bd8566141c8f63413869ebe46d2938b67b28` collapses stages and drops exit boolean | explicit Win32 load with official interfaces | runtime observation |
| Is validator part of WF0? | no | validator blob `391623664128d9d141abe3c14887c1359b70407a` instantiates classes | set validator off; do not run | no |
| Is AGain exact and distributable for local proof? | yes, pinned open SDK example | MIT SDK and pinned VSTGUI license | positive fixture remains AGain | binary imports and roster |
| Does AGain have three expected classes? | yes, source-derived in exact order | entry/CID/version blobs | normalized result must match all exact fields | live factory return |
| Can accepted WR0 env be reused? | no | authority and protection law | create separate disposable root | no |
| Can Runtime/Proton process law be reused? | yes, as read-only lock and conceptual implementation input | digest `2d64df1d36786ca2d0e955c553005423dc2b5bdd714bd0a17872622e33912547`, contract digest `c6543004fbcd70252393f0e0cab4f9ad7a31e85f433ce3e903e690244cc79541` | new WF0 supervisor, same locked launch root | exact scanner topology |
| What changed in the Bitwig protected fixture? | application version `6.0.11` -> `6.1` and app commit `7b66aed37386ffff99e40bbd486f90ceee7ac1d5c59508c7952094122cebf50e` -> `8a048e733e74dda8b897339436153a2d5df952f29d362dfcaca9d8e0d6f6c231`; current ref is stable | bounded V3 reconciliation | bind the current app identity separately from historical proof packets | no |
| Did Bitwig runtime/configuration posture also drift? | no; runtime commit, scope, user-shadow absence, and user/system override hashes remain exact | bounded V3 reconciliation | preserve those exact live identities before/after WF0 | future drift only |
| Does `6.1` become an HP1 acceptance fixture? | no | HP1 evidence remains exact for `6.0.11` | retain explicit historical/current separation | current `6.1` HP1 behavior remains unproved and out of WF0 scope |
| Does V3 require another owner, path, proof row, or blocker? | no | owner/path/proof audit | use existing `ProtectedFixtureSnapshot`, implementation paths, row 34, and `WF0_PROTECTED_FIXTURE_DRIFT` | no |

### Implementability verdict

```text
IMPLEMENTABLE
```

This verdict means the design has one coherent implementation route. It does not mean the build or runtime claim is already true. The post-install compiler identity, AGain link closure, artifact reproducibility, exact imports/exports, scanner topology, factory returns, and clean unload remain executable proof gates.

## 5. Owner map

Exactly ten owners exist. A script or function may implement more than one owner only if their state and outputs remain distinct. No manager, broker, transport, proxy, product registry, or general VST host is introduced.

| # | Owner | Inputs | Outputs and authoritative identities | Permitted mutation | Failures and cleanup responsibility | Explicitly does not own |
|---:|---|---|---|---|---|---|
| 1 | `WindowsToolchainLock` | installed SDK inventory, Flathub ref/commit, source manifest, full tool paths | lock schema binding SDK commit, extension commit, tool versions/targets/search dirs, runtime DLL hashes | separately approved exact user extension install; lock/receipt files | missing/wrong deployment -> toolchain blocker; never changes a wrong preexisting install; records reversible command | SDK source, build outputs, scan env, runner selection |
| 2 | `ReferenceFixtureBuild` | clean pinned SDK/submodules, exact toolchain lock, AGain source, implementation source manifest | two-build receipts; verified scanner/AGain/negative PE artifacts; sorted bundle/import/export manifests; artifact-set digest | two cache build roots and one verified artifact root | remove only owned build roots on prepublication failure; retain bounded receipt; classify SDK/reference/scanner build failures | live module loading, normalization, Flatpak installation authority |
| 3 | `WindowsFactoryProbe` | exact command contract, module path/hash, gate, official VST3 headers | bounded scanner events and raw factory census; exact module/factory/class stages | ready record and stdout/stderr inside owned session; module process memory only | release each acquired interface, call optional exit, attempt unload, emit exact exit code | process ancestry, environment deletion, evidence publication, class instantiation |
| 4 | `FactoryCensusNormalizer` | bounded raw scanner bytes, expected schema/identities, supervisor stage facts | canonical `linux-vst-bridge-wf0-factory-census/v1` object and validation receipt | transient normalized file then atomic evidence-stage file | reject malformed, oversized, duplicate, identity-mismatched, or incomplete output; delete invalid staged output | scanner truth invention, sorting classes, process cleanup |
| 5 | `WF0ScanEnvironment` | run ID, runner digest, artifact manifest, initial-absence proof | marker-bound transaction root, exact Windows payload paths, environment receipt, retirement proof | only `.wf0-factory-census.stage-<run-id>` and its contents | refuse unknown object; remove only exact marker-bound root after owned descendants are zero | WR0 environment, `.wine`, Steam compatdata, product environment |
| 6 | `WF0Supervisor` | `/usr/bin/python3` identity, runner/runtime lock, environment receipt, artifact hashes, exact command vector | Runtime-root topology observations, validated stage timeline, timeout/crash classification, zero-descendant proof | process spawn, gate publication, scoped TERM/KILL, transient handshake files | stage-aware timeout; revalidate identities before signals; prove unrelated sentinel survival | module API calls, factory data semantics, global process cleanup |
| 7 | `RunnerRuntimeLock` | accepted WR0 lock and live installed files | immutable selected Runtime/Proton identity and exact launch vector prefix | none | identity mismatch stops before launch; no repair/update | scan environment, workload args, product runner selection |
| 8 | `ScannerProcessSession` | run nonce, exact scanner/module/artifact IDs, process census | session identity, command binding, readiness/gate pairing, scanner exit and last validated stage | session/ready/gate artifacts; transient raw PID/start identities | stale/mismatched session refusal; session artifact cleanup after supervisor drains | module/factory result interpretation, other sessions, evidence authority |
| 9 | `CensusEvidencePacket` | normalized census, build/env/process/protection receipts, explicit nonclaims | fixed tracked evidence roster, `fixture.json`, sorted `hashes.sha256`, packet digest | evidence staging and exact tracked packet paths | refuse incomplete/unsanitized packet; atomic file writes and readback; no binary retention | primary runtime facts, source/build mutation, approval |
| 10 | `ProtectedFixtureSnapshot` | exact historical packet/helper Git identities plus current Bitwig `6.1` app/runtime/scope/shadow/override identities and other accepted allow-listed safe metadata before/after | separately labeled historical-evidence equality, current-application equality, other protected equality, and aggregate protected-state digest | none | any labeled drift -> `WF0_PROTECTED_FIXTURE_DRIFT`; never launches, downgrades, upgrades, repairs, or recursively inspects proprietary state | toolchain extension (intentional scope), Bitwig behavior, vendor contents, credentials |

When memory and physical state disagree, physical readback wins and the relevant owner blocks. No friendly name, basename, path suffix, branch label, PID alone, or in-memory boolean authorizes mutation.

## 6. State machine

### Toolchain lifecycle

| State | Authoritative facts | Entry proof | Exit operation |
|---|---|---|---|
| `toolchain_absent` | exact extension ref absent; selected SDK exact | both Flatpak installations queried | separately authorized install |
| `toolchain_available` | remote exact ref/commit and source identity available | fresh remote-info readback | install or stop |
| `toolchain_installed_unverified` | exact ref exists in user installation | install return plus deployed ref | full identity probes |
| `toolchain_exact` | SDK/extension commits and every tool/runtime identity match lock | independent readback receipt | build only |
| `toolchain_wrong` | any required identity differs | exact mismatch record | block; no repair of preexisting install |

### Build lifecycle

```text
build_absent
  -> build_configured_a -> build_complete_a -> build_verified_a
  -> build_configured_b -> build_complete_b -> build_verified_b
  -> build_comparison_validated
  -> artifact_set_published
```

Each configure uses an empty owned build root, exact Release options, no network, full compiler paths, fixed locale/time/build epoch, and the same clean source identities. `artifact_set_published` means a sorted path/mode/size/SHA-256 manifest was atomically written and read back; it does not mean binaries entered Git.

### Disposable-environment lifecycle

```text
environment_absent
  -> environment_stage_created
  -> environment_marker_durable
  -> environment_payload_ready
  -> first_supervised_launch_initializing
  -> scanner_launched
  -> scanner_readiness_announced
  -> scanner_identified
  -> scanner_gated
  -> scan_terminal
  -> process_draining
  -> process_clean
  -> evidence_inputs_captured
  -> environment_retired
```

Before `first_supervised_launch_initializing`, `WF0ScanEnvironment` may only create the exact marker-bound stage, create the declared compatdata/prefix payload directories, copy the verified scanner/module/runtime-DLL payload, and verify exact paths, modes, sizes, and hashes. It must not launch `wineboot`, `cmd.exe`, Proton, Wine, another bootstrap executable, or any separate Windows initialization workload. The first exact Runtime 4/Proton 11 scanner launch owns all required prefix initialization. Its 180-second spawn-to-readiness bound ends only at the fully validated `scanner_readiness_announced` record.

Stage, path, marker, directory, or payload-copy failure is `WF0_SCAN_ENVIRONMENT_BLOCKED`. Runtime/Proton prefix initialization or scanner-start failure before readiness is `WF0_SCANNER_LAUNCH_BLOCKED`. An unexpected existing stage never becomes an input. `environment_retired` requires marker-bound deletion, parent-directory durability, and exact root absence after zero owned descendants. On a cleanup failure, the root is retained for operator inspection and classified; it is never mistaken for a future empty environment.

### Scanner lifecycle and closed call-attempt events

The scanner retains exactly these 23 lifecycle names. Branch alternatives are mutually exclusive; not every name appears in a successful run.

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

V2 introduced two typed event kinds, `call_started` and `call_completed`, over one closed 15-operation enum; V3 preserves them unchanged. These are not free-form lifecycle names. The truthful structural counts are therefore 23 lifecycle names, two call-event kinds, and 15 allowed call operations. The overall scanner-event cap is 2,048 records so the 256-class bound and up to three exact information-tier attempts per ordinal remain representable inside the independent stdout byte cap.

Every scanner JSONL record carries a strictly increasing unsigned `sequence`, beginning at one with no duplicate or backward value. Immediately before every potentially blocking module/factory call, the scanner writes one complete canonical record and synchronously flushes the stdout pipe:

```json
{
  "event": "call_started",
  "sequence": 12,
  "operation": "get_class_info_unicode",
  "interface": "IPluginFactory3",
  "ordinal": 1,
  "tier": "factory_3_unicode"
}
```

The record must be fully visible in the pipe before control enters the call. A write or flush failure prevents the call and becomes `WF0_OUTPUT_NORMALIZATION_BLOCKED`. An ordinary success or ordinary error return receives exactly one subsequent `call_completed` record with a new record `sequence`, the original `attempt_sequence`, the same closed operation/interface/ordinal/tier tuple, a closed return kind, and bounded raw result fields. It never retains a pointer or free-form status. A missing completion means the attempt remained in flight.

The closed operation enum is:

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

`interface` is one of `IPluginFactory`, `IPluginFactory2`, `IPluginFactory3`, or `null`. `ordinal` and `tier` are non-null only for a class-info attempt; `ordinal` is the exact factory ordinal and `tier` is one of `factory_3_unicode`, `factory_2`, or `factory_1`. A sequence gap, unknown enum, mismatched attempt/completion tuple, completion without an attempt, second completion, or another attempt before the prior synchronous call completes is malformed output.

The success path retains the 23-state ordering above, selects one entry branch, `factory_export_found`, `factory_obtained`, one exit branch, and ends in `scanner_completed`. Each lifecycle result is causally backed by the applicable completed attempt. On timeout, exception, NTSTATUS exit, signal, malformed termination, or output closure, the supervisor uses the last fully validated `call_started` record without a completion; only when no attempt is in flight may it fall back to the last validated lifecycle state. Cleanup attempts may add a secondary failure but never replace an earlier primary failure.

### Stage ownership and blocker mapping

Every named scanner state has one owner and one primary blocked result. Optional absence is a successful observation; its mapped result applies only if the record is malformed or inconsistent.

| State | Owner | Primary blocked result |
|---|---|---|
| `scanner_started` | `ScannerProcessSession` | `WF0_SCANNER_LAUNCH_BLOCKED` |
| `readiness_announced` | `ScannerProcessSession` | `WF0_PROCESS_IDENTITY_BLOCKED` |
| `supervisor_gate_accepted` | `WF0Supervisor` | `WF0_PROCESS_IDENTITY_BLOCKED` |
| `module_open_started` | `WindowsFactoryProbe` | `WF0_MODULE_OPEN_BLOCKED` |
| `module_opened` | `WindowsFactoryProbe` | `WF0_MODULE_OPEN_BLOCKED` |
| `module_entry_absent` | `WindowsFactoryProbe` | `WF0_MODULE_ENTRY_BLOCKED` |
| `module_entry_succeeded` | `WindowsFactoryProbe` | `WF0_MODULE_ENTRY_BLOCKED` |
| `module_entry_failed` | `WindowsFactoryProbe` | `WF0_MODULE_ENTRY_BLOCKED` |
| `factory_export_missing` | `WindowsFactoryProbe` | `WF0_FACTORY_GET_BLOCKED` |
| `factory_export_found` | `WindowsFactoryProbe` | `WF0_FACTORY_GET_BLOCKED` |
| `factory_get_started` | `WindowsFactoryProbe` | `WF0_FACTORY_GET_BLOCKED` |
| `factory_obtained` | `WindowsFactoryProbe` | `WF0_FACTORY_GET_BLOCKED` |
| `factory_get_failed` | `WindowsFactoryProbe` | `WF0_FACTORY_GET_BLOCKED` |
| `factory_info_obtained` | `WindowsFactoryProbe` | `WF0_FACTORY_INFO_BLOCKED` |
| `factory_interface_versions_recorded` | `WindowsFactoryProbe` | `WF0_FACTORY_INFO_BLOCKED` |
| `class_count_obtained` | `WindowsFactoryProbe` | `WF0_CLASS_ENUMERATION_BLOCKED` |
| `class_enumeration_in_progress` | `WindowsFactoryProbe` | `WF0_CLASS_ENUMERATION_BLOCKED` |
| `class_enumeration_complete` | `WindowsFactoryProbe` | `WF0_CLASS_ENUMERATION_BLOCKED` |
| `module_exit_absent` | `WindowsFactoryProbe` | `WF0_MODULE_EXIT_BLOCKED` |
| `module_exit_succeeded` | `WindowsFactoryProbe` | `WF0_MODULE_EXIT_BLOCKED` |
| `module_exit_failed` | `WindowsFactoryProbe` | `WF0_MODULE_EXIT_BLOCKED` |
| `module_unloaded` | `WindowsFactoryProbe` | `WF0_MODULE_UNLOAD_BLOCKED` |
| `scanner_completed` | `ScannerProcessSession` | `WF0_OUTPUT_NORMALIZATION_BLOCKED` |

When a call attempt is in flight, its exact operation is the authoritative crash/timeout owner:

| Closed operation | Required interface / binding | In-flight blocker |
|---|---|---|
| `load_library` | `null`; exact absolute module path already validated | `WF0_MODULE_OPEN_BLOCKED` |
| `init_dll` | `null` | `WF0_MODULE_ENTRY_BLOCKED` |
| `get_plugin_factory` | `null` | `WF0_FACTORY_GET_BLOCKED` |
| `get_factory_info` | `IPluginFactory` | `WF0_FACTORY_INFO_BLOCKED` |
| `query_factory_2` | `IPluginFactory` | `WF0_FACTORY_INFO_BLOCKED` |
| `query_factory_3` | `IPluginFactory` | `WF0_FACTORY_INFO_BLOCKED` |
| `count_classes` | `IPluginFactory` | `WF0_CLASS_ENUMERATION_BLOCKED` |
| `get_class_info_unicode` | `IPluginFactory3`; exact ordinal and `factory_3_unicode` tier | `WF0_CLASS_ENUMERATION_BLOCKED` |
| `get_class_info_2` | `IPluginFactory2`; exact ordinal and `factory_2` tier | `WF0_CLASS_ENUMERATION_BLOCKED` |
| `get_class_info_1` | `IPluginFactory`; exact ordinal and `factory_1` tier | `WF0_CLASS_ENUMERATION_BLOCKED` |
| `release_factory_3` | `IPluginFactory3` | `WF0_FACTORY_RELEASE_BLOCKED` |
| `release_factory_2` | `IPluginFactory2` | `WF0_FACTORY_RELEASE_BLOCKED` |
| `release_factory_base` | `IPluginFactory` | `WF0_FACTORY_RELEASE_BLOCKED` |
| `exit_dll` | `null` | `WF0_MODULE_EXIT_BLOCKED` |
| `free_library` | `null` | `WF0_MODULE_UNLOAD_BLOCKED` |

Every operation above has exactly one in-flight blocker. A returned interface `release` retains its bounded unsigned reference-count result, but a nonzero advisory count alone is not failure because other acquired references may still exist. Missing completion, crash, timeout, abnormal termination, or an internal acquisition/release-ledger mismatch is `WF0_FACTORY_RELEASE_BLOCKED`. Each successful acquisition still requires exactly one corresponding release attempt in reverse acquisition order.

### Windows module/factory transition law

1. The scanner validates its command contract, atomically writes the ready record, emits `readiness_announced`, and waits. It cannot resolve or load AGain before the exact gate is read.
2. On the scanner's single main thread, require nonzero success from exactly:

   ```cpp
   BOOL defaults_ok =
       SetDefaultDllDirectories(LOAD_LIBRARY_SEARCH_SYSTEM32);
   ```

   `LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR` is prohibited here. On failure, retain only the operation name `set_default_dll_directories`, bounded unsigned `GetLastError`, and the exact scanner/module identities, then return `WF0_MODULE_OPEN_BLOCKED` without attempting module load.
3. Require a fully qualified canonical Windows module path and `hFile == nullptr`, emit and flush `call_started(load_library)`, then call exactly:

   ```cpp
   HMODULE module = LoadLibraryExW(
       exact_absolute_module_path,
       nullptr,
       LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR |
           LOAD_LIBRARY_SEARCH_SYSTEM32);
   ```

   Check the return. On null, complete the attempt with bounded unsigned `GetLastError`. `LOAD_WITH_ALTERED_SEARCH_PATH`, application-directory search, user-directory search, current-directory search, and ambient `PATH` search are prohibited. Only the copied module directory and System32 have dependency-search authority.
4. Resolve optional `InitDll`, optional `ExitDll`, and required `GetPluginFactory` with exact case. Record presence separately; export lookup does not call module code.
5. If `InitDll` exists, emit and flush `call_started(init_dll)`, call it once, and retain the exact completion. `false` is `module_entry_failed`; do not call the factory. Absence is `module_entry_absent` and does not block.
6. If entry is absent or succeeds, require `GetPluginFactory`, emit and flush `call_started(get_plugin_factory)`, call it once, and require a non-null `IPluginFactory*`.
7. Zero-initialize all output structures. Emit exact attempts for `get_factory_info`, `query_factory_2`, and `query_factory_3`. Query only exact `IPluginFactory2` and `IPluginFactory3` IIDs. A query is supported only for `kResultOk` plus non-null. `kNoInterface` plus null is unsupported. Every inconsistent status/pointer pair blocks.
8. Emit `call_started(count_classes)` and require `0 <= countClasses <= 256`. Enumerate exact ordinals without sorting. Before each tier call, emit the exact ordinal and tier: `get_class_info_unicode`, then on non-success `get_class_info_2`, then on non-success `get_class_info_1`. Retain every completion sequence. If no tier succeeds, block at that ordinal.
9. Never call `createInstance` or any returned-class interface. The command contract, schema, and closed operation enum contain no class-instantiation operation.
10. After all factory data is copied, emit and complete attempts for each acquired interface release in reverse acquisition order: `release_factory_3`, `release_factory_2`, then `release_factory_base`. Alias pointer values do not suppress releases because each successful query owns one reference. A release cleanup failure is secondary to any earlier primary entry/factory/class failure.
11. If `ExitDll` exists, emit and flush `call_started(exit_dll)` and call it exactly once for every successfully loaded module before unload, even on an earlier entry/factory failure, matching the official helper's cleanup posture. Record `false` as a cleanup failure without suppressing the original primary failure. Absence is an observation.
12. Only after all reachable release attempts and exit handling, emit and flush `call_started(free_library)` and call `FreeLibrary`. A false result is `WF0_MODULE_UNLOAD_BLOCKED`; scanner process exit remains the supervisor's final containment.

### Scanner exit codes

| Exit | Meaning | Blocked result |
|---:|---|---|
| `0` | complete success | none |
| `64` | invalid usage/identity/command contract | `WF0_SCANNER_LAUNCH_BLOCKED` |
| `65` | readiness or gate contract failure | `WF0_PROCESS_IDENTITY_BLOCKED` |
| `70` | module open failure | `WF0_MODULE_OPEN_BLOCKED` |
| `71` | present `InitDll` returned false | `WF0_MODULE_ENTRY_BLOCKED` |
| `72` | required export absent | `WF0_FACTORY_GET_BLOCKED` |
| `73` | factory call failed or returned null | `WF0_FACTORY_GET_BLOCKED` |
| `74` | factory info/interface result invalid | `WF0_FACTORY_INFO_BLOCKED` |
| `75` | class count invalid/excessive | `WF0_CLASS_ENUMERATION_BLOCKED` |
| `76` | class info failed at an ordinal | `WF0_CLASS_ENUMERATION_BLOCKED` |
| `77` | duplicate class ID | `WF0_CLASS_ENUMERATION_BLOCKED` |
| `78` | malformed SDK string or output bound | `WF0_OUTPUT_NORMALIZATION_BLOCKED` |
| `79` | present `ExitDll` returned false | `WF0_MODULE_EXIT_BLOCKED` |
| `80` | `FreeLibrary` failed | `WF0_MODULE_UNLOAD_BLOCKED` |
| `81` | acquisition/release ledger invariant failed | `WF0_FACTORY_RELEASE_BLOCKED` |
| `82` | scanner invariant/internal failure | in-flight attempt's blocker, otherwise last validated lifecycle state's blocker, otherwise `WF0_SCANNER_LAUNCH_BLOCKED` |

Windows exception/NTSTATUS termination is never coerced to this table. The supervisor records an unsigned raw exit classification transiently and emits the blocker for the last fully validated attempt without a completion; if none is in flight, it uses the last validated lifecycle state's blocker or `WF0_SCANNER_LAUNCH_BLOCKED` before any scanner record.

### Legal transitions and failure ownership

| From | Operation | To | Required readback | Failure classification |
|---|---|---|---|---|
| `toolchain_available` | exact user install | `toolchain_installed_unverified` | deployed exact ref | `WF0_TOOLCHAIN_INSTALL_BLOCKED` |
| `toolchain_installed_unverified` | identity probes | `toolchain_exact` | commit/tool/target/runtime receipt | `WF0_TOOLCHAIN_INSTALL_BLOCKED` |
| `toolchain_exact` | configure/build twice | `build_comparison_validated` | two independent manifests | SDK/reference/scanner build blocker |
| `build_comparison_validated` | publish artifact manifest | `artifact_set_published` | atomic receipt readback | `WF0_EVIDENCE_BLOCKED` |
| `environment_absent` | create stage and marker | `environment_marker_durable` | marker and parent readback | `WF0_SCAN_ENVIRONMENT_BLOCKED` |
| `environment_marker_durable` | create declared payload directories and copy/verify artifacts only | `environment_payload_ready` | Windows paths, modes, sizes, and hashes | `WF0_SCAN_ENVIRONMENT_BLOCKED` |
| `environment_payload_ready` | spawn the exact Runtime/Proton/scanner launch that owns first-prefix initialization | `first_supervised_launch_initializing` | Runtime root PID/start and exact launch vector transiently | `WF0_SCANNER_LAUNCH_BLOCKED` |
| `first_supervised_launch_initializing` | observe exact scanner process under the launch root | `scanner_launched` | scanner PID/start, vector, and ancestry transiently | `WF0_SCANNER_LAUNCH_BLOCKED` |
| `scanner_launched` | receive complete ready file/event within 180 seconds | `scanner_readiness_announced` | exact ready bytes and event | absent/timeout/termination -> `WF0_SCANNER_LAUNCH_BLOCKED`; malformed/mismatched -> `WF0_PROCESS_IDENTITY_BLOCKED` |
| `scanner_readiness_announced` | validate ready/topology, payload hashes, prefix confinement, and WR0 preservation | `scanner_identified` | fresh complete descendant census and identity receipt | `WF0_PROCESS_IDENTITY_BLOCKED` |
| `scanner_identified` | atomic gate publication | `scanner_gated` | exact gate readback | `WF0_PROCESS_IDENTITY_BLOCKED` |
| `scanner_gated` | module/factory lifecycle | `scan_terminal` | bounded stages and exit | stage-specific blocker |
| `scan_terminal` | drain/cleanup | `process_clean` | zero owned descendants | `WF0_PROCESS_CLEANUP_BLOCKED` |
| `process_clean` | normalize/capture | `evidence_inputs_captured` | schema/identity readback | `WF0_OUTPUT_NORMALIZATION_BLOCKED` |
| `evidence_inputs_captured` | delete exact stage | `environment_retired` | absence and parent durability | `WF0_SCAN_ENVIRONMENT_BLOCKED` |
| `environment_retired` | publish packet | retained evidence | packet hashes and Git validation | `WF0_EVIDENCE_BLOCKED` |

### Forbidden transitions

- wrong/dirty SDK or toolchain -> build;
- dirty, missing, extra, or manifest-different governed implementation source -> build or live evidence reuse;
- unverified artifact -> scan environment;
- unknown/preexisting environment -> adopt/delete;
- payload-ready environment -> separate `wineboot`, `cmd.exe`, Proton, Wine, or bootstrap workload;
- readiness absent or topology stale -> gate;
- gate absent -> `module_open_started`, `call_started(load_library)`, or module load;
- entry failure -> factory call;
- missing/null factory -> class count;
- invalid count -> enumeration;
- class metadata -> `createInstance`;
- live factory references -> `ExitDll` or unload;
- live owned descendant -> environment deletion;
- failed normalization/protection equality -> evidence success;
- any WF0 state -> WR0 mutation, Bitwig, Serum, `.wine`, or Steam compatdata.

### Restart/retry reconciliation

Build roots are disposable and may be removed/recreated only after their safe-relative path and implementation-source-manifest identity are verified. A new supervisor scans only marker-bound WF0 stage roots. If a first launch partially initializes the WF0 prefix, that state remains owned by the same scan transaction; retry requires zero owned descendants and either exact-root retirement/recreation or an explicit same-identity retry receipt. It never authorizes an ungated bootstrap run. If an owned process identity survives, the supervisor revalidates exact root ancestry before scoped termination; if identity is uncertain, it stops and preserves the root. A marker-bound root with no process may be retained for bounded read-only diagnosis, then removed. Unknown roots are never adopted. Evidence staging is regenerated from retained bounded receipts; a partially published tracked packet is invalid until the fixed roster and hashes read back. The persistent Flatpak extension is reconciled by deployed commit and pre-install snapshot, not by an in-memory “installed by us” flag.

## 7. Fallible-operation and mutation ledger

There are exactly 14 material operations. Ordinary temporary compiler files inside an already-owned build root are covered by their enclosing build operation.

| # | Precondition | Fallible operation | Immediate physical result if it succeeds and next step fails | Durability/readback | Next authority state | Failure owner | Recovery |
|---:|---|---|---|---|---|---|---|
| 1 | extension absent, remote exact, later approval | install/deploy exact user Flatpak extension | user deployment exists, possibly unverified | `flatpak info --show-commit` and full tool probes | installed-unverified/exact | `WindowsToolchainLock` | never modify wrong preexisting ref; retain exact install with receipt or explicit later uninstall using prestate |
| 2 | toolchain exact, sources clean | create/configure clean build A | owned cache root and CMake cache exist | cache options and compiler paths parsed/read back | configured A | `ReferenceFixtureBuild` | remove only exact owned root; no source change |
| 3 | configured A | build/verify A targets | partial or complete untracked PE/bundle outputs exist | target success plus independent PE/roster/import/export manifest | verified A | `ReferenceFixtureBuild` | remove A or retain bounded logs; block by target owner |
| 4 | verified A | independently create/build/verify B | second isolated output set exists | same independent checks as A | verified B | `ReferenceFixtureBuild` | remove B or retain bounded logs |
| 5 | A/B verified | compare and atomically publish artifact-set receipt | verified artifact root and temp/final receipt may exist | file fsync, rename, parent fsync, full manifest readback | artifact published | `ReferenceFixtureBuild` | refuse partial receipt; remove only marker-bound artifact root |
| 6 | scan root absent, artifacts exact | create stage root and ownership marker | empty owned stage with durable marker exists | marker file fsync, parent fsync, exact readback | marker durable | `WF0ScanEnvironment` | delete only if exact marker/run/root bind; otherwise preserve/block |
| 7 | marker durable | create declared compatdata/prefix payload directories; copy and verify payload only | partial directories or payload may exist; no Windows process has run | exact payload Windows paths/modes/sizes/hashes and environment receipt | environment payload ready | `WF0ScanEnvironment` | remove only exact marker-bound root after zero descendants; WR0 never touched |
| 8 | payload ready, no forbidden workloads | spawn the one exact Runtime/Proton/scanner launch that owns required first-prefix initialization | live exact-root descendants and partial prefix state may exist before ready | bounded `/proc` census, exact launch vector, scanner readiness, prefix confinement, payload rehash | first launch initializing, scanner launched, then readiness announced | `WF0Supervisor` | pre-readiness failure is `WF0_SCANNER_LAUNCH_BLOCKED`; scoped TERM/KILL after fresh identity readback; prove zero |
| 9 | exact ready/topology, gate absent | atomically publish gate | gate exists; scanner may immediately start loading module | temp write/fsync/rename/parent fsync/readback plus fresh topology | scanner gated | `ScannerProcessSession` | no rollback claim after gate; supervise to terminal/cleanup |
| 10 | gate accepted | checked DLL-search setup; open module; optional entry; factory/census | module memory and possibly factory references exist | monotonic pre-call/completion records plus bounded lifecycle/census records | scan terminal or exact in-flight stage failure | `WindowsFactoryProbe` | release acquired references where reachable; exit/unload attempt; process containment |
| 11 | module loaded, all data copied | emit attempts; release refs; optional exit; unload | module may be exited/unloaded or cleanup failure recorded | exact release/exit/unload attempts and completions before process exit | scan terminal | `WindowsFactoryProbe` | preserve primary plus `WF0_FACTORY_RELEASE_BLOCKED`/exit/unload cleanup failures; supervisor contains process |
| 12 | bounded raw output, process terminal | strict normalization | transient normalized file may exist | schema, limits, identities, expected AGain comparison | normalized | `FactoryCensusNormalizer` | delete invalid stage output; retain only bounded failure receipt |
| 13 | process terminal | drain owned processes, capture inputs, remove environment | zero processes and either retained or deleted exact stage root | empty-scope verifier; evidence input hashes; delete + parent durability + absence | environment retired | `WF0Supervisor` / `WF0ScanEnvironment` | survivors block deletion; deletion failure preserves exact root and blocks |
| 14 | normalized data, retirement, equality | publish evidence packet | some staged/final tracked evidence files may exist | fixed roster, per-file SHA-256, packet digest, sanitization/readback | evidence retained | `CensusEvidencePacket` | refuse incomplete packet; regenerate from bounded receipts; never claim success |

### Commit points

```text
Flatpak deployment:
  persistent user state after exact deployed-commit readback;
  it is not silently removed because other work may use it.

Artifact publication:
  authoritative only after atomic manifest readback;
  build directories alone have no authority.

First supervised launch:
  prefix initialization is transaction-owned mutation inside the exact scanner launch;
  readiness, not prefix-directory existence, is the first scanner admission fact.

Gate publication:
  `module_open_started` and `call_started(load_library)` may occur only after this point;
  rollback becomes supervised completion/termination, never “gate deletion means nothing ran.”

Evidence publication:
  claimable only after environment retirement, protected equality, fixed-roster hashes,
  and clean tracked packet validation.
```

## 8. Process, thread, and topology model

| Role | Created by | Parent/owner | Required process/thread | Lifetime | Exit/cleanup owner |
|---|---|---|---|---|---|
| Linux run driver | operator/CI | repository invocation | native Python | whole transaction | caller after packet result |
| WF0 supervisor | run driver | `WF0Supervisor` | native Python; new session/process group root | launch through drain | itself |
| Runtime 4 root/wrappers | supervisor | exact Runtime root identity | native Runtime processes | scanner session | supervisor |
| Proton script | Runtime root | exact runner lock | native script/process | scanner session | supervisor |
| Wine/wineserver/preloader | Proton | scanner session | exact runner files | scanner session/drain | supervisor |
| Windows scanner | Wine | `ScannerProcessSession` | one Windows process; factory calls on main thread | readiness through scanner exit | scanner then supervisor |
| AGain module | scanner | `WindowsFactoryProbe` | DLL mapped into scanner; all WF0 API calls on scanner main thread | module open through unload | scanner |

### Expected topology

```text
tools/wf0-factory-census/run.py
  -> tools/wf0-factory-census/supervise.py [new session/group]
      -> <RUNTIME_4>/_v2-entry-point --verb=run --
          -> <RUNTIME_4>/run / pressure-vessel-unruntime
              -> <PROTON_11>/proton runinprefix
                  -> exact runner Wine/wineserver/preloader descendants
                      -> C:\wf0\bin\wf0-factory-probe.exe
                          -> LoadLibraryExW(C:\wf0\fixture\again.vst3\Contents\x86_64-win\again.vst3)
```

The Steam client is not launched and no Steam game-launch ancestor is permitted. Neutral app ID `0` and the accepted WR0 empty-environment law are reused, but every XDG/compat/runtime variable points to the new WF0 stage. Ambient compatibility overrides, Proton logs/tuning, library injection, UMU, yabridge, and caller executable paths are absent.

### Exact scanner command contract

The `runinprefix` workload is one ordered vector, with no shell:

```text
C:\wf0\bin\wf0-factory-probe.exe
--session <32-lowercase-hex>
--scanner-sha256 <64-lowercase-hex>
--implementation-source-manifest-sha256 <64-lowercase-hex>
--module C:\wf0\fixture\again.vst3\Contents\x86_64-win\again.vst3
--module-sha256 <64-lowercase-hex>
--bundle-manifest-sha256 <64-lowercase-hex>
--ready C:\wf0\session\<session>.ready
--gate C:\wf0\session\<session>.gate
--max-classes 256
--stdout-cap 1048576
```

No alternate order, duplicate option, relative path, basename match, extra argument, or different module is accepted. No class-instantiation command or option exists.

### Readiness/gate contract

Ready and gate files are UTF-8, LF-only, at most 1,024 bytes, and contain schema, session, scanner hash, module hash, bundle-manifest hash, implementation-source-manifest hash, and exact run ordinal. They are atomically written. The scanner emits the same binding in a bounded `readiness_announced` event and waits without emitting `module_open_started` or `call_started(load_library)` and without loading the module. Immediately before the gate, the supervisor repeats its process census and revalidates the exact Runtime root, Proton role, Wine-hosted scanner vector, both complete ancestry chains, implementation-source and artifact identities, payload hashes, prefix confinement, ready bytes/stdout, accepted-WR0 preservation, and gate absence.

One dedicated real held-gate proof uses the exact production scanner and disposable environment. After exact readiness and process identity are observed, the supervisor deliberately withholds the gate for the declared 15-second bound and requires all of:

- no `module_open_started` lifecycle event and no `call_started` record whose operation is `load_library`;
- no exact staged AGain module device/inode/path mapping in the scanner process, using transient `/proc` map data and retaining only a sanitized `again_module_mapped=false` result;
- no factory or class event;
- the exact scanner remains alive in its bounded waiting state;
- accepted WR0 state remains unchanged.

The held-gate run then exercises the ordinary scoped cleanup law, proves zero owned descendants, captures its bounded receipt, and retires the exact environment. It does not publish a gate later in that run. Any module-open record, module mapping, factory/class event, scanner departure from waiting, protected drift, survivor, or retirement failure blocks; absence is causally observed rather than inferred from a successful later scan.

### Bounds and deadlines

| Bound | Exact value |
|---|---:|
| observed process identities | 256 |
| process poll interval | 0.05 seconds |
| first-prefix spawn to readiness | 180 seconds |
| reused-stage spawn to readiness | 90 seconds |
| supervisor-held gate | 15 seconds |
| each individual load/entry/factory/query/count/class-info/release/exit/unload call after its attempt record | 15 seconds |
| class enumeration | 30 seconds |
| total scanner time after gate | 120 seconds |
| ordinary process drain | 20 seconds |
| TERM/KILL empty-scope cleanup | 10 seconds |
| stdout | 1,048,576 bytes |
| stderr | 65,536 bytes |
| scanner event count | 2,048 |
| ordinary stage event | 4,096 bytes |
| final raw census record | 786,432 bytes |

The supervisor reads both streams concurrently. Cap overflow is a hard failure and triggers owned cleanup. No wait is unbounded.

### Reentrancy, callbacks, and thread affinity

WF0 makes synchronous calls from the scanner main thread into the module. It does not supply a host context, instantiate a class, receive plugin callbacks, create an editor, or start audio. The factory may execute arbitrary module code during these synchronous calls; therefore the supervisor deadline and process boundary are the containment. No factory pointer crosses a thread or process boundary.

### Cleanup and unrelated-process law

Observation follows the complete descendant tree of the exact Runtime root PID/start identity. Group/session membership is recorded separately and cannot substitute for ancestry. TERM targets the isolated group only after revalidation; KILL follows only after the deadline. Any observed identity outside the group is signaled only after exact PID/start/ancestry revalidation. Stale/reused PID detection blocks rather than killing by name. Success requires zero owned descendants and absent handshake artifacts. A live same-family sentinel outside the ownership tree must survive the cleanup proof.

## 9. Identity and authorization ledger

| Operation | Required identity | Freshness/readback | Refusal cases |
|---|---|---|---|
| install extension | separate implementation authority + exact ref/branch/remote commit + prestate | immediately before install and after deploy | wrong preexisting commit, remote commit drift, no approval |
| use compiler | SDK commit + extension commit + absolute tool path + version + target + search dirs | each build invocation | ambient compiler, path-only match, different target |
| read SDK | root commit/tree + all seven submodule commits + clean/no-untracked state | before each configure | dirty, missing, wrong commit, symlink substitution |
| accept implementation source | clean implementation commit + `linux-vst-bridge-wf0-implementation-source/v1` exact 26-record path/mode/blob manifest | before every configure/build/live run and regenerated at final evidence head | dirty source, wrong commit, missing/extra governed path, mode/blob/digest change |
| accept scanner/fixture artifacts | implementation-source manifest + toolchain receipt + sorted artifact path/mode/size/hash + PE architecture/import/export roster | before copy, after copy, before spawn | missing, changed, undeclared artifact |
| create/remove environment | canonical parent + exact 32-hex run ID + marker schema + marker hash + runner/artifact identities | before every mutation/deletion | unknown object, mismatched marker, live descendants |
| spawn scanner | environment identity + runtime/runner digest + exact ordered command + artifact/source hashes + session nonce | pre-spawn and readiness census; first launch owns prefix initialization | forbidden process, stale sibling, alternate vector, separate bootstrap workload |
| run Linux harness | `/usr/bin/python3` + exact version `3.13.5` + clean source-manifest identities; standard library only | preflight and process start | alternate interpreter, injected module path, unexpected dependency |
| publish gate | session nonce + ready bytes + scanner/module hashes + exact PID/start/ancestry + gate absence | one fresh census immediately before rename | disappeared/changed identity, stale ready, existing gate |
| load module | exact gate + canonical Windows path + bundle manifest + module SHA-256 + successful System32-only default search configuration + exact `LoadLibraryExW` flags | scanner validates immediately before `call_started(load_library)` | relative/alternate path, hash mismatch, no gate, invalid/default-widening flag, non-null `hFile` |
| enumerate factory | live base factory acquisition + exact SDK ABI definitions + bounded count | within scanner thread | null base, invalid query result, count outside 0..256 |
| terminate process | session/run + exact Runtime root + exact PID/start + current ancestry/group | immediately before each signal | PID alone, name match, stale census, unrelated process |
| publish census | raw-output hash + scanner exit + monotonic attempt/completion timeline + schema + expected source fixture ID + `create_instance_called=false` | normalization and packet readback | malformed/oversized/incomplete/mismatched data, unknown operation, source-manifest drift, tripwire marker |
| claim preservation | before/after labeled protected snapshot equality + mutation-root receipt; exact historical packet/helper Git identities; exact current Bitwig `6.1` Flatpak projection | before toolchain install, before each live exercise, after process clean, and before packet publication | any drift, unknown item, unavailable comparison, or attempted substitution of historical `6.0.11` proof for current `6.1` behavior |

Class identity is the exact 16-byte TUID plus its canonical FUID rendering. A name, category, ordinal, module path, or vendor string is never sufficient identity.

## 10. Data, durability, recovery, and migration

| Data class | Owner | Location | Sensitive? | Durability | Backup/rollback | Migration law |
|---|---|---|---|---|---|---|
| toolchain deployed state | `WindowsToolchainLock` | user Flatpak installation | no credentials; shared user state | Flatpak deployment + commit readback | explicit exact-ref reversal only; no automatic removal of shared exact install | exact commit only; new commit requires design review |
| implementation-source manifest | `ReferenceFixtureBuild` / `CensusEvidencePacket` | canonical untracked build receipt, then byte-identical object in `BUILD_MANIFEST.json` | no | clean Git commit plus path/mode/blob records and canonical digest | source change invalidates artifacts and live evidence; rebuild/rerun | exact `linux-vst-bridge-wf0-implementation-source/v1`; no implicit roster change |
| build roots | `ReferenceFixtureBuild` | `<HOME>/.cache/linux-vst-bridge/wf0/builds/<source-id>/{a,b}` | private path transient | disposable | delete exact owned root | recreate; no migration |
| verified artifacts | `ReferenceFixtureBuild` | `<HOME>/.cache/linux-vst-bridge/wf0/artifacts/<artifact-set-id>` | binaries, not credentials | atomic manifest and readback | regenerate from source | schema/version change requires design review |
| scan environment | `WF0ScanEnvironment` | declared `.wf0-factory-census.stage-<run-id>` root | potentially runtime metadata; open fixtures only | marker/file and parent durability where authority changes; first launch owns prefix initialization | delete exact owned root after zero descendants | never migrate/adopt; recreate; never bootstrap separately |
| raw scanner output | `ScannerProcessSession` | bounded memory and WF0 session receipt staging | private paths/PIDs possible before sanitization | transient only | discard after normalized/evidence input hashes | raw schema mismatch blocks |
| normalized census | `FactoryCensusNormalizer` | evidence staging then `CENSUS.json` | sanitized | atomic file and schema readback | regenerate from bounded raw receipt within run; otherwise rerun | exact schema version; no implicit upgrade |
| process observations | `WF0Supervisor` | bounded transient memory then sanitized stage timeline | raw PID/cmdline transient | only role-based sanitized record retained | rerun; no PID restoration | topology schema change requires review |
| protected snapshot | `ProtectedFixtureSnapshot` | bounded transient values; separately labeled historical-evidence and current-Bitwig identities; equality/digest retained | no proprietary contents | before/after receipt | no repair, downgrade, upgrade, launch, or reclassification | changed roster or Bitwig identity requires review |
| evidence packet | `CensusEvidencePacket` | fixed tracked `evidence/wf0-windows-vst3-factory-census/` roster | sanitized | per-file atomic writes, sorted hashes, Git blob durability | regenerate before commit | explicit packet/schema version only |

Atomic publication uses same-filesystem temporary files, file flush/fsync, rename, parent-directory fsync, and exact readback. Directories are authority only through an exact marker/manifest; mere existence is not success. Retrying an identical completed operation verifies and reuses its immutable receipt. A partial or identity-different result is refused. No predecessor environment or user content is migrated because the WF0 environment is disposable.

The raw scanner record is not retained in Git. If a failure prevents safe normalization, retained evidence contains only the bounded stage, result code, relevant artifact identities, sanitized error class, and raw-output SHA-256; it never embeds arbitrary scanner bytes.

## 11. Factory census schema

The retained schema is `linux-vst-bridge-wf0-factory-census/v1`. The scanner emits bounded JSON Lines to a transient stream; only the strict Linux-normalized object below is retained. JSON numbers are used only where the exact range is bounded and lossless; hashes and 32-bit result values use canonical lowercase hex strings where signedness could be ambiguous.

```json
{
  "schema": "linux-vst-bridge-wf0-factory-census/v1",
  "scanner_build_identity": {
    "source_manifest_sha256": "<64-lowercase-hex>",
    "pe_sha256": "<64-lowercase-hex>",
    "toolchain_lock_sha256": "<64-lowercase-hex>"
  },
  "implementation_source_identity": {
    "schema": "linux-vst-bridge-wf0-implementation-source/v1",
    "commit": "<40-lowercase-hex>",
    "record_count": 26,
    "manifest_sha256": "<64-lowercase-hex>"
  },
  "reference_fixture_identity": {
    "sdk_root_commit": "3cdf9ca5d1f5b1b21e0a86832aa4abe55607bd96",
    "again_entry_blob": "13b920b4b7a74137301bf213cf048e96e82861d4",
    "again_cid_blob": "d32d1640ea187e718baba9cbb039d3a436d53cce",
    "build_manifest_sha256": "<64-lowercase-hex>"
  },
  "module_digest": "<64-lowercase-hex>",
  "module_bundle_identity": {
    "schema": "linux-vst-bridge-wf0-bundle-manifest/v1",
    "sha256": "<64-lowercase-hex>",
    "binary_safe_path": "again.vst3/Contents/x86_64-win/again.vst3"
  },
  "dll_search_contract": {
    "set_default_dll_directories_flags_u32_hex": "00000800",
    "load_library_ex_flags_u32_hex": "00000900",
    "absolute_module_path": true,
    "hfile_null": true
  },
  "module_entry_presence_and_result": {
    "present": true,
    "called": true,
    "result": true
  },
  "factory_interface_support": {
    "IPluginFactory": {"supported": true, "source": "GetPluginFactory"},
    "IPluginFactory2": {"supported": true, "query_result_u32_hex": "00000000"},
    "IPluginFactory3": {"supported": true, "query_result_u32_hex": "00000000"}
  },
  "factory_vendor": {"text": "...", "source_encoding": "utf8", "source_bytes_hex": "..."},
  "factory_url": {"text": "...", "source_encoding": "utf8", "source_bytes_hex": "..."},
  "factory_email": {"text": "...", "source_encoding": "utf8", "source_bytes_hex": "..."},
  "factory_flags": {"raw_i32": 16, "known": ["kUnicode"], "unknown_bits_u32_hex": "00000000"},
  "class_count": 3,
  "classes": [
    {
      "ordinal": 0,
      "class_id": "84E8DE5F92554F5396FAE4133C935A18",
      "class_id_raw_tuid_hex": "5FDEE8845592534F96FAE4133C935A18",
      "cardinality": 2147483647,
      "category": {"text": "Audio Module Class", "source_encoding": "utf8", "source_bytes_hex": "..."},
      "name": {"text": "AGain VST3", "source_encoding": "utf16le", "source_bytes_hex": "..."},
      "class_info_tier": "IPluginFactory3.PClassInfoW",
      "tier_attempts": [{"tier": "IPluginFactory3.PClassInfoW", "result_u32_hex": "00000000"}],
      "class_flags_if_exposed": {"raw_i32": 1, "known": ["kDistributable"], "unknown_bits_u32_hex": "00000000"},
      "subcategories_if_exposed": {"text": "Fx", "source_encoding": "utf8", "source_bytes_hex": "..."},
      "vendor_if_exposed": {"text": "", "source_encoding": "utf16le", "source_bytes_hex": ""},
      "version_if_exposed": {"text": "3.8.1.0", "source_encoding": "utf16le", "source_bytes_hex": "..."},
      "sdk_version_if_exposed": {"text": "VST 3.8.1", "source_encoding": "utf16le", "source_bytes_hex": "..."}
    }
  ],
  "module_exit_presence_and_result": {
    "present": true,
    "called": true,
    "result": true
  },
  "module_unload_result": {"attempted": true, "succeeded": true},
  "call_event_contract": {
    "closed_operation_count": 15,
    "started_count": 15,
    "completed_count": 15,
    "last_in_flight_operation": null
  },
  "create_instance_called": false,
  "scanner_exit": {"code_u32_hex": "00000000", "classification": "scanner_completed", "last_stage": "scanner_completed"},
  "stage_timeline_sha256": "<64-lowercase-hex>",
  "explicit_nonclaims": ["no_class_instantiation", "no_audio", "no_gui", "no_bitwig", "no_serum"]
}
```

### Canonicalization and bounds

- `class_id_raw_tuid_hex` is exactly the 16 bytes returned in the Windows COM-compatible `TUID`, uppercase hex, no punctuation. It is the duplicate key.
- `class_id` reconstructs the four logical FUID 32-bit words using the SDK's Windows `INLINE_UID` byte law, prints each word as eight uppercase hex digits, and concatenates them. It is never derived by blindly hex-printing memory.
- Class order is exact increasing factory ordinal. The normalizer never sorts, deduplicates, merges, or fills data from another field.
- Maximum class count is 256. Negative count or any count above 256 blocks before allocation/enumeration. `classes.length` must exactly equal `class_count`, and ordinals must be contiguous `0..count-1`.
- Factory byte capacities are vendor 64, URL 256, and email 128 including terminator. Class byte capacities are category 32, name 64, vendor 64, version 64, SDK version 64, and subcategories 128 including terminator. A `PClassInfoW` field has the corresponding 64 UTF-16-code-unit capacity.
- The scanner zeroes every entire structure, then requires a NUL terminator inside each fixed array. It retains only bytes/code units before the first terminator, not padding. UTF-8 and UTF-16LE decoding is strict; embedded NUL, overlong UTF-8, invalid scalar, or unpaired surrogate blocks.
- A 63-code-unit UTF-16 field can normalize to at most 252 UTF-8 bytes. No retained decoded field may exceed that derived limit; URL remains at most 255 bytes, email/subcategory 127, and category 31.
- Duplicate raw TUIDs block, even when other metadata differs. Printable-ID disagreement with the raw bytes blocks.
- Factory 1 support is required. Only exact IIDs for factory 2 and 3 are queried. `kNoInterface` plus null is retained as unsupported; unknown interfaces are not probed. Unexpected result/pointer pairs block.
- Tier order is factory 3 Unicode, factory 2, factory 1. Every attempted result is retained. Data not exposed by the selected tier is JSON `null`, never an empty invented value. Source-empty exposed data remains empty.
- Factory/class flags retain the signed raw value, decoded known names, and unknown bit mask. Unknown bits are observations; an AGain mismatch against its exact expected flags fails the positive fixture comparison.
- Every call event must satisfy the closed operation/interface/tier enum, strict sequence, attempt/completion pairing, ordinal law, and one-in-flight synchronous-call law in section 6. The successful AGain result has no in-flight attempt at exit. A crash/timeout record retains the exact final uncompleted attempt and its blocker.
- `create_instance_called` is required and must be `false`. The scanner command, schema, and operation enum expose no instantiation request; the tripwire proof in section 16 independently requires absence of its deterministic marker.
- The implementation-source schema, commit, 26-record count, and digest must equal the clean source used for both builds and every live run and must regenerate identically at the final evidence-only head.
- Maximum raw stdout, stderr, event count, event size, final record, and normalized output are the process bounds in section 8. Multiple final census records, data after the final record, a lifecycle or call-event sequence violation, or identity mismatch is malformed output.
- Native pointer values, handles, addresses, raw PIDs, unstable timestamps, and private paths are forbidden in the retained schema.

The transient raw stream is hashed and discarded after successful packet publication. The normalized `CENSUS.json`, sanitized `STAGE_TIMELINE.json`, and their hashes are retained. `STAGE_TIMELINE.json` preserves each validated record sequence, closed lifecycle/event kind, call operation/interface/ordinal/tier, attempt/completion link, bounded return classification, in-flight blocker, and primary-versus-cleanup precedence; it contains no raw pointer, handle, PID, private path, or arbitrary module text. The held-gate receipt records `module_open_started=false`, `load_library_attempted=false`, `again_module_mapped=false`, `factory_or_class_event=false`, scanner waiting, cleanup/retirement results, and protected-state equality.

## 12. Build and toolchain lock

### Repository-owned CMake toolchain

`cmake/WF0Toolchain.cmake` must set, without ambient fallback:

```cmake
set(CMAKE_SYSTEM_NAME Windows)
set(CMAKE_SYSTEM_PROCESSOR x86_64)
set(CMAKE_C_COMPILER /usr/lib/sdk/mingw-w64/bin/x86_64-w64-mingw32-gcc)
set(CMAKE_CXX_COMPILER /usr/lib/sdk/mingw-w64/bin/x86_64-w64-mingw32-g++)
set(CMAKE_RC_COMPILER /usr/lib/sdk/mingw-w64/bin/x86_64-w64-mingw32-windres)
set(CMAKE_AR /usr/lib/sdk/mingw-w64/bin/x86_64-w64-mingw32-ar)
set(CMAKE_NM /usr/lib/sdk/mingw-w64/bin/x86_64-w64-mingw32-nm)
set(CMAKE_OBJDUMP /usr/lib/sdk/mingw-w64/bin/x86_64-w64-mingw32-objdump)
set(CMAKE_RANLIB /usr/lib/sdk/mingw-w64/bin/x86_64-w64-mingw32-ranlib)
set(CMAKE_STRIP /usr/lib/sdk/mingw-w64/bin/x86_64-w64-mingw32-strip)
set(CMAKE_FIND_ROOT_PATH
    /usr/lib/sdk/mingw-w64/x86_64-w64-mingw32
    /usr/lib/sdk/mingw-w64)
set(CMAKE_FIND_ROOT_PATH_MODE_PROGRAM NEVER)
set(CMAKE_FIND_ROOT_PATH_MODE_LIBRARY ONLY)
set(CMAKE_FIND_ROOT_PATH_MODE_INCLUDE ONLY)
set(CMAKE_FIND_ROOT_PATH_MODE_PACKAGE ONLY)
set(CMAKE_TRY_COMPILE_TARGET_TYPE STATIC_LIBRARY)
```

The same file declares the exact Windows system import libraries required by current VSTGUI because GCC does not treat MSVC `#pragma comment(lib, ...)` as link authority. The declared candidate roster from reconnaissance is `d2d1`, `dwrite`, `windowscodecs`, `dwmapi`, `shlwapi`, `imm32`, `d3d11`, `dxgi`, `ole32`, `oleaut32`, `uuid`, `comdlg32`, `comctl32`, `shell32`, `user32`, `gdi32`, `advapi32`, and `winmm`. A future link requiring an undeclared library is not patched ad hoc; it is `WF0_REFERENCE_BUILD_BLOCKED` and, if the envelope changes, `RETURN_TO_DESIGN_GATE`.

The pinned SDK's Windows bundle post-build rule also invokes a host command named `attrib` three times for `desktop.ini`, `PlugIn.ico`, and the bundle directory. `tools/wf0-factory-census/host-shims/attrib` is an audited Linux host shim used only in the AGain build. It accepts exactly `+s` plus one existing canonical path beneath the current owned AGain build root, rejects symlinks/escape/extra arguments, and returns success without claiming Windows Explorer attributes. Its path is explicitly prepended to `/usr/bin`; all compiler/binutils paths remain absolute. The build receipt binds all three accepted invocations. This is build driving, not an upstream SDK patch.

### Exact build entry points

Both builds run in the exact SDK Flatpak, with source read-only and only the selected build root writable. Environment begins empty except the declared locale/time/build variables and Flatpak-required basics:

```text
LC_ALL=C
LANG=C
TZ=UTC
SOURCE_DATE_EPOCH=1786457692
PATH=<REPOSITORY_ROOT>/tools/wf0-factory-census/host-shims:/usr/bin
```

The repository root `CMakeLists.txt` gains an early `WF0_BUILD_ONLY` branch before the existing HP0 Linux-only guard. That branch loads `WF0DependencyLock`, verifies the same pinned SDK recursively, disables SDK examples/VSTGUI/validator/module-info/plugin links, adds the pinned SDK as an out-of-tree subdirectory, then adds only `windows-factory-probe` and `windows-fixtures/wf0` before returning. The probe links the official `base`/`pluginterfaces` targets needed for `IPluginFactory` IIDs and structures; it does not link `sdk_hosting` or its `Win32Module` helper. Fault modules link only the minimum official SDK factory targets. The ordinary HP0 branch remains byte-for-byte behaviorally unchanged when `WF0_BUILD_ONLY` is off.

The repository scanner/negative-fixture configure is:

```text
/usr/bin/cmake -S <REPOSITORY_ROOT> -B <WF0_REPOSITORY_BUILD_ROOT> -G Ninja
  -DCMAKE_BUILD_TYPE=Release
  -DCMAKE_MAKE_PROGRAM=/usr/bin/ninja
  -DCMAKE_TOOLCHAIN_FILE=<REPOSITORY_ROOT>/cmake/WF0Toolchain.cmake
  -DWF0_VST3_SDK_ROOT=<PINNED_SDK_ROOT>
  -DWF0_BUILD_ONLY=ON
  -DCMAKE_FIND_USE_PACKAGE_REGISTRY=OFF
  -DCMAKE_FIND_USE_SYSTEM_PACKAGE_REGISTRY=OFF
```

Only `wf0-factory-probe`, its internal loader-adapter tests, and the named WF0 fault-fixture targets are built.

The AGain configure drives the pinned SDK root directly:

```text
/usr/bin/cmake -S <PINNED_SDK_ROOT> -B <WF0_AGAIN_BUILD_ROOT> -G Ninja
  -DCMAKE_BUILD_TYPE=Release
  -DCMAKE_MAKE_PROGRAM=/usr/bin/ninja
  -DCMAKE_TOOLCHAIN_FILE=<REPOSITORY_ROOT>/cmake/WF0Toolchain.cmake
  -DSMTG_ENABLE_VST3_PLUGIN_EXAMPLES=ON
  -DSMTG_ENABLE_VST3_HOSTING_EXAMPLES=OFF
  -DSMTG_ENABLE_VSTGUI_SUPPORT=ON
  -DSMTG_ENABLE_WAYLAND_SUPPORT=OFF
  -DSMTG_CREATE_BUNDLE_FOR_WINDOWS=ON
  -DSMTG_RUN_VST_VALIDATOR=OFF
  -DSMTG_CREATE_MODULE_INFO=OFF
  -DSMTG_CREATE_PLUGIN_LINK=OFF
  -DCMAKE_FIND_USE_PACKAGE_REGISTRY=OFF
  -DCMAKE_FIND_USE_SYSTEM_PACKAGE_REGISTRY=OFF
```

Only target `again` is built. No validator, module-info tool execution, host example, all-target build, configure-time download, `FetchContent`, package registry, source patch, or generated link into a plug-in directory is permitted.

### Negative fixture targets

One repository-owned source file is compiled with fixed definitions into this bounded family:

```text
wf0-missing-factory
wf0-null-factory
wf0-no-entry
wf0-factory1-only
wf0-factory2-only
wf0-factory3-fallback
wf0-init-false
wf0-factory-info-false
wf0-count-negative
wf0-count-excessive
wf0-class-info-false
wf0-duplicate-class-id
wf0-hang-entry
wf0-hang-factory
wf0-hang-class
wf0-crash-entry
wf0-crash-factory
wf0-crash-class
wf0-hang-release
wf0-crash-release
wf0-exit-false
wf0-create-instance-tripwire
```

They implement only enough official factory ABI to assign scanner-stage ownership. They never become positive compatibility fixtures. The existing entry/factory/class crash and hang targets bind their exact closed operation in the expected timeline; class faults bind one declared ordinal and tier. The two release targets exercise `WF0_FACTORY_RELEASE_BLOCKED`. Deterministic call-mapping tests enumerate all 15 operation values, while source-owned Runtime/Proton faults remain proportional rather than becoming a general malformed-VST3 laboratory.

Deterministic unload-failure and call-completion mapping uses the scanner's narrow internal `Win32LoaderApi` test adapter; the production scanner always binds the real Win32 calls. No claim is made that a real valid `HMODULE` can be forced to make `FreeLibrary` fail under Proton.

`wf0-create-instance-tripwire` exposes the expected bounded factory/class metadata and implements `IPluginFactory::createInstance` only as a tripwire: if invoked it creates one deterministic session marker and returns failure. A complete census must leave that marker absent and normalize `create_instance_called=false`. The scanner production sources are separately token/AST-checked to contain no `IPluginFactory::createInstance` invocation, and the exact scanner command, schema, and closed operation enum contain no class-instantiation request. The tripwire method is never invoked merely to prove that it is otherwise not invoked.

### Build verification and reproducibility

Each A/B build independently verifies:

1. exact toolchain/SDK receipt, exact 26-record implementation-source manifest, and no network;
2. Release-only output and no debug-path leakage;
3. PE32+ x86_64 machine type for scanner and every DLL;
4. required/optional export tables;
5. recursively closed import table;
6. only declared Windows system imports plus exact locked extension runtime DLLs;
7. AGain bundle safe-relative roster, modes, sizes, and hashes;
8. scanner and negative-fixture artifact roster;
9. no symlink, device, FIFO, absolute path, traversal, or untracked third-party source;
10. no generated artifact inside Git.

Both builds must originate from the same clean implementation commit and byte-identical `linux-vst-bridge-wf0-implementation-source/v1` manifest later used by every live exercise. A dirty governed path, missing/extra governed path, mode/blob mismatch, or source/configuration change after a live exercise invalidates the artifact and all retained live evidence; the exact build and live proof must be repeated.

Link flags explicitly request no inserted PE timestamp and source/build prefix mapping. The A/B comparison first requires byte SHA-256 equality. If bytes differ, the reproducibility normalizer may zero only declared PE COFF `TimeDateStamp` and checksum fields after proving those are the sole offsets that differ. It retains both raw hashes, exact differing offsets/field names, and a normalized digest. Any other difference is a build blocker. The design never promises byte identity before observation.

Expected dynamic GCC runtime candidates are `libgcc_s_seh-1.dll`, `libstdc++-6.dll`, and `libwinpthread-1.dll`; independent `objdump` closure decides which are actually needed. Each required DLL is copied from the exact extension into the scanner directory and the AGain module directory, hashed, and kept untracked. An unexpected non-system import is blocked. The build receipt includes all applicable license/notice identities.

## 13. Security, privacy, licensing, and proprietary material

### Threat model

| Threat | Boundary | Prevention | Negative proof |
|---|---|---|---|
| malicious or malformed DLL | Windows scanner process | disposable prefix, no credentials/network, bounded calls/output/time, supervisor containment | crash/hang/stage fixtures and zero cleanup |
| dependency substitution | Windows loader | copied exact artifacts; checked System32-only defaults; exact module-directory plus System32 `LoadLibraryExW` flags; import/hash manifest | alternate DLL/hash, invalid API flag, non-null `hFile`, and widened-search rejection |
| stale process identity | host process table | PID + start ticks + complete ancestry + session binding | stale/unrelated sentinel test |
| prefix adoption/deletion | filesystem | exact canonical root, initial absence, durable ownership marker, safe-relative deletion | unknown marker/object refusal |
| output memory/disk exhaustion | scanner/supervisor/normalizer | fixed class/field/event/stream/process caps | excessive count/output fixtures |
| false compatibility claim | evidence boundary | exact fixture/version/capability schema, `create_instance_called=false`, and explicit nonclaims | packet validator and class-creation tripwire reject a widened claim/path |
| credential/proprietary capture | diagnostics | open fixtures only, allow-list packet, raw streams transient, no vendor roots | sanitization scan and forbidden-token/path checks |
| build supply-chain drift | Flatpak/SDK/toolchain/governed source | exact deployed commits, clean submodules, exact 26-record path/mode/blob source manifest, offline configure/build, full paths | wrong commit/compiler/dirty SDK/source, extra/missing path, and final-manifest mismatch negatives |
| WR0 or user-state corruption | mutation boundary | separate WF0 roots; no writable path to WR0/current Bitwig/Serum/Steam compatdata; historical evidence checked by Git identity rather than rewritten | before/after labeled equality plus mutation-root audit |

Wine/Proton and Flatpak are not described as security sandboxes. They are runtime/build boundaries with explicit mutation and containment laws.

### Sensitive/proprietary exclusions

Git, retained logs, screenshots, and evidence may never contain credentials, cookies, tokens, account identifiers, hostnames, IP/MAC addresses, device serials, raw PIDs, native pointer values, private home/check-out paths, a complete prefix, vendor installer, license file, activation data, Serum binary/state, Bitwig project data, compiled SDK/AGain/scanner/runtime DLLs, or paid content. Artifact SHA-256 values and safe-relative paths are permitted.

### Network and authorization

Only the separately approved user Flatpak extension installation may use network access. Configure, build, verification, scan, normalization, and evidence publication run with network denied. No browser, vendor login, authorization, TLS interception, protocol analysis, or credential flow exists.

### Third-party licensing

- Pinned VST3 SDK root, `public.sdk`, `pluginterfaces`, and SDK CMake: MIT notices retained.
- Pinned VSTGUI: its permissive license notice/disclaimer/no-endorsement terms retained.
- MinGW extension: Flathub advertises `ZPL-2.1`, GPL/LGPL versions, and public-domain components. Exact installed notices and runtime-DLL obligations are part of `WindowsToolchainLock`.
- Runtime 4/Proton: already accepted installed fixture, read-only; WF0 makes no redistribution claim.
- AGain and MinGW runtime binaries: local untracked proof artifacts only. Any future distribution requires a separate license/distribution design.
- No yabridge source is copied or linked.

## 14. Approved code topology

### Component responsibilities

| Component | Owner responsibility | Public interface | Must not own |
|---|---|---|---|
| `cmake/WF0Toolchain.cmake` | full-path Windows compiler/sysroot/import-library selection | CMake toolchain variables | install state, SDK patching |
| `cmake/WF0DependencyLock.cmake` | exact SDK/submodule and build-option assertions | configure-time fail-closed checks | network or source acquisition |
| `windows-factory-probe` | `WindowsFactoryProbe`; bounded ABI/load/census events | exact command contract and JSONL v1 | process kill, normalization, class instantiation |
| `windows-fixtures/wf0` | bounded stage-fault modules | fixed target roster | positive compatibility claims |
| `build.py` / `verify.py` | `ReferenceFixtureBuild`, exact implementation-source manifest, and toolchain/artifact readback | immutable source/build receipts | scan runtime |
| `environment.py` | `WF0ScanEnvironment` | marker/create/copy/retire operations | WR0 or unknown-root mutation |
| `supervise.py` | `WF0Supervisor` and process session | launch/gate/stage/cleanup receipt | factory semantics |
| `normalize.py` | `FactoryCensusNormalizer` | census v1 or exact rejection | data repair/invention |
| `evidence.py` | `CensusEvidencePacket`, source-manifest equality, and protection packet assembly | fixed evidence roster/hash closure | primary fact generation |
| `run.py` | dependency-ordered orchestration only | one explicit WF0 transaction | becoming a manager/product runner |

### Exact proposed implementation paths: 40

These are the only proposed implementation/evidence paths. Separate review/approval/authority records are not implementation paths and remain governed by their own later envelopes.

| # | Path |
|---:|---|
| 1 | `.gitignore` |
| 2 | `CMakeLists.txt` |
| 3 | `cmake/WF0Toolchain.cmake` |
| 4 | `cmake/WF0DependencyLock.cmake` |
| 5 | `docs/WF0_WINDOWS_TOOLCHAIN_LOCK.md` |
| 6 | `windows-factory-probe/CMakeLists.txt` |
| 7 | `windows-factory-probe/include/linux_vst_bridge/wf0_probe/events.h` |
| 8 | `windows-factory-probe/include/linux_vst_bridge/wf0_probe/census.h` |
| 9 | `windows-factory-probe/source/main.cpp` |
| 10 | `windows-factory-probe/source/win32_module.h` |
| 11 | `windows-factory-probe/source/win32_module.cpp` |
| 12 | `windows-factory-probe/source/factory_census.h` |
| 13 | `windows-factory-probe/source/factory_census.cpp` |
| 14 | `windows-fixtures/wf0/CMakeLists.txt` |
| 15 | `windows-fixtures/wf0/source/fault_fixture.cpp` |
| 16 | `tools/wf0-factory-census/README.md` |
| 17 | `tools/wf0-factory-census/common.py` |
| 18 | `tools/wf0-factory-census/build.py` |
| 19 | `tools/wf0-factory-census/verify.py` |
| 20 | `tools/wf0-factory-census/environment.py` |
| 21 | `tools/wf0-factory-census/supervise.py` |
| 22 | `tools/wf0-factory-census/normalize.py` |
| 23 | `tools/wf0-factory-census/negative_tests.py` |
| 24 | `tools/wf0-factory-census/evidence.py` |
| 25 | `tools/wf0-factory-census/run.py` |
| 26 | `tools/wf0-factory-census/host-shims/attrib` |
| 27 | `evidence/wf0-windows-vst3-factory-census/BASIS.md` |
| 28 | `evidence/wf0-windows-vst3-factory-census/TOOLCHAIN.md` |
| 29 | `evidence/wf0-windows-vst3-factory-census/BUILD.md` |
| 30 | `evidence/wf0-windows-vst3-factory-census/BUILD_MANIFEST.json` |
| 31 | `evidence/wf0-windows-vst3-factory-census/ENVIRONMENT.md` |
| 32 | `evidence/wf0-windows-vst3-factory-census/LAUNCH_AND_PROCESS.md` |
| 33 | `evidence/wf0-windows-vst3-factory-census/STAGE_TIMELINE.json` |
| 34 | `evidence/wf0-windows-vst3-factory-census/CENSUS.json` |
| 35 | `evidence/wf0-windows-vst3-factory-census/NEGATIVE_TESTS.md` |
| 36 | `evidence/wf0-windows-vst3-factory-census/PRESERVATION.md` |
| 37 | `evidence/wf0-windows-vst3-factory-census/FINDINGS.md` |
| 38 | `evidence/wf0-windows-vst3-factory-census/SANITIZATION.md` |
| 39 | `evidence/wf0-windows-vst3-factory-census/fixture.json` |
| 40 | `evidence/wf0-windows-vst3-factory-census/hashes.sha256` |

### Exact implementation-source manifest: 26 paths

The canonical source schema is:

```text
linux-vst-bridge-wf0-implementation-source/v1
```

It is UTF-8 canonical JSON with one trailing LF. It binds the clean implementation commit and exactly 26 stable records. Each record contains only `path`, six-digit `git_mode`, and 40-lowercase-hex `git_blob`. Records are sorted by raw UTF-8 path bytes; duplicates, missing records, extra records, symlinks outside the declared mode, noncanonical serialization, or any other field are rejected. The literal lexically sorted path roster is:

```text
.gitignore
CMakeLists.txt
cmake/WF0DependencyLock.cmake
cmake/WF0Toolchain.cmake
docs/WF0_WINDOWS_TOOLCHAIN_LOCK.md
tools/wf0-factory-census/README.md
tools/wf0-factory-census/build.py
tools/wf0-factory-census/common.py
tools/wf0-factory-census/environment.py
tools/wf0-factory-census/evidence.py
tools/wf0-factory-census/host-shims/attrib
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

The implementation sequence is exact:

1. Create one clean committed implementation head containing all 26 paths.
2. Generate and read back the canonical source manifest from that commit.
3. Build every artifact only from that commit and manifest.
4. Run every negative and live exercise only from those artifacts and that exact manifest.
5. Add only the 14 evidence paths numbered 27–40 in a later evidence-only commit.
6. At the final PR head, re-enumerate the same 26 paths from the final tree, preserve the recorded implementation-commit field, regenerate the canonical bytes, and require byte and SHA-256 equality with the build/live manifest.

The canonical pre-build receipt remains untracked in the verified artifact root and is retained byte-for-byte as the `implementation_source_manifest` object in `evidence/wf0-windows-vst3-factory-census/BUILD_MANIFEST.json`; it is distinct from the VST3 SDK identity, MinGW toolchain lock, artifact-set manifest, AGain bundle manifest, and evidence hash manifest.

The WF0-owned governed roots are `windows-factory-probe/`, `windows-fixtures/wf0/`, and `tools/wf0-factory-census/`; the root/configuration singleton paths are the other five records above. Dirty governed source, an unexpected tracked path beneath a governed root, an absent roster path, or any mode/blob/digest change blocks before build. Any source/configuration change after live execution invalidates all generated artifacts and live evidence and requires rebuilding and rerunning the exact proof.

The 40-path envelope is unchanged. The implementation may use one clean 26-path source commit before live execution and one final 14-path evidence-only commit if later governance authorizes that posture. Generated build products never enter tracked paths.

## 15. Changed-path and external-mutation envelope

### Design-phase envelope

This V3 design-amendment branch may differ from amendment basis
`15523c69567d24b256cb3c65cb6f06bfa07854be` only at:

```text
CURRENT_SLICE.md
docs/slices/WF0/BITWIG_6_1_FIXTURE_RECONCILIATION.md
docs/slices/WF0/IMPLEMENTATION_DESIGN_V3.md
docs/slices/WF0/ADVERSARIAL_DESIGN_REVIEW_V3.md
```

The first amendment commit contains only the first three paths. A later review
commit may add only the fresh V3 review record after reviewing the immutable
first amendment commit. A future approval receipt is not part of the design or
review commits and requires a separate exact operator approval. No
implementation path above is authorized on this branch.

### Future implementation tracked envelope

Only the 40 paths in section 14 may change in the eventual implementation PR. Existing HP0/native-probe and WR0 source/evidence are read-only. A need to change them is a demonstrated shared-interface discovery and requires design review before editing.

### Permitted future external mutation

```text
user Flatpak ref:
  org.freedesktop.Sdk.Extension.mingw-w64/x86_64/25.08 at the exact approved commit

build cache:
  <HOME>/.cache/linux-vst-bridge/wf0/builds/<source-id>/{a,b}
  <HOME>/.cache/linux-vst-bridge/wf0/artifacts/<artifact-set-id>

disposable scan root:
  <HOME>/.local/share/linux-vst-bridge/environments/
    .wf0-factory-census.stage-<32-hex-run-id>

bounded runtime transients:
  objects owned by that exact Runtime-root session beneath /tmp, /dev/shm,
  and XDG_RUNTIME_DIR, removed at completion
```

### Protected state law

The accepted WR0 environment identity, its sibling roster, Runtime/Proton installed assets, WR0 contract source, `.wine`, Steam compatdata, SteamOS read-only state, HP0 publication/evidence, HP1 Bitwig state/evidence, current Bitwig `6.1` installation and Flatpak configuration, Serum/vendor material, other worktrees, and every repository path outside the envelope are protected. The implementation has no writable handle or path into them.

`ProtectedFixtureSnapshot` retains content-free labeled equality. It treats
historical proof identity and current application identity as separate labels:

- exact Git modes/blobs and packet hash manifests preserve the accepted SR0,
  HP0, HP1, WR0, and WR0A records with their truthful `6.0.11` statements;
- live Flatpak readback preserves the current Bitwig `6.1` ref, app commit,
  runtime ref/commit, installation scope, user-shadow absence, override-byte
  hashes, and permission-output hash.

The snapshot is implemented within the WF0-owned paths. It does not invoke the
historical WR0 fixture-capture helper as current Bitwig authority and does not
edit that helper. It may read declared non-sensitive WR0 markers/manifests and
safe metadata. It does not traverse credentials, registry/browser data, Serum,
Bitwig projects, or other proprietary contents. For prohibited roots,
preservation proof combines unchanged allow-listed identity records, pre/post
metadata equality where safe, and an exhaustive WF0 mutation-root/process
contract; it does not claim a recursive secret-content hash.

## 16. Proof matrix

V3 retains V2's 35-row matrix. The six causal/source ownerships added in V2 cannot truthfully be inferred from the V1 rows, and the Bitwig reconciliation is fully owned by existing row 34. One production run may satisfy several rows only where its receipts map them explicitly; no target test count is implied.

| # | Claim | Positive proof | Negative/fault proof | Real fixture? | Production owner exercised? | Retained evidence | Ceiling |
|---:|---|---|---|---|---|---|---|
| 1 | exact toolchain deployed | commit/path/version/target/runtime readback | missing/wrong commit or tool | installed Flatpak | yes, `WindowsToolchainLock` | `TOOLCHAIN.md` | no compile claim |
| 2 | exact clean SDK | root/tree/submodule/clean manifest | wrong/dirty/missing submodule | pinned checkout | yes, dependency lock | `BUILD.md` | no binary claim |
| 3 | implementation source is exact and stable | clean implementation commit; canonical 26-record path/mode/blob manifest; byte equality at final head | dirty/missing/extra/mode/blob change blocks; any post-live source change invalidates artifacts/evidence and forces rebuild/rerun | repository trees | yes, build/evidence owners | `BUILD_MANIFEST.json`, `BUILD.md` | source identity only |
| 4 | build is offline/nonambient | denied network, full compiler paths, cache audit | ambient compiler/package registry injection refused | build fixture | yes, build driver | `BUILD.md` | no reproducibility claim |
| 5 | two-build reproducibility | A/B raw hashes or allowed-field normalized digest | undeclared byte difference blocks | real scanner/AGain | yes, build/verify | `BUILD_MANIFEST.json` | truthful declared level only |
| 6 | scanner is PE32+ x86_64 | independent file/objdump machine readback | wrong-machine scanner | real scanner | yes, verifier | `BUILD.md` | no runtime claim |
| 7 | AGain bundle/exports/imports exact | roster, module PE, required/optional exports, closed imports | missing/extra roster/export/import | real AGain | yes, verifier | `BUILD_MANIFEST.json` | no factory-call claim |
| 8 | missing fixture is refused | verified artifact root succeeds | remove/rename fixture in disposable test root | copied fixture | yes, env/supervisor | `NEGATIVE_TESTS.md` | no module open |
| 9 | wrong PE architecture is refused | x86_64 fixture admitted | repository-owned non-x86_64 header fixture rejected before launch | source-owned negative | yes, verifier | `NEGATIVE_TESTS.md` | admission only |
| 10 | wrong scanner/module hash is refused | exact hashes pass before/copy/spawn | one-byte copy mutation refused | copied artifacts | yes, env/supervisor | `NEGATIVE_TESTS.md` | no semantic claim |
| 11 | DLL search authority is exact | checked System32-only defaults; absolute module; null `hFile`; DLL-load-dir plus System32 load flags; AGain loads | invalid/default-widening flag, relative path, non-null `hFile`, application/user/current/PATH search rejected by loader-adapter tests | adapter + real AGain | yes, probe | timeline/negative | dependency search only |
| 12 | first launch alone owns prefix initialization | payload-only prestate; exact Runtime/Proton/scanner ancestry; readiness within 180 seconds; prefix confined beneath stage | separate bootstrap command rejected; prefix/scanner failure before readiness maps to launch blocker | real Runtime/Proton | yes, environment/supervisor | `ENVIRONMENT.md`, `LAUNCH_AND_PROCESS.md` | no product environment |
| 13 | gate causally prevents module load | exact scanner reaches readiness during 15-second held gate; no open/load attempt, AGain mapping, factory/class event; scanner waits | injected early event/map/wait departure blocks and cleanup still reaches zero | exact scanner/Runtime/Proton | yes, session/supervisor | timeline/process/preservation | no post-gate factory claim |
| 14 | complete AGain factory census | valid 23-lifecycle-state path plus closed attempt/completion records; exact metadata/three ordered classes; releases/exit/unload complete | any expected-field/order/attempt mismatch blocks | exact AGain + Runtime/Proton | yes, all production owners | census/timeline | factory boundary only |
| 15 | required export ownership | AGain export found | `wf0-missing-factory` | source-owned negative | yes, probe | `NEGATIVE_TESTS.md` | no factory obtained |
| 16 | null factory ownership | AGain non-null | `wf0-null-factory` | source-owned negative | yes, probe | `NEGATIVE_TESTS.md` | no class query |
| 17 | absent optional entry is observation | AGain entry result retained | `wf0-no-entry` completes factory path without entry blocker | source-owned negative | yes, probe | `NEGATIVE_TESTS.md` | absence not generalized |
| 18 | entry false is stage failure | AGain entry succeeds | `wf0-init-false` | source-owned negative | yes, probe | `NEGATIVE_TESTS.md` | cleanup still attempted |
| 19 | factory-info failure owned | AGain factory info exact | `wf0-factory-info-false` | source-owned negative | yes, probe | `NEGATIVE_TESTS.md` | no class claim |
| 20 | factory-version/tier fallback exact | support booleans and selected tier retained | factory 3/2 non-success variants fall back or block per contract | AGain + source-owned negative | yes, probe/normalizer | census/negative | only IPluginFactory1/2/3 |
| 21 | invalid/excessive count rejected | AGain count 3 | negative and 257-count fixtures | source-owned negative | yes, probe | `NEGATIVE_TESTS.md` | no allocation/enumeration |
| 22 | ordinal class failure owned | all AGain ordinals succeed | `wf0-class-info-false` at fixed ordinal/tier | source-owned negative | yes, probe | `NEGATIVE_TESTS.md` | no partial census success |
| 23 | duplicate class ID rejected | three unique AGain raw TUIDs | `wf0-duplicate-class-id` | source-owned negative | yes, probe/normalizer | `NEGATIVE_TESTS.md` | no dedup repair |
| 24 | malformed/oversized output rejected | bounded canonical output | truncation, duplicate final, invalid UTF, cap+1 stream/event count | bounded raw test inputs | yes, normalizer/supervisor | `NEGATIVE_TESTS.md` | no data recovery |
| 25 | scanner crash is exact-call attributed | ordinary completions; all 15 enum values have one mapping | entry/factory/class/release live crash variants plus deterministic operation-mapping tests; last uncompleted attempt owns | source-owned negatives/adapters | yes, supervisor/probe | timeline/negative | process containment only |
| 26 | scanner timeout is exact-call attributed | each ordinary attempt completes inside bound | entry/factory/class/release live hang variants plus deterministic operation-mapping tests; last uncompleted attempt owns | source-owned negatives/adapters | yes, supervisor/probe | timeline/negative | no real-time claim |
| 27 | factory-reference release is attributed | exactly one reverse-order attempt/completion per acquisition; ledger closes | `wf0-hang-release`, `wf0-crash-release`, and acquisition/release-ledger mismatch map to release blocker without erasing primary | source-owned negative | yes, probe/supervisor | timeline/negative | cleanup only |
| 28 | absent optional exit is observation | AGain exit retained | no-exit variant completes/unloads | source-owned negative | yes, probe | `NEGATIVE_TESTS.md` | absence not generalized |
| 29 | exit false owned | AGain exit true | `wf0-exit-false` | source-owned negative | yes, probe | `NEGATIVE_TESTS.md` | unload still attempted |
| 30 | unload result mapping | AGain `FreeLibrary` true | internal production-loader adapter returns false and validates attempt mapping | adapter + real AGain success | same module owner | unit receipt + live timeline | no claim real valid-handle failure inducible |
| 31 | no class instantiation occurs | production scanner token/AST scan; command/schema/operation absence; tripwire census completes with marker absent and `create_instance_called=false` | injected tripwire marker or scanner invocation token blocks | source-owned tripwire + exact scanner | yes, probe/verifier/normalizer | `NEGATIVE_TESTS.md`, census | no class lifecycle claim |
| 32 | exact cleanup and unrelated survival | zero owned descendants after success/fail | held-gate and stage-failure cleanup with same-family sentinel | real Runtime/Proton | yes, supervisor | `LAUNCH_AND_PROCESS.md` | no global process cleanup |
| 33 | unknown environment refused | exact marker root accepted | forged/missing/mismatched marker preserved untouched | filesystem fixture | yes, environment owner | `NEGATIVE_TESTS.md` | no recovery/adoption |
| 34 | accepted WR0/protected state unchanged | before/after equality and mutation-root audit; historical SR0/HP0/HP1/WR0/WR0A Git/packet identities exact; current Bitwig `6.1` app/runtime/scope/shadow/override/permission identities exact | deliberate mismatch in a historical identity, current Bitwig identity, WR0 identity, or another labeled protected record blocks packet | live safe readback plus repository trees | yes, `ProtectedFixtureSnapshot`/evidence | `PRESERVATION.md` | no Bitwig execution or `6.1` HP1 claim; no secret-content inspection |
| 35 | evidence is bounded/sanitized/hash-closed | fixed roster, sanitizer, hashes, Git diff | forbidden token/path/binary/extra file blocks | retained packet | yes, evidence owner | `SANITIZATION.md`, hashes | evidence is not broader compatibility |

## 17. Failure and blocked-result taxonomy

Exactly 23 blocked results exist. Secondary cleanup failures are retained beside the primary stage failure; they never overwrite it.

| # | Result code | Exact owner/stage | Preserved state | Cleanup | Next lawful action |
|---:|---|---|---|---|---|
| 1 | `WF0_DESIGN_PREFLIGHT_BLOCKED` | design authority/fixture preflight | read-only mismatch facts | none | restore authority externally or choose again |
| 2 | `WF0_WINDOWS_TOOLCHAIN_DESIGN_BLOCKED` | unavailable/incapable proposed toolchain | remote/source metadata | none | return to design gate |
| 3 | `WF0_REFERENCE_FIXTURE_DESIGN_BLOCKED` | AGain unlawful/incapable as selected | source/license findings | none | return to design gate; no substitute |
| 4 | `WF0_TOOLCHAIN_INSTALL_BLOCKED` | `WindowsToolchainLock` install/readback | prestate and deployed ref facts | no change to wrong preexisting install | operator resolves exact deployment, then retry |
| 5 | `WF0_SDK_BUILD_BLOCKED` | pinned SDK configure/library build | bounded logs and source receipt | remove owned build roots | repair only inside envelope or return gate |
| 6 | `WF0_REFERENCE_BUILD_BLOCKED` | AGain build/PE/bundle/dependencies | bounded build/verify receipt | remove/retain owned caches only | repair exact build or return gate |
| 7 | `WF0_SCANNER_BUILD_BLOCKED` | scanner/negative build/PE checks | bounded build/verify receipt | remove/retain owned caches only | repair exact scanner build |
| 8 | `WF0_SCAN_ENVIRONMENT_BLOCKED` | create/copy/retire environment | marker/root readback | only exact owned root; preserve unknown | resolve exact root state |
| 9 | `WF0_SCANNER_LAUNCH_BLOCKED` | spawn or pre-stage crash | launch receipt/last stage | scoped process cleanup | repair launch contract |
| 10 | `WF0_PROCESS_IDENTITY_BLOCKED` | readiness/topology/gate identity | sanitized mismatch and exact session | signal only revalidated owned identities | resolve topology/design |
| 11 | `WF0_MODULE_OPEN_BLOCKED` | `LoadLibraryExW` | module/artifact IDs and sanitized error class | optional exit if present, unload attempt, process drain | repair dependency/path or return gate |
| 12 | `WF0_MODULE_ENTRY_BLOCKED` | present `InitDll=false` or invalid entry record | entry presence/result | optional exit, unload, process drain | fixture/scanner repair; absence alone is not blocker |
| 13 | `WF0_FACTORY_GET_BLOCKED` | missing export, call failure/null | export/factory stage | release any acquired ref, exit/unload/drain | fixture/scanner repair |
| 14 | `WF0_FACTORY_INFO_BLOCKED` | info/query inconsistency | bounded factory results | release, exit/unload/drain | scanner/fixture repair |
| 15 | `WF0_CLASS_ENUMERATION_BLOCKED` | count/tier/ordinal/duplicate | valid prefix only, never partial success | release, exit/unload/drain | scanner/fixture repair |
| 16 | `WF0_FACTORY_RELEASE_BLOCKED` | factory 3/2/base release in flight or acquisition/release-ledger mismatch | earlier primary result plus exact release attempt/result prefix | continue reachable reverse-order releases, exit/unload/drain; never erase primary | repair release/ledger owner or return gate |
| 17 | `WF0_MODULE_EXIT_BLOCKED` | present `ExitDll=false` or `exit_dll` attempt does not complete | primary result plus exit attempt/result | still attempt unload/drain | fixture/runtime analysis; absence alone not blocker |
| 18 | `WF0_MODULE_UNLOAD_BLOCKED` | real/injected false or `free_library` attempt does not complete | prior stages and unload attempt/result | scanner process containment/drain | repair inside owner or return gate |
| 19 | `WF0_OUTPUT_NORMALIZATION_BLOCKED` | malformed/oversized/incomplete lifecycle, call, or census data | raw hash, last full attempt/lifecycle state, exact identities | discard raw/partial normalized bytes, drain | fix scanner/normalizer contract |
| 20 | `WF0_PROCESS_CLEANUP_BLOCKED` | owned descendant survives | exact transient identities; sanitized role record | preserve environment, no broad kill | operator/design resolves containment |
| 21 | `WF0_PROTECTED_FIXTURE_DRIFT` | before/after mismatch | content-free labeled mismatch | stop; do not repair protected state | operator investigation |
| 22 | `WF0_EVIDENCE_BLOCKED` | incomplete/unsanitized/unhashable packet or final source-manifest inequality | bounded valid inputs | remove partial staged evidence only | regenerate or rebuild/rerun exact proof as required |
| 23 | `RETURN_TO_DESIGN_GATE` | material model/envelope discovery | bounded read-only discovery evidence | stop implementation safely | fresh design/review/approval |

Optional `InitDll` or `ExitDll` absence is explicitly an observation. Only a malformed absence record, a false result from a present export, or another exact stage violation maps to a blocker.

## 18. Material-discovery stop conditions

Implementation immediately stops and returns `RETURN_TO_DESIGN_GATE` if any observation changes the owner map, lifecycle/transition set, mutation root, durability/recovery boundary, process/thread topology, record direction/payload law, identity law, security/privacy/license posture, exact fixture, primary claim, claim ceiling, path envelope, or proof matrix.

Slice-specific stop conditions are:

- the exact MinGW route cannot build the pinned fixture/probe;
- AGain must change or any substitute fixture is proposed;
- any VST3 SDK/submodule source must be patched;
- any of the 15 closed call attempts cannot be flushed and independently attributed before control enters the call;
- scanner lifecycle/call events cannot remain independently observable within the declared bounds;
- valid DLL-search configuration requires application, user, current-directory, altered-path, or ambient-`PATH` authority;
- class enumeration requires class instantiation;
- Runtime 4/Proton 11 identity or launch route must change;
- the accepted WR0 environment must be mutated or reused;
- a persistent product environment is required;
- first-prefix initialization requires a separate bootstrap workload;
- actual process topology materially differs from the declared owner model;
- a new IPC, C ABI, proxy, audio, GUI, Bitwig, Serum, authorization, network, manager, broker, or product-registry owner is required;
- current Bitwig preservation would require launch, downgrade, upgrade, repair,
  reconfiguration, project inspection, or a behavioral claim;
- HP1 `6.0.11` evidence would need to be treated as current `6.1` acceptance;
- an HP0, HP1, WR0, WR0A, historical evidence, or historical helper path would
  need to change to implement the current protected snapshot;
- implementation paths materially exceed the exact 40-path envelope;
- the exact 26-path implementation-source manifest cannot bind all governed source or reproduce at the final evidence-only head;
- dependency closure needs an undeclared runtime or changes distribution posture;
- the proof matrix cannot assign a failure to one owner;
- evidence would require retaining private/proprietary content.

A normal defect inside the approved owner/path/model is repaired there. It does not authorize adjacent work.

### Open unknowns carried into implementation

1. post-install executable/runtime-DLL identities;
2. actual MinGW link compatibility of current VSTGUI and explicit system library closure;
3. exact PE import/export/bundle roster;
4. byte versus normalized reproducibility result;
5. live AGain entry/exit/factory/class results;
6. scanner-specific Runtime/Proton process topology;
7. real successful unload under the fixture;
8. whether real unload failure is observable beyond the bounded adapter test.

Each unknown already has a proof row and blocker. None is permission to change design silently.

## 19. Implementation sequence

This is dependency order within WF0, not current authorization or a roadmap:

```text
1. Reverify exact authority, fixture, forbidden-process posture, historical
   packet/helper Git identities, current Bitwig `6.1` Flatpak projection, and
   every other protected snapshot label.
2. Under separate approval, establish WindowsToolchainLock at the exact user Flatpak commits.
3. Implement only paths 1–26 of the unchanged 40-path envelope and their deterministic tests.
4. Create one clean committed implementation head containing all 26 paths; generate and read back `linux-vst-bridge-wf0-implementation-source/v1` with the exact literal roster.
5. Run two clean offline Release builds only from that commit and manifest; independently verify PE, exports, imports, bundle, licenses, and comparison; publish the untracked artifact-set receipt.
6. Exercise the exact DLL-search, all-operation attribution mapping, source-owned crash/hang/release fixtures, no-instantiation tripwire, and other bounded negatives through production owners using only those artifacts.
7. Create a fresh payload-only WF0 stage and run the real held-gate proof through the first supervised Runtime/Proton scanner launch; prove readiness without load/mapping, then clean to zero and retire it.
8. Create another fresh payload-only stage and execute the positive AGain census through the exact gated Runtime/Proton route; the first scanner launch alone owns prefix initialization.
9. Drain to zero, capture bounded inputs, retire the disposable environment, and reverify protected equality.
10. Add only the 14 evidence paths numbered 27–40 in a later evidence-only commit; do not change source/configuration.
11. At the final PR head, regenerate the 26-record source manifest and require byte/digest equality with the manifest used for every build and live execution.
12. Publish and validate the fixed evidence packet; run diff/path/fence/link/hash checks and leave the implementation PR unmerged for independent audit.
```

No live evidence is accepted from dirty or uncommitted governed source. Any change to any of the 26 source/configuration records after live execution, including a Git mode change or an extra/missing tracked governed path, invalidates the artifacts and every live result and requires rebuilding and rerunning before evidence publication.

## 20. Adversarial design review disposition

```yaml
review_path: docs/slices/WF0/ADVERSARIAL_DESIGN_REVIEW_V3.md
reviewed_revision: wf0-design-v3
reviewed_commit: external_identity_from_design_commit
reviewed_design_blob: external_identity_from_design_commit
reviewed_design_sha256: external_identity_from_design_commit
review_result: review_required
prior_v2_review_github_id: 5082923871
prior_v2_review_result: DESIGN_CLEAR
unresolved_findings: fresh independent V3 adversarial review required
```

The V2 review remains immutable and does not approve V3. A fresh independent
reviewer must inspect the exact V3 commit, tree, Git blob, SHA-256, the full
unchanged V2 design inherited by this complete card, and the live reconciliation.
At minimum the reviewer must determine whether:

1. historical `6.0.11` evidence remains truthful and immutable;
2. current Bitwig `6.1` preservation is exact without becoming an HP1 or WF0
   behavior claim;
3. the existing `ProtectedFixtureSnapshot` owner, row 34, blocker, and WF0 paths
   fully own the amendment without a hidden shared-helper edit;
4. protection is revalidated before toolchain installation and every live run;
5. the primary claim, owner/state/process model, mutation envelope, 40 paths,
   35 proof rows, 23 blockers, and nonclaims remain unchanged.

The design-authoring context does not independently review or approve its own
card.

## 21. Approved design identity

The card cannot self-embed its own Git blob or SHA-256 without changing those identities. They are computed externally from the final design commit and must be copied verbatim into the later review and approval records.

```yaml
card_path: docs/slices/WF0/IMPLEMENTATION_DESIGN_V3.md
card_git_blob: external_identity_from_design_commit
card_sha256: external_identity_from_design_commit
design_revision: wf0-design-v3
amendment_basis_commit: 15523c69567d24b256cb3c65cb6f06bfa07854be
amendment_basis_tree: a6e2fc8c7a564f50e9033094fde847387478de62
fixture_reconciliation_path: docs/slices/WF0/BITWIG_6_1_FIXTURE_RECONCILIATION.md
approval_receipt_path: docs/slices/WF0/DESIGN_APPROVAL_V3.md
approval_receipt_identity: null
implementation_authorized: false
```

No V3 design approval exists. A later accepted approval receipt must bind this
exact revision, Git blob, SHA-256, amendment basis, reconciliation record,
current Bitwig identities, primary claim, 40-path envelope, 35-row proof matrix,
and 23-result taxonomy. `CURRENT_SLICE.md` must separately grant implementation
authority before any implementation action.

## 22. Implementation handoff

No implementation prompt or handoff is produced during this phase. The V2
implementation prompt and old implementation-branch basis are suspended. After
independent review, exact approval, approval-receipt retention, merge, and exact
`main` readback, a separate authority-owned handoff must include the new basis,
this card's external identities, approval receipt, one primary claim, exact
fixture, exact tracked/external mutation envelope, proof matrix, stop law,
explicit nonclaims, and PR posture. The paused implementation branch may then
be advanced only by the separately authorized authority transition. The handoff
must not use the full design dossier as an implementation task contract.

```text
implementation_authorized=false
```
