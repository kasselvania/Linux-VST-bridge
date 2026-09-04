# Development Process

**Document class:** canonical human-facing development sequence.  
**Applies to:** product selection, design, implementation, proof-harness maintenance, diagnostic execution, acceptance, review, merge, and closure.  
**Rule:** other process documents reference this sequence rather than restating it.

## 1. Why the old loop failed

The repository previously treated nearly every workload-launching Steam Deck run as an acceptance event. An inconclusive harness failure could therefore trigger:

```text
runtime discovery
→ return to product design
→ revised design
→ adversarial review
→ design repair
→ fresh review
→ operator approval
→ authority finalization
→ authority merge
→ branch reconstruction
→ another scarce workload run
→ extensive failure packaging
```

That preserved evidence honesty, but made development intolerably slow. The same product contract was repeatedly constitutionalized because the proof instrument—not the product behavior—was still being developed.

The new process keeps strict acceptance and makes development iterative.

## 2. The complete reduced sequence

```text
1. establish accepted frontier
2. select one product contract
3. design and approve the product contract once
4. implement product code and proof harness locally
5. run deterministic/local validation
6. use a bounded non-authoritative diagnostic campaign when real-fixture debugging is needed
7. repair proof-harness defects under maintenance authority without reopening product design
8. freeze one exact acceptance candidate
9. run one strict acceptance transaction
10. audit, review, merge, and close status
```

A product slice should not repeat steps 2–3 unless the product contract itself changes.

## 3. Two independent axes

### Change class

```text
PRODUCT_CONTRACT_CHANGE
PROOF_HARNESS_MAINTENANCE
MECHANICAL_MAINTENANCE
```

### Execution class

```text
READ_ONLY_RECONCILIATION
DIAGNOSTIC_NON_AUTHORITATIVE
ACCEPTANCE_CANDIDATE
```

These axes answer different questions:

- **What changed?**
- **What kind of observation is this run?**

A proof-harness repair may use a diagnostic run and later a fresh acceptance run. A product change may also use diagnostics while developing its final candidate.

## 4. Phase A — accepted frontier and product selection

The technical lead establishes:

- current accepted capability;
- first unproved product boundary;
- one bounded primary claim;
- exact fixture;
- one failure owner;
- explicit nonclaims;
- expected development and acceptance cost.

The selection analysis must include:

```text
change_class: PRODUCT_CONTRACT_CHANGE
expected_diagnostic_budget
expected_acceptance_budget
expected_windows_producer_budget
expected_live_negative_budget
```

Selection does not authorize implementation.

## 5. Phase B — product design and approval

A high-risk product contract receives one implementation design card, one independent adversarial review, and one exact operator approval.

The design freezes:

- product owner and lifecycle;
- VST3 calls and interpretation;
- fixture semantics;
- product-facing normalization;
- containment and shutdown claims;
- product proof matrix;
- source envelope;
- diagnostic campaign plan, if needed;
- final acceptance plan and budget.

The approved product design remains valid through proof-harness maintenance as long as these product facts do not change.

### Return to product design

Return only when one of these changes:

1. primary claim or claim ceiling;
2. VST3 operation roster, order, or return interpretation;
3. product ownership or reference-count law;
4. product lifecycle state or transition;
5. Windows product or ABI behavior;
6. fixture meaning;
7. product-facing normalization;
8. containment or shutdown semantics claimed by the product;
9. protocol, trust, authorization, privacy, or security boundary;
10. product proof requirement or compatibility claim.

Use:

```text
RETURN_TO_PRODUCT_DESIGN
```

Do not use a generic design return for a harness-only defect.

## 6. Phase C — local implementation lane

Implement product code and the proof harness without live workload execution first.

Required cheap work includes the applicable subset of:

- compiler and syntax checks;
- deterministic owner/state tests;
- schema and canonical-JSON tests;
- source/build/Deck/renderer invalidation tests;
- static selected/prohibited-call checks;
- result-admission tests;
- diagnostic-publication tests;
- dry-run external-effect plan.

The full acceptance producer is not an edit/compile loop.

A local defect inside the approved product contract is an implementation repair. A proof-harness design defect enters proof-harness maintenance. Neither automatically reopens product design.

## 7. Phase D — bounded diagnostic campaign

Use a diagnostic campaign only when real fixture behavior is needed to develop or debug the instrument.

### DiagnosticCampaignIdentity

