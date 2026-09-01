# Development Process

**Document class:** human-facing repository process and authority law.  
**Applies to:** successor selection, reconnaissance, implementation design, coding, proof, review, merge, and status closure.  
**Purpose:** preserve strict engineering while moving architectural discovery and adversarial reasoning before expensive implementation.

## 1. Why this process exists

This project crosses native plug-in ABIs, Windows compatibility runtimes, process supervision, durable environments, real-time constraints, Flatpak boundaries, vendor authorization, proprietary content, and user project state. A convincing demo can be technically real while still establishing the wrong owner, recovery boundary, identity, or claim.

WR0 proved that frontier models can discover and implement difficult real-machine behavior. It also exposed a process weakness: a material topology discovery was followed by local implementation repairs while the full owner/state/fault model was still evolving. Technical-lead PR review became the first place where several architecture-wide invariants were audited together.

The correction is not lower rigor, smaller ambition, or weaker models. The correction is to separate:

```text
authority
reconnaissance
design
adversarial design review
implementation
independent implementation audit
technical-lead merge review
```

The repository freezes each boundary before moving to the next.

## 2. Governing distinction

> **Selecting a slice does not automatically authorize implementation.**

A selected high-risk slice first enters `reconnaissance_and_design`. Implementation begins only after an exact design revision has passed independent adversarial review and has an accepted design-approval receipt.

A low-risk slice may receive an explicit design-gate waiver. Silence is not a waiver.

## 3. Lifecycle

```text
NO_ACTIVE_SLICE
  -> NEXT_SLICE_ANALYSIS
  -> OPERATOR_SELECTION
  -> RECONNAISSANCE_AND_DESIGN
  -> DESIGN_REVIEW
  -> DESIGN_APPROVAL
  -> IMPLEMENTATION
  -> PRE_PR_AUDIT
  -> TECHNICAL_LEAD_REVIEW
  -> MERGED_IMPLEMENTATION
  -> STATUS_CLOSURE
  -> NO_ACTIVE_SLICE
```

A material discovery can transition:

```text
RECONNAISSANCE_AND_DESIGN -> revised design
IMPLEMENTATION -> RETURN_TO_DESIGN_GATE
PRE_PR_AUDIT -> RETURN_TO_DESIGN_GATE
TECHNICAL_LEAD_REVIEW -> RETURN_TO_DESIGN_GATE
```

It must not silently become a patch to an obsolete design.

## 4. Roles

The same frontier model may perform multiple roles, but design review and implementation audit must use fresh independent contexts that did not author the artifact being reviewed.

### Operator

- Owns product direction and explicit slice/design approval.
- Supplies lawful proprietary fixtures and credentials only through vendor-controlled surfaces.
- Decides material product/design amendments.

### Technical lead

- Establishes the accepted boundary.
- Chooses or recommends one bounded successor.
- Classifies the design gate.
- Reviews the implementation design and adversarial findings.
- Issues the exact design-approval receipt.
- Reviews the final PR head and decides merge/repair/return-to-design.

### Design agent

- Performs approved reconnaissance.
- Produces the implementation design card.
- Does not write product implementation during the design phase.
- Revises the card in response to design review.

### Adversarial design reviewer

- Uses a fresh context.
- Attacks the owner map, complete state space, fault boundaries, identities, topology, security, proof matrix, and claim ceiling before code exists.
- Does not implement a preferred solution.

### Implementation agent

- Implements only the exact approved design revision.
- Stops on material discovery.
- Does not self-amend design authority.

### Pre-PR implementation auditor

- Uses a fresh context.
- Compares the exact implementation head to the approved design.
- Distinguishes implementation defects from design defects.
- Returns design defects to the design gate rather than repairing around them.

## 5. Phase A: next-slice analysis

Use [`prompts/CHOOSE_NEXT_SLICE.md`](prompts/CHOOSE_NEXT_SLICE.md).

This phase is analysis only. It verifies accepted `main`, states the current proved route, identifies the first unproved boundary, presents at most three candidates, recommends exactly one, classifies the design gate, drafts a selection receipt, and emits the exact operator approval sentence.

It does not edit the repository or authorize implementation.

## 6. Phase B: operator selection

The operator supplies the exact approval sentence emitted by the selection analysis.

For a design-gated slice:

```text
I explicitly approve selecting <SLICE_ID> — <TITLE> and replacing the
no-active-slice card with its bounded reconnaissance-and-design authority.
This approval does not authorize implementation. Implementation requires a
separate approved design revision.
```

For a legitimately waived gate:

