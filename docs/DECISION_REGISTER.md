# Decision Register

Decisions are separated into **accepted**, **provisional**, and **open**. Implementation convenience does not silently change their state.

## Accepted product and architecture rulings

### D-001 — Product boundary

**Decision:** Build a managed compatibility platform, not merely a bridge binary or Proton launcher.

**Consequences:** Installation, authorization handoff, scanning, organization, publication, diagnostics, update, repair, and rollback are first-class product components.

### D-002 — Native/Windows boundary

**Decision:** A native Linux DAW loads a native Linux proxy. A supervised Windows host loads the Windows plug-in under a pinned runner. IPC crosses the boundary.

**Rejected alternative:** Attempt to load a Windows VST3 DLL directly in Bitwig through Proton.

### D-003 — Primary language

**Decision:** Rust is the primary product language.

**Rust owners:** model, manager, broker, runner/environment registry, installer supervision, profiles, scanner orchestration, publication, diagnostics, transport core, CLI, and likely UI.

### D-004 — VST3 SDK language edge

**Decision:** Use narrow C++20 shells for the native Linux VST3 proxy and Windows VST3 host, connected to Rust through a small explicit C ABI.

**Reason:** Official SDK/object-model alignment and reduced first-product risk given current Rust binding coverage and licensing uncertainty.

**Revisit condition:** A complete, maintained, permissively licensed Rust host/proxy binding is independently proved against required Bitwig/Serum interfaces without widening risk.

### D-005 — First real plug-in fixture

**Decision:** Serum 2 VST3 is the first real commercial plug-in target.

**Clarification:** Open SDK/reference modules are test infrastructure, not a replacement target.

### D-006 — Kontakt role

**Decision:** Kontakt is a later hostile-systems fixture for Native Access, large/relocatable content, activation, editions, third-party libraries, and legacy behavior.

**Rejected alternative:** Make Kontakt the first bridge implementation target.

### D-007 — Detached editor first

**Decision:** First usable editor mode is a supervised detached top-level window. Embedded or captured/streamed presentation is later and separately proved.

### D-008 — Exact versioned compatibility

**Decision:** Compatibility profiles and support claims bind exact plug-in build, license channel, runner, environment, proxy/protocol, DAW, sandbox, and capability matrix.

### D-009 — No arbitrary profile code

**Decision:** Compatibility profiles are declarative, closed-schema data. They cannot contain arbitrary scripts or commands.

### D-010 — No automatic successor

**Decision:** Completion or merge of one slice does not automatically authorize the next. A separate status closure returns the repository to no-active-slice posture, and the technical lead performs explicit successor analysis.

## Accepted proof-boundary rulings

### D-011 — Native VST3 host boundary accepted

**Decision:** HP0 and HP1 establish the exact current native Linux boundary on the accepted Steam Deck fixture: a repository-owned VST3 can be built, officially validated, published under `~/.vst3`, loaded inside the exact Bitwig Flatpak sandbox, discovered by normal Bitwig launches, and admitted as an exact native instance.

**Nonclaims:** No Windows plug-in, bridge, audio, automation, project recall, or general Linux compatibility follows.

### D-012 — Controlled installed-runner boundary accepted

**Decision:** WR0 establishes a repository-owned controlled process lane through the exact installed Steam Linux Runtime 4 and Proton 11 assets into an isolated project-owned Windows environment.

**Accepted laws:** exact runner lock, exact command identity, causal process gate, complete descendant ownership, scoped cleanup, reversible pre-commit replacement, durable post-commit authority, predecessor retirement, and evidence finalization.

**Nonclaims:** The installed runner is not selected as the final product runner and no Windows VST3-hosting or real-time claim follows.

## Accepted development-process rulings

### D-013 — Slice selection and implementation authorization are separate

**Decision:** Selecting a high-risk slice authorizes bounded reconnaissance and design only. Implementation requires a second approval bound to one exact reviewed implementation design revision.

**Reason:** A detailed task prompt is not a substitute for an owner/state/fault/proof design.

### D-014 — Mandatory implementation-design gate

**Decision:** A design card is mandatory when work introduces or materially changes ownership, state, durable mutation, transactions/recovery, process supervision, protocols, real-time behavior, reentrancy, identity/authorization, security/privacy/licensing, third-party runtime behavior, persistent user data, or compatibility claims.

**Waiver:** Must be explicit and positively justified. Small scope or model capability is not sufficient.

