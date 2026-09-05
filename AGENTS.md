# AGENTS.md — Repository Execution Rules

## Mission

Build a managed Windows-audio compatibility platform for native Linux DAWs. A supported user should be able to install, authorize, publish, run, save, reopen, update, diagnose, and roll back a Windows plug-in without manually administering Wine, Proton, prefixes, generated proxies, Flatpak paths, runtime versions, or opaque recovery steps.

The Steam Deck, SteamOS, Bitwig, AGain, Serum, and Kontakt are fixtures. They do not define the universal product boundary.

## Completed AP0 ruling — Offline AGain sample processing

AP0 is accepted through PR #49 after exact-head technical-lead review. PC0 remains an accepted prerequisite. `CURRENT_SLICE.md` owns current status, the bounded accepted claim and the next proposed product direction.

The AP0 work order authorized implementation, necessary host builds, scoped diagnostics and fresh acceptance in one task. That task is now complete. Its original instructions and spending limits remain in Git history and the retained AP0 records; they do not authorize another build, workload, candidate or successor after this merge. Preserve the immutable evidence and all cumulative failed/diagnostic histories. Do not rerun successful work for status closure.

No implementation successor is active. Read-only investigation and review remain permitted; a subsequent scoped work order must define any new audio transport or processing capability. Preserve process containment, useful failure retention, private-state protection and honest evidence classification.

## Authority order

When instructions conflict, use this order:

1. explicit operator product or process ruling;
2. `AGENTS.md`;
3. `GOVERNANCE.md`;
4. `CURRENT_SLICE.md`;
5. exact approval or maintenance receipt for the active work;
6. exact approved implementation design, when one exists;
7. `docs/DEVELOPMENT_PROCESS.md`;
8. accepted architecture and decision records;
9. fixture cards and product dossier;
10. issue, pull-request, or implementation convenience.

Stop on a real conflict. Do not invent a third authority.

## Canonical process ownership

Do not duplicate the full process across files.

- `AGENTS.md` owns hard execution rules.
- `GOVERNANCE.md` owns authority and truth classes.
- `docs/DEVELOPMENT_PROCESS.md` owns the detailed sequence and decision rules.
- `CURRENT_SLICE.md` owns the active work and operator-facing frontier.
- Slice records own only slice-specific product contracts.

## Mandatory two-axis classification

Before editing or running anything, classify both axes.

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

A missing or ambiguous class means no workload launch is authorized.

## Product-contract change

Return to product design only when a change alters or invalidates one of:

- primary claim or claim ceiling;
- VST3 call roster, order, or return interpretation;
- product ownership or reference-count law;
- product lifecycle states or transitions;
- Windows product or ABI behavior;
- accepted fixture meaning;
- product-facing normalization or compatibility interpretation;
- containment or shutdown semantics claimed by the product;
- product protocol, trust, authorization, privacy, or security boundary;
- product proof requirements or user-visible compatibility claim.

A product-contract change requires the normal selection/design/review/approval path, except for an explicit scoped technical-lead ruling above.

## Proof-harness maintenance

A change is proof-harness maintenance only when all product-contract facts above remain unchanged and the work is confined to concerns such as:

- bounded diagnostic retention;
- failure sidecars and stream hashes;
- result admission and evidence rendering;
- transaction bookkeeping;
- source, artifact, or Deck orchestration;
- lost-acknowledgement recovery;
- read-only preflight;
- deterministic harness tests.

High-risk proof-harness maintenance requires one maintenance receipt and one independent technical review unless the active scoped ruling supplies its task authority and review timing. It does not reopen product selection merely because the harness changed.

Old evidence may never be relabelled. Final product acceptance still requires a fresh exact acceptance observation.

## Execution lanes

### READ_ONLY_RECONCILIATION

May inspect identities, stores, locks, processes, protected state, and retained results. It launches no plug-in workload and consumes no diagnostic or acceptance batch.

It must still be bounded, identity-checked, privacy-safe, and truthfully counted as orchestration.

### DIAGNOSTIC_NON_AUTHORITATIVE

A development observation used to understand integration or proof-harness behavior.

Required properties:

- exact diagnostic campaign identity;
- exact source, artifact, fixture, runner, and closed diagnostic plan;
- `acceptance_eligible: false` permanently;
- no tracked product evidence packet;
- no product proof row may become PASS from it;
- bounded diagnostic output and exact cleanup;
- no silent product-scope expansion;
- no promotion or relabelling into acceptance.

Default live budget for the open AGain fixture is **two diagnostic batches per DiagnosticCampaignIdentity** unless an explicit scoped ruling specifies otherwise. PC0-D1 retains its historical eight-reservation cumulative ceiling. A slice may choose fewer. Other increases require an explicit operator ruling or specifically delegated technical-lead decision. Commercial, stateful, destructive, or authorization-sensitive fixtures default to one or zero.

### ACCEPTANCE_CANDIDATE

The final authoritative measurement for one exact candidate.

Required properties:

- exact AcceptanceCandidateIdentity;
- frozen candidate source;
- exact artifact, fixture, runtime, and closed acceptance plan;
- one acceptance workload batch by default;
- strict result admission, evidence, cleanup, audit, and claim ceiling;
- only this class can support capability merge.

