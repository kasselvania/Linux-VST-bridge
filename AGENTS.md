# AGENTS.md — Repository Execution Rules

## Mission

Build a managed Windows-audio compatibility platform for native Linux DAWs. A supported user should be able to install, authorize, publish, run, save, reopen, update, diagnose, and roll back a Windows plug-in without manually administering Wine, Proton, prefixes, generated proxies, Flatpak paths, or bridge synchronization.

The project begins with Bitwig Studio on the maintainer's Steam Deck because that is the available real fixture. Neither Steam Deck, SteamOS, Flatpak, Bitwig, Serum 2, nor Kontakt defines the universal product boundary.

## Authority order

When instructions conflict, use this order:

1. explicit operator product/design ruling;
2. `AGENTS.md`;
3. `GOVERNANCE.md`;
4. `CURRENT_SLICE.md`;
5. exact approved design-approval receipt for the current slice;
6. exact approved implementation design card for the current slice;
7. `docs/ARCHITECTURE.md`;
8. applicable accepted entries in `docs/DECISION_REGISTER.md`;
9. applicable fixture card in `docs/FIXTURE_CARDS.md`;
10. `docs/DESIGN_DOSSIER.md`;
11. `docs/RESEARCH_BASIS.md`;
12. issue / pull-request scope;
13. implementation convenience.

Stop and surface a conflict. Do not silently reconcile two authorities by inventing a third design.

## Repository roles

- The **design dossier** is human north-star material. It describes the intended whole product, difficult fixtures, failure posture, and product laws.
- `docs/ARCHITECTURE.md` owns provisional technical boundaries accepted for the repository.
- `docs/DEVELOPMENT_PROCESS.md` owns the human selection/design/implementation/review lifecycle.
- `CURRENT_SLICE.md` owns exactly one active slice and authority phase.
- A **slice-selection receipt** records operator selection.
- An **implementation design card** freezes the approved owner/state/fault/proof design for high-risk work.
- A **design-approval receipt** binds implementation authority to one exact card revision.
- Code and tests state what is implemented.
- Retained evidence states what was observed on a declared fixture.
- A compatibility profile states a bounded claim for exact versions and conditions; it is not universal truth.

Do not hand the full dossier to an implementation agent as its task contract. Distill one bounded slice and, when required, one approved design revision.

## Authority-phase gate

Before editing production implementation, inspect `CURRENT_SLICE.md`.

Implementation is permitted only when either:

```text
authority_phase: implementation
implementation_authorized: true
```

and the exact approved design card/receipt exist, or an explicit design-gate waiver is recorded.

When authority is `reconnaissance_and_design`:

- read-only fixture reconnaissance and the declared design records are allowed;
- product implementation is forbidden;
- implementation scaffolding that commits to an unapproved owner/state/topology is forbidden;
- the agent must not treat a detailed implementation prompt as implicit design approval.

When there is no active slice, implementation is forbidden.

## Mandatory design gate

A high-risk slice requires an implementation design card and adversarial design review before code. Triggers include:

- new owner or lifecycle;
- durable mutation;
- transaction/rollback/migration/recovery;
- process creation/supervision/termination;
- cross-process or cross-language protocol;
- real-time/deadline behavior;
- thread affinity/reentrancy;
- identity/authorization;
- security/privacy/licensing;
- third-party runtime;
- persistent user data/content;
- compatibility claims.

A waiver must be explicit. Small code size or model confidence is not a waiver.

Use:

- `docs/templates/IMPLEMENTATION_DESIGN_CARD.md`;
- `docs/prompts/ADVERSARIAL_DESIGN_REVIEW.md`;
- `docs/templates/DESIGN_APPROVAL_RECEIPT.md`.

## Material-discovery stop law

Implementation must stop and return `RETURN_TO_DESIGN_GATE` when real evidence changes or invalidates any approved:

- owner;
- state/transition;
- mutation root;
- durability/commit/recovery boundary;
- process/thread/callback topology;
- protocol direction or payload law;
- identity/authorization rule;
- security/privacy/licensing posture;
- fixture;
- primary claim or claim ceiling;
- changed-path envelope;
- proof matrix.

The agent may retain bounded read-only evidence. It may not patch forward, widen authority, or rewrite the design after implementation and call it documentation.

## Independent review law

The adversarial design reviewer and pre-PR implementation auditor use fresh contexts that did not author the reviewed artifact.

Before technical-lead PR review, run the process in `docs/prompts/PRE_PR_IMPLEMENTATION_AUDIT.md`.

An implementation defect can be repaired against the approved design. A design defect requires a new design revision, fresh adversarial review, and new approval.

## Core product invariants

- A native Linux DAW never directly loads a Windows plug-in binary.
- The DAW loads a native Linux proxy; a supervised Windows host loads the Windows module under a pinned runner.
- Proton is a source of runtime shape and selected technology, not magic, not a DAW plug-in API, and not necessarily the final distributed runtime.
- The Steam client must not be a product dependency.
- Runtime identity, environment identity, installed-module identity, plug-in class identity, proxy identity, content identity, authorization posture, and host registration are distinct facts.
- A filesystem path is a location, not a durable plug-in identity.
- A friendly plug-in name is discovery metadata, not identity.
- No ambient `latest` runner may alter an already working environment without an explicit update transaction.
- A runner update, plug-in update, environment migration, content relocation, or authorization transition must be observable and reversible where the vendor permits it.
- One giant Wine prefix containing every vendor is prohibited as the default product architecture.
- One process containing every bridged plug-in is prohibited as the default runtime architecture.
- Scanner, installer, authorization, editor, and real-time audio failure domains remain distinguishable.
- A scanner or editor crash must not be described as an audio-processing failure.
- A compatibility profile may automate declared setup; it must never hide unsupported behavior or claim to bypass licensing.
- Vendor authorization belongs to the vendor and user. This project may present and supervise authorized flows; it must not defeat, emulate, forge, intercept, or redistribute licensing material.
- Never commit or log credentials, session cookies, refresh tokens, license files, machine-bound activation data, serial numbers, account identifiers, proprietary installers, plug-in binaries, vendor presets, or paid sample libraries.
- Stable machine identity is part of compatibility. Prefix recreation or runner change must not casually create a new licensed machine.
- The DAW remains audio-session and project authority. The bridge translates and supervises; it does not become a DAW.
- Host project state and plug-in state must survive save, close, process restart, machine reboot, and reopen before a plug-in is called usable.
- Preset indexing and visual organization are projections over exact underlying identities. They do not rewrite vendor content or plug-in state.
- Wrong recovery is worse than explicit failure. Never silently substitute another plug-in build, class, preset, environment, or content root.

## Real-time laws

The audio callback is a hard boundary.

On the real-time path:

- no heap allocation after activation;
- no filesystem access;
- no process creation;
- no network access;
- no ordinary logging;
- no unbounded lock acquisition;
- no prefix, registry, installer, authorization, or compatibility-profile work;
- no control-plane serialization with unbounded size;
- no waiting on the manager UI;
- no recovery policy decision;
- no synchronous editor operation.

Use preallocated shared memory for audio and bounded event/control structures. Every wait has a declared bound and failure result. A late or dead Windows host must produce a defined silence/bypass/failure posture without deadlocking the DAW.

Do not make “lock-free” claims from type names or intention. Prove the actual callback path, allocation posture, blocking posture, and timing behavior.

## Cross-boundary rules

- Rust is the product language.
- C++20 is permitted only at the official VST3 SDK boundary or where an accepted slice proves another narrow need.
- C++ objects, exceptions, STL containers, allocators, ownership, or RTTI do not cross the Rust/C++ ABI boundary.
- The language boundary is a versioned C ABI with explicit ownership and error codes.
- The process boundary is a separately versioned protocol.
- ABI version, IPC protocol version, cache schema version, compatibility-profile schema version, and runner version are separate version axes.
- Host-to-plug-in and plug-in-to-host calls are reentrant. A single blocking request socket is not an acceptable general protocol.
- Win32 GUI/message-loop obligations stay on the correct Windows host thread.
- Linux host GUI obligations stay on the correct DAW-facing thread.
- VST3 factory, class, component, controller, view, context-menu, parameter, state, bus, event, and process interfaces are not collapsed into an imagined generic RPC object without explicit coverage.
- Unknown or unsupported interfaces produce inspectable capability results, not undefined behavior.