### D-015 — Adversarial design review precedes high-risk implementation

**Decision:** A fresh context that did not author the design card must attack owner completeness, complete state space, fallible-operation boundaries, physical-state recovery, topology, identity, security, proof strength, and claim ceiling before implementation authorization.

### D-016 — Material discovery returns to design

**Decision:** Real evidence that changes an approved owner, state, mutation root, durability boundary, process/thread topology, protocol, identity rule, security/licensing posture, fixture, claim, changed paths, or proof matrix stops implementation and returns the slice to the design gate.

**Rejected behavior:** Patch forward and rewrite the design afterward.

### D-017 — Independent pre-PR implementation audit

**Decision:** A fresh context audits the exact candidate head against the approved design before technical-lead PR review.

**Outcomes:** implementation repair, return to design gate, clear, or blocked.

### D-018 — Fault-boundary completeness rule

**Decision:** Every high-risk design must state what is physically true when each fallible operation succeeds and the next one fails. Every externally observable intermediate state belongs in the design state machine.

### D-019 — Physical state outranks stale memory

**Decision:** Recovery and cleanup use exact durable records and physical readback. In-memory booleans, names, paths, or summaries are not sole authority.

### D-020 — Proof matrix binds claims to production behavior

**Decision:** Every claim maps to positive proof, negative/fault proof, real or synthetic fixture, production-helper coverage, retained evidence, and claim ceiling. Test names and toy assertions are insufficient by themselves.

### D-021 — Reusable next-slice selection prompt

**Decision:** `docs/prompts/CHOOSE_NEXT_SLICE.md` is the default human-facing technical-lead prompt after status closure. It performs analysis only, recommends exactly one slice, classifies the design gate, drafts the selection receipt, and emits the exact operator approval sentence.

## Provisional technical choices

### P-001 — Runner shape

**Choice:** An immutable audio-oriented Wine/Proton-derived runner, independent from Steam, with side-by-side revisions.

**Not yet selected:** exact upstream base, patch set, graphics stack, synchronization stack, container use, and distribution packaging.

### P-002 — Audio transport

**Choice:** Preallocated shared memory for audio and bounded event data; separate synchronization/control/callback channels.

**Not yet selected:** exact primitive, ring/buffer topology, queue depth, or codec.

### P-003 — Broker

**Choice:** A project-owned local broker/supervisor launches exact bindings and owns host process groups, health, shared memory, editors, and cleanup.

**Not yet selected:** long-lived service versus on-demand lifetime; sandbox placement.

### P-004 — Environment sharing

**Choice:** Vendor/compatible-family environments by default, with profile-declared isolation or sharing. No global prefix.

**Not yet selected:** exact Xfer environment policy after Serum fixture observation.

### P-005 — Canonical state storage

**Choice:** Transactional structured registry with content-addressed manifests; SQLite is a likely first implementation.

**Not yet selected:** exact database/schema and migration law.

### P-006 — Initial user surface

**Choice:** Typed CLI plus structured diagnostics before committing to a GUI toolkit.

**Nonclaim:** CLI is not the intended final ordinary-user experience.

### P-007 — Flatpak publication

**Choice:** Current exact Bitwig accepts a standard user VST3 publication. A matching Linux Audio extension and narrow host-broker shape remain separate production packaging decisions.

## Open decisions

### O-001 — First post-DG0 product slice

After DG0 closes, use the exact next-slice selection process against accepted SR0/HP0/HP1/WR0. Likely candidates include Windows VST3 module reconnaissance/scanning, a narrow Windows host factory census, or another prerequisite design/reconnaissance slice. None is selected by DG0.

### O-002 — Exact product runner base

Options include:

- selected current Wine Staging build;
- custom Proton-derived build;
- UMU-launched Proton for experimentation;
- another maintained audio-oriented Wine base.

Decision evidence must include install/auth, process/IPC, graphics, synchronization, performance, reproducibility, and distribution obligations. WR0's installed Proton fixture is evidence, not final selection.

### O-003 — IPC codec and reentrancy mechanism

Need proof of typed coverage, bounded parsing, recursion, thread affinity, multiple in-flight calls, and diagnostics.

### O-004 — Flatpak broker placement

Options include:

- all runtime pieces inside the DAW sandbox;
- narrow host broker;
- native DAW/user session for non-Flatpak cases.

### O-005 — User-interface toolkit