It binds:

```text
product_contract_identity
Windows artifact/build identity
fixture identity
Runtime/Proton identity
closed diagnostic plan
diagnostic harness source identity
campaign nonce
```

### Default live budget

For the open AGain fixture:

```text
diagnostic batches: 2 maximum per DiagnosticCampaignIdentity
```

Why two:

1. reproduce and diagnose the real-fixture problem;
2. verify the resulting harness repair.

More than two indicates that the campaign is not bounded or the defect is not understood. It requires an explicit operator ruling, not a silent retry.

Commercial or stateful fixtures default to one. Destructive or authorization-sensitive fixtures default to zero until explicitly designed.

### Diagnostic result law

Every diagnostic batch records:

```text
execution_class: diagnostic_non_authoritative
acceptance_eligible: false
diagnostic_campaign_identity
exact source/artifact/fixture/runtime identities
bounded diagnostic result
cleanup and protected-state disposition
actual effect counts
```

It cannot:

- create the tracked product evidence packet;
- set product proof rows to PASS;
- update the accepted frontier;
- be promoted or relabelled as acceptance;
- widen product calls or lifecycle;
- weaken cleanup.

### Diagnostic failure closure

Once process cleanup and protected-state readback are complete, post-failure work is limited to:

1. publish or retrieve one bounded diagnostic JSON object and sidecar;
2. validate that pair once;
3. retain cleanup and actual effect counts;
4. return one concise report.

Do not run a full pre-PR audit, render product success evidence, replay the product proof matrix, or create a design amendment merely to polish a diagnostic failure.

A failure report should answer only:

```text
what was attempted
what exact identity ran
where it failed
what cleanup is true
what budget remains
what the next development action is
```

If bounded closure itself fails, preserve the diagnostic bytes and return:

```text
DIAGNOSTIC_CLOSURE_BLOCKED
```

The failure report is not another slice.

## 8. Phase E — proof-harness maintenance

A harness repair stays outside product design when all are true:

- product claim unchanged;
- Windows product behavior unchanged;
- VST3 calls and order unchanged;
- product owner/lifecycle unchanged;
- fixture meaning unchanged;
- product normalization unchanged;
- containment/shutdown semantics unchanged;
- old evidence not relabelled;
- final acceptance will use a fresh candidate.

### Mechanical maintenance

Examples: comments, formatting, harmless renderer projection, deterministic test cleanup. Requires ordinary review.

### High-risk maintenance

Examples:

- durable failed-run diagnostics;
- result admission;
- process supervision and cleanup implementation;
- transaction recovery;
- identity joins;
- live-budget enforcement.

Required sequence:

```text
maintenance receipt
→ focused independent maintenance review
→ implementation
→ deterministic tests
→ optional bounded diagnostic campaign
→ merge
```

It does not require a new product selection, product design revision, or operator approval unless fixture semantics, security/privacy, external mutation, or live budgets change.

Use:

```text
PROOF_HARNESS_MAINTENANCE_REQUIRED
```

## 9. Phase F — freeze the acceptance candidate

Diagnostics end before acceptance begins.

Freeze:

```text
AcceptanceCandidateIdentity
exact candidate source
WindowsBuildInputIdentity
artifact and custody identity
fixture identity
Runtime/Proton identity
closed acceptance plan
acceptance authority
```

Entry conditions:

- deterministic tests green;
- no unresolved diagnostic process, environment, lock, or mutation;
- product contract unchanged;
- source frozen;
- artifact and fixture admitted;
- exact external-effect plan displayed;
- candidate has not already consumed its acceptance batch.

A diagnostic result is never promoted. Acceptance always makes a fresh product observation.

## 10. Phase G — strict acceptance lane

Default budget:

```text
acceptance batches: 1 per AcceptanceCandidateIdentity
```

Only acceptance may produce evidence supporting product merge.

### Acceptance outcomes

#### Product success

```text
retain exact result
→ render tracked evidence
→ independent pre-PR audit
→ technical-lead exact-head review
```

#### Conclusive product failure

If the product behavior itself fails under a functioning harness:

```text
ACCEPTANCE_CANDIDATE_FAILED
```

Classify as implementation repair or return to product design according to the exact contract boundary.

#### Harness-inconclusive failure

If product behavior was not conclusively observed because the instrument failed:

```text
ACCEPTANCE_CANDIDATE_INCONCLUSIVE
→ PROOF_HARNESS_MAINTENANCE_REQUIRED
```