## Slice discipline

Every selected slice states:

- exact base commit and tree;
- basis documents and exact headings;
- one primary claim;
- exact fixture and versions;
- authority phase;
- design-gate decision;
- files/components in scope;
- dependencies permitted to change;
- permitted external mutation;
- protected state;
- explicit non-goals and nonclaims;
- executable acceptance criteria;
- negative/failure acceptance criteria;
- retained evidence requirements;
- cleanup and rollback posture;
- material-discovery stop conditions.

A slice is complete only when its one claim is demonstrably true at the stated fixture and claim level.

Do not combine uncertainty domains merely because the combined demo would look impressive. Keep separate where practical:

- installer supervision versus plug-in ABI bridging;
- factory/class discovery versus audio processing;
- audio transport versus GUI embedding;
- vendor activation versus generic process launch;
- host-visible proxy publication versus Flatpak extension packaging;
- state round-trip versus preset browsing;
- plug-in update versus runner update;
- runner construction versus runner selection;
- process crash containment versus state recovery;
- Serum 2 compatibility versus Kontakt compatibility;
- Steam Deck proof versus general Linux support;
- VST3 proof versus CLAP support;
- open reference fixtures versus proprietary commercial fixtures.

## Implementation design requirements

For a design-gated slice, the card must include:

- owner map;
- complete state machine, including OS/runtime-visible intermediate states;
- transition ledger;
- fallible-operation and mutation ledger;
- physical result if each operation succeeds and the next fails;
- commit points and pre-/post-commit recovery law;
- process/thread/callback topology;
- exact identity and authorization ledger;
- data/durability/migration law;
- security/privacy/licensing posture;
- approved code topology;
- proof matrix;
- blocked-result taxonomy;
- material-discovery stop conditions.

Do not infer missing design during implementation.

## Required development sequence discipline

The dossier contains a dependency-ordered proof sequence, not a calendar or release promise. A later capability may not be claimed from an earlier seam.

In particular:

- seeing a plug-in name does not prove factory fidelity;
- factory fidelity does not prove component/controller lifecycle;
- lifecycle does not prove audio correctness;
- audio output does not prove timing, events, automation, state, presets, GUI, activation, crash containment, update safety, or Flatpak packaging;
- one successful session does not prove persistence;
- one plug-in does not prove a vendor or format class;
- one machine does not prove a distro;
- a vendor installer completing does not prove lawful authorization or continuing usability.

## Fixture law

### Serum 2

Serum 2 is the first real commercial VST3 fixture. It pressures:

- a current and widely used 64-bit VST3;
- a substantial resizable editor;
- preset and content roots;
- browser-based or offline authorization;
- Splice Rent-to-Own versus Xfer-owned authorization differences;
- automation, MIDI, note expression where exposed, state, and project recall.

Use only the maintainer's lawful installer and license. Do not commit them. Record the exact acquisition/authorization class used. “Serum 2 works” is invalid without exact version, license channel, runner, host, proxy, environment, and tested capabilities.

### Kontakt

Kontakt is a later hostile systems fixture, not an early bridge target. It pressures:

- Native Access as a companion installation/activation system;
- account login and network dependency;
- application, download, and content locations;
- large external libraries;
- locate/repair flows;
- Player versus full Kontakt behavior;
- licensed versus unlicensed third-party libraries;
- legacy formats and historical paths;
- missing-content and version-mismatch recovery.

Do not make Kontakt block the first protocol proof.

### Open reference modules

Openly distributable SDK examples or purpose-built reference modules are test instrumentation. They exist for CI, deliberate malformed behavior, deterministic state/audio assertions, and validator work. They do not replace the first commercial fixture.

## Flatpak and SteamOS rules

- SteamOS's immutable base must not be disabled as a product requirement.
- Prefer user-space installs, Flatpak extensions, and explicit broker contracts.
- Do not make `pacman`, root filesystem edits, Steam launch entries, or the user's Steam compatibility-tools directory part of the normal product contract.
- Treat Bitwig Flatpak's current permissions as observed version-specific facts, not permanent guarantees.
- Do not assume a host path is visible in a sandbox; prove it on the exact app/runtime version.
- Do not use unbounded host escape as the architecture. Any host broker must have an explicit narrow contract and threat model.
- A packaged `org.freedesktop.LinuxAudio.Plugins` extension must match the application runtime branch and be tested across runtime transitions.
- The initial fixture may use `~/.vst3` if the exact Bitwig Flatpak build sees it. That is a proof mechanism, not automatically the production packaging decision.

