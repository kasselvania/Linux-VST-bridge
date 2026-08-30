# Linux Audio Compatibility Bridge

> Working repository name. This is not yet a consumer brand and is not affiliated with Bitwig, Valve, Steinberg, Xfer Records, Native Instruments, or any plug-in vendor.

This project is designing and building a managed compatibility platform that lets native Linux DAWs use supported Windows audio plug-ins without asking the user to manage Wine prefixes, bridge synchronization, Flatpak paths, runtime versions, installer workarounds, or opaque recovery steps.

The target experience is:

```text
choose an installer
    -> install into a managed environment
    -> complete any vendor-authorized activation
    -> scan in an isolated process
    -> publish a native Linux proxy to the selected DAW
    -> use, save, reopen, update, diagnose, and roll back predictably
```

The project is **not** based on the fiction that Proton lets a native Linux DAW load a Windows DLL directly. The native DAW loads a native Linux proxy. A supervised Windows host loads the actual plug-in under a pinned audio-oriented Wine/Proton-derived runner. Versioned IPC carries factory calls, state, parameters, events, audio, editor coordination, and diagnostics across that boundary.

## Current posture

- The repository contains a human-facing soft design dossier and provisional architecture.
- No production bridge implementation exists yet.
- No implementation slice has been selected beyond the completed repository-design seed.
- **Serum 2 is the first real commercial VST3 fixture.**
- An openly distributable VST3 reference module may be used as test infrastructure, but it is not a substitute for the Serum 2 product proof.
- **Kontakt is a later hostile-system fixture** for Native Access, large and relocatable content, authorization, historical library layouts, and version compatibility.

## Authority and reading order

1. [`AGENTS.md`](AGENTS.md)
2. [`GOVERNANCE.md`](GOVERNANCE.md)
3. [`CURRENT_SLICE.md`](CURRENT_SLICE.md)
4. [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md)
5. [`docs/DESIGN_DOSSIER.md`](docs/DESIGN_DOSSIER.md)
6. [`docs/FIXTURE_CARDS.md`](docs/FIXTURE_CARDS.md)
7. [`docs/DECISION_REGISTER.md`](docs/DECISION_REGISTER.md)
8. [`docs/RESEARCH_BASIS.md`](docs/RESEARCH_BASIS.md)

The dossier is north-star material for the operator and technical lead. Coding agents receive only the current bounded slice, the applicable architecture sections, ordinary interfaces, and focused acceptance requirements.

## Repository shape

```text
AGENTS.md                    repository execution law
GOVERNANCE.md                design/implementation/evidence separation
CURRENT_SLICE.md             the only active implementation authority
docs/DESIGN_DOSSIER.md       human product and experience north star
docs/ARCHITECTURE.md         provisional component boundaries and laws
docs/FIXTURE_CARDS.md        Serum 2, Kontakt, and reference-fixture pressure
docs/DECISION_REGISTER.md    accepted, provisional, and open decisions
docs/RESEARCH_BASIS.md       exact internal and external grounding
compatibility/               future versioned compatibility profiles and schema
evidence/                    sanitized retained experimental evidence
```

No proprietary plug-in binary, installer, preset library, activation file, token, credential, account data, or licensed content belongs in this repository.

## Language ruling

The initial language shape is **Rust-primary with a narrow C++20 VST3 boundary**:

- Rust owns the manager, environment model, installer supervision, process broker, IPC contract, compatibility profiles, diagnostics, state registry, CLI, and later user interface.
- C++20 owns only the surfaces where the official VST3 SDK and its COM-style object model make a direct C++ boundary materially safer: the native Linux proxy shell and the Windows VST3 host shell.
- The Rust/C++ boundary is a small explicit C ABI. C++ objects never cross it.
- Process boundaries use a versioned protocol; audio uses preallocated shared memory with bounded synchronization.
- CLAP may later be implemented much more directly in Rust because its public ABI is C-based.

See [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) for the reasons and nonclaims.

## Naming and licensing

`Linux-VST-bridge` is a technical repository name, not the final product name. A consumer-facing name must not imply vendor affiliation and must receive an explicit trademark review before release.

No repository-wide software license has been selected. That is intentional while the clean-room boundary, third-party SDK obligations, Wine/Proton distribution obligations, commercial model, and possible open-core split remain open decisions.
