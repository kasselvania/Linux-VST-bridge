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

Options:

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

Candidates:

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
