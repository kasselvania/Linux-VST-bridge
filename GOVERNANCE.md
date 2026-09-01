# Governance

## Purpose

This repository must retain a large coherent product intention without turning that intention into an unbounded implementation task. It separates product design, architecture, successor selection, active authority phase, approved implementation design, implementation fact, retained evidence, compatibility claims, and open decisions.

Strictness is preserved by moving design and adversarial review before expensive implementation—not by weakening acceptance after code exists.

## Truth classes

### 1. Operator-approved product design

`docs/DESIGN_DOSSIER.md` describes the intended product experience, user-visible laws, pressure fixtures, prohibited outcomes, and dependency-ordered proof shape. It is human north-star material.

It does not prove that any capability exists and does not automatically select a slice.

### 2. Repository architecture

`docs/ARCHITECTURE.md` defines accepted component boundaries, identity classes, latency classes, language boundaries, and technical nonclaims.

Architecture guides slice selection and implementation design. It does not create active implementation authority.

### 3. Successor-selection analysis

A next-slice analysis establishes the accepted boundary, first unproved edge, material unknowns, candidate slices, one recommendation, design-gate classification, and draft approval sentence.

It is analysis only. It does not edit the repository, select a slice, or authorize implementation.

Use `docs/prompts/CHOOSE_NEXT_SLICE.md`.

### 4. Slice-selection receipt

A slice-selection receipt records the operator's explicit choice of one bounded slice, exact basis, primary claim, fixture, authority phase, changed paths, external mutation, protected state, nonclaims, and design-gate decision.

For a design-gated slice it authorizes only reconnaissance and design. It must state `implementation_authorized: false`.

Use `docs/templates/SLICE_SELECTION_RECEIPT.md`.

### 5. Current slice authority

`CURRENT_SLICE.md` owns exactly one active slice and its current authority phase. No issue, prompt, pull-request description, comment, cleanup change, or exciting demo may silently widen it.

Recognized authority phases are:

```text
no_active_slice
reconnaissance_and_design
implementation
documentation_and_governance
```

When no slice is selected, `CURRENT_SLICE.md` must say so. For a design-gated slice, it may not say implementation is authorized until the exact design approval has been retained.

### 6. Approved implementation design

A high-risk implementation design card freezes the exact owner map, state machine, transitions, mutation/fault ledger, topology, identity rules, durability/recovery behavior, security/licensing posture, proof matrix, stop conditions, code topology, changed paths, fixture, and nonclaims.

Normal path:

```text
docs/slices/<SLICE_ID>/IMPLEMENTATION_DESIGN.md
```

An implementation design is not authority merely because it exists. It becomes approved only through:

- independent adversarial design review;
- technical-lead disposition;
- exact operator approval;
- a design-approval receipt identifying the immutable card revision;
- matching `CURRENT_SLICE.md` implementation authority.

Use `docs/templates/IMPLEMENTATION_DESIGN_CARD.md` and `docs/templates/DESIGN_APPROVAL_RECEIPT.md`.

### 7. Implementation fact

Code, tests, build definitions, generated artifacts, and accepted pull requests state what currently exists. Implementation fact does not silently amend product design or the approved implementation design.

### 8. Retained evidence

Evidence records what occurred on an exact fixture. It names versions, conditions, identities, result, proof class, and claim ceiling. Evidence from one fixture does not become universal compatibility.

Evidence may reveal a material design discovery. It does not authorize patching around that discovery.

### 9. Compatibility claim

A compatibility profile or matrix entry is a bounded claim over exact versions and declared conditions. It references evidence and explicit nonclaims. “Verified” without a declared capability set is prohibited.

### 10. Open decision

`docs/DECISION_REGISTER.md` separates accepted rulings, provisional choices, and unresolved questions. An open decision is not resolved by implementation convenience.

## Precedence

When language conflicts:

1. explicit operator product/design ruling;
2. `AGENTS.md`;
3. this governance file;
4. `CURRENT_SLICE.md` for active slice and authority phase;
5. exact approved design-approval receipt;
6. exact approved implementation design card;
7. accepted architecture and decision entries;
8. product dossier;
9. fixture cards and research basis;
10. issue / pull-request language;
11. implementation convenience.

