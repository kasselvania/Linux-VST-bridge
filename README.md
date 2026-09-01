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

## Accepted boundary

The repository currently retains four accepted proof layers:

```text
SR0  exact Steam Deck / Bitwig / existing Serum-artifact reconnaissance
HP0  repository-owned native Linux VST3 build, validation, publication, and sandbox load
HP1  normal Bitwig discovery and exact native instance admission
WR0  controlled Runtime 4 / Proton 11 Windows-command execution and isolated environment ownership
```

Those layers do **not** yet establish a Windows VST3 scanner, Windows plug-in host, native bridge proxy, bridge IPC, shared-memory audio transport, Serum operation, authorization, or product compatibility.

Serum 2 remains the first real commercial VST3 fixture. Open reference modules are test instrumentation, not a substitute for the Serum proof. Kontakt remains a later hostile-system fixture for Native Access, large and relocatable content, authorization, historical library layouts, and version compatibility.

## Development method

High-risk work uses a two-approval process:

```text
select one bounded slice
    -> bounded reconnaissance
    -> freeze an implementation design card
    -> independent adversarial design review
    -> approve the exact design revision
    -> implement only that revision
    -> independent pre-PR implementation audit
    -> technical-lead exact-head review
```

Selecting a slice does not automatically authorize implementation. A material discovery about ownership, state, topology, mutation, recovery, security, licensing, real-time behavior, or proof coverage returns the slice to the design gate rather than becoming an improvised patch.

See [`docs/DEVELOPMENT_PROCESS.md`](docs/DEVELOPMENT_PROCESS.md).

## Authority and reading order

1. [`AGENTS.md`](AGENTS.md)
2. [`GOVERNANCE.md`](GOVERNANCE.md)
3. [`CURRENT_SLICE.md`](CURRENT_SLICE.md)
4. [`docs/DEVELOPMENT_PROCESS.md`](docs/DEVELOPMENT_PROCESS.md)
5. the approved current-slice implementation design and approval receipt, when present
6. [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md)
7. [`docs/DESIGN_DOSSIER.md`](docs/DESIGN_DOSSIER.md)
8. [`docs/FIXTURE_CARDS.md`](docs/FIXTURE_CARDS.md)
9. [`docs/DECISION_REGISTER.md`](docs/DECISION_REGISTER.md)
10. [`docs/RESEARCH_BASIS.md`](docs/RESEARCH_BASIS.md)

The dossier is north-star material for the operator and technical lead. A coding agent receives one bounded authority phase, the approved design revision when implementation is authorized, applicable architecture sections, ordinary interfaces, and focused acceptance requirements.

## Repository shape

```text
AGENTS.md                              repository execution law
GOVERNANCE.md                          authority and truth-class law
CURRENT_SLICE.md                       the only active slice and authority phase
docs/DEVELOPMENT_PROCESS.md            human selection/design/implementation/review process
docs/templates/                        design and authority receipt templates
docs/prompts/                          reusable technical-lead and independent-review prompts
docs/slices/<SLICE_ID>/                future approved per-slice design/review/approval records
docs/DESIGN_DOSSIER.md                 human product and experience north star
docs/ARCHITECTURE.md                   provisional component boundaries and laws
docs/FIXTURE_CARDS.md                  Serum 2, Kontakt, and reference-fixture pressure
docs/DECISION_REGISTER.md              accepted, provisional, and open decisions
docs/RESEARCH_BASIS.md                 exact internal and external grounding
compatibility/                         future versioned compatibility profiles and schema
evidence/                              sanitized retained experimental evidence
```

No proprietary plug-in binary, installer, preset library, activation file, token, credential, account data, or licensed content belongs in this repository.

## Reusable process artifacts

- [Implementation design card template](docs/templates/IMPLEMENTATION_DESIGN_CARD.md)
- [Slice-selection receipt template](docs/templates/SLICE_SELECTION_RECEIPT.md)
- [Design-approval receipt template](docs/templates/DESIGN_APPROVAL_RECEIPT.md)
- [Prompt: choose the next slice](docs/prompts/CHOOSE_NEXT_SLICE.md)
- [Prompt: adversarially review an implementation design](docs/prompts/ADVERSARIAL_DESIGN_REVIEW.md)
- [Prompt: independently audit an implementation before PR review](docs/prompts/PRE_PR_IMPLEMENTATION_AUDIT.md)

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