```text
I explicitly approve activating <SLICE_ID> — <TITLE>, replacing the
no-active-slice card, and implementing only its exact bounded claim and
changed-path/mutation envelope.
```

The selected authority is recorded using [`templates/SLICE_SELECTION_RECEIPT.md`](templates/SLICE_SELECTION_RECEIPT.md).

## 7. When the design gate is mandatory

The gate is required if a slice creates or materially changes any of these:

- component or fact ownership;
- state machine or lifecycle;
- durable filesystem, database, registry, or project mutation;
- transaction, rollback, migration, repair, or recovery;
- process creation, supervision, timeout, signalling, or termination;
- cross-process or cross-language protocol;
- real-time path or deadline behavior;
- thread affinity, callback recursion, or reentrancy;
- identity, substitution, authorization, or access decisions;
- security, privacy, trust, or secret handling;
- vendor licensing or activation;
- third-party runtime contract;
- persistent user data or content;
- externally visible compatibility claim.

A gate may be waived only when the technical lead positively states why none of these conditions applies. Documentation-only, read-only reconnaissance, or genuinely mechanical maintenance may qualify. “The change is small” is not sufficient.

## 8. Phase C: bounded reconnaissance

Reconnaissance establishes facts needed to design. It may inspect an exact fixture and retain bounded evidence. It does not improvise implementation.

Before reconnaissance, the slice card declares:

- exact fixture;
- read-only or permitted mutation envelope;
- exact unknowns;
- collection bounds;
- privacy/proprietary exclusions;
- blocked-result law.

If reconnaissance changes the expected owner or topology, that is a design input—not implementation permission.

## 9. Phase D: implementation design card

Use [`templates/IMPLEMENTATION_DESIGN_CARD.md`](templates/IMPLEMENTATION_DESIGN_CARD.md).

The normal path is:

```text
docs/slices/<SLICE_ID>/IMPLEMENTATION_DESIGN.md
```

A high-risk design card is not a prose wish list. It freezes:

1. primary claim and claim ceiling;
2. exact fixture and accepted prerequisites;
3. owner map;
4. complete state machine, including externally observable intermediate states;
5. transition ledger;
6. mutation and fault ledger;
7. process/thread/reentrancy topology where applicable;
8. identity and authorization ledger;
9. durability and recovery law;
10. security/privacy/licensing posture;
11. proof matrix;
12. live versus synthetic test requirements;
13. material-discovery stop conditions;
14. approved code topology and changed paths;
15. open unknowns and blocked-result classifications.

### The syscall-boundary rule

For every fallible external operation, the design must answer:

> What is physically true if this operation succeeds and the very next operation fails?

If the operating system, runtime, DAW, database, or vendor process can expose a distinct intermediate state, that state belongs in the design.

Examples include:

```text
renamed but parent directory not fsynced
promoted but durable commit not written
process spawned but ownership not yet proved
gate written but callback not yet acknowledged
state stored but host project not yet committed
new version active while predecessor retirement is incomplete
```

### The physical-state rule

Recovery decisions use durable records plus exact physical readback. In-memory booleans are event hints, not authority over filesystem or process reality.

## 10. Phase E: adversarial design review

Use [`prompts/ADVERSARIAL_DESIGN_REVIEW.md`](prompts/ADVERSARIAL_DESIGN_REVIEW.md) in a fresh context.

The reviewer attacks the whole invariant class, not only obvious lines:

- owner completeness;
- unreachable or omitted states;
- fallible-operation gaps;
- crash/reboot/retry behavior;
- concurrent actors;
- process ancestry and cleanup;
- thread affinity and reentrancy;
- identity substitution;
- secret/proprietary leakage;
- licensing boundaries;
- proof strength;
- whether tests call production helpers;
- evidence truth before and after commit points;
- claim overreach.

The result is either:

```text
DESIGN_CLEAR
```

or:

```text
DESIGN_REPAIR_REQUIRED
```

A finding that changes ownership, state, mutation, topology, or proof strategy requires a revised card and another exact review.

## 11. Phase F: design approval

The technical lead and operator approve one exact design revision using [`templates/DESIGN_APPROVAL_RECEIPT.md`](templates/DESIGN_APPROVAL_RECEIPT.md).

The approval binds:

- slice ID;
- card path;
- Git blob and/or SHA-256;
- primary claim;
- fixture;
- owner/state/fault/proof model;
- changed paths and external mutation;
- stop conditions;
- explicit nonclaims.

The design branch may be merged only when the final card, adversarial review disposition, approval receipt, and `CURRENT_SLICE.md` agree. Implementation branches start from that accepted design basis.

## 12. Phase G: implementation

The implementation agent receives:

- exact main basis;
- `AGENTS.md` and `GOVERNANCE.md`;
- `CURRENT_SLICE.md` showing `implementation_authorized: true`;
- exact approved design card and digest;
- exact design-approval receipt;
- applicable architecture and fixture sections;
- exact code interfaces and tests.

The agent implements the card, not the full dossier.

### Material discovery

A discovery is material when it changes or invalidates any approved:

- owner;
- state or transition;
- mutation root;
- durability or recovery boundary;
- process/thread topology;
- protocol or callback direction;
- identity/authorization rule;
- security/privacy/licensing posture;
- fixture;
- primary claim or claim ceiling;
- changed-path envelope;
- proof matrix.

On material discovery, the agent must:

1. stop before widening or patching around it;
2. preserve safe read-only evidence;
3. report `RETURN_TO_DESIGN_GATE` with the exact mismatch;
4. revise and re-review the design;
5. obtain a new design approval before resuming.

A local implementation detail that remains inside all approved laws does not require reapproval.

## 13. Phase H: independent pre-PR audit

Use [`prompts/PRE_PR_IMPLEMENTATION_AUDIT.md`](prompts/PRE_PR_IMPLEMENTATION_AUDIT.md) in a fresh context before technical-lead PR review.

The auditor reviews the exact candidate head against every design section and may return:

```text
PRE_PR_AUDIT_CLEAR
PRE_PR_REPAIR_REQUIRED
RETURN_TO_DESIGN_GATE
```

- `PRE_PR_REPAIR_REQUIRED` means code or tests fail an already approved design and may be repaired without redesign.
- `RETURN_TO_DESIGN_GATE` means the approved design is incomplete, contradicted, or materially changed.

The audit should include a cross-invariant scan: after finding one defect, inspect every operation governed by the same invariant.

## 14. Phase I: technical-lead review and merge

The technical lead reviews the exact PR head, not the branch name or summary.

The review confirms:

- exact basis and topology;
- approved design identity;
- changed-path envelope;
- owner/state/fault fidelity;
- production helper behavior;
- live fixture evidence where required;
- negative/failure tests;
- security/privacy/licensing boundaries;
- claim ceiling and nonclaims;
- no unresolved material design discovery.

Outcomes:

```text
CLEAR_AND_MERGED
REPAIR_REQUIRED
RETURN_TO_DESIGN_GATE
BLOCKED_RESULT_ACCEPTED
```

A repair prompt must state whether it is an implementation repair or a design return. Repeated local repairs are a signal to perform a whole invariant-class audit, not merely make the latest line pass.

## 15. Phase J: status closure

After implementation merge, a separate status-only change returns `CURRENT_SLICE.md` to:

```text
status: no_active_implementation_slice
implementation: forbidden
```

It records the accepted claim, exact merge identity, retained evidence, and claim ceiling. It does not auto-select a successor.

## 16. Per-slice repository records

A design-gated slice should retain:

```text
docs/slices/<SLICE_ID>/
  IMPLEMENTATION_DESIGN.md
  ADVERSARIAL_DESIGN_REVIEW.md
  DESIGN_APPROVAL.md
```

The active slice card references their exact paths and identities. Historical cards remain immutable after the implementation basis is accepted; a material amendment creates a new revision rather than rewriting history invisibly.

## 17. Proof quality

Every design claim maps to:

| Claim | Positive proof | Negative proof | Required fixture | Production helper exercised | Retained evidence | Claim ceiling |
|---|---|---|---|---|---|---|

A test name is not proof. A test that restates a condition in a toy lambda is not equivalent to exercising the production owner. Synthetic tests are appropriate for controlled faults; live tests are required when the real runtime topology, ABI, DAW, process, or vendor behavior is the claim.

## 18. Process anti-patterns

Prohibited:

- treating the implementation prompt as design approval;
- handing the full dossier to an agent as one task;
- implementing while reconnaissance is still changing ownership;
- patching forward after material discovery;
- allowing PR review to be the first adversarial architecture review;
- using in-memory flags as sole authority over physical state;
- naming tests after requirements without exercising production behavior;
- selecting the next slice automatically after merge;
- converting blocked evidence into a different unapproved route;
- weakening strictness because the process exposed difficult facts.

## 19. Success criterion for the process

The process succeeds when strictness is retained while most design defects are found before implementation, and technical-lead PR review primarily verifies fidelity rather than discovering the implementation architecture for the first time.
