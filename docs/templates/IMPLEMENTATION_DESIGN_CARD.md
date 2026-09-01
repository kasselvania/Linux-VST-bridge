# Implementation Design Card — `<SLICE_ID>`

**Template class:** mandatory design artifact for a design-gated slice.  
**Normal path:** `docs/slices/<SLICE_ID>/IMPLEMENTATION_DESIGN.md`.  
**Rule:** remove all instructional placeholder text before approval. An implementation agent must not infer omitted design.

---

## 1. Identity and authority

```text
slice_id:
title:
design_revision:
design_status: draft | review_required | approved | superseded
repository:
basis_commit:
basis_tree:
selection_receipt_path:
selection_receipt_identity:
current_slice_path: CURRENT_SLICE.md
authority_phase: reconnaissance_and_design
implementation_authorized: false
prepared_by:
prepared_at:
```

### Design-gate trigger

State every trigger that made the gate mandatory:

```text
[ ] owner boundary
[ ] state machine/lifecycle
[ ] durable mutation
[ ] transaction/rollback/migration/recovery
[ ] process supervision/termination
[ ] cross-process or cross-language protocol
[ ] real-time/deadline behavior
[ ] thread affinity/reentrancy
[ ] identity/authorization
[ ] security/privacy
[ ] licensing/vendor activation
[ ] third-party runtime
[ ] persistent user data/content
[ ] compatibility claim
```

## 2. Primary claim and claim ceiling

### Primary claim

One bounded sentence stating exactly what becomes true.

> `<PRIMARY CLAIM>`

### End-to-end route

```text
accepted owner/input
    -> new boundary
    -> new owner/output
```

### Claim ceiling

State everything that the slice does not prove. Include later adjacent capabilities that a successful demo might tempt someone to infer.

## 3. Exact fixture and accepted prerequisites

| Fixture/prerequisite | Exact identity | Accepted source/evidence | Mutation permitted? |
|---|---|---|---|
|  |  |  |  |

State what must be reverified immediately before implementation and live exercise.

## 4. Reconnaissance findings

List only facts actually observed or established by primary documentation.

| Question | Observed answer | Evidence | Design consequence | Still unknown |
|---|---|---|---|---|
|  |  |  |  |  |

### Implementability verdict

```text
IMPLEMENTABLE
```

or:

```text
DESIGN_BLOCKED
```

A blocked design must state the exact missing fact and must not invent an implementation route.

## 5. Owner map

Every consequential fact has exactly one authoritative owner.

| Fact | Authoritative owner | Durable representation | Readers | Forbidden competing authority |
|---|---|---|---|---|
|  |  |  |  |  |

Questions:

- Who may create the fact?
- Who may mutate it?
- Who may retire it?
- What exact readback proves it?
- What happens if in-memory state and physical state disagree?

## 6. State machine

### Legal states

| State | Authoritative facts | Physical/external reality | Entry proof | Exit operation |
|---|---|---|---|---|
|  |  |  |  |  |

Include every state another process, the filesystem, the DAW, the runtime, or a crash/restart can observe.

### Legal transitions

```text
STATE_A
  -> STATE_B
  -> STATE_C
```

| From | Operation | To | Durability/readback required | Failure classification |
|---|---|---|---|---|
|  |  |  |  |  |

### Forbidden transitions

List transitions that must fail closed.

### Restart/retry reconciliation

For every durable/intermediate state, state what a new process does after crash, reboot, cancellation, or retry.

## 7. Fallible-operation and mutation ledger

For every external mutation or fallible operation, answer the syscall-boundary question:

> What is physically true if this operation succeeds and the next operation fails?

| # | Precondition | Fallible operation | Immediate physical result | Durability barrier | Exact readback | Next authority state | Failure owner | Recovery action |
|---:|---|---|---|---|---|---|---|---|
| 1 |  |  |  |  |  |  |  |  |

Include as applicable:

- file create/write/rename/delete;
- parent-directory `fsync`;
- database transaction/commit;
- registry mutation;
- process spawn;
- IPC handshake;
- shared-memory grant;
- network/browser handoff;
- DAW scan/admission;
- vendor authorization transition;
- state save/project commit;
- content move/locate;
- evidence publication.

### Commit points

State every irreversible authority boundary.

```text
before <COMMIT POINT>:
    rollback owner and exact restoration law

after <COMMIT POINT>:
    authoritative new state and prohibited rollback behavior
```

## 8. Process, thread, callback, and topology model

Omit only when demonstrably inapplicable.

| Role | Created by | Parent/owner | Required thread/process | Lifetime | Exit/cleanup owner |
|---|---|---|---|---|---|
|  |  |  |  |  |  |

### Actual or expected topology

```text
owner
  -> child
      -> descendant
```

State:

- exact ownership identity;
- process-group/session behavior;
- thread affinity;
- callback direction;
- recursive/reentrant calls;
- allowed blocking;
- deadlines;
- timeout output;
- cleanup and empty proof;
- stale PID/object protection;
- unrelated same-name protection.