Evaluate after canonical service/API and installation event model exist. Required qualities: Linux/Flatpak distribution, accessible controls, small-screen support, foreground vendor-window coordination, and rich diagnostics.

### O-006 — Open-source/commercial split

Questions:

- open bridge/protocol/runner with paid manager/profiles/support;
- fully open core plus commercial service;
- proprietary clean bridge with required open-source runtime compliance;
- contributor and vendor adoption implications.

No repository-wide license is selected until this is resolved.

### O-007 — Product name

`Linux-VST-bridge` is technical only. Select a vendor-neutral consumer brand after trademark and affiliation review.

### O-008 — VST3 Rust binding re-evaluation

Before implementing additional C++ shells, inspect current permissive bindings and host libraries again. Change only from evidence, not language preference.

### O-009 — Editor beyond detached mode

Candidates include:

- X11/XWayland embedding;
- native presentation using newer VST3 rendering/input interfaces where supported;
- captured/streamed editor surface with input forwarding;
- detached-only support for some profiles.

### O-010 — Process grouping law for plug-in hosts

Determine exact module/instance grouping through VST3 semantics, Bitwig sandbox modes, vendor communication needs, performance, and crash containment.

### O-011 — State encryption and sensitive environment backups

Define encryption, key storage, exports, machine-binding warnings, and vendor-license boundaries before user backups are productized.

### O-012 — Compatibility-profile publication and trust

Define review, signatures, evidence links, withdrawal, third-party contribution, and local override posture.

### O-013 — First second-vendor fixture

Select only after Serum reveals which architectural assumptions need counter-pressure.

## Accepted reconciliation rulings

### D-022 — Final WR0 repair reconciliation accepted

**Decision:** WR0A reconciles current repository truth to the exact post-merge final WR0 repair represented by archive commit `52be94316664f88a19df630e164105a0ca50b875`, without rerunning or mutating the existing live WR0 environment.

**Accepted implementation:** Design authority merge `5aa757e46f6e8c15751743b68a8504386fbe840c`; implementation PR #13; reconciliation-owner commit `c78e5845ccae7ca0f5d1e1c802f1839d519ee096`; exact archive-adoption commit `60f05fb97ae1438a10b5360f47b5812fabf131a0`; reviewed evidence head `48b66e87f8e65a8e8e57832039ec567452ed857b` / tree `d46b907ecac1df970d83d364884b8fdd2ed57d82`; merge `8e6a5e55a4c731defd518618286adb308d9640ed` / tree `d46b907ecac1df970d83d364884b8fdd2ed57d82`.

**Accepted source and evidence:** Exactly 13 final-repair archive blobs replace their stale merged counterparts; 11 shared WR0 path/mode/blob identities remain exact; archived `CURRENT_SLICE.md` is excluded; the canonical WR0 packet is the archive's exact 14-file packet; and a separate eight-file WR0A packet records the reconciliation. The final WR0 contract-source digest is `c6543004fbcd70252393f0e0cab4f9ad7a31e85f433ce3e903e690244cc79541`; the WR0A reconciliation-source digest is `c0675f824d2f4cd12b84486c5378a7af65a582cf07ad06ff06a1ed5e2cc0d9c8`.

**Accepted live join:** Read-only Deck verification matched transaction `wr0-20260901T053317Z-183afd5bbf0736e4`, environment identity `d3ed38d7ed53e9a5973cbf22fc504f2479dbdd15616fe1bfc1d5e1cd0bf5f4c4`, runner/runtime identity `2d64df1d36786ca2d0e955c553005423dc2b5bdd714bd0a17872622e33912547`, workload `4518ca37b8d7e005f01b441de5273447b70fa7c8c9c3d2b9c194a91782cfecac`, and retired record `917c939c605f457376b0bdd53e06d252d4aecee4b39728850b2e1a61aa102b3e`. No live workload or live-environment mutation occurred.

**Claim ceiling:** WR0A proves repository/live-state reconciliation only. It does not prove Windows VST3 factory loading or hosting, Serum authorization or operation, Bitwig scanning of a Windows plug-in, audio, parameters, state transport, GUI/editor behavior, IPC, shared memory, real-time safety, packaging, Steam-independent distribution, another DAW, or general Linux compatibility.

**Successor:** None selected. The next product slice must be chosen through `docs/prompts/CHOOSE_NEXT_SLICE.md` from the post-closure `main` commit and tree.

## Accepted Windows VST3 boundary rulings

### D-023 — Supervised Windows VST3 factory boundary accepted

