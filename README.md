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

The project is **not** based on the fiction that Proton lets a native Linux DAW load a Windows DLL directly. The native DAW loads a native Linux proxy. A supervised Windows host loads the actual plug-in under a pinned audio-oriented Wine/Proton-derived runner. Versioned IPC will carry factory calls, state, parameters, events, audio, editor coordination, and diagnostics across that boundary.

## Accepted boundary

The repository currently retains these accepted proof layers:

```text
SR0  exact Steam Deck / Bitwig / existing Serum-artifact reconnaissance
HP0  repository-owned native Linux VST3 build, validation, publication, and sandbox load
HP1  normal Bitwig discovery and exact native instance admission
WR0  controlled Runtime 4 / Proton 11 Windows-command execution and isolated environment ownership
WF0  supervised Windows VST3 module loading, exact factory metadata, and ordered class census
WC0  exact AGain processor IComponent creation, initialization, termination, and object quiescence
WA0  exact IAudioProcessor interface acquisition, release, and inherited clean shutdown
DX0  split Windows-build, fixture, Deck-execution, and renderer identities with retained-result reuse
PX0  diagnostic development separated from product acceptance
PX1  unclassified live proof execution hard-gated
```

The accepted **product** frontier remains WA0. PC0—the Initialized-state census of audio/event buses, `BusInfo`, current speaker arrangements, and `kSample32`/`kSample64` support—is suspended after an inconclusive acceptance attempt. It is neither accepted nor rejected. PX2 is local-only proof-harness maintenance to build the class-aware transaction core required before PC0 can resume through a bounded diagnostic campaign and a fresh acceptance candidate.

These layers do **not** establish bus or sample-format behavior, processing setup, audio processing, a native bridge proxy, bridge IPC, shared-memory transport, Bitwig hosting of a Windows plug-in, Serum operation or authorization, packaging, release suitability, or general compatibility.

Serum 2 remains the first real commercial VST3 fixture. Open reference modules are test instrumentation, not a substitute for the Serum proof. Kontakt remains a later hostile-system fixture for Native Access, large and relocatable content, authorization, historical library layouts, and version compatibility.

## Development method

The repository distinguishes **what changes** from **why a live workload runs**.

```text
change class:
  PRODUCT_CONTRACT_CHANGE
  PROOF_HARNESS_MAINTENANCE
  MECHANICAL_MAINTENANCE

execution class:
  READ_ONLY_RECONCILIATION
  DIAGNOSTIC_NON_AUTHORITATIVE
  ACCEPTANCE_CANDIDATE
```

The normal sequence is:

```text
approve one bounded product contract
    -> implement and validate locally
    -> use a bounded, permanently non-authoritative diagnostic campaign only when real-fixture debugging is needed
    -> repair proof-harness defects without reopening unchanged product design
    -> freeze one exact acceptance candidate
    -> run one strict acceptance transaction
    -> audit successful acceptance evidence only
```

Diagnostic observations are permanently `acceptance_eligible: false`; they cannot satisfy product proof rows, produce tracked product evidence, or advance the accepted frontier. Failed diagnostics and inconclusive acceptance candidates receive bounded private failure closure rather than success-style evidence, full proof-matrix replay, or a product audit.

A change returns to product design only when it changes the product claim, VST3 operation roster or interpretation, ownership, lifecycle, Windows product behavior, fixture meaning, product-facing normalization, containment claim, security boundary, product proof obligation, or compatibility claim.

See [`docs/DEVELOPMENT_PROCESS.md`](docs/DEVELOPMENT_PROCESS.md) and [`docs/process/DIAGNOSTIC_AND_ACCEPTANCE_RULING.md`](docs/process/DIAGNOSTIC_AND_ACCEPTANCE_RULING.md).

## Authority and reading order

1. [`AGENTS.md`](AGENTS.md)
2. [`GOVERNANCE.md`](GOVERNANCE.md)
3. [`CURRENT_SLICE.md`](CURRENT_SLICE.md)
4. [`docs/DEVELOPMENT_PROCESS.md`](docs/DEVELOPMENT_PROCESS.md)
5. the exact active product-design approval or proof-harness-maintenance receipt
6. [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md)
7. accepted entries in [`docs/DECISION_REGISTER.md`](docs/DECISION_REGISTER.md)
8. [`docs/FIXTURE_CARDS.md`](docs/FIXTURE_CARDS.md)
9. [`docs/DESIGN_DOSSIER.md`](docs/DESIGN_DOSSIER.md)
10. [`docs/RESEARCH_BASIS.md`](docs/RESEARCH_BASIS.md)

The dossier is north-star material for the operator and technical lead. An engineering agent receives one bounded product or maintenance contract—not the whole repository history as an implementation prompt.

## Repository shape

```text
AGENTS.md                              hard repository execution law
GOVERNANCE.md                          authority and truth-class law
CURRENT_SLICE.md                       active work and derived operator frontier
docs/DEVELOPMENT_PROCESS.md            detailed development and review sequence
docs/process/                          accepted cross-slice process rulings
docs/maintenance/                      focused proof-harness maintenance receipts and reviews
docs/templates/                        product and maintenance authority templates
docs/prompts/                          reusable technical-lead and independent-review prompts
docs/slices/<SLICE_ID>/                product slice design/review/approval records
docs/DESIGN_DOSSIER.md                 human product and experience north star
docs/ARCHITECTURE.md                   provisional component boundaries and laws
docs/FIXTURE_CARDS.md                  Serum 2, Kontakt, and reference-fixture pressure
docs/DECISION_REGISTER.md              accepted, provisional, and open decisions
docs/RESEARCH_BASIS.md                 exact internal and external grounding
compatibility/                         future versioned compatibility profiles and schema
evidence/                              sanitized accepted product evidence only
tools/proof-run.py                     classified proof command
tools/proof_execution_policy.py        finite execution-authority policy
```

No proprietary plug-in binary, installer, preset library, activation file, token, credential, account data, or licensed content belongs in this repository.

## Reusable process artifacts

- [Implementation design card template](docs/templates/IMPLEMENTATION_DESIGN_CARD.md)
- [Slice-selection receipt template](docs/templates/SLICE_SELECTION_RECEIPT.md)
- [Design-approval receipt template](docs/templates/DESIGN_APPROVAL_RECEIPT.md)
- [Proof-harness maintenance receipt template](docs/templates/PROOF_HARNESS_MAINTENANCE_RECEIPT.md)
- [Prompt: choose the next slice](docs/prompts/CHOOSE_NEXT_SLICE.md)
- [Prompt: adversarially review product implementation design](docs/prompts/ADVERSARIAL_DESIGN_REVIEW.md)
- [Prompt: independently review proof-harness maintenance](docs/prompts/PROOF_HARNESS_MAINTENANCE_REVIEW.md)
- [Prompt: independently audit a successful acceptance candidate](docs/prompts/PRE_PR_IMPLEMENTATION_AUDIT.md)

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
