# Current Slice: DX0 — Split Build Identity and Single-Command Proof Transaction

## Status

```text
status: active_design_slice
authority_phase: reconnaissance_and_design
implementation_authorized: false
slice: DX0
target: main
selection_basis_commit: 859422be75f65da4dd7dc51394b84470ad594358
selection_basis_tree: 7b9a721f3e691974dd1720c20d9ef030932978a9
selection_receipt: docs/slices/DX0/SLICE_SELECTION.md
design_gate: required
design_revision: dx0-design-v1
design_status: proposed_for_adversarial_review
design_card: docs/slices/DX0/IMPLEMENTATION_DESIGN.md
design_blob: b71024010e828fba9acc5c9a2f82546a5bb6e6c9
design_sha256: de40a98822bc7ea4fda8684047f77007177626e5b8a49cdfeda7bed2d845682e
design_branch: codex/dx0-split-build-identity-proof-transaction-design
successor_selection_authorized: false
```

## Primary claim

> From one exact Mac-side command and one closed proof plan, the existing proof system derives a Windows-build-input identity separate from the complete implementation-source identity, reuses an accepted content-addressed AGain fixture without rebuilding it, produces or reuses the exact compatible Windows host artifact, performs source custody, Mac-to-Deck handoff, detached-worktree admission, one supervised Deck proof batch, cleanup, and evidence return, and proves that a non-build-only source change causes zero Windows builds.

## Why DX0 is active

WA0 established the exact `IAudioProcessor` interface boundary, but also established that the development system invalidates and repeats too much work:

```text
non-build evidence or orchestration change
    -> whole implementation source identity changes
    -> full Windows acceptance build and downstream transaction repeat

floating windows-2022 runner
    -> unchanged accepted fixture source can produce different bytes
    -> repeated six-to-seven-minute build attempts become runner roulette

manual cross-plane operation
    -> implementation agent coordinates build, custody, bundle, SSH handoff,
       Deck ref, worktree, artifact admission, proof runs, and evidence return
```

Accepted decision D-026 prohibits another VST3 product-capability slice until this execution-cost boundary is paid down.

## Exact design boundary

DX0 designs only the proof-pipeline paydown. It adds no VST3 capability and authorizes no product implementation.

The design must cover:

- a Windows-build-input identity distinct from complete implementation source;
- a content-addressed accepted-fixture identity and reuse law;
- a cheap development-validation lane distinct from final acceptance production;
- one repository-owned Mac-side orchestration entry point;
- one closed proof-plan shape;
- deterministic versus live-proof classification;
- source-freeze, invalidation, restart, cleanup, and evidence laws;
- measurable developer-cost acceptance criteria.

## Hard scope and cost ceiling

The future implementation design must target:

```text
new architectural owners:       3 maximum
source/configuration paths:      target 8-10; hard stop above 12
new Windows C++ product paths:   0
new VST3 calls:                  0
new fault-fixture modules:       0
new retained evidence files:     5 maximum
Windows acceptance builds:       1 maximum
live Deck proof batches:          1 maximum
new product capability:           none
```

The design may not create a new runner, build plane, custody system, transport, supervisor, broker, database, persistent service, release pipeline, or generic workflow language.

## Design-phase execution prohibition

Reconnaissance and design are repository-read-only except for the bounded design-authority files. During this phase, do not:

- run GitHub Actions;
- build Windows artifacts;
- download or transfer artifacts;
- contact the Steam Deck;
- launch Runtime, Proton, Wine, Bitwig, Serum, or a scanner;
- modify the accepted fixture store or protected state;
- create implementation source or evidence;
- select a successor.

## Approved design-phase path envelope

The cumulative design branch may differ from the selection basis only at:

```text
CURRENT_SLICE.md
docs/slices/DX0/SLICE_SELECTION.md
docs/slices/DX0/IMPLEMENTATION_DESIGN.md
```

The activation commit contains only the first two paths. The implementation design remains unapproved and implementation remains unauthorized.

## Next lawful action

A design agent may perform bounded repository reconnaissance and create `docs/slices/DX0/IMPLEMENTATION_DESIGN.md`. A fresh independent review and separate operator approval are required before implementation.