**Decision:** WF0 establishes the first exact Windows VST3 object-model boundary on the accepted Steam Deck fixture. A repository-owned supervised Windows x86_64 probe, built from one exact source identity through the supported Windows Server 2022 / Visual Studio 2022 / MSVC lane, loaded the pinned official AGain VST3 through the accepted Runtime 4 / Proton 11 route, obtained its factory, retained exact factory metadata and the complete ordered three-class census, released every acquired factory interface, called the applicable module exit, unloaded cleanly, drained its owned processes, retired its disposable environments, and preserved protected state.

**Accepted implementation:** Immutable design `wf0-design-v7`, commit `1d13fefc60ad6c2c49e384cd30631f60be2a3de2`, tree `752b885643c730378ceefab99c7d7ec9277fdf56`, design blob `3c8ac56cfaa8c95cb99fa327b392ed447453a87b`, SHA-256 `844d646509933516ff60eeb2c6213cd1c2e89a8d22b5c986ac986fb104a19d77`; design review `5084789559`; design-authority merge `17646ff1cd5342d58ecf9346e26fad6f963f8a6a`; authority-readback basis `ce049eb410d4cff91de13fdb8bf4f0a3c4b03ece` / tree `2a782b5bdba4021dcbfeb5d2df5d9ff165936542`; implementation PR #25; source commit `8b76ab886fd75079c72e3f820781beb5d1b36ae9` / tree `829aae74e221a169ccfbb46387004b5edb04ba37`; reviewed evidence head `0096010a36ebf31a36149064d64059142ce7cfed` / tree `58401f5b9d3caab9ffe53155fb2f0517d1801427`; technical-lead review `5090768080`; merge `e694cc84344394553c4a3eff6b13f34226b368ae` / tree `58401f5b9d3caab9ffe53155fb2f0517d1801427`.

**Accepted build and custody:** The canonical 26-record source-manifest digest is `03c3c017f7d6eb357ae657e992ef3c3988932f6870ac3a18192dfd9e343ee05f`. Accepted workflow run `33601279364` used `windows-2022`, Visual Studio Enterprise `17.14.37614.0`, MSVC `19.44.35228`, linker `14.44.35228.0`, toolset `v143`, Windows SDK `10.0.19041.0`, and CMake `3.31.6`. Two distinct Release roots produced a byte-identical 48-path comparison. The scanner SHA-256 is `36643c2447811b52e1ad1455eb47e0ced9b5f5bf03849f9625d80979d7c5798f`; the AGain module SHA-256 is `60aa9ff6b9918d4330449e7b3ab34b588dd93cba09f37413a3cd91f6e7d2e18f`; artifact-manifest identity is `217d38dddb5e8ae4ee6b60245cc03b3174710cf2692e2e6e1a4c4ea1c04b7684`; exact Actions artifact ID is `9835459546`; its upload, REST, and raw-wrapper digest bytes join at `7ccb8f53aee02748d98abaa641a53e479ebd6f1d7b8bf381c1c2f11644d44d53`. These establish exact custody, not trusted-builder, provenance, signing, or release claims.

**Accepted runtime proof:** A 15-second held gate prevented module loading. The complete 22-module negative family assigned exact module-entry, factory, class-enumeration, release, exit, and unload failures, including crash and timeout attribution. The positive run completed all 15 paired calls, retained `create_instance_called=false`, observed `IPluginFactory`, `IPluginFactory2`, and `IPluginFactory3`, and returned this exact factory order: `84E8DE5F92554F5396FAE4133C935A18` (`AGain VST3`), `D39D5B65D7AF42FA843F4AC841EB04F0` (`AGain VST3Controller`), and `41347FD6FED64094AFBB12B7DBA1D441` (`AGain SideChain VST3`). Reverse release returned `2`, `1`, and `0`; `ExitDll` and `FreeLibrary` succeeded; all owned descendants drained; every disposable environment retired; the accepted WR0 environment and Bitwig 6.1 protected projection remained exact; and the Deck performed no GitHub operation.

**Architecture consequence:** The Mac-control / supported-Windows-build / Steam-Deck-execution split is accepted as the exact WF0 proof topology. It demonstrates that Windows artifacts and exact implementation source can reach a rootless SteamOS execution fixture without making the Deck a GitHub or Windows-build authority. It does not yet select the final product build, distribution, broker, proxy, or runner architecture.

**Claim ceiling:** WF0 ends before class instantiation. It does not establish component/controller lifecycle, host context, connection points, buses, parameters, events, state, processing, audio, timing, automation, presets, GUI/editor behavior, native proxy publication, IPC, shared memory, Bitwig hosting of a Windows plug-in, Serum operation or authorization, packaging, signing, release suitability, product-runner selection, another plug-in, another DAW, or general Windows VST3/Linux compatibility.

**Successor:** None selected. The next bounded product slice must be chosen through `docs/prompts/CHOOSE_NEXT_SLICE.md` from the post-closure `main` commit and tree.

### D-024 — Supervised Windows VST3 processor component boundary accepted

**Decision:** WC0 establishes the first exact instantiated Windows VST3 processor-component lifecycle on the accepted Steam Deck fixture. The existing supervised Windows host created the pinned AGain processor class directly as `IComponent`, verified its exact declared controller class ID before initialization, initialized it with one minimal repository-owned `IHostApplication`, terminated and released it, proved component and host-object quiescence, and only then completed the inherited factory/module shutdown and physical cleanup.

**Accepted implementation:** Immutable design `wc0-design-v2`, commit `064db624056f0fdf4daeda3b5ae394ab6210bee2`, tree `6d7baf83e29704e803a76106c36d346cdbf7be03`, design blob `df31b8467af9dcd97bc06b6afdf5b4b8d6be7018`, SHA-256 `ca68cde6f68b02320e3c950b445aca9db99fdac1301fa7dfb50d510f83c7d78c`; design review `5092052158`; design-authority merge and implementation basis `333b66f6aa689f01bb5c025b587ab7e469780524` / tree `d9aaae75d384c7b29313adde1a71049635615366`; implementation PR #29; source commit `9c0096930df86fc5b171cdebec40b306a198316a` / tree `e60286740a3aff7589e0dc9bb3b278e68a23374e`; evidence head `77bb40dbf35e19fd93b9f79b5286a43d56b9cc21` / tree `43fab0b6341e2549775b7f4223965031521ff338`; technical-lead review `5093764278`; implementation merge `cb831c38e1be88f4bb6a0ab6f2fca2d94164891b` / tree `43fab0b6341e2549775b7f4223965031521ff338`.

**Accepted build and custody:** The canonical 19-record source-manifest digest is `38699a1d2026cb1078a569dc1997122b0111c78e608f294777dcf4afc49c8b25`. Accepted workflow run `33666394555`, attempt 1, used the accepted Windows 2022 / Visual Studio 2022 / MSVC lane and produced scanner SHA-256 `51b899b7936921b24265ead9ff12180f14249d49b532ead9a18414559f5f83f7`. The exact AGain module retained SHA-256 `60aa9ff6b9918d4330449e7b3ab34b588dd93cba09f37413a3cd91f6e7d2e18f`; exact Actions artifact ID is `9861033341`; its upload, REST, and raw-wrapper digest bytes join at `db23a2a1781e9eb88dc43fbe9599cb74ff14f7c4f67a2884ca83113a274ff642`; artifact-manifest/cache identity is `25bd47471f01ef57b06b3c8232bb6cc7e40c767f281c30186fa5426130f8ae82`. These preserve the accepted Mac-build/Deck-execution custody split and make no signing, provenance, or release claim.

**Accepted object lifecycle:** The exact processor logical CID is `84E8DE5F92554F5396FAE4133C935A18`; requested `IComponent` logical IID is `E831FF31F2D54301928EBBEE25697802`; the zero-initialized controller output returned exact controller logical CID `D39D5B65D7AF42FA843F4AC841EB04F0` before initialization. Create, initialize, terminate, component release, and host-owner release completed successfully. Component release returned zero, host references followed `1 -> 2 -> 1 -> 0`, all nine object-quiescence facts were true, and the positive trace closed all 20 started calls with 20 completions and no operation in flight. No controller was created and no forbidden component method was called.

**Accepted fault and cleanup law:** Twenty deterministic owner/attribution checks and eighteen focused live exercises established exact create/result-pointer, controller-ID, host-context, initialize, terminate, and release ownership. A nonzero component release enters `component_retirement_incomplete` and is never retried. Nonzero release, unresolved host references, or timeout/crash with unproved quiescence suppress factory release, `ExitDll`, and `FreeLibrary`; accepted process containment proves physical retirement only and does not manufacture a clean in-process shutdown claim. The focused proof matrix passed 29/29, every owned process drained, every disposable environment retired, and WR0, Runtime/Proton, SteamOS read-only posture, and Bitwig 6.1 protected state remained exact.