The product design remains approved. After maintenance, freeze a **new** AcceptanceCandidateIdentity and perform a fresh acceptance observation. Preserve the earlier attempt in cumulative history.

#### Unknown remote outcome

Reconcile exact physical state before another launch. A lost acknowledgement does not authorize duplicate work.

### Acceptance failure closure

An unsuccessful acceptance attempt does not receive a success evidence packet or full proof-matrix audit. Retain one bounded failure diagnostic, cleanup, effect counts, and a concise classification. Full evidence and audit occur only after a successful acceptance candidate.

## 11. Read-only reconciliation lane

Read-only operations may:

- discover SSH availability;
- inspect exact locks and intents;
- verify artifact and fixture stores;
- inspect processes and protected state;
- retrieve already published results;
- reconcile lost acknowledgements.

They launch no workload and consume no diagnostic or acceptance batch. They remain bounded and truthfully counted as orchestration.

## 12. Budget ownership

| Budget | Identity owner |
|---|---|
| Windows producer | `WindowsBuildInputIdentity` |
| Artifact download/custody | exact artifact/custody identity |
| Diagnostic workload | `DiagnosticCampaignIdentity` |
| Acceptance workload | `AcceptanceCandidateIdentity` |
| Live negative replay | `FaultPlanIdentity` |
| Source transfer | source-handoff identity |
| Evidence rendering | `EvidenceRendererIdentity` plus retained result |
| Read-only reconciliation | orchestration count only |

Rules:

- diagnostic execution does not consume acceptance budget;
- acceptance does not consume diagnostic budget;
- renderer correction consumes neither live budget;
- artifact reuse consumes no producer budget;
- recovery of an existing operation does not count as a duplicate physical launch;
- every actual attempt remains in cumulative cost accounting.

## 13. Evidence classes

### Diagnostic evidence

Private or explicitly diagnostic. Always:

```text
acceptance_eligible: false
```

### Acceptance result

Private exact result from one AcceptanceCandidateIdentity. Must pass strict admission.

### Tracked product evidence

Rendered only from an admitted acceptance result. It retains producer P, execution E, consumer C, exact fixture, cleanup, protected state, proof dispositions, and claim ceiling.

No class is promoted into another by renaming a field or copying a file.

## 14. Review and merge

### Product implementation audit

Runs only after successful acceptance evidence exists. It audits the exact candidate head against the approved product design.

### Proof-harness maintenance review

Verifies:

- product contract unchanged;
- maintenance scope exact;
- diagnostic results remain ineligible;
- budgets and cleanup exact;
- deterministic tests exercise production helpers.

### Technical-lead review

Reviews exact head, evidence class, actual costs, cleanup, and claim ceiling.

### Merge

- Product capability merges only from successful acceptance evidence.
- Proof-harness maintenance may merge from deterministic and explicitly diagnostic evidence without advancing the product frontier.
- Status closure records the accepted product capability separately.

## 15. Operator-facing frontier

`CURRENT_SLICE.md` contains one derived, non-authoritative summary:

```text
last accepted capability
current target
current state
next capability if accepted
explicit nonclaim
```

It derives from authoritative fields and evidence. It is not a second authority owner.

## 16. Rules that remain strict

- exact P/E/C identities;
- exact source, build, artifact, fixture, runner, and plan identity;
- reconcile-before-repeat;
- zero-descendant cleanup and environment retirement;
- protected-state comparison;
- bounded canonical diagnostics;
- no credentials or proprietary state;
- no diagnostic promotion;
- independent acceptance audit;
- exact-head technical-lead review;
- no automatic successor.

## 17. Prohibited process patterns

- acceptance as an edit/debug loop;
- a single budget for all Deck contact;
- product redesign for a harness-only defect;
- unlimited diagnostic retries;
- elaborate tracked evidence for failed development runs;
- full audit of an unsuccessful candidate;
- replaying inherited proof matrices without changed invariants;
- manual run-ID, artifact-ID, SSH-path, or receipt choreography;
- hiding failed attempts from cumulative costs;
- allowing failure reporting to take longer than the experiment needed to teach its next actionable fact.

## 18. Process success criterion

The process succeeds when:

```text
strict acceptance evidence remains exact
and
development can fail cheaply, diagnose quickly, and iterate without repeatedly reopening an unchanged product constitution
```