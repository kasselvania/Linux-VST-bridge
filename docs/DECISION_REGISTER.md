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