**Architecture consequence:** The Windows plug-in plane now owns a live initialized processor component rather than only module and factory metadata. The immediate product frontier is the processor object's remaining interface and paired-controller behavior; build, custody, source handoff, SteamOS, Runtime/Proton, and process-supervision infrastructure are accepted prerequisites rather than successor-slice subjects.

**Claim ceiling:** WC0 does not establish edit-controller creation or initialization, processor/controller connection, `IConnectionPoint`, `IAudioProcessor` interface admission or census, buses, parameters, state, processing setup, audio/events, timing, automation, presets, GUI/editor behavior, native proxy publication, C ABI, IPC, shared memory, Bitwig hosting of a Windows plug-in, Serum operation or authorization, packaging, signing, release suitability, product-runner selection, another plug-in, another DAW, or general Windows VST3/Linux compatibility.

**Successor:** None selected. The next bounded product slice must be chosen through `docs/prompts/CHOOSE_NEXT_SLICE.md` from the post-closure `main` commit and tree.

### D-025 — Windows VST3 audio-processor interface boundary accepted

**Decision:** WA0 establishes the exact processor-side interface boundary required before any processing method can be considered. On the accepted Steam Deck fixture, the initialized AGain `IComponent` exposed the exact `Steinberg::Vst::IAudioProcessor` interface. The repository-owned Windows host acquired one interface lease, retained exact result/output/reference ownership, released the lease back to the component-owner baseline, proved interface quiescence, and then completed the accepted WC0 component and inherited factory/module shutdown without calling an `IAudioProcessor` method.

**Accepted implementation:** Immutable design `wa0-design-v1`, commit `0d5a41936c171f5d01d01b4c933875a1cfbe724a`, tree `787f1adf99e6083c3c90fa73996f6c151d2de50c`, design blob `d927c6dae430ffe7a811193e3d9119e573cfe316`, SHA-256 `eb5bd8f3aa439944f5933ecc9f0c4bcb90d46b3657499b365d11a82c0e2858ac`; design review `5094205619`; design-authority merge and implementation basis `47aeb7dcbaaec271292408fb9bbe0f2e4f9d00a9` / tree `2dc9d64fc296670469b2c5b8ca0dccff59f45c01`; implementation PR #32; source commit `24b7e6da7e29a5bd358097a6b89c5c59b747c413` / tree `d7c43098a14d86bf36b9c428e3836e1eb5353613`; evidence head `99478005e9f2675036100486c87952b76d411f84` / tree `56e861611d0a0fdfeeafac517e40f8bb7ea9bb8f`; technical-lead review `5096090726`; implementation merge `5d084ba5032dc8a7ce93be51e7d75dfd0d37ee22` / tree `56e861611d0a0fdfeeafac517e40f8bb7ea9bb8f`.

**Accepted build and custody:** The canonical 17-record source-manifest digest is `499a7e4879c1b1194f0f6d852e93f49cb914e994fdfc0a794474236cf7757a91`. Accepted workflow run `33689659595`, attempt 2, used runner image `windows-2022@20260824.284.2`, Visual Studio Enterprise `17.14.37614.0`, MSVC `19.44.35228`, linker `14.44.35228.0`, toolset `v143`, Windows SDK `10.0.19041.0`, and CMake `3.31.6`. Two clean build roots produced a byte-identical 35-path transfer roster. Scanner SHA-256 is `6ba5dab82d03cc2f736adc5c5d65b585b5673bdf0871ce445af8b59ba0a06122`; AGain SHA-256 remains `60aa9ff6b9918d4330449e7b3ab34b588dd93cba09f37413a3cd91f6e7d2e18f`; artifact ID is `9869994854`; upload, REST, and independently downloaded raw-wrapper digest bytes join at `81f3d431a0cb00c4cc121799818ce5ac1b8487dfde583d03eb48b98adda7bf3b`; artifact-manifest identity is `028228c6a8cc638b4aaf1f477b317359a22eb1dd84e90a12193bb3c5d9159070`.

