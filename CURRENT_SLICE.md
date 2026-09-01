# Current Slice: DG0 — Implementation Design Gate and Successor Selection Protocol

## Status

```text
status: active_governance_slice
authority_phase: documentation_and_governance
implementation_authorized: false
slice: DG0
branch: codex/dg0-implementation-design-gate
basis commit: a01c6121de63097fa56fa4f81825456b467b09ed
basis tree: c887e3acfe34c56cdd0f141dec761f4598dea098
target: main
```

The operator explicitly directed the technical lead to immortalize the implementation-design process and the next-slice selection prompt in the human-facing repository.

## Primary claim

DG0 makes the repository's development process explicit and enforceable:

```text
accepted main / no active slice
    -> technical-lead successor analysis
    -> operator selects one bounded slice
    -> bounded reconnaissance and design authority
    -> implementation design card
    -> independent adversarial design review
    -> exact design approval receipt
    -> implementation authorization
    -> implementation against the approved revision
    -> independent pre-PR implementation audit
    -> technical-lead exact-head review
    -> merge
    -> separate no-active-slice closure
```

It distinguishes slice selection from implementation authorization, defines when the implementation-design gate is mandatory, defines material-discovery stop behavior, defines independent review roles, and provides reusable prompts and templates.

## Changed-path envelope

Only these paths may change:

```text
README.md
AGENTS.md
GOVERNANCE.md
CURRENT_SLICE.md
docs/DEVELOPMENT_PROCESS.md
docs/DESIGN_DOSSIER.md
docs/DECISION_REGISTER.md
docs/templates/IMPLEMENTATION_DESIGN_CARD.md
docs/templates/SLICE_SELECTION_RECEIPT.md
docs/templates/DESIGN_APPROVAL_RECEIPT.md
docs/prompts/CHOOSE_NEXT_SLICE.md
docs/prompts/ADVERSARIAL_DESIGN_REVIEW.md
docs/prompts/PRE_PR_IMPLEMENTATION_AUDIT.md
```

## Acceptance

DG0 is complete when:

- the human process is readable from the repository root;
- the authority phases are unambiguous;
- high-risk design-gate triggers and waiver law are explicit;
- the implementation design card requires owner, state, mutation/fault, identity, proof, stop-condition, and code-topology decisions;
- material discoveries return work to the design gate;
- design review and pre-PR audit use independent fresh contexts;
- the exact operator selection and design-approval receipts are templated;
- the reusable next-slice selection prompt performs analysis only and emits an exact approval sentence;
- all relative document links resolve;
- no product implementation or external fixture mutation occurs.

## Explicit non-goals

DG0 does not:

- choose the Windows VST3 scanner slice;
- authorize implementation of any successor;
- modify the accepted WR0 environment;
- launch Proton, Wine, Bitwig, Serum, or another fixture;
- change Flatpak, SteamOS, `.wine`, Steam compatdata, or user content;
- refactor WR0;
- add source code, dependencies, CI, schemas, or compatibility profiles;
- amend the product architecture or claim compatibility.

## Last accepted implementation boundary

WR0 is accepted through implementation PR #7 and merge commit `8237b96ce7c885edcf4e7a0923f2ac78d05a928d`. Its status closure merged as `a01c6121de63097fa56fa4f81825456b467b09ed`.

DG0 is a process-governance response to the lesson that empirical discoveries, implementation design, coding, and architectural review must not collapse into one patch-forward loop. It does not diminish WR0's accepted Windows-execution result.
