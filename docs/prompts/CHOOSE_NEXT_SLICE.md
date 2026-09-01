# Prompt — Choose the Next Slice

Use this prompt in a fresh technical-lead context after the repository has returned to no-active-slice posture.

```text
/goal

Act as the technical lead responsible for choosing the next bounded slice for
the Linux Audio Compatibility Bridge project.

This prompt authorizes analysis and recommendation only.

It does not authorize:

- editing the repository;
- replacing CURRENT_SLICE.md;
- creating a branch;
- implementing code;
- installing dependencies;
- launching a fixture;
- mutating an environment;
- changing Bitwig, Flatpak, Wine, Proton, Steam, or user content;
- opening or merging a pull request;
- selecting more than one active slice.

Repository:
kasselvania/Linux-VST-bridge

Expected working repository:
<EXACT REPOSITORY PATH OR GITHUB REPOSITORY>

Pinned current main:
commit: <CURRENT MAIN COMMIT>
tree:   <CURRENT MAIN TREE>

Last accepted slice:
<LAST ACCEPTED SLICE ID AND TITLE>

Last accepted implementation merge:
<MERGE COMMIT>

Last status-closure merge:
<STATUS CLOSURE COMMIT>

────────────────────────────────────────────────────────────────────
1. VERIFY THE BASIS
────────────────────────────────────────────────────────────────────

Confirm:

- the repository is the expected repository;
- main is exactly the pinned commit and tree;
- CURRENT_SLICE.md records no active implementation slice;
- the last accepted slice and status closure are present;
- no open implementation PR already owns a successor slice;
- accepted evidence packets relevant to the frontier verify;
- the repository state relevant to successor selection is internally
  consistent.

If any basis differs, return:

NEXT_SLICE_SELECTION_BLOCKED

State the exact mismatch. Do not silently select a newer basis, repair state,
or infer that an open branch is accepted implementation.

────────────────────────────────────────────────────────────────────
2. READ THE HUMAN AUTHORITY
────────────────────────────────────────────────────────────────────

Read in full:

- README.md
- AGENTS.md
- GOVERNANCE.md
- CURRENT_SLICE.md
- docs/DEVELOPMENT_PROCESS.md
- docs/ARCHITECTURE.md
- docs/DESIGN_DOSSIER.md
- docs/DECISION_REGISTER.md
- docs/FIXTURE_CARDS.md
- docs/RESEARCH_BASIS.md

Read:

- the implementation design card for the last accepted high-risk slice, if
  one exists;
- the complete retained evidence for the last accepted slice;
- the merged implementation paths for the last accepted slice;
- the previous one or two accepted boundaries where they materially constrain
  the next choice;
- unresolved decisions relevant to the current frontier.

Do not treat the dossier's dependency-ordered proof sequence as an automatic
roadmap.

────────────────────────────────────────────────────────────────────
3. ESTABLISH THE CURRENT ACCEPTED BOUNDARY
────────────────────────────────────────────────────────────────────

State exactly what the repository now proves.

Separate:

- accepted implementation fact;
- fixture-observed fact;
- retained evidence;
- provisional architecture;
- open decision;
- explicit nonclaim.

Describe the current end-to-end proved route:

accepted component
    -> accepted boundary
    -> accepted component
    -> current frontier
    -> first unproved boundary

Do not describe an intended component as though it exists.

────────────────────────────────────────────────────────────────────
4. IDENTIFY MATERIAL UNKNOWNS
────────────────────────────────────────────────────────────────────

List only unknowns that materially affect the next implementation decision.

For every unknown, classify it as:

- requires read-only reconnaissance;
- requires implementation proof;
- requires operator decision;
- requires vendor or external documentation;
- blocked by a missing fixture;
- deliberately deferred.

Identify whether any proposed implementation would combine multiple ownership
or uncertainty domains so that a failure would be uninterpretable.

────────────────────────────────────────────────────────────────────
5. IDENTIFY THE NEXT DEPENDENCY EDGE
────────────────────────────────────────────────────────────────────

Determine the smallest meaningful boundary immediately beyond the accepted
implementation.

The next slice must:

- establish one primary claim;
- have one comprehensible failure owner;
- preserve accepted fixtures;
- avoid combining independent uncertainty domains where practical;
- advance the product toward the design dossier;
- produce evidence useful to the next decision;
- remain valuable if it returns a truthful blocked result.

Do not choose work merely because it is visually exciting or listed next in a
prior sequence.

Do not choose cleanup merely because code has grown. Cleanup requires a
specific ownership, correctness, operability, or implementation-pressure
reason.

────────────────────────────────────────────────────────────────────
6. PRESENT AT MOST THREE CANDIDATES
────────────────────────────────────────────────────────────────────

For each candidate provide:

- slice ID and title;
- one-sentence primary claim;
- precise owner boundary;
- exact fixture;
- reason it is now implementable;
- accepted prerequisites it consumes;
- unresolved uncertainty domains;
- expected tracked-path/component envelope;
- permitted external mutation;
- prohibited mutation;
- success evidence;
- meaningful blocked result;
- explicit nonclaims;
- material stop conditions;
- whether reconnaissance is required first;
- whether an implementation design card is mandatory;
- applicable design-risk triggers:
  - owner boundary;
  - durable state;
  - process supervision;
  - cross-process protocol;
  - real-time behavior;
  - thread affinity/reentrancy;
  - security/privacy;
  - authorization/licensing;
  - third-party runtime;
  - migration/recovery;
  - compatibility claim;
  - user-owned content.

A candidate is not ready when its primary failure could belong to several
unseparated owners.

────────────────────────────────────────────────────────────────────
7. RECOMMEND EXACTLY ONE
────────────────────────────────────────────────────────────────────

Choose exactly one recommended slice.

Explain:

- why it is the smallest meaningful next proof;
- why it follows from the accepted boundary;
- why the other candidates are premature, broader, or lower-value;
- what becomes decidable after it;
- what it deliberately leaves unproved.

If no candidate is sufficiently bounded, recommend a reconnaissance or design
slice instead of forcing implementation.

────────────────────────────────────────────────────────────────────
8. CLASSIFY THE DESIGN GATE
────────────────────────────────────────────────────────────────────

Return one of:

DESIGN_GATE_REQUIRED

or:

DESIGN_GATE_WAIVED

The gate is required when the slice introduces or materially changes:

- an owner;
- a state machine;
- durable mutation;
- transaction, rollback, migration, repair, or recovery;
- process creation, supervision, signalling, or termination;
- cross-process or cross-language communication;
- real-time behavior;
- thread affinity or reentrancy;
- security/privacy;
- licensing/authorization;
- third-party runtime behavior;
- persistent user data;
- compatibility claims.

A waiver must positively explain why none of those conditions applies. Small
size or confidence is not a waiver reason.

When required, state:

- reconnaissance needed before design;
- implementation-design-card path;
- required independent adversarial design review;
- material-discovery stop conditions;
- implementation remains unauthorized until a separate design approval.

────────────────────────────────────────────────────────────────────
9. DRAFT THE SLICE-SELECTION RECEIPT
────────────────────────────────────────────────────────────────────

Use the schema from docs/templates/SLICE_SELECTION_RECEIPT.md and produce:

SLICE_SELECTION_RECEIPT

repository:
basis_commit:
basis_tree:
selected_slice:
selected_title:
primary_claim:
exact_fixture:
authority_phase:
  reconnaissance_and_design
  or
  implementation
implementation_authorized:
  false
  or
  true
design_gate:
design_gate_reason:
design_card_path:
allowed_changed_paths:
permitted_external_mutation:
protected_state:
explicit_nonclaims:
material_discovery_requires_stop: true
successor_selection_authorized: false

For a design-gated slice:

- authority_phase must be reconnaissance_and_design;
- implementation_authorized must be false.

Do not draft an implementation prompt for a design-gated slice before its
implementation design has been approved.

────────────────────────────────────────────────────────────────────
10. PRODUCE THE EXACT OPERATOR APPROVAL SENTENCE
────────────────────────────────────────────────────────────────────

For a design-gated slice, produce exactly:

I explicitly approve selecting <SLICE_ID> — <TITLE> and replacing the
no-active-slice card with its bounded reconnaissance-and-design authority.
This approval does not authorize implementation. Implementation requires a
separate approved design revision.

For a legitimately waived design gate, produce exactly:

I explicitly approve activating <SLICE_ID> — <TITLE>, replacing the
no-active-slice card, and implementing only its exact bounded claim and
changed-path/mutation envelope.

Do not interpret the generated sentence as approval. The operator must provide
it separately.

────────────────────────────────────────────────────────────────────
11. OUTPUT FORMAT
────────────────────────────────────────────────────────────────────

Return exactly these sections:

NEXT_SLICE_SELECTION_READY
or
NEXT_SLICE_SELECTION_BLOCKED

BASIS

CURRENT_ACCEPTED_BOUNDARY

FIRST_UNPROVED_BOUNDARY

MATERIAL_UNKNOWNS

CANDIDATE_A

CANDIDATE_B
if applicable

CANDIDATE_C
if applicable

RECOMMENDED_SLICE

WHY_THE_OTHER_CANDIDATES_WAIT

DESIGN_GATE_DECISION

REQUIRED_RECONNAISSANCE

DRAFT_SLICE_SELECTION_RECEIPT

EXACT_OPERATOR_APPROVAL_SENTENCE

No repository edit, branch, implementation prompt, or pull request is part of
this task.
```
