# Decision Register

Decisions are separated into **accepted**, **provisional**, and **open**. Implementation convenience does not silently change their state.

## Accepted design rulings

### D-001 — Product boundary

**Decision:** Build a managed compatibility platform, not merely a bridge binary or Proton launcher.

**Consequences:** Installation, authorization handoff, scanning, organization, publication, diagnostics, update, repair, and rollback are first-class product components.

**Nonclaim:** No implementation exists.

### D-002 — Native/Windows boundary

**Decision:** A native Linux DAW loads a native Linux proxy. A supervised Windows host loads the Windows plug-in under a pinned runner. IPC crosses the boundary.

**Rejected alternative:** Attempt to load a Windows VST3 DLL directly in Bitwig through Proton.

### D-003 — Primary language

**Decision:** Rust is the primary product language.

**Rust owners:** model, manager, broker, runner/environment registry, installer supervision, profiles, scanner orchestration, publication, diagnostics, transport core, CLI, and likely UI.

### D-004 — VST3 SDK language edge

**Decision:** Use narrow C++20 shells for the native Linux VST3 proxy and Windows VST3 host, connected to Rust through a small explicit C ABI.

**Reason:** Official SDK/object model alignment and reduced first-product risk given current Rust binding coverage/licensing uncertainty.

**Revisit condition:** A complete, maintained, permissively licensed Rust host/proxy binding is independently proven against required Bitwig/Serum interfaces without widening risk.

### D-005 — First real plug-in fixture

**Decision:** Serum 2 VST3 is the first real commercial plug-in target.

**Clarification:** Open SDK/reference modules are test infrastructure, not a replacement target.

### D-006 — Kontakt role

**Decision:** Kontakt is a later hostile systems fixture for Native Access, large/relocatable content, activation, editions, third-party libraries, and legacy behavior.

**Rejected alternative:** Make Kontakt the first bridge implementation target.

### D-007 — Detached editor first

**Decision:** First usable editor mode is a supervised detached top-level window. Embedded or captured/streamed presentation is later and separately proven.

### D-008 — Exact versioned compatibility

**Decision:** Compatibility profiles and support claims bind exact plug-in build, license channel, runner, environment, proxy/protocol, DAW, sandbox, and capability matrix.

### D-009 — No arbitrary profile code

**Decision:** Compatibility profiles are declarative, closed-schema data. They cannot contain arbitrary scripts or commands.

### D-010 — No-active-slice after design seed

**Decision:** Repository setup does not automatically authorize implementation. The operator and technical lead choose the next bounded slice from current evidence.

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

**Choice:** First prove a user-controlled native proxy path; evaluate a matching `org.freedesktop.LinuxAudio.Plugins` extension and narrow host broker separately.

## Open decisions

### O-001 — Next active slice

Candidates:

- Serum 2 fixture reconnaissance;
- native Bitwig VST3 host probe;
- combined Serum factory census vertical.

Current technical-lead recommendation: first two as separate slices, then the combined boundary becomes narrow.

### O-002 — Exact runner base

Options include:

- selected current Wine Staging build;
- custom Proton-derived build;
- UMU-launched Proton for experimentation;
- another maintained audio-oriented Wine base.

Decision evidence must include install/auth, process/IPC, graphics, synchronization, performance, reproducibility, and distribution obligations.

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

Before implementing C++ shells, inspect current permissive bindings and host libraries again. Change only from evidence, not language preference.

### O-009 — Editor beyond detached mode

Candidates:

- X11/XWayland embedding;
- native presentation using newer VST3 rendering/input interfaces where supported;
- captured/streamed editor surface with input forwarding;
- detached-only support for some profiles.

### O-010 — Process grouping law

Determine exact module/instance grouping through VST3 semantics, Bitwig sandbox modes, vendor communication needs, performance, and crash containment.

### O-011 — State encryption and sensitive environment backups

Define encryption, key storage, exports, machine-binding warnings, and vendor-license boundaries before user backups are productized.

### O-012 — Compatibility profile publication and trust

Define review, signatures, evidence links, withdrawal, third-party contribution, and local override posture.

### O-013 — First second-vendor fixture

Select only after Serum reveals which architectural assumptions need counter-pressure.