A diagnostic result is never promoted into an acceptance result.

## Failure-closure budget

Failure must be cheap to report.

After a diagnostic or inconclusive acceptance workload exits and cleanup readback completes, post-run work is limited to:

1. publish or retrieve one bounded diagnostic JSON object and sidecar;
2. verify cleanup and protected-state disposition;
3. record actual external-effect counts;
4. return one concise failure summary.

Do **not** render the success evidence packet, replay the proof matrix, run the full pre-PR audit, create a product-design amendment, or spend hours polishing a failed run.

The default hard closure limit is one bounded diagnostic-validation pass and one short report. If that cannot be completed promptly, preserve the raw bounded diagnostic and return `DIAGNOSTIC_CLOSURE_BLOCKED`. Incremental privacy-safe capture before normalization and cleanup remains permitted; this reporting limit must not erase an original observation. The failure report itself is not another slice.

## Budget ownership

Budgets attach to the identity of the expensive operation:

| Budget | Owner |
|---|---|
| Windows producer | `WindowsBuildInputIdentity` |
| Artifact custody | exact artifact and custody identity |
| Diagnostic Deck batches | `DiagnosticCampaignIdentity` |
| Acceptance Deck batch | `AcceptanceCandidateIdentity` |
| Live negative fixtures | `FaultPlanIdentity` |
| Source transfer | exact source-handoff identity |
| Evidence render | `EvidenceRendererIdentity` plus retained result |
| Read-only preflight | orchestration count only |

One budget does not silently consume another. A renderer correction consumes no live budget. A diagnostic batch consumes no acceptance budget. A lost acknowledgement requires reconciliation, not a duplicate launch.

## Required development sequence

For a product slice:

```text
approve product contract once
→ implement locally
→ deterministic/local validation
→ bounded diagnostic campaign when needed
→ repair proof harness under maintenance authority
→ freeze acceptance candidate
→ one strict acceptance execution
→ independent audit
→ exact-head technical-lead review
→ merge and status closure
```

Do not use an acceptance transaction as the edit/compile/debug loop. The active scoped ruling may combine implementation, candidate preparation and reporting into one task without intermediate handoffs.

## Stop results

Use specific outcomes:

```text
RETURN_TO_PRODUCT_DESIGN
PROOF_HARNESS_MAINTENANCE_REQUIRED
DIAGNOSTIC_BUDGET_EXHAUSTED
DIAGNOSTIC_CLOSURE_BLOCKED
ACCEPTANCE_CANDIDATE_INCONCLUSIVE
ACCEPTANCE_CANDIDATE_FAILED
IMPLEMENTATION_REPAIR_REQUIRED
```

`RETURN_TO_DESIGN_GATE` is retained only for historical records. New work must name whether the product contract or the proof harness owns the problem.

## Evidence integrity

- Producer P, execution E, and consumer C remain distinct.
- Every observation names its execution class and eligibility.
- Diagnostic observations are permanently acceptance-ineligible.
- No retroactive relabelling or promotion.
- Exact build, artifact, fixture, source, runtime, and plan identities.
- Canonical bounded JSON and external sidecars.
- Strict admission rather than a success flag.
- Incomplete observations remain unknown.
- Screenshots and logs are supporting material, not architecture or audio proof.

## Process, cleanup, and security invariants

Every workload launch must retain exact process ownership, bounded timeout, zero-descendant cleanup, process-group disposition, environment retirement, and protected-state comparison. Diagnostic mode does not weaken cleanup.

Never commit or log credentials, cookies, license files, machine-bound activation data, account identifiers, proprietary installers, plug-in binaries, paid presets, private environment dumps, raw unbounded logs, home paths, hostnames, IP addresses, PIDs, or pointer values.

The Deck performs no GitHub operation and receives no GitHub credentials or forwarded SSH agent. SteamOS's immutable base is not disabled as a product requirement.

Wrong recovery is worse than explicit failure. Never silently substitute another plug-in build, class, environment, fixture, artifact, or content root.

## Core architecture invariants

- A native Linux DAW loads a native Linux proxy; a supervised Windows host loads the Windows module.
- Rust is the product language. C++20 is permitted only at the official VST3 boundary or another explicitly proved narrow edge.
- C++ objects, exceptions, STL ownership, and RTTI do not cross the Rust/C++ ABI.
- Language and process boundaries use separately versioned typed contracts.
- VST3 interfaces are not collapsed into an imagined generic RPC object without explicit coverage.
- Host-to-plug-in and plug-in-to-host calls may be reentrant.
- Win32 and Linux thread-affinity obligations remain on their correct threads.
- The DAW remains project and audio-session authority.
- A friendly name or path is not durable identity.
- One giant global Wine prefix and one process hosting every plug-in are prohibited defaults.

## Real-time laws

On the real-time path: no allocation after activation, filesystem or network access, process creation, ordinary logging, unbounded locks, configuration work, recovery decisions, or synchronous editor work. Timing and lock-free claims require direct proof.

## Review law

Independent review is required for product design and final implementation. Proof-harness maintenance uses its own focused review and must first prove that the product contract is unchanged.

A green test suite is necessary but insufficient. Repeated implementation failures are a signal to stop the current diagnostic campaign or repair the harness—not to create successive product constitutions.
