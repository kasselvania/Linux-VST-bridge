# Implementation Design Card — WF0

## 1. Identity and authority

```yaml
slice: WF0
slice_id: WF0
title: Supervised Windows VST3 Factory Census Probe
design_revision: wf0-design-v1
design_status: proposed_for_adversarial_review
repository: kasselvania/Linux-VST-bridge
basis_commit: 745ca63bdd8641ade85cb9a024c1dc842681192d
basis_tree: 275521adff574f165bb2b3e883c2ec909bae4c66
activation_head: 199569513ebba076e4fca90c75f7ae91f79c110d
activation_tree: 69d418d8c638bd8606b10d4e662d6c1a1cf25eff
selection_receipt_path: docs/slices/WF0/SLICE_SELECTION.md
selection_receipt_git_blob: 64721fbe3697f50f87cd16ae5fbf636f921143fc
current_slice_path: CURRENT_SLICE.md
current_slice_git_blob: 158e5bff3439a4881b3849f99bdde09445cc4fb7
authority_phase: reconnaissance_and_design
implementation_authorized: false
prepared_by: Codex design agent
prepared_at: 2026-09-01
```

The exact operator selection receipt says:

> I explicitly approve selecting WF0 — Supervised Windows VST3 Factory Census Probe and replacing the no-active-slice card with its bounded reconnaissance-and-design authority. This approval does not authorize implementation. Implementation requires a separate approved design revision.

This card proposes a design. It does not approve itself. No implementation, toolchain installation, build, Windows execution, or scan-environment mutation is authorized until a separate review and exact design-approval receipt are accepted and `CURRENT_SLICE.md` is changed by that later authority.

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
| Freedesktop SDK | `org.freedesktop.Sdk/x86_64/25.08`, version `freedesktop-sdk-25.08.16`, commit `b90ed309cc1d505dea48b6a2121c5dcfac22868120eee643b0596d31f96b9bb8` | live Flatpak readback | no; selected installed object is read-only |
| MinGW extension | `org.freedesktop.Sdk.Extension.mingw-w64/x86_64/25.08`, Flathub commit `f15a5a88eb09557f645860bfcb3c4a6bc267683fce06cb68436bd76376fca694`; absent at design time | Flathub remote metadata; source commit `345e5766c6cf8016cc62cb7755bd1711a7d739aa` | user-scope install only after separate approval |
| VST3 SDK | root `3cdf9ca5d1f5b1b21e0a86832aa4abe55607bd96`, tree `38343890fd1a0cedd48b7ec80ef17da15231b6c8`, recursive submodules in the identity ledger | HP0 dependency lock and clean live checkout | read-only |
| Positive fixture source | official AGain target; entry blob `13b920b4b7a74137301bf213cf048e96e82861d4`; CID blob `d32d1640ea187e718baba9cbb039d3a436d53cce`; target blob `f2616195f4f0b92b55ade45ac2fa448a4674ec79` | pinned `public.sdk` commit | source read-only; generated artifact only |
| Expected factory | vendor `Steinberg Media Technologies`; URL `http://www.steinberg.net`; email `mailto:info@steinberg.de`; flags `16`; three classes | pinned factory/version source | no source mutation |
| WF0 scan environment | initially absent `.wf0-factory-census.stage-<run-id>` below the declared environment root | new WF0 owner | only exact transaction root; disposable |

Immediately before implementation, build, and each live exercise, all identities above must be re-read. The implementation must also re-prove a clean repository/SDK, forbidden-process absence, no WR0 transaction sibling, exact Flatpak commits, exact runner files, scan-root absence, and a separate accepted record that grants implementation authority. This proposed card continues to state `implementation_authorized: false`. A drift is a blocker, not a repair invitation.

## 4. Reconnaissance findings

The complete record is [`docs/slices/WF0/RECONNAISSANCE.md`](RECONNAISSANCE.md).

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
| 10 | `ProtectedFixtureSnapshot` | accepted identities and allow-listed safe metadata before/after | labeled equality results and aggregate protected-state digest | none | drift -> `WF0_PROTECTED_FIXTURE_DRIFT`; never repairs or recursively inspects proprietary state | toolchain extension (intentional scope), vendor contents, credentials |

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
  -> environment_payload_verified
  -> environment_ready
  -> scanner_launched
  -> scanner_identified
  -> scanner_gated
  -> scan_terminal
  -> process_draining
  -> process_clean
  -> evidence_inputs_captured
  -> environment_retired