## 9. Identity and authorization ledger

Every dangerous operation states the complete identity that authorizes it.

| Operation | Required identity | Freshness/readback | Refusal cases |
|---|---|---|---|
|  |  |  |  |

Examples:

```text
terminate process:
    exact PID + start ticks + exact owned ancestry + session/run identity

restore predecessor:
    transaction-derived canonical path + exact predecessor identity + snapshot equality

load plug-in class:
    exact module digest + VST3 class ID + environment revision + runner revision
```

Never use a friendly name, basename, path suffix, PID alone, or in-memory boolean as sufficient identity.

## 10. Data, durability, recovery, and migration

| Data class | Owner | Location | Sensitive? | Durability | Backup/rollback | Migration law |
|---|---|---|---|---|---|---|
|  |  |  |  |  |  |  |

State:

- atomicity model;
- durable commit record;
- reconciliation after partial failure;
- idempotency/retry behavior;
- predecessor retention/retirement;
- version/schema migration;
- unknown-object refusal;
- user-visible recovery result.

## 11. Security, privacy, licensing, and proprietary material

### Threats

| Threat | Boundary | Prevention | Negative proof |
|---|---|---|---|
|  |  |  |  |

### Sensitive/proprietary exclusions

State what may never enter Git, logs, diagnostics, screenshots, or support bundles.

### Network and authorization

State exact permitted network/browser/vendor behavior. Unknown vendor behavior remains unknown.

### Third-party licensing

List dependencies, licenses, distribution obligations, and clean-room constraints.

## 12. Approved code topology

| Component/file | Owner responsibility | Public interface | Must not own |
|---|---|---|---|
|  |  |  |  |

Even when an early proof uses one source file, identify conceptual owners and their APIs. Do not allow one procedure to become the implicit owner of unrelated state, process, transaction, evidence, and UI concerns.

## 13. Changed-path and external-mutation envelope

### Allowed tracked paths

```text
<EXACT PATHS OR PREFIXES>
```

### Permitted external mutation

```text
<EXACT USER-SPACE PATHS / APPLICATION STATE / NONE>
```

### Protected state

List exact identities that must remain unchanged.

### Prohibited mutation

List adjacent repositories, runtimes, DAW settings, proprietary fixtures, and user data that must not change.

## 14. Proof matrix

| Claim | Positive proof | Negative/fault proof | Real fixture required? | Production helper exercised? | Retained evidence | Claim ceiling |
|---|---|---|---|---|---|---|
|  |  |  |  |  |  |  |

Rules:

- Test names are not proof.
- A toy lambda is not equivalent to the production owner.
- Synthetic faults are appropriate for deterministic injection.
- Real runtime, ABI, DAW, process, authorization, and timing claims require real fixture evidence.
- Every incomplete or capped observation propagates to `unknown`, not absence.

## 15. Failure and blocked-result taxonomy

| Result code | Exact owner/stage | Preserved state | Cleanup | Next lawful action |
|---|---|---|---|---|
|  |  |  |  |  |

Do not reuse a broad “launch failed” result for observability, cleanup, identity, authorization, content, or evidence failures.

## 16. Material-discovery stop conditions

Implementation stops and returns to the design gate if any observation changes:

```text
[ ] owner map
[ ] state or transition set
[ ] mutation root
[ ] durability/commit/recovery boundary
[ ] process/thread/callback topology
[ ] protocol direction or payload law
[ ] identity/authorization law
[ ] security/privacy/licensing posture
[ ] exact fixture
[ ] primary claim or claim ceiling
[ ] changed-path envelope
[ ] proof matrix
```

Add slice-specific stop conditions:

- `<CONDITION>`

The agent may retain bounded read-only evidence but may not patch forward.

## 17. Implementation sequence

This is dependency order within the slice, not a roadmap.

```text
1. ...
2. ...
3. ...
```

Identify the clean implementation commit required before live evidence when source identity matters.

## 18. Adversarial design review disposition

```text
review_path:
review_identity:
review_result: DESIGN_CLEAR | DESIGN_REPAIR_REQUIRED
findings_resolved:
unresolved_findings:
```

Summarize every material design change made after review.

## 19. Approved design identity

The card does not approve itself.

```text
card_path:
card_git_blob:
card_sha256:
design_revision:
approval_receipt_path:
approval_receipt_identity:
implementation_authorized: false
```

Implementation becomes authorized only when a separate accepted approval receipt and `CURRENT_SLICE.md` reference this exact revision.

## 20. Implementation handoff

The implementation prompt must include:

- exact accepted main commit/tree;
- exact design card path and identity;
- exact approval receipt;
- one primary claim;
- exact fixture;
- exact changed paths and external mutation;
- acceptance and negative proof matrix;
- material-discovery stop law;
- explicit nonclaims;
- PR and merge posture.

Do not hand the implementation agent the whole dossier as its task contract.
