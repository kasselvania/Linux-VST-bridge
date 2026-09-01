# Current Slice: None Selected

## Status

```text
status: no_active_slice
authority_phase: no_active_slice
implementation_authorized: false
last accepted slice: DG0 — Implementation Design Gate and Successor Selection Protocol
```

No Windows VST3 scanner, Windows host, native proxy/factory crossing, bridge IPC, shared-memory transport, Serum operation, Bitwig operation, installer/authorization session, manager, broker, editor, CLAP work, runner change, cleanup program, design amendment, or compatibility claim is selected or implied.

## Last accepted governance slice

```text
slice: DG0
pull request: #9
basis commit: a01c6121de63097fa56fa4f81825456b467b09ed
basis tree: c887e3acfe34c56cdd0f141dec761f4598dea098
reviewed head: 75c76813a602b0408d1e92823981df91c8161a26
reviewed tree: 7d3d2d157e03c1c315a71b8e867b0cc58ba706ed
squash merge: 04d7712aac1eb8ce05491419c24d137b64b4c3eb
```

## Accepted process

The repository now requires this lifecycle for high-risk work:

```text
accepted main / no active slice
    -> technical-lead successor analysis
    -> explicit operator selection
    -> bounded reconnaissance and design authority
    -> implementation design card
    -> independent adversarial design review
    -> exact design approval receipt
    -> implementation authorization
    -> implementation against the approved revision
    -> independent pre-PR implementation audit
    -> technical-lead exact-head review
    -> merge
    -> separate status closure
```

Selecting a high-risk slice does not authorize implementation. Material discoveries about ownership, state, mutation, durability/recovery, topology, protocol, identity, security/licensing, fixture, claim, paths, or proof coverage return work to the design gate.

The human process is:

```text
docs/DEVELOPMENT_PROCESS.md
```

The default successor-analysis prompt is:

```text
docs/prompts/CHOOSE_NEXT_SLICE.md
```

The design and approval templates are:

```text
docs/templates/IMPLEMENTATION_DESIGN_CARD.md
docs/templates/SLICE_SELECTION_RECEIPT.md
docs/templates/DESIGN_APPROVAL_RECEIPT.md
```

The required fresh-context reviews are:

```text
docs/prompts/ADVERSARIAL_DESIGN_REVIEW.md
docs/prompts/PRE_PR_IMPLEMENTATION_AUDIT.md
```

## Current accepted product frontier

The last accepted product implementation remains WR0:

```text
native Linux VST3 build and validation
    -> normal Bitwig discovery and native instance admission
    -> controlled Runtime 4 / Proton 11 Windows-command execution
    -> isolated project-owned Windows environment
```

No Windows VST3 module factory census, Windows plug-in host, native bridge proxy, cross-process VST3 protocol, audio/event transport, Serum operation, activation, or compatibility claim exists yet.

## Next lawful action

Run the exact analysis-only process in `docs/prompts/CHOOSE_NEXT_SLICE.md` against the current accepted `main` commit and tree.

That analysis may recommend one slice and emit an operator approval sentence. It may not edit the repository, activate a slice, or draft implementation for a design-gated successor.
