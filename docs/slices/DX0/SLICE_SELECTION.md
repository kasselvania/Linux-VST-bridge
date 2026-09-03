# DX0 Slice Selection Receipt

## Selection identity

```text
schema: linux-vst-bridge-slice-selection/v1
slice: DX0
title: Split Build Identity and Single-Command Proof Transaction
selection_status: selected_for_reconnaissance_and_design
authority_phase: reconnaissance_and_design
implementation_authorized: false
design_gate: required
repository: kasselvania/Linux-VST-bridge
selection_basis_commit: 859422be75f65da4dd7dc51394b84470ad594358
selection_basis_tree: 7b9a721f3e691974dd1720c20d9ef030932978a9
last_accepted_slice: WA0 — Windows VST3 Audio-Processor Interface Admission
design_branch: codex/dx0-split-build-identity-proof-transaction-design
design_card: docs/slices/DX0/IMPLEMENTATION_DESIGN.md
successor_selection_authorized: false
```

## Operator approval

> I explicitly approve selecting DX0 — Split Build Identity and Single-Command Proof Transaction and replacing the no-active-slice card with its bounded reconnaissance-and-design authority. This approval does not authorize implementation. Implementation requires a separate approved design revision.

## Selection reason

WA0 proved the processor-side `IAudioProcessor` interface boundary and also produced direct evidence that the proof system is not sufficiently amortized. The exact accepted repository now records D-026, which makes developer wall-clock cost, agent/token cost, invalidation granularity, and manual cross-plane choreography first-class architecture concerns.

Observed failure shape:

```text
floating hosted Windows runner image
    -> unchanged fixture source can rebuild to different bytes
    -> acceptance attempts become runner roulette

non-build-only implementation change
    -> complete source identity changes
    -> Windows build, artifact custody, source handoff, Deck admission,
       live proof, and evidence transaction are all invalidated

manual proof transaction
    -> the implementation agent repeatedly operates the Mac, GitHub Actions,
       SSH handoff, Deck worktree, artifact cache, Runtime/Proton, and evidence
       return as separate procedural steps
```

Selecting another VST3 capability before correcting this would knowingly repeat an accepted development-system defect.

## Primary claim

> From one exact Mac-side command and one closed proof plan, the existing proof system derives a Windows-build-input identity separate from the complete implementation-source identity, reuses an accepted content-addressed AGain fixture without rebuilding it, produces or reuses the exact compatible Windows host artifact, performs source custody, Mac-to-Deck handoff, detached-worktree admission, one supervised Deck proof batch, cleanup, and evidence return, and proves that a non-build-only source change causes zero Windows builds.

## Intended developer boundary

Before DX0:

```text
agent manually operates build, artifact lookup, custody, source bundle,
SSH transfer, Deck ref, detached worktree, artifact admission, preflight,
proof exercises, cleanup, evidence handoff, and final identity closure
```

After DX0:

```text
host-proof run --source <commit> --plan <closed-plan>
```

The single command owns the existing mechanisms and returns one exact transaction result. The caller does not manually coordinate cross-plane shipping steps.

## Required design results

The implementation design must freeze the smallest coherent solution for:

1. **WindowsBuildInputIdentity** — a deterministic identity over only Windows-binary-affecting repository blobs, pinned SDK/toolchain inputs, build configuration, and required accepted host-build dependencies.

2. **AcceptedFixtureIdentity** — a content-addressed identity for the already accepted AGain fixture bytes, with an admission/reuse law that does not rebuild the fixture merely because a successor slice changes.

3. **ProofTransactionDriver** — one Mac-side owner that composes existing build selection or production, custody, source handoff, Deck admission, one proof batch, cleanup, evidence return, and final identity closure.

The design must also define:

- complete-source identity versus build-input identity;
- exactly which path changes invalidate the Windows host artifact;
- exactly which path changes do not;
- source freeze before expensive external proof;
- cheap local/development validation before acceptance production;
- deterministic owner/state tests versus genuinely live Proton tests;
- artifact and fixture reuse without weakening exact custody;
- one closed proof-plan format, not a general workflow language;
- failure ownership and resumability without manual procedural repair;
- transaction output and evidence small enough to review;
- measurable developer-cost acceptance criteria.

## Hard scope and implementation budget

The future design is constrained to:

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

A design that requires more than twelve source/configuration paths, repeated Windows builds, multiple live Deck batches, another negative-fixture family, or another cross-plane architecture is not DX0 and must stop for technical-lead review.

## Accepted infrastructure to compose, not redesign

DX0 consumes the accepted WF0/WC0/WA0 mechanisms where sound:

- Windows/MSVC host build;
- exact artifact byte custody;
- source-bundle transport;
- ordinary non-forwarded SSH;
- rootless Steam Deck detached worktrees;
- content-addressed artifact admission;
- Runtime 4 / Proton 11 launch;
- process ownership and cleanup;
- protected-state verification;
- evidence handoff and sanitization.

The point is to put these behind one correct owner and split their invalidation domains. DX0 may make bounded internal changes needed for that composition, but may not create a replacement build plane, custody architecture, transfer lane, process supervisor, runner, broker, or service.

## Explicit nonclaims

DX0 does not establish or perform:

- another VST3 interface or method;
- bus negotiation;
- sample-size capability;
- processing setup;
- audio or event processing;
- controller creation or connection;
- native proxy work;
- C ABI or IPC;
- Bitwig hosting of a Windows plug-in;
- Serum operation or authorization;
- product packaging, signing, or release suitability;
- final product-runner selection;
- general Windows VST3 or Linux compatibility.

## Reconnaissance authority

Design-phase reconnaissance is limited to read-only inspection of repository source, accepted evidence, workflow history already represented in the repository or GitHub metadata, and official documentation required to interpret the existing mechanisms.

No design-phase GitHub Actions run, Windows build, artifact transfer, Deck contact, Runtime/Proton launch, fixture mutation, or evidence production is authorized.

## Design-phase path envelope

The activation commit changes exactly:

```text
CURRENT_SLICE.md
docs/slices/DX0/SLICE_SELECTION.md
```

A design commit may additionally create only:

```text
docs/slices/DX0/IMPLEMENTATION_DESIGN.md
```

Implementation requires a fresh independent adversarial review and a separate operator approval bound to one exact design revision.