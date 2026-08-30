# Governance

## Purpose

This repository must retain a large coherent product intention without turning that intention into an unbounded implementation task. It separates design, architecture, active authority, implementation fact, evidence, compatibility claims, and open decisions.

## Truth classes

### 1. Operator-approved product design

`docs/DESIGN_DOSSIER.md` describes the intended product experience, user-visible laws, pressure fixtures, prohibited outcomes, and dependency-ordered proof shape. It is human north-star material.

It does not, by itself, prove that any capability exists.

### 2. Repository architecture

`docs/ARCHITECTURE.md` defines accepted component boundaries, identity classes, latency classes, language boundaries, and technical nonclaims.

Architecture guides slice selection. It does not create an implementation claim.

### 3. Current implementation authority

`CURRENT_SLICE.md` owns exactly one active bounded implementation claim. No issue, prompt, pull-request description, comment, cleanup change, or exciting demo may silently widen it.

When no slice is selected, `CURRENT_SLICE.md` must say so.

### 4. Implementation fact

Code, tests, build definitions, generated artifacts, and accepted pull requests state what currently exists. Implementation fact does not silently amend the product design.

### 5. Retained evidence

Evidence records what occurred on an exact fixture. It must name versions, conditions, result, and claim ceiling. Evidence from one fixture does not become universal compatibility.

### 6. Compatibility claim

A compatibility profile or matrix entry is a bounded claim over exact versions and declared conditions. It references evidence and explicit nonclaims. “Verified” without a declared capability set is prohibited.

### 7. Open decision

`docs/DECISION_REGISTER.md` separates accepted rulings, provisional choices, and unresolved questions. An open decision is not resolved by implementation convenience.

## Precedence

When language conflicts:

1. explicit operator ruling;
2. `AGENTS.md`;
3. this governance file;
4. `CURRENT_SLICE.md` for active work;
5. accepted architecture and decision entries;
6. product dossier;
7. fixture cards and research basis;
8. implementation convenience.

A current slice may narrow architecture for one proof. It may not silently reverse a product law. A proposed reversal requires an explicit design amendment.

## Design amendments

A design amendment must identify:

- affected document and exact heading;
- current language;
- proposed language;
- reason for change;
- evidence or pressure that motivated it;
- implementation consequences;
- compatibility/migration consequences;
- explicit operator decision.

A roadmap, status update, prompt, issue, PR, refactor, or profile cannot amend design indirectly.

## Evidence-before-generalization

The proof progression is:

```text
exact fixture
    -> exact observation
    -> bounded implementation claim
    -> repeated matrix evidence
    -> compatibility-class proposal
    -> operator-approved generalization
```

The following transformations are invalid:

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

It must not become a build/runtime/configuration dependency of the GEP or Push repositories merely because development happens alongside them. Likewise, this repository does not import authority from the central dossier library at runtime.

Cross-project reuse requires an explicit interface or package decision. Git repository topology is not product topology.

## Design versus implementation-agent context

The full dossier is for the operator and technical lead. An implementation agent receives:

- exact repository basis;
- `AGENTS.md` and `GOVERNANCE.md`;
- the current slice;
- only the applicable architecture sections;
- exact existing interfaces;
- exact fixture requirements;
- tests and evidence obligations.

The agent independently scans implementability and reports a blocker before editing. It does not infer a whole program from the dossier.

## Status discipline

Status language must distinguish:

- described;
- researched;
- fixture-observed;
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

The repository contains schemas, source code, test fixtures with redistribution rights, hashes, recipes, and sanitized evidence. It does not contain the user's commercial software or licensing state.

Environment backups, preset indices, content databases, project-state blobs, and authorization state are product data stored outside Git. Their export and privacy rules must be designed before production use.

## Merge posture

A pull request should remain open until its exact claim, changed-path envelope, tests, evidence, and nonclaims have been reviewed. Documentation-only seeding may be merged only when explicitly accepted by the operator or when the operator's request directly authorizes repository setup.

Implementation slices are not selected or merged solely because they are the next item in a sequence. The technical lead must inspect current implementation and present one bounded decision.