**Accepted interface lifecycle:** The exact `IAudioProcessor` logical IID is `42043F99B7DA453CA569E79D9AAEC33D`, raw Windows TUID `993F0442DAB73C45A569E79D9AAEC33D`. The positive query returned success with a non-null, consistent output and exactly one owned lease. The one interface release returned reference count `1`, the exact separate `IComponent` owner baseline; the pointer was cleared; interface quiescence was true; no WA0 call remained in flight; and the accepted WC0 host sequence `1 -> 2 -> 1 -> 0`, component release `0`, reverse factory release, `ExitDll`, and `FreeLibrary` remained exact. No `IAudioProcessor` method was called and no controller was created.

**Accepted fault and cleanup law:** Twenty-three deterministic owner checks and eight focused live fixtures established failure/null, success/null, failure/non-null, query timeout/crash, release unexpected-count, and release timeout/crash behavior. Every non-null output became owned cleanup state. Failure/non-null was released exactly once while preserving the query inconsistency as primary. Query/release ownership unknown or incomplete suppressed component termination, component/factory release, `ExitDll`, and `FreeLibrary`; physical process containment did not manufacture clean in-process retirement. All nine live executions reached zero descendants and exact disposable-environment retirement, while WR0, Runtime/Proton, SteamOS read-only posture, and Bitwig 6.1 protected state remained exact.

**Claim ceiling:** WA0 does not establish any `IAudioProcessor` method, bus arrangement or enumeration, sample-size capability, latency/tail behavior, processing setup, `setProcessing`, `process`, audio/events, timing, automation, parameters, state, edit-controller creation or initialization, processor/controller connection, `IConnectionPoint`, GUI/editor behavior, native proxy publication, C ABI, IPC, shared memory, Bitwig hosting of a Windows plug-in, Serum operation or authorization, packaging, signing, deterministic hosted-runner behavior, release suitability, product-runner selection, another plug-in, another DAW, or general Windows VST3/Linux compatibility.

**Successor:** None selected. Successor analysis is constrained by D-026 and must not select another VST3 capability before the execution-cost paydown is addressed.

### D-026 — Proof-pipeline invalidation and developer-cost boundary

**Decision:** Developer wall-clock cost, agent/token cost, invalidation granularity, and manual cross-plane choreography are first-class architecture and acceptance concerns. Reusing proof code is not sufficient amortization when each narrow product change still forces repeated full Windows builds, artifact custody, source handoff, Deck admission, live execution, and evidence generation.

**Accepted evidence:** WA0 changed only `tools/wf0-factory-census/evidence.py` between source `4a5ff302acc4927142003e29bd401368920b275b` and final source `24b7e6da7e29a5bd358097a6b89c5c59b747c413`; the checked workflow, dependency lock, build/common tooling, C++ scanner source, and focused fixture source remained byte-identical. Nevertheless, the whole-source identity invalidated the complete Windows acceptance and downstream custody/execution transaction. Separately, floating runner image `20260830.290.1` rebuilt the unchanged accepted AGain source to SHA-256 `9c65a11bd67fdfc2416cc509b363c7a9ad83998cc1db5fa3cf2640e2c7431da8`, while image `20260824.284.2` reproduced the accepted fixture SHA-256. The failed attempts spent roughly six to seven minutes in the final build step before revealing that mismatch.

**Required architectural correction:** The proof system must introduce a Windows-build-input identity separate from non-build orchestration and evidence source; treat accepted immutable fixtures as content-addressed inputs rather than rebuilding them on every successor; separate cheap development validation from the final two-root acceptance producer; expose one repository-owned Mac-to-Windows-to-Deck orchestration entry point; batch preflight and handoff work; and keep deterministic owner/state tuples out of live Proton execution unless they establish genuinely new live-runtime uncertainty.

**Rejected behavior:** Do not use the full acceptance producer as an edit/compile loop. Do not rerun unchanged source hoping a different floating hosted runner image appears. Do not invalidate and repeat the complete build/custody/handoff/Deck transaction solely because a non-build-affecting evidence or orchestration path changed. Do not require an implementation agent to act as a manual shipping clerk between the Mac, GitHub Actions, and the Steam Deck.

**Successor constraint:** No additional VST3 product-capability slice may be selected until a bounded execution-cost paydown is accepted. This ruling does not select or activate that paydown slice; explicit successor analysis and operator approval remain required.

### D-027 — Split proof identities and bounded one-command transaction accepted

**Decision:** DX0 is accepted for the closed `wa0-positive-regression-v1` plan. The Mac driver separates complete source, Windows build inputs, accepted fixture, Deck execution inputs, evidence renderer, and transaction identity. It reuses accepted AGain bytes rather than compiling the fixture, selects host artifacts by build-input identity, and reuses successful retained execution results when only Mac-side rendering or orchestration changes.