```

An unexpected existing stage never becomes an input. `environment_retired` requires marker-bound deletion, parent-directory durability, and exact root absence after zero owned descendants. On a cleanup failure, the root is retained for operator inspection and classified; it is never mistaken for a future empty environment.

### Scanner lifecycle: 23 named states

The scanner emits only these lifecycle names. Branch alternatives are mutually exclusive; not every name appears in a successful run.

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

The count is 23 distinct names. The success path is ordered exactly as written, selecting one entry branch, `factory_export_found`, `factory_obtained`, one exit branch, and ending in `scanner_completed`. A failure path records its last validated stage, performs applicable cleanup stages, and exits without fabricating skipped success stages.

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

### Windows module/factory transition law

1. The scanner validates its command contract, atomically writes the ready record, emits `readiness_announced`, and waits. It cannot resolve or load AGain before the exact gate is read.
2. On the scanner's single main thread, call `SetDefaultDllDirectories` and `LoadLibraryExW` on the exact copied module with `LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR | LOAD_LIBRARY_SEARCH_SYSTEM32`. The current directory and ambient `PATH` are not dependency authority.
3. Resolve optional `InitDll`, optional `ExitDll`, and required `GetPluginFactory` with exact case. Record presence separately.
4. If `InitDll` exists, call it once. `false` is `module_entry_failed`; do not call the factory. Absence is `module_entry_absent` and does not block.
5. If entry is absent or succeeds, require `GetPluginFactory`, call it once, and require a non-null `IPluginFactory*`.
6. Zero-initialize all output structures. Call `getFactoryInfo`; query only exact `IPluginFactory2` and `IPluginFactory3` IIDs. A query is supported only for `kResultOk` plus non-null. `kNoInterface` plus null is unsupported. Every inconsistent status/pointer pair blocks.
7. Require `0 <= countClasses <= 256`. Enumerate exact ordinals without sorting. Prefer `IPluginFactory3::getClassInfoUnicode`; on a non-success result, attempt factory 2 then factory 1 and retain the result sequence. If no tier succeeds, block at that ordinal.
8. Never call `createInstance` or any returned-class interface.
9. After all factory data is copied, release each `queryInterface` acquisition in reverse query order (`IPluginFactory3`, then `IPluginFactory2`) and release the base factory last. Alias pointer values do not suppress releases because each successful query owns one reference.
10. If `ExitDll` exists, call it exactly once for every successfully loaded module before unload, even on an earlier entry/factory failure, matching the official helper's cleanup posture. Record `false` as a cleanup failure without suppressing the original primary failure. Absence is an observation.
11. Attempt `FreeLibrary` only after all references are released and exit handling completes. A false result is `WF0_MODULE_UNLOAD_BLOCKED`; scanner process exit remains the supervisor's final containment.

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
| `81` | scanner invariant/internal failure | last validated stage's blocker; otherwise `WF0_SCANNER_LAUNCH_BLOCKED` |

Windows exception/NTSTATUS termination is never coerced to this table; the supervisor records an unsigned raw exit classification transiently and emits `WF0_SCANNER_LAUNCH_BLOCKED` or the blocker for the last validated stage.

### Legal transitions and failure ownership

| From | Operation | To | Required readback | Failure classification |
|---|---|---|---|---|
| `toolchain_available` | exact user install | `toolchain_installed_unverified` | deployed exact ref | `WF0_TOOLCHAIN_INSTALL_BLOCKED` |
| `toolchain_installed_unverified` | identity probes | `toolchain_exact` | commit/tool/target/runtime receipt | `WF0_TOOLCHAIN_INSTALL_BLOCKED` |
| `toolchain_exact` | configure/build twice | `build_comparison_validated` | two independent manifests | SDK/reference/scanner build blocker |
| `build_comparison_validated` | publish artifact manifest | `artifact_set_published` | atomic receipt readback | `WF0_EVIDENCE_BLOCKED` |
| `environment_absent` | create stage and marker | `environment_marker_durable` | marker and parent readback | `WF0_SCAN_ENVIRONMENT_BLOCKED` |
| `environment_marker_durable` | copy/verify artifacts | `environment_ready` | Windows paths and hashes | `WF0_SCAN_ENVIRONMENT_BLOCKED` |
| `environment_ready` | spawn exact launch root | `scanner_launched` | PID/start and root identity transiently | `WF0_SCANNER_LAUNCH_BLOCKED` |
| `scanner_launched` | validate ready/topology | `scanner_identified` | fresh complete descendant census | `WF0_PROCESS_IDENTITY_BLOCKED` |
| `scanner_identified` | atomic gate publication | `scanner_gated` | exact gate readback | `WF0_PROCESS_IDENTITY_BLOCKED` |
| `scanner_gated` | module/factory lifecycle | `scan_terminal` | bounded stages and exit | stage-specific blocker |
| `scan_terminal` | drain/cleanup | `process_clean` | zero owned descendants | `WF0_PROCESS_CLEANUP_BLOCKED` |
| `process_clean` | normalize/capture | `evidence_inputs_captured` | schema/identity readback | `WF0_OUTPUT_NORMALIZATION_BLOCKED` |
| `evidence_inputs_captured` | delete exact stage | `environment_retired` | absence and parent durability | `WF0_SCAN_ENVIRONMENT_BLOCKED` |
| `environment_retired` | publish packet | retained evidence | packet hashes and Git validation | `WF0_EVIDENCE_BLOCKED` |

### Forbidden transitions

- wrong/dirty SDK or toolchain -> build;
- unverified artifact -> scan environment;
- unknown/preexisting environment -> adopt/delete;
- readiness absent or topology stale -> gate;
- gate absent -> module load;
- entry failure -> factory call;
- missing/null factory -> class count;
- invalid count -> enumeration;
- class metadata -> `createInstance`;
- live factory references -> `ExitDll` or unload;
- live owned descendant -> environment deletion;
- failed normalization/protection equality -> evidence success;
- any WF0 state -> WR0 mutation, Bitwig, Serum, `.wine`, or Steam compatdata.

### Restart/retry reconciliation

Build roots are disposable and may be removed/recreated only after their safe-relative path and source-manifest identity are verified. A new supervisor scans only marker-bound WF0 stage roots. If an owned process identity survives, it revalidates exact root ancestry before scoped termination; if identity is uncertain, it stops and preserves the root. A marker-bound root with no process may be retained for bounded read-only diagnosis, then removed. Unknown roots are never adopted. Evidence staging is regenerated from retained bounded receipts; a partially published tracked packet is invalid until the fixed roster and hashes read back. The persistent Flatpak extension is reconciled by deployed commit and pre-install snapshot, not by an in-memory “installed by us” flag.

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
| 7 | marker durable | initialize compatdata and copy payload | partial prefix/runtime state or payload may exist | exact payload Windows paths/hashes and environment receipt | environment ready | `WF0ScanEnvironment` | production supervisor cleanup, then exact-root retirement; WR0 never touched |
| 8 | environment ready, no forbidden workloads | spawn Runtime/Proton/scanner | live exact-root descendants may exist before ready | bounded `/proc` census with PID/start identities | scanner launched | `WF0Supervisor` | scoped TERM/KILL after fresh identity readback; prove zero |
| 9 | exact ready/topology, gate absent | atomically publish gate | gate exists; scanner may immediately start loading module | temp write/fsync/rename/parent fsync/readback plus fresh topology | scanner gated | `ScannerProcessSession` | no rollback claim after gate; supervise to terminal/cleanup |
| 10 | gate accepted | open module, optional entry, factory/census | module memory and possibly factory references exist | ordered stage events and bounded scanner records | scan terminal or stage failure | `WindowsFactoryProbe` | release acquired references where reachable; exit/unload attempt; process containment |
| 11 | module loaded, all data copied | release refs, optional exit, unload | module may be exited/unloaded or cleanup failure recorded | exact exit/unload stages before process exit | scan terminal | `WindowsFactoryProbe` | preserve primary plus cleanup failures; supervisor contains process |
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

Gate publication:
  module code may execute after this point;
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
--module C:\wf0\fixture\again.vst3\Contents\x86_64-win\again.vst3
--module-sha256 <64-lowercase-hex>
--bundle-manifest-sha256 <64-lowercase-hex>
--ready C:\wf0\session\<session>.ready
--gate C:\wf0\session\<session>.gate
--max-classes 256
--stdout-cap 1048576
```

