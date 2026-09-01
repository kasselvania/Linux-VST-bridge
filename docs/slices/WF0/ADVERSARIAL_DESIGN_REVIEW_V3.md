# WF0 V3 Adversarial Design Review

## Review identity

```yaml
reviewed_design_commit: 158e1229d1e3b6e84fbb05fc472ebf7f75887965
reviewed_design_tree: 2c312d069cc54cefb3b382dc87638d3fa1dc82fa
reviewed_design_path: docs/slices/WF0/IMPLEMENTATION_DESIGN_V3.md
reviewed_design_blob: 523d81f4b449ff96251dd1e52abb741b20033f95
reviewed_design_sha256: 948fbc249e399de82db8f7cd50f46a357e32be67ccdb7598b4d69183b921fc78
reviewed_revision: wf0-design-v3
review_context: fresh_read_only_context_that_did_not_author_v3
review_result: DESIGN_REPAIR_REQUIRED
implementation_authorized: false
```

The independent reviewer read the exact immutable V3 object, applicable
authority, complete V2 design/review/approval lineage, live reconciliation,
and historical helper boundaries. The review did not edit the repository,
install tooling, launch a workload, mutate Git, or contact an external
service.

## Authority, identity, and envelope check

The review confirmed:

- commit, tree, V3 blob, and V3 SHA-256 are exact;
- the V3 commit changes exactly `CURRENT_SLICE.md`,
  `docs/slices/WF0/BITWIG_6_1_FIXTURE_RECONCILIATION.md`, and
  `docs/slices/WF0/IMPLEMENTATION_DESIGN_V3.md`;
- SR0, HP0, HP1, WR0, and WR0A evidence and governed-helper trees are
  byte-identical to amendment basis
  `15523c69567d24b256cb3c65cb6f06bfa07854be`;
- the declared structural counts are exact: ten owners, 14 material
  operations, 23 lifecycle names, 15 call operations, 40 unique implementation
  paths split into 26 source/configuration and 14 evidence paths, 35 proof
  rows, and 23 blocked results; and
- the current Bitwig `6.1` protected snapshot is correctly separate from
  historical `6.0.11` evidence and from HP1 or WF0 behavioral acceptance.

No implementation diff was present. Authority remained
`reconnaissance_and_design` with `implementation_authorized: false`.

## Finding 1 — Binding V2 call-order clarifications are not explicit in V3

**Severity:** P1

**Affected V3 headings:** Section 6, scanner lifecycle and Windows
module/factory transition law; Section 20, adversarial review disposition;
Section 21, approved design identity; Section 22, implementation handoff.

**Violated invariant:** A successor design and its new approval must preserve
every binding implementation clarification from the authority it supersedes,
unless the amendment explicitly changes that law and receives review and
approval for doing so.

**Concrete false-acceptance state:** The immutable V2 review and approval bind
two requirements:

1. emit and synchronously flush each `call_completed` immediately after
   capturing the bounded return value and before any later lifecycle event or
   call attempt; and
2. resolve and check required `GetPluginFactory` before invoking optional
   `InitDll`, with a missing export taking `factory_export_missing` without
   executing `InitDll`.

V3 calls V2 historical authority but does not restate either clarification as
normative V3 language. Its call law requires only a subsequent completion
record, and its transition law does not explicitly prohibit `InitDll` after a
missing required export. Its future approval and handoff requirements also do
not bind the two clauses. A future implementation could therefore place a
lifecycle event between an ordinary call return and its completion record, or
execute `InitDll` before rejecting a missing `GetPluginFactory` export, while
appearing to follow V3.

**Why existing proof could miss it:** V3 retains the same operation enum,
lifecycle names, proof-row count, and blocked-result taxonomy. Those structural
counts do not prove the two causal ordering requirements survived into the new
approval contract.

**Required successor-design repair:** Restate both clarifications normatively
in the complete successor card. Require its future approval receipt and
implementation handoff to bind them. Cross-audit all 15 call completions,
required-export/optional-entry ordering, cleanup-failure precedence, and proof
rows 14 through 18 and 25 through 30.

**Adjacent operations under the same invariant:** `LoadLibraryExW`, optional
`InitDll`, required `GetPluginFactory`, factory-info and factory-interface
queries, class count, every class-info tier, all three release operations,
optional `ExitDll`, `FreeLibrary`, their lifecycle events, stage attribution,
cleanup precedence, retained timeline, approval receipt, and implementation
handoff.

**Additional live reconnaissance required:** false

The bounded repair requires no new owner, state, implementation path, call
operation, proof row, blocker, mutation, fixture execution, or external
reconnaissance.

## V3 disposition

```text
DESIGN_REPAIR_REQUIRED
implementation_authorized=false
```

The Bitwig `6.1` protected-fixture amendment is otherwise coherent and
fail-closed. A complete successor revision, fresh independent adversarial
review, and separate exact operator approval are required before implementation
may resume.