**Accepted identities:** Design `dx0-design-v2`, blob `5dd758d681f9712de02ab4c45d58d3d815aef53a`, SHA-256 `85de95acfe171678c0efedbbdc6aebdc0ff134a716dfd8d29f52feed9c53df6c`; basis `404966e6bfcd6403a1abb9ed8005210e93d9ad02`; source `be046dd2d44a7915ca408c01a06212639ccea51e` / tree `1322e4eb7b5bd54244bd2fa7fc83327ea6a0c183`; evidence head `85840920844693f7611306492298817e16dd2a6c` / tree `35ddde4e56a810ab1ce44970313bf2f42e585908`; implementation PR #35; technical-lead review `5098295411 / DX0_CLEAR`; merge `1f71487717eabdb3cd5285a4559df2ce2915c8d8` / tree `35ddde4e56a810ab1ce44970313bf2f42e585908`. The two implementation commits contain exactly ten source/configuration paths and five evidence paths. Source-manifest digest is `f849fb07db6925c5a1bdb722703f2a458b3f69d6f77da038af789952e9a29e3b`.

**Repair and dependency boundary:** Review `5097953677` found that renderer-only `evidence.py` was imported by Deck execution without participating in the Deck identity. The accepted repair removes that dependency, retains a strict Deck-local validator, and tests the exact seven-path transitive repository import/execution closure and Deck/Mac validator parity. Windows build inputs and the renderer remain unchanged. Dependency identity must cover actual executed/imported code, not merely a declared filename list.

**Accepted reuse:** Windows producer P remains `bb592b68fc025eed1f2332952ed38d70de5d3b54`, run `33707854242` attempt 1, artifact `9875911040`, build identity `d8ed6019769c9239a6eb6bfcc79742b3ca834b1fb1d33b65c33b1c4b31b7d950`. Repaired Deck execution E is `57fe231f50e0013c38ace5cfc4b3e62531e8d7c7`; final consumer C is `be046dd2d44a7915ca408c01a06212639ccea51e`. Execution identity is `a44a72c1ec8de7366ac4cd21050c9e6f016a14b15bd851188cc6e13d3699820e`; retained result is `c132c445ae6559cd176dae11f4cb235f62737593e1c26c7be500d30533eeb434`; packet is `3ae6510123f28ba559a164d62035f9eec3792667ca33f94540c55b13056376c5`. C reports E's historical observation, not fresh execution or current physical-state inspection.

**Execution accounting:** Operator comment `5520861137` explicitly authorized the final corrective positive Deck batch with zero Windows builds/dispatches, downloads, custody operations, fixture seeds, or negative live exercises. The retained repair ledger reports one source transfer and one positive Deck execution for E; C then performs zero external effects and one local evidence render. E's successful result was retained before a Mac-only evidence-accounting failure. The recovery preserves that ten-phase partial transaction honestly rather than claiming its failed render completed. Earlier superseded observations are not proof for the repaired identity, and the full implementation history is not represented as one live exercise. One-time fixture seeding is separately accounted for.

**Proof and limitations:** Fourteen rows distinguish twelve deterministic rows from two rows sharing the authorized corrective live observation. The retained result proves interface release 1, component release 0, paired calls, quiescence, factory/module shutdown, zero descendants, environment retirement, and original protected-state equality. The final fresh-context audit is recorded as `AUDIT_CLEAR`; technical-lead review inspected source, Git identities, and retained evidence without another build or Deck run. No duration, token, or monetary savings percentage is claimed. The observed reuse is concrete; universal future-slice speed is not.

**Successor consequence:** This bounded acceptance satisfies D-026's prerequisite for considering product-capability work, while its ergonomics and invalidation rules remain binding. Current DX0 source/basis/ref guards and its proof plan are still specific: an arbitrary successor does not already execute unchanged. A successor must bind its authorized source and focused plan while reusing these owners; it must not recreate manual build/custody/SSH choreography or rebuild unchanged artifacts for renderer-only fixes. No successor is selected here.

**Claim ceiling:** DX0 adds no VST3 capability, audio processing, buses, parameters, state, controller/connection, GUI/editor, native proxy, C ABI, IPC, Bitwig or Serum operation, packaging, signing, immutable hosted runner, release suitability, or general compatibility. The accepted product frontier remains WA0.
