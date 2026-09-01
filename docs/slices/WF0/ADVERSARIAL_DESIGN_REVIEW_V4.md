# WF0 V4 Adversarial Design Review

## Review identity

```yaml
reviewed_design_commit: 566625445c1b3dca3a253da54756513b0c5862e3
reviewed_design_tree: 9e03cc47caf044237310a9ffd155c4d908142894
reviewed_design_path: docs/slices/WF0/IMPLEMENTATION_DESIGN_V4.md
reviewed_design_blob: 6701ddf068dc1853ad746b15df413fc12fe095fd
reviewed_design_sha256: 138f545ff28f9d1f4f76e049d4d0fa30186bfcebae627f18140b0be0eb55ad09
reviewed_revision: wf0-design-v4
review_context: fresh_read_only_context_that_did_not_author_v4
review_result: DESIGN_REPAIR_REQUIRED
implementation_authorized: false
```

The independent reviewer read the exact immutable V4 object, applicable
authority, the complete V2 through V4 design/review/approval lineage, the live
Bitwig reconciliation, and historical helper boundaries. The review did not
edit the repository, install tooling, launch a workload, mutate Git, or contact
an external service.

## Authority, identity, and envelope check

The review confirmed:

- commit, tree, V4 blob, and V4 SHA-256 are exact;
- V4 changes only `CURRENT_SLICE.md` and
  `docs/slices/WF0/IMPLEMENTATION_DESIGN_V4.md` from its repair basis;
- the complete amendment-basis delta contains only `CURRENT_SLICE.md`, the
  Bitwig reconciliation, V3 design, V3 review, and V4 design;
- SR0, HP0, HP1, WR0, and WR0A evidence and governed-helper trees are
  byte-identical to amendment basis
  `15523c69567d24b256cb3c65cb6f06bfa07854be`;
- the declared structural counts remain exact: ten owners, 14 material
  operations, 23 lifecycle names, 15 call operations, 40 unique implementation
  paths split into 26 source/configuration and 14 evidence paths, 35 proof
  rows, and 23 blocked results;
- current Bitwig `6.1` remains protected state only and is not promoted to HP1
  or WF0 behavioral acceptance; and
- the all-15-call immediate paired completion-and-flush law and cleanup-failure
  precedence are otherwise retained.

No implementation diff was present. Authority remained
`reconnaissance_and_design` with `implementation_authorized: false`.

## Finding 1 — V4 contains incompatible lifecycle-order requirements

**Severity:** P1

**Affected V4 headings:** Section 6, scanner lifecycle and Windows
module/factory transition law; Section 16, proof matrix; Section 22,
implementation handoff.

**Violated invariant:** A successful branch must have one conforming lifecycle
timeline. Implementation may not choose between contradictory order laws.

**Concrete false-acceptance state:** V4 retains the 23-state lifecycle order in
which `module_entry_*` precedes `factory_export_*` and says successful execution
retains that ordering. Its repaired transition law instead requires emitting
`factory_export_found`, then resolving and invoking optional `InitDll`, then
emitting the entry result. Its row-18 proof clause likewise requires `InitDll`
to occur only after the `factory_export_found` event.

Every required-export-present branch therefore lacks a conforming timeline:

- following the transition and row-18 law violates the retained 23-state
  order; and
- following the retained 23-state order violates the transition and row-18
  law.

This is an authority gap. Implementation cannot select either interpretation
without inventing design. V4 therefore does not fully resolve the V3 P1.

**Required successor-design repair:**

1. Resolve and check `GetPluginFactory` before optional `InitDll` without
   requiring immediate publication of `factory_export_found`.
2. On a missing export, emit `factory_export_missing`, never invoke `InitDll`,
   and preserve exact `ExitDll`/`FreeLibrary` cleanup and primary-failure
   precedence.
3. On a present export, perform optional entry handling first, including the
   immediate paired `call_completed` flush, then emit the entry lifecycle
   result, `factory_export_found`, and only afterward attempt
   `GetPluginFactory`.
4. Require row 18 to prove that `InitDll` is reachable only after successful
   required-export resolution and check, not after the later lifecycle event.
5. Bind that distinction and the all-15-call completion law into rows 14
   through 18 and 25 through 30, the future approval receipt, and the future
   implementation handoff.

**Adjacent operations under the same invariant:** required-export resolution,
optional `InitDll`, `factory_export_missing`, `factory_export_found`, the entry
lifecycle result, `GetPluginFactory`, paired call completions, cleanup
attribution and precedence, proof rows 14 through 18 and 25 through 30,
approval receipt, and implementation handoff.

**Additional live reconnaissance required:** false

The bounded repair requires no new owner, state, call operation,
implementation path, proof row, blocker, mutation, fixture execution, or
external reconnaissance.

## V4 disposition

```text
DESIGN_REPAIR_REQUIRED
implementation_authorized=false
```

The Bitwig `6.1` protected-fixture amendment and V4's other laws are otherwise
coherent and fail-closed. A complete successor revision, fresh independent
adversarial review, and separate exact operator approval are required before
implementation may resume.