A lower authority cannot widen a higher one. An implementation design may narrow architecture for one proof, but may not silently reverse a product law. Conflicts stop work.

## Two approvals for high-risk work

High-risk work requires two distinct approvals:

### Selection approval

Authorizes:

- one active slice;
- bounded reconnaissance;
- implementation-design work;
- exact design-review artifacts.

It does not authorize product implementation.

### Design approval

Authorizes implementation only against one exact reviewed design revision.

It binds:

- card path and identity;
- primary claim and fixture;
- owner/state/fault/proof model;
- changed paths and external mutation;
- stop conditions;
- explicit nonclaims.

An implementation prompt is not a substitute for either approval.

## Mandatory implementation-design gate

The gate is mandatory when a slice creates or materially changes:

- fact or component ownership;
- state machine or lifecycle;
- durable filesystem/database/registry/project mutation;
- transaction, rollback, migration, repair, or recovery;
- process creation, supervision, timeout, signalling, or termination;
- cross-process or cross-language communication;
- real-time behavior or deadlines;
- thread affinity, callback recursion, or reentrancy;
- identity, substitution, authorization, or access decisions;
- security, privacy, trust, or secret handling;
- vendor licensing or activation;
- third-party runtime behavior;
- persistent user data or content;
- externally visible compatibility claims.

A design-gate waiver must be explicit and explain why none of these conditions applies. Documentation-only, read-only reconnaissance, and genuinely mechanical maintenance may qualify. Small scope, model capability, or confidence is not a waiver.

## Design branch and review posture

For a design-gated slice:

1. activate `reconnaissance_and_design` authority;
2. perform bounded reconnaissance;
3. create or revise the implementation design card;
4. run independent adversarial design review in a fresh context;
5. resolve findings in the card;
6. retain the review disposition;
7. create the exact design-approval receipt;
8. update `CURRENT_SLICE.md` to `implementation` with `implementation_authorized: true`;
9. merge the approved design basis;
10. begin the implementation branch from that basis.

No production implementation belongs in the design-review diff.

## Material-discovery stop law

A discovery is material when it changes or invalidates an approved:

- owner;
- state or transition;
- mutation root;
- durability, commit, rollback, or recovery boundary;
- process/thread/callback topology;
- protocol direction or payload law;
- identity/authorization rule;
- security/privacy/licensing posture;
- fixture;
- primary claim or claim ceiling;
- changed-path envelope;
- proof matrix.

A material discovery requires:

```text
stop implementation
    -> retain bounded read-only evidence
    -> report RETURN_TO_DESIGN_GATE
    -> revise the design card
    -> independent re-review
    -> new approval receipt
    -> resume from the new approved basis
```

It must not become an improvised implementation repair.

A local detail that remains entirely inside every approved law does not require design reapproval.

## Independent-context law

The adversarial design reviewer and pre-PR implementation auditor must use fresh contexts that did not author the reviewed artifact.

The same frontier model may serve these roles in separate contexts. Independence means freedom from the authoring context's assumptions, not necessarily a different vendor or human.

Technical-lead PR review must not be the first independent architecture review.

## Implementation-agent context

An implementation agent receives:

- exact repository basis;
- `AGENTS.md` and `GOVERNANCE.md`;
- `CURRENT_SLICE.md` showing implementation authority;
- exact approved implementation design and identity;
- exact design-approval receipt;
- applicable architecture sections;
- exact existing interfaces;
- exact fixture requirements;
- tests and evidence obligations.

It does not receive the full dossier as one task contract and does not infer missing design.

## Fault-boundary completeness

Every implementation design lists all fallible external operations and answers:

> What is physically true if operation N succeeds and operation N+1 fails?

Externally observable intermediate states belong in the state machine. Examples include:

- rename completed but parent directory not fsynced;
- process spawned but ownership not proved;
- candidate promoted but durable commit not written;
- durable commit written but predecessor retirement incomplete;
- state saved but project commit incomplete;
- authorization succeeded but durable environment readback incomplete;
- evidence rendered but not published.

Recovery decisions use exact durable/physical readback. In-memory flags are not sole authority.

## Proof-matrix law

Every approved claim maps to:

```text
positive proof
negative/fault proof
real or synthetic fixture
production helper exercised
retained evidence
claim ceiling
```

A test name is not proof. A toy assertion is not equivalent to exercising the production owner. Synthetic tests are appropriate for deterministic faults; real runtime, ABI, DAW, process, authorization, and timing claims require real fixture evidence.

## Independent pre-PR audit

Before technical-lead PR review, a fresh-context auditor uses `docs/prompts/PRE_PR_IMPLEMENTATION_AUDIT.md` and returns one of:

```text
PRE_PR_AUDIT_CLEAR
PRE_PR_REPAIR_REQUIRED
RETURN_TO_DESIGN_GATE
PRE_PR_AUDIT_BLOCKED
```

Implementation defects may be repaired against the existing design. Design defects return to the design gate.

After finding one defect, the auditor inspects every operation governed by the same invariant.

## Design amendments

A material design amendment must identify:

- affected card and revision;
- current language;
- proposed language;
- reason and new evidence;
- owner/state/mutation/proof consequences;
- compatibility/migration consequences;
- adversarial review disposition;
- exact operator approval.

A roadmap, status update, prompt, issue, PR, refactor, or profile cannot amend design indirectly.

## Evidence-before-generalization

The proof progression is:

```text
exact fixture
    -> exact observation
    -> approved bounded implementation design
    -> bounded implementation claim
    -> repeated matrix evidence
    -> compatibility-class proposal
    -> operator-approved generalization
```

Invalid transformations include:

```text
worked once -> supported
plug-in opened -> plug-in works
sound came out -> real-time safe
project saved -> state round-trip is correct
installer exited zero -> installation is valid
browser opened -> authorization is supported
Steam Deck worked -> Linux works
Bitwig Flatpak worked -> every Flatpak DAW works
Serum 2 worked -> JUCE plug-ins work
Kontakt loaded -> Native Access is supported
```

## Repository independence

This repository may cite and learn from the GEP, science-dossier library, Standalone Bitwig Push, Wine, Proton, yabridge, format SDKs, DAW packaging, and vendor documentation.

It must not become a build/runtime/configuration dependency of those repositories merely because development happens alongside them. Cross-project reuse requires an explicit interface or package decision. Git repository topology is not product topology.

## Status discipline

Status language distinguishes:

- described;
- selected;
- reconnaissance-authorized;
- design-drafted;
- design-reviewed;
- design-approved;
- implementation-authorized;
- scaffolded;
- implemented;
- exercised;
- retained;
- verified at an exact matrix;
- generalized;
- production-supported.

Do not use “done,” “works,” “supported,” “production,” or “compatible” without a stated claim level.

## Compatibility lifecycle

A profile has a lifecycle independent from a plug-in installation:

```text
proposed
-> locally exercised
-> evidence retained
-> reviewed
-> verified for an exact matrix
-> superseded / withdrawn / known-regressed
```

Updating a profile cannot mutate already bound environments without an explicit transaction. A withdrawn profile remains traceable for old environments and project recovery.

## User data and proprietary material

The repository contains schemas, source code, test fixtures with redistribution rights, hashes, recipes, design records, and sanitized evidence. It does not contain commercial software or licensing state.

Environment backups, preset indices, content databases, project-state blobs, and authorization state are product data stored outside Git. Their export and privacy rules must be designed before production use.

## Merge posture

A pull request remains open until its exact claim, approved design identity where required, changed-path envelope, tests, evidence, nonclaims, and independent audit are reviewed.

Implementation slices are not selected or merged merely because they are next in a sequence. The technical lead performs explicit successor analysis.

After implementation merge, a separate status-only change records the accepted boundary and returns the repository to no-active-slice posture. It does not auto-select a successor.