No alternate order, duplicate option, relative path, basename match, extra argument, or different module is accepted.

### Readiness/gate contract

Ready and gate files are UTF-8, LF-only, at most 1,024 bytes, and contain schema, session, scanner hash, module hash, bundle-manifest hash, and exact run ordinal. They are atomically written. The scanner emits the same binding in a bounded `readiness_announced` event and waits without loading the module. Immediately before the gate, the supervisor repeats its process census and revalidates the exact Runtime root, Proton role, Wine-hosted scanner vector, both complete ancestry chains, source/artifact identities, ready bytes/stdout, and gate absence.

### Bounds and deadlines

| Bound | Exact value |
|---|---:|
| observed process identities | 256 |
| process poll interval | 0.05 seconds |
| first-prefix spawn to readiness | 180 seconds |
| reused-stage spawn to readiness | 90 seconds |
| supervisor-held gate | 15 seconds |
| module open/entry/factory-info individual stage | 15 seconds |
| class enumeration | 30 seconds |
| exit/unload individual stage | 15 seconds |
| total scanner time after gate | 120 seconds |
| ordinary process drain | 20 seconds |
| TERM/KILL empty-scope cleanup | 10 seconds |
| stdout | 1,048,576 bytes |
| stderr | 65,536 bytes |
| lifecycle event count | 64 |
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
| accept scanner/fixture | implementation source manifest + toolchain receipt + sorted artifact path/mode/size/hash + PE architecture/import/export roster | before copy, after copy, before spawn | missing, changed, undeclared artifact |
| create/remove environment | canonical parent + exact 32-hex run ID + marker schema + marker hash + runner/artifact identities | before every mutation/deletion | unknown object, mismatched marker, live descendants |
| spawn scanner | environment identity + runtime/runner digest + exact ordered command + artifact hashes + session nonce | pre-spawn and readiness census | forbidden process, stale sibling, alternate vector |
| run Linux harness | `/usr/bin/python3` + exact version `3.13.5` + clean source-manifest identities; standard library only | preflight and process start | alternate interpreter, injected module path, unexpected dependency |
| publish gate | session nonce + ready bytes + scanner/module hashes + exact PID/start/ancestry + gate absence | one fresh census immediately before rename | disappeared/changed identity, stale ready, existing gate |
| load module | exact gate + canonical Windows path + bundle manifest + module SHA-256 | scanner validates immediately before `LoadLibraryExW` | relative/alternate path, hash mismatch, no gate |
| enumerate factory | live base factory acquisition + exact SDK ABI definitions + bounded count | within scanner thread | null base, invalid query result, count outside 0..256 |
| terminate process | session/run + exact Runtime root + exact PID/start + current ancestry/group | immediately before each signal | PID alone, name match, stale census, unrelated process |
| publish census | raw-output hash + scanner exit + stage timeline + schema + expected source fixture ID | normalization and packet readback | malformed/oversized/incomplete/mismatched data |
| claim preservation | before/after labeled protected snapshot equality + mutation-root receipt | after process clean and before packet publication | any drift, unknown item, unavailable comparison |