## Compatibility-profile rules

A profile is versioned data, not executable arbitrary code.

It may declare:

- exact vendor/product/module/version match;
- runner constraints;
- environment-family policy;
- dependencies and fonts;
- graphics backend posture;
- DPI and editor mode;
- installer switches;
- known child processes or services;
- network/authorization capabilities;
- content-root expectations;
- scan timeout and quarantine policy;
- process-group policy;
- tested host/runtime matrix;
- support state and nonclaims;
- evidence references.

It may not contain shell commands, arbitrary scripts, credentials, vendor secrets, binary patches, DRM bypasses, hidden remote downloads, or unreviewed destructive repair operations.

## Evidence requirements

Retain useful sanitized evidence under `evidence/` when the slice requires it:

- host, kernel, distro, graphical session, and architecture;
- Flatpak app/runtime version and permissions;
- runner identity and digest;
- environment manifest and non-sensitive registry/file deltas;
- installer process tree and declared exit result;
- scanner/factory/class census;
- VST3 interface/capability census;
- bus, parameter, event, state, and editor metadata;
- audio hashes or reference comparisons;
- buffer-size/sample-rate/instance stress results;
- callback latency and deadline misses;
- process crash, timeout, quarantine, restart, and orphan checks;
- project save/reopen evidence;
- preset/content-root relocation evidence;
- activation-flow classification without secret data;
- Flatpak path/mount/IPC evidence;
- rollback evidence.

Evidence names identify the exact fixture and claim. A screenshot alone is not protocol proof. A log alone is not audio correctness. A successful test run is not an architecture substitute.

Every approved proof-matrix row identifies whether production helpers, synthetic faults, or real fixtures are required. Test names do not substitute for execution.

## Security and privacy

- Treat managed environments as potentially sensitive because they can contain account state, browser state, machine identifiers, and license material.
- Diagnostic exports are allow-list based. Never zip an entire prefix by default.
- Redact usernames, home paths where appropriate, hostnames, IP addresses, emails, serials, tokens, cookies, and vendor account identifiers.
- Installer and plug-in binaries are untrusted code. Scanners and installers require process supervision, resource limits, and explicit filesystem/network posture.
- Network capture, TLS interception, credential scraping, or vendor-protocol reverse engineering is not authorized by ordinary compatibility work.
- Do not claim sandbox security merely because a process runs under Wine or Flatpak.
- Destructive repair requires an explicit preview, backup/rollback posture, and user action.

## Third-party and clean-room rules

- Do not copy yabridge source into this repository.
- Yabridge may be read as prior art and implementation evidence. Record concepts, observed constraints, and clean-room decisions in prose; implement original code from public specifications and accepted repository interfaces.
- Do not include GPL-derived code in a proprietary boundary by accident. Licensing decisions are explicit and reviewed.
- Do not redistribute Proton, Wine, DXVK, VST3 SDK files, vendor installers, or plug-ins until exact distribution obligations and notices are recorded.
- Preserve third-party license texts and source offers where required by future distribution.
- Do not use the repository name or documentation to imply certification or affiliation by a DAW, format owner, runtime vendor, or plug-in vendor.

## Review standard

Review the exact pull-request head against:

- the current slice's one claim;
- exact approved design identity when required;
- actual component ownership;
- complete state/fault model;
- real-time laws;
- reentrancy and thread affinity;
- exact identity preservation;
- failure containment and physical-state recovery;
- state and project recall where in scope;
- Flatpak and path assumptions;
- security/privacy posture;
- license boundary;
- proof matrix and production-helper coverage;
- retained evidence;
- explicit nonclaims;
- independent pre-PR audit disposition.

A green test suite is necessary but not sufficient. Do not merge a convincing demo that establishes the wrong boundary.

After finding a defect, inspect the entire invariant class. Repeated local repairs require a design-return decision rather than indefinite patch-forward behavior.