Class identity is the exact 16-byte TUID plus its canonical FUID rendering. A name, category, ordinal, module path, or vendor string is never sufficient identity.

## 10. Data, durability, recovery, and migration

| Data class | Owner | Location | Sensitive? | Durability | Backup/rollback | Migration law |
|---|---|---|---|---|---|---|
| toolchain deployed state | `WindowsToolchainLock` | user Flatpak installation | no credentials; shared user state | Flatpak deployment + commit readback | explicit exact-ref reversal only; no automatic removal of shared exact install | exact commit only; new commit requires design review |
| build roots | `ReferenceFixtureBuild` | `<HOME>/.cache/linux-vst-bridge/wf0/builds/<source-id>/{a,b}` | private path transient | disposable | delete exact owned root | recreate; no migration |
| verified artifacts | `ReferenceFixtureBuild` | `<HOME>/.cache/linux-vst-bridge/wf0/artifacts/<artifact-set-id>` | binaries, not credentials | atomic manifest and readback | regenerate from source | schema/version change requires design review |
| scan environment | `WF0ScanEnvironment` | declared `.wf0-factory-census.stage-<run-id>` root | potentially runtime metadata; open fixtures only | marker/file and parent durability where authority changes | delete exact owned root after zero descendants | never migrate/adopt; recreate |
| raw scanner output | `ScannerProcessSession` | bounded memory and WF0 session receipt staging | private paths/PIDs possible before sanitization | transient only | discard after normalized/evidence input hashes | raw schema mismatch blocks |
| normalized census | `FactoryCensusNormalizer` | evidence staging then `CENSUS.json` | sanitized | atomic file and schema readback | regenerate from bounded raw receipt within run; otherwise rerun | exact schema version; no implicit upgrade |
| process observations | `WF0Supervisor` | bounded transient memory then sanitized stage timeline | raw PID/cmdline transient | only role-based sanitized record retained | rerun; no PID restoration | topology schema change requires review |
| protected snapshot | `ProtectedFixtureSnapshot` | bounded transient values; equality/digest retained | no proprietary contents | before/after receipt | no repair | changed roster requires review |
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
- Maximum raw stdout, stderr, event count, event size, final record, and normalized output are the process bounds in section 8. Multiple final census records, data after the final record, a lifecycle sequence violation, or identity mismatch is malformed output.
- Native pointer values, handles, addresses, raw PIDs, unstable timestamps, and private paths are forbidden in the retained schema.

The transient raw stream is hashed and discarded after successful packet publication. The normalized `CENSUS.json`, sanitized `STAGE_TIMELINE.json`, and their hashes are retained.

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
wf0-exit-false
```

They implement only enough official factory ABI to assign scanner-stage ownership. They never become positive compatibility fixtures. Deterministic unload-failure mapping uses the scanner's narrow internal `Win32LoaderApi` test adapter; the production scanner always binds the real Win32 calls. No claim is made that a real valid `HMODULE` can be forced to make `FreeLibrary` fail under Proton.

### Build verification and reproducibility

Each A/B build independently verifies:

1. exact toolchain/SDK/source receipt and no network;
2. Release-only output and no debug-path leakage;
3. PE32+ x86_64 machine type for scanner and every DLL;
4. required/optional export tables;
5. recursively closed import table;
6. only declared Windows system imports plus exact locked extension runtime DLLs;
7. AGain bundle safe-relative roster, modes, sizes, and hashes;
8. scanner and negative-fixture artifact roster;
9. no symlink, device, FIFO, absolute path, traversal, or untracked third-party source;
10. no generated artifact inside Git.

Link flags explicitly request no inserted PE timestamp and source/build prefix mapping. The A/B comparison first requires byte SHA-256 equality. If bytes differ, the reproducibility normalizer may zero only declared PE COFF `TimeDateStamp` and checksum fields after proving those are the sole offsets that differ. It retains both raw hashes, exact differing offsets/field names, and a normalized digest. Any other difference is a build blocker. The design never promises byte identity before observation.

Expected dynamic GCC runtime candidates are `libgcc_s_seh-1.dll`, `libstdc++-6.dll`, and `libwinpthread-1.dll`; independent `objdump` closure decides which are actually needed. Each required DLL is copied from the exact extension into the scanner directory and the AGain module directory, hashed, and kept untracked. An unexpected non-system import is blocked. The build receipt includes all applicable license/notice identities.

## 13. Security, privacy, licensing, and proprietary material

### Threat model

| Threat | Boundary | Prevention | Negative proof |
|---|---|---|---|
| malicious or malformed DLL | Windows scanner process | disposable prefix, no credentials/network, bounded calls/output/time, supervisor containment | crash/hang/stage fixtures and zero cleanup |
| dependency substitution | Windows loader | copied exact artifacts, constrained DLL search, import/hash manifest | alternate DLL/hash rejection |
| stale process identity | host process table | PID + start ticks + complete ancestry + session binding | stale/unrelated sentinel test |
| prefix adoption/deletion | filesystem | exact canonical root, initial absence, durable ownership marker, safe-relative deletion | unknown marker/object refusal |
| output memory/disk exhaustion | scanner/supervisor/normalizer | fixed class/field/event/stream/process caps | excessive count/output fixtures |
| false compatibility claim | evidence boundary | exact fixture/version/capability schema and explicit nonclaims | packet validator rejects missing ceiling |
| credential/proprietary capture | diagnostics | open fixtures only, allow-list packet, raw streams transient, no vendor roots | sanitization scan and forbidden-token/path checks |
| build supply-chain drift | Flatpak/SDK/toolchain | exact deployed commits, clean submodules, offline configure/build, full paths | wrong commit/compiler/dirty SDK negatives |
| WR0 or user-state corruption | mutation boundary | separate WF0 roots; no writable path to WR0/Bitwig/Serum/Steam compatdata | before/after equality plus mutation-root audit |

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
| `build.py` / `verify.py` | `ReferenceFixtureBuild` and toolchain/artifact readback | immutable build receipt | scan runtime |
| `environment.py` | `WF0ScanEnvironment` | marker/create/copy/retire operations | WR0 or unknown-root mutation |
| `supervise.py` | `WF0Supervisor` and process session | launch/gate/stage/cleanup receipt | factory semantics |
| `normalize.py` | `FactoryCensusNormalizer` | census v1 or exact rejection | data repair/invention |
| `evidence.py` | `CensusEvidencePacket` and protection packet assembly | fixed evidence roster/hash closure | primary fact generation |
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

The implementation may use one clean source commit before live execution and one final evidence-only commit if governance authorizes that commit posture. Generated build products never enter these tracked paths.

## 15. Changed-path and external-mutation envelope

### Design-phase envelope

This current design branch differs from the selection basis only at:

```text
CURRENT_SLICE.md
docs/slices/WF0/SLICE_SELECTION.md
docs/slices/WF0/RECONNAISSANCE.md
docs/slices/WF0/IMPLEMENTATION_DESIGN.md
```

No implementation path above is authorized on this branch.

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

The accepted WR0 environment identity, its sibling roster, Runtime/Proton installed assets, WR0 contract source, `.wine`, Steam compatdata, SteamOS read-only state, HP0 publication/evidence, HP1 Bitwig state/evidence, Bitwig/Flatpak configuration, Serum/vendor material, other worktrees, and every repository path outside the envelope are protected. The implementation has no writable handle or path into them.

`ProtectedFixtureSnapshot` retains content-free labeled equality. It may read declared non-sensitive WR0 markers/manifests and safe metadata. It does not traverse credentials, registry/browser data, Serum, or other proprietary contents. For prohibited roots, preservation proof combines unchanged allow-listed identity records, pre/post metadata equality where safe, and an exhaustive WF0 mutation-root/process contract; it does not claim a recursive secret-content hash.

## 16. Proof matrix

The matrix has 29 rows because those are the distinct claim/failure ownerships required by this design, not because a target count was chosen. One production run may satisfy several rows only where its receipts map them explicitly.

| # | Claim | Positive proof | Negative/fault proof | Real fixture? | Production owner exercised? | Retained evidence | Ceiling |
|---:|---|---|---|---|---|---|---|
| 1 | exact toolchain deployed | commit/path/version/target/runtime readback | missing/wrong commit or tool | installed Flatpak | yes, `WindowsToolchainLock` | `TOOLCHAIN.md` | no compile claim |
| 2 | exact clean SDK | root/tree/submodule/clean manifest | wrong/dirty/missing submodule | pinned checkout | yes, dependency lock | `BUILD.md` | no binary claim |
| 3 | build is offline/nonambient | denied network, full compiler paths, cache audit | ambient compiler/package registry injection refused | build fixture | yes, build driver | `BUILD.md` | no reproducibility claim |
| 4 | two-build reproducibility | A/B raw hashes or allowed-field normalized digest | undeclared byte difference blocks | real scanner/AGain | yes, build/verify | `BUILD_MANIFEST.json` | truthful declared level only |
| 5 | scanner is PE32+ x86_64 | independent file/objdump machine readback | wrong-machine scanner | real scanner | yes, verifier | `BUILD.md` | no runtime claim |
| 6 | AGain bundle/exports/imports exact | roster, module PE, required/optional exports, closed imports | missing/extra roster/export/import | real AGain | yes, verifier | `BUILD_MANIFEST.json` | no factory-call claim |
| 7 | missing fixture is refused | verified artifact root succeeds | remove/rename fixture in disposable test root | copied fixture | yes, env/supervisor | `NEGATIVE_TESTS.md` | no module open |
| 8 | wrong PE architecture is refused | x86_64 fixture admitted | repository-owned non-x86_64 header fixture rejected before launch | source-owned negative | yes, verifier | `NEGATIVE_TESTS.md` | admission only |
| 9 | wrong scanner/module hash is refused | exact hashes pass before/copy/spawn | one-byte copy mutation refused | copied artifacts | yes, env/supervisor | `NEGATIVE_TESTS.md` | no semantic claim |
| 10 | complete AGain factory census | full 23-state-valid path, exact metadata/three ordered classes, exit/unload 0 | any expected-field/order mismatch blocks | exact AGain + Runtime/Proton | yes, all production owners | census/timeline | factory boundary only |
| 11 | required export ownership | AGain export found | `wf0-missing-factory` | source-owned negative | yes, probe | `NEGATIVE_TESTS.md` | no factory obtained |
| 12 | null factory ownership | AGain non-null | `wf0-null-factory` | source-owned negative | yes, probe | `NEGATIVE_TESTS.md` | no class query |
| 13 | absent optional entry is observation | AGain entry result retained | `wf0-no-entry` completes factory path without entry blocker | source-owned negative | yes, probe | `NEGATIVE_TESTS.md` | absence not generalized |
| 14 | entry false is stage failure | AGain entry succeeds | `wf0-init-false` | source-owned negative | yes, probe | `NEGATIVE_TESTS.md` | cleanup still attempted |
| 15 | factory-info failure owned | AGain factory info exact | `wf0-factory-info-false` | source-owned negative | yes, probe | `NEGATIVE_TESTS.md` | no class claim |
| 16 | factory-version/tier fallback exact | support booleans and selected tier retained | factory 3/2 non-success variants fall back or block per contract | AGain + source-owned negative | yes, probe/normalizer | census/negative | only IPluginFactory1/2/3 |
| 17 | invalid/excessive count rejected | AGain count 3 | negative and 257-count fixtures | source-owned negative | yes, probe | `NEGATIVE_TESTS.md` | no allocation/enumeration |
| 18 | ordinal class failure owned | all AGain ordinals succeed | `wf0-class-info-false` at fixed ordinal | source-owned negative | yes, probe | `NEGATIVE_TESTS.md` | no partial census success |
| 19 | duplicate class ID rejected | three unique AGain raw TUIDs | `wf0-duplicate-class-id` | source-owned negative | yes, probe/normalizer | `NEGATIVE_TESTS.md` | no dedup repair |
| 20 | malformed/oversized output rejected | bounded canonical output | truncation, duplicate final, invalid UTF, cap+1 stream | bounded raw test inputs | yes, normalizer/supervisor | `NEGATIVE_TESTS.md` | no data recovery |
| 21 | scanner crash classified by stage | ordinary exits | entry/factory/class crash variants | source-owned negative under real runner | yes, supervisor/probe | timeline/negative | process containment only |
| 22 | scanner timeout classified by stage | stages within deadlines | entry/factory/class hang variants | source-owned negative under real runner | yes, supervisor/probe | timeline/negative | no real-time claim |
| 23 | absent optional exit is observation | AGain exit retained | no-exit variant completes/unloads | source-owned negative | yes, probe | `NEGATIVE_TESTS.md` | absence not generalized |
| 24 | exit false owned | AGain exit true | `wf0-exit-false` | source-owned negative | yes, probe | `NEGATIVE_TESTS.md` | unload still attempted |
| 25 | unload result mapping | AGain `FreeLibrary` true | internal production-loader adapter returns false deterministically | adapter + real AGain success | same module owner | unit receipt + live timeline | no claim real valid-handle failure inducible |
| 26 | exact cleanup and unrelated survival | zero owned descendants after success/fail | gate-held and stage-failure cleanup with same-family sentinel | real Runtime/Proton | yes, supervisor | `LAUNCH_AND_PROCESS.md` | no global process cleanup |
| 27 | unknown environment refused | exact marker root accepted | forged/missing/mismatched marker preserved untouched | filesystem fixture | yes, environment owner | `NEGATIVE_TESTS.md` | no recovery/adoption |
| 28 | accepted WR0/protected state unchanged | before/after equality and mutation-root audit | deliberate protected-snapshot mismatch blocks packet | live safe readback | yes, protection/evidence | `PRESERVATION.md` | no secret-content inspection |
| 29 | evidence is bounded/sanitized/hash-closed | fixed roster, sanitizer, hashes, Git diff | forbidden token/path/binary/extra file blocks | retained packet | yes, evidence owner | `SANITIZATION.md`, hashes | evidence is not broader compatibility |

## 17. Failure and blocked-result taxonomy

Exactly 22 blocked results exist. Secondary cleanup failures are retained beside the primary stage failure; they never overwrite it.

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
| 16 | `WF0_MODULE_EXIT_BLOCKED` | present `ExitDll=false` | primary result plus exit result | still attempt unload/drain | fixture/runtime analysis; absence alone not blocker |
| 17 | `WF0_MODULE_UNLOAD_BLOCKED` | real or injected unload false | prior stages and unload result | scanner process containment/drain | repair inside owner or return gate |
| 18 | `WF0_OUTPUT_NORMALIZATION_BLOCKED` | malformed/oversized/incomplete data | raw hash, last stage, exact identities | discard raw/partial normalized bytes, drain | fix scanner/normalizer contract |
| 19 | `WF0_PROCESS_CLEANUP_BLOCKED` | owned descendant survives | exact transient identities; sanitized role record | preserve environment, no broad kill | operator/design resolves containment |
| 20 | `WF0_PROTECTED_FIXTURE_DRIFT` | before/after mismatch | content-free labeled mismatch | stop; do not repair protected state | operator investigation |
| 21 | `WF0_EVIDENCE_BLOCKED` | incomplete/unsanitized/unhashable packet | bounded valid inputs | remove partial staged evidence only | regenerate/rerun exact proof |
| 22 | `RETURN_TO_DESIGN_GATE` | material model/envelope discovery | bounded read-only discovery evidence | stop implementation safely | fresh design/review/approval |

Optional `InitDll` or `ExitDll` absence is explicitly an observation. Only a malformed absence record, a false result from a present export, or another exact stage violation maps to a blocker.

## 18. Material-discovery stop conditions

Implementation immediately stops and returns `RETURN_TO_DESIGN_GATE` if any observation changes the owner map, lifecycle/transition set, mutation root, durability/recovery boundary, process/thread topology, record direction/payload law, identity law, security/privacy/license posture, exact fixture, primary claim, claim ceiling, path envelope, or proof matrix.

Slice-specific stop conditions are:

- the exact MinGW route cannot build the pinned fixture/probe;
- AGain must change or any substitute fixture is proposed;
- any VST3 SDK/submodule source must be patched;
- scanner stages cannot remain independently observable;
- class enumeration requires class instantiation;
- Runtime 4/Proton 11 identity or launch route must change;
- the accepted WR0 environment must be mutated or reused;
- a persistent product environment is required;
- actual process topology materially differs from the declared owner model;
- a new IPC, C ABI, proxy, audio, GUI, Bitwig, Serum, authorization, network, manager, broker, or product-registry owner is required;
- implementation paths materially exceed the exact 40-path envelope;
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
1. Reverify exact authority, fixture, forbidden-process posture, and protected snapshots.
2. Under separate approval, establish WindowsToolchainLock at the exact user Flatpak commits.
3. Implement only the exact 40-path source/tool envelope and its deterministic tests.
4. Create a clean implementation source commit and freeze its sorted Git source manifest.
5. Run two clean offline Release builds; independently verify PE, exports, imports, bundle, licenses, and comparison.
6. Publish the untracked artifact-set receipt; reverify protected state.
7. Exercise source-owned negative fixtures through production probe/supervisor/normalizer owners.
8. Create a fresh exact WF0 scan stage and execute the positive AGain run through the gated Runtime/Proton route.
9. Drain to zero, capture bounded inputs, retire the disposable environment, and reverify protected equality.
10. Publish and validate the fixed evidence packet; commit only sanitized evidence/source.
11. Run diff/path/fence/link/hash checks and leave the implementation PR unmerged for independent audit.
```

No live evidence is accepted from dirty or uncommitted governed source. The final evidence-only commit may not change scanner, supervisor, toolchain, fixture, or normalizer source without invalidating and rerunning all live evidence.

## 20. Adversarial design review disposition

```yaml
review_path: absent
review_identity: null
review_result: review_required
findings_resolved: 0
unresolved_findings: fresh independent adversarial review required
```

The reviewer must use a fresh independent context and review the exact Git blob/SHA-256 of this card. This design branch must not create the review record itself.

## 21. Approved design identity

The card cannot self-embed its own Git blob or SHA-256 without changing those identities. They are computed externally from the final design commit and must be copied verbatim into the later review and approval records.

```yaml
card_path: docs/slices/WF0/IMPLEMENTATION_DESIGN.md
card_git_blob: external_identity_from_design_commit
card_sha256: external_identity_from_design_commit
design_revision: wf0-design-v1
approval_receipt_path: docs/slices/WF0/DESIGN_APPROVAL.md
approval_receipt_identity: null
implementation_authorized: false
```

No design approval exists. A later accepted approval receipt must bind this exact revision, Git blob, SHA-256, selection basis, primary claim, 40-path envelope, 29-row proof matrix, and 22-result taxonomy. `CURRENT_SLICE.md` must separately grant implementation authority before any implementation action.

## 22. Implementation handoff

No implementation prompt or handoff is produced during this phase. After independent review and exact approval, a separate authority-owned handoff must include the accepted main commit/tree, this card's external identities, approval receipt, one primary claim, exact fixture, exact tracked/external mutation envelope, proof matrix, stop law, explicit nonclaims, and PR posture. It must not use the full design dossier as an implementation task contract.

```text
implementation_authorized=false
```
