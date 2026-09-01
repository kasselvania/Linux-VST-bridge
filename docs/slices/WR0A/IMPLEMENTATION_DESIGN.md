# Implementation Design Card — WR0A

## 1. Identity and authority

```text
slice_id: WR0A
title: Post-Merge Final WR0 Repair Reconciliation and Adoption
design_revision: wr0a-design-v3
design_status: proposed_for_adversarial_review
repository: kasselvania/Linux-VST-bridge
basis_commit: b517f96bafa388107ea28c9d2e91528fd37b00d9
basis_tree: 839cda6145f592642f06b5273855b04ffa0562ac
selection_receipt_path: docs/slices/WR0A/SLICE_SELECTION.md
selection_receipt_mode: 100644
selection_receipt_identity: git-blob 4f3131c93ef7b8178a623032146316226071a10b; sha256 6fd547646d1501831b6264ec9f0f729fa1feac7cf6f482278330a0a96644da2a
selection_receipt_lineage: original git-blob fe7b3f84f5f427cc48ce6b41752686812f678051 clarified without changing the operator selection
current_slice_path: CURRENT_SLICE.md
authority_phase: reconnaissance_and_design
implementation_authorized: false
prepared_by: repository reconnaissance and implementation-design agent
prepared_at: 2026-09-01T14:23:36Z
```

```text
supersedes_design:
  commit: 2aeac55f0297b5d8c6b843eb5ce489d936999923
  tree: 6be95aaca2eddf8008a0177bf07bac48921133c0
  path: docs/slices/WR0A/IMPLEMENTATION_DESIGN.md
  blob: 9375172252f6dca73636916536a1bcbd8641115d
  sha256: 0ccf184612d7312334e65658181f1bb32ae5d3ddb847cb8bab042d470533b15e
  revision: wr0a-design-v2
  verdict: DESIGN_REPAIR_REQUIRED
  github_review_id: 5080009434
```

Revision v3 resolves the supplied v2 review as one exact-byte, transaction-
ownership, and bootstrap-order repair. It is not clear, approved, merged
authority, or implementation authority. A fresh independent context must
review the exact committed v3
blob. No field in this card is designed to be rewritten after that review.

### Design-gate trigger

```text
[x] owner boundary
[x] state machine/lifecycle
[x] durable mutation
[x] transaction/rollback/migration/recovery
[x] process supervision/termination (accepted source semantics must be preserved)
[ ] cross-process or cross-language protocol
[ ] real-time/deadline behavior
[ ] thread affinity/reentrancy
[x] identity/authorization
[x] security/privacy
[ ] licensing/vendor activation
[x] third-party runtime
[x] persistent user data/content
[x] compatibility claim
```

WR0A reconciles four independently durable truths—merged Git history,
immutable archive source, the current live environment, and current main—so
the design gate is mandatory even though the intended adoption contains no
new product behavior.

## 2. Primary claim and claim ceiling

### Primary claim

> Current main adopts the exact content-addressed final WR0 repair represented
> by archive commit `52be94316664f88a19df630e164105a0ca50b875`
> without re-running or mutating the live WR0 environment.

### End-to-end route

```text
current main b517f96 / merged WR0 historical truth
    -> exact immutable archive commit/tree/parent verification
    -> exact 13-blob allow-listed adoption (never archive CURRENT_SLICE)
    -> exact ten-file WR0 contract-source digest c6543004...
    -> exact final WR0 canonical evidence packet
    -> bounded read-only live environment comparison
    -> dedicated WR0A reconciliation evidence
    -> independently audited implementation PR
    -> technical-lead merge
    -> separate status closure
```

### Claim ceiling

WR0A proves only repository/live-state reconciliation. It does not create a
new WR0 execution result, rerun any accepted workload, replace or migrate the
environment, modify runner/runtime files, select a product runner, load/scan/
host a Windows VST3, inspect or launch Serum, launch Bitwig, process audio,
implement a native proxy, bridge, IPC, shared memory, manager, broker, GUI,
authorization flow, Rust component, real-time path, Steam-independent
distribution, or compatibility claim beyond the exact accepted Deck fixture.

## 3. Exact fixture and accepted prerequisites

| Fixture/prerequisite | Exact identity | Accepted source/evidence | Mutation permitted? |
|---|---|---|---|
| Selection-basis main | `b517f96bafa388107ea28c9d2e91528fd37b00d9` / tree `839cda6145f592642f06b5273855b04ffa0562ac` | technical-lead receipt plus exact local object | none; implementation begins only after future authority merge |
| Current selection receipt | `100644` / blob `4f3131c93ef7b8178a623032146316226071a10b`; SHA-256 `6fd547646d1501831b6264ec9f0f729fa1feac7cf6f482278330a0a96644da2a`; predecessor blob `fe7b3f84...` | exact reviewed v2 tree plus original operator text | none after v3 commit |
| Merged WR0 history | `9228217b2abf7314b9dfaecc5fc4323d5f3d7a89` / tree `8da6817eba1f259d3565e377fcb50098ab8f3cf2`; merge `8237b96...` | main Git history and PR #7 | no |
| Final repair archive | `52be94316664f88a19df630e164105a0ca50b875` / tree `21601814bd352114fe2e55833c88a81e47e13e41` / parent `3deb414...` | technical-lead GitHub receipt plus exact local object | no; read-only source |
| Live WR0 environment | transaction `wr0-20260901T053317Z-183afd5bbf0736e4`; identity `d3ed38d7ed53e9a5973cbf22fc504f2479dbdd15616fe1bfc1d5e1cd0bf5f4c4` | archive packet and read-only Deck inspection | no |
| Runner/runtime lock | `linux-vst-bridge-wr0-launch-critical/v1` / `2d64df1d36786ca2d0e955c553005423dc2b5bdd714bd0a17872622e33912547` | archive/current WR0 evidence | no |
| Final contract source | `linux-vst-bridge-wr0-contract-source/v1` / `c6543004fbcd70252393f0e0cab4f9ad7a31e85f433ce3e903e690244cc79541` | independently reconstructed ten-file manifest | exact Git blob adoption only |
| Workload | SHA-256 `4518ca37b8d7e005f01b441de5273447b70fa7c8c9c3d2b9c194a91782cfecac` | archive blob `04c8707...`; already identical on main | no |
| Retired replacement record | `linux-vst-bridge-wr0-replacement-commit/v1` / `917c939c605f457376b0bdd53e06d252d4aecee4b39728850b2e1a61aa102b3e` | archive packet and live readback | no |
| DG0 governance | current-main blobs for process docs/templates/prompts | `04d7712...` and closure `b517f96...` | no, except separately approved current-slice transition and narrow decision entry |

Immediately before implementation, reverify all Git objects/trees/parent,
current-main basis, clean worktree, approved design receipt, no open competing
successor PR, zero forbidden workloads, SteamOS read-only posture, zero WR0
transaction siblings, and exact read-only live fixture. Any mismatch blocks
before archive blob import.

## 4. Reconnaissance findings

| Question | Observed answer | Evidence | Design consequence | Still unknown |
|---|---|---|---|---|
| Why do repository and live state differ? | PR #7 merged `9228217...`; the final repair completed later at `52be943...` | Git graph, PR state/body/reviews/comments | reconcile; do not call random drift or restore older fixture | none material |
| Is archive source exact? | commit, tree, and parent exact in Deck object database | local `cat-file`/`rev-parse`; technical-lead receipt | direct SHA access is sufficient | none |
| Does live state match archive? | every retained substantive identity exact | production read-only inspector and archive packet | adoption may proceed without live execution | future time-of-implementation drift |
| What differs? | 14/25 WR0 paths; archive `CURRENT_SLICE.md` is the sole non-adoptable difference | raw path/mode/blob comparison | exact 13-blob allowlist | none |
| Does archive close the defect? | yes; rename, first fsync, verification, and physical recovery are one guarded production operation | helper/source audit | adopt exact source, do not reimplement | independent design reviewer may challenge proof |
| Are tests production-oriented? | yes; ten new cases call real backup/recovery helpers; retained 80/80 | source and fixture ledger | preserve exact source/evidence | no rerun required for adoption |
| How should evidence be represented? | canonical current packet is stale; archive packet is internally exact | packet hashes and identity comparison | exact canonical replacement plus separate reconciliation packet | final packet schema details fixed below |
| Does DG0 need redesign? | no | current governance read in full | preserve governance byte-for-byte | none |

### Implementability verdict

```text
IMPLEMENTABLE
```

Implementation remains forbidden until fresh independent v3 review, a
separate exact design-approval receipt, an implementation-authority
`CURRENT_SLICE.md`, and merge of those authority records into main.

## 5. Owner map

| Fact | Authoritative owner | Durable representation | Readers | Forbidden competing authority |
|---|---|---|---|---|
| Merged WR0 historical truth | Git commit `9228217...` and merge `8237b96...` | immutable Git objects/history | maintainers, auditors | status prose rewriting the commit |
| Final repair source truth | archive commit `52be943...` | exact commit/tree/parent and path/mode/blob objects | adoption verifier | mutable branch name alone, hand-edited equivalent source |
| Current live environment truth | archive transaction marker plus retired replacement record | exact files under owned environment; bounded read-only digest | live inspector, later slices | current main prose, path existence alone |
| Current main truth | GitHub `main` exact commit/tree | immutable commit and branch ref | implementation preflight | stale local `origin/main` alone |
| Adoption delta | WR0A exact blob allowlist | reconciliation fixture and candidate Git tree | implementation/audit agents | broad directory restore or cherry-pick |
| Final WR0 source identity | ten-file contract-source manifest | schema + stable sorted modes/blobs + digest | environment marker, source verifier | implementation commit SHA alone |
| Final WR0 environment identity | live marker/snapshot algorithm | environment identity digest plus bounded facts | read-only verifier | transaction name or path alone |
| Historical evidence | Git history at merged head | old blobs reachable by SHA | auditors | copying stale packet under a current-looking name |
| Superseding evidence | exact archive WR0 packet | canonical `evidence/wr0-proton-bootstrap/` at adopted head | future agents | regenerated or normalized archive evidence |
| Reconciliation evidence | WR0A packet | allow-listed Markdown/JSON/hash manifest | technical lead/auditors | raw logs or private live data |
| DG0 governance | current-main governance blobs | Git path/mode/blob identities | all agents | archived WR0 `CURRENT_SLICE.md` |
| Active authority | merged `CURRENT_SLICE.md` plus selection/review/approval receipts | exact Git blobs in future design-authority merge | implementation agent | unmerged design branch, PR comments, or archive source |

Physical Git/live state outranks in-memory booleans, branch names, prose, and
cached observations. No owner may mutate the immutable archive or live fixture
during WR0A.

## 6. State machine

### Design-authority merge law

1. Commit v3.
2. A fresh reviewer records `DESIGN_CLEAR` against the exact unchanged v3 blob.
3. The operator supplies the approval sentence and `DESIGN_APPROVAL.md` binds
   the unchanged v3 identity and final clear-review-file identity.
4. `CURRENT_SLICE.md` is updated to name the v3/review/approval identities, the
   expected implementation branch, `authority_phase=implementation`, and
   `implementation_authorized=true`.
5. The design-authority branch is merged into main.
6. The implementation branch starts only from that exact merge commit/tree.
7. The implementation PR never edits the immutable design-authority records.
8. Implementation merge and status closure remain separate.

The selection basis remains `b517f96...`; it is not the implementation basis.
The implementation basis is the exact future `DESIGN_AUTHORITY_MERGED` commit
and tree, which is unknown until that merge is read back.

The design-authority finalization envelope is exactly these six paths:

```text
CURRENT_SLICE.md
docs/slices/WR0A/SLICE_SELECTION.md
docs/slices/WR0A/RECONNAISSANCE.md
docs/slices/WR0A/IMPLEMENTATION_DESIGN.md
docs/slices/WR0A/ADVERSARIAL_DESIGN_REVIEW.md
docs/slices/WR0A/DESIGN_APPROVAL.md
```

The selection receipt consumed by v3 is exact mode/blob
`100644/4f3131c93ef7b8178a623032146316226071a10b`, SHA-256
`6fd547646d1501831b6264ec9f0f729fa1feac7cf6f482278330a0a96644da2a`.
Its predecessor blob `fe7b3f84...` retains the original operator selection;
the current receipt only clarifies the later merge lifecycle. After the exact
v3 commit, final authority may append the review, create the sixth path, and
update `CURRENT_SLICE.md`; the selection receipt, reconnaissance, and reviewed
v3 card remain byte-identical. The six paths describe the cumulative authority
tree relative to the selection basis, not permission to rewrite all six during
finalization.

### Legal states

| State | Authoritative facts | Physical/external reality | Entry proof | Exit operation |
|---|---|---|---|---|
| `MAIN_ACCEPTS_OLDER_WR0` | main contains merged WR0 source/evidence | live environment already newer | `b517f96...` tree plus live mismatch-to-main/archive match | identify exact archive |
| `ARCHIVE_FINAL_REPAIR_IDENTIFIED` | exact archive commit/tree/parent known | no repository/live mutation | local object and technical-lead receipt | bounded archive audit |
| `LIVE_ENVIRONMENT_MATCHES_ARCHIVE` | live identity equals archive packet | environment untouched | read-only exact comparison | path/blob delta audit |
| `ARCHIVE_DELTA_AUDITED` | 25-path roster, 14 differences, 13 adoptable blobs fixed | design records only | `RECONNAISSANCE.md` | independent design review |
| `DESIGN_V3_PROPOSED` | exact immutable v3 design exists | implementation remains forbidden | committed v3 path/blob/SHA | fresh independent review |
| `DESIGN_V3_REVIEWED_CLEAR` | external review records `DESIGN_CLEAR` against unchanged v3 | source/evidence still unmodified | exact v3 and review-file identities | operator approval |
| `DESIGN_APPROVAL_RECORDED` | operator receipt binds v3 and clear review | implementation still not begun | exact approval blob | update implementation authority |
| `DESIGN_AUTHORITY_MERGED` | design/review/approval/current-slice implementation authority is on main | accepted implementation basis exists | merge commit/tree readback | create implementation branch |
| `IMPLEMENTATION_BRANCH_CREATED` | branch/worktree starts from exact design-authority merge | clean Git-only worktree | exact HEAD/tree/status | create tool commit |
| `RECONCILIATION_TOOL_COMMIT_CREATED` | only the two WR0A tool paths are added in a clean commit | committed tool is the production owner before it acts | commit/tree/parent/two-path delta and clean status | measure source and run committed tests |
| `WR0A_IMPLEMENTATION_SOURCE_MATCHED` | two-file WR0A manifest is measured at the tool commit | reconciler implementation frozen | production manifest plus negative tests | verify authority and hold archive provenance |
| `ARCHIVE_PROVENANCE_HELD` | committed tool verified authority and exact local provenance ref | archive object cannot be pruned during adoption/closure | ref/object/tree/parent and authority readback | verify adoption plan |
| `ADOPTION_PLAN_VERIFIED` | fixed 25/14/13/11/exclusion ledgers exact | no adopted WR0 source yet | committed production `plan` output | stage archive blobs |
| `ARCHIVE_BLOBS_STAGED` | exact 13 archive modes/blobs are the only staged changes | live environment unchanged | index/worktree readback | create adoption commit |
| `ADOPTION_COMMIT_CREATED` | exact 13 imports form a clean child of the tool commit | tool source remains unchanged and reachable | commit/tree/parent/thirteen-path delta | source/evidence/live verification |
| `WR0_SOURCE_MATCHED` | adoption commit reproduces ten-file digest `c654...` | no live mutation | production manifest | canonical evidence verification |
| `CANONICAL_EVIDENCE_MATCHED` | exact archive 14-file packet current/hash-valid | old packet remains in history | roster/blob/hash readback | read-only live inspection |
| `DIAGNOSTIC_LIVE_READBACK_MATCHED` | standalone bounded diagnostics agree | no retained evidence consumes this result | stdout-only result and exact session cleanup | render retained evidence independently |
| `RETAINED_LIVE_READBACK_MATCHED` | renderer-owned inspection agrees inside its exact transaction | live environment untouched | transaction marker, bounded snapshot, before/after equality | render packet in same transaction |
| `RECONCILIATION_EVIDENCE_STAGED` | eight sanitized WR0A files staged by the renderer transaction | tool and adoption commits remain reachable | packet validation and session publication receipt | evidence-only commit |
| `FINAL_EVIDENCE_COMMIT_CREATED` | third implementation commit contains evidence only and all source identities reproduce | final candidate head exists | final-head audit | pre-PR audit |
| `PRE_PR_AUDIT_CLEAR` | independent exact-head audit is clear | PR remains unmerged | exact audit identity | technical-lead merge |
| `WR0A_IMPLEMENTATION_MERGED` | technical lead merged exact reviewed head | current main contains final repair | merge commit/tree readback | separate closure |
| `STATUS_CLOSED` | no active slice; WR0A accepted fact retained | no product mutation | closure commit/tree | successor selection only |

### Legal transitions

```text
MAIN_ACCEPTS_OLDER_WR0
  -> ARCHIVE_FINAL_REPAIR_IDENTIFIED
  -> LIVE_ENVIRONMENT_MATCHES_ARCHIVE
  -> ARCHIVE_DELTA_AUDITED
  -> DESIGN_V3_PROPOSED
  -> DESIGN_V3_REVIEWED_CLEAR
  -> DESIGN_APPROVAL_RECORDED
  -> DESIGN_AUTHORITY_MERGED
  -> IMPLEMENTATION_BRANCH_CREATED
  -> RECONCILIATION_TOOL_COMMIT_CREATED
  -> WR0A_IMPLEMENTATION_SOURCE_MATCHED
  -> ARCHIVE_PROVENANCE_HELD
  -> ADOPTION_PLAN_VERIFIED
  -> ARCHIVE_BLOBS_STAGED
  -> ADOPTION_COMMIT_CREATED
  -> WR0_SOURCE_MATCHED
  -> CANONICAL_EVIDENCE_MATCHED
  -> DIAGNOSTIC_LIVE_READBACK_MATCHED
  -> RETAINED_LIVE_READBACK_MATCHED
  -> RECONCILIATION_EVIDENCE_STAGED
  -> FINAL_EVIDENCE_COMMIT_CREATED
  -> PRE_PR_AUDIT_CLEAR
  -> WR0A_IMPLEMENTATION_MERGED
  -> STATUS_CLOSED
```

| From | Operation | To | Durability/readback required | Failure classification |
|---|---|---|---|---|
| older-main | verify immutable archive | archive identified | commit/tree/parent exact | `WR0A_SOURCE_IDENTITY_BLOCKED` |
| archive identified | read-only live comparison | live matches archive | exact marker/snapshot/record and sibling/process guard | `WR0A_ARCHIVE_LIVE_MISMATCH` |
| live matched | raw path/mode/blob and semantic audit | delta audited | exact 25-path roster | `WR0A_ARCHIVE_DELTA_BLOCKED` |
| delta audited | commit v3, fresh review, operator approval | approval recorded | exact immutable v3/review/approval identities | `RETURN_TO_DESIGN_GATE` |
| approval recorded | merge design authority | authority merged | main merge commit/tree | `DESIGN_AUTHORITY_MERGE_BLOCKED` |
| authority merged | create branch from exact merge | implementation branch created | HEAD/tree/clean status | `IMPLEMENTATION_BASIS_MISMATCH` |
| implementation branch | add only two tool files and commit | tool commit created | exact parent/tree/two-path delta and clean status | `ADOPTION_TOOL_SOURCE_MISMATCH` |
| tool commit | measure source and run production negative tests | WR0A source matched | exact two-file manifest and test ledger | `ADOPTION_TOOL_SOURCE_MISMATCH` |
| WR0A source matched | committed tool verifies authority and creates/verifies exact provenance ref | archive provenance held | authority/ref/object/tree/parent exact | `ARCHIVE_PROVENANCE_BLOCKED` |
| archive provenance held | committed tool verifies fixed plan | plan verified | exact 25/14/13/11/exclusion manifests | `ADOPTION_PLAN_MISMATCH` |
| plan verified | import exact 13 blobs | archive blobs staged | exact index/worktree modes/blobs | `WR0A_BLOB_IMPORT_BLOCKED` |
| blobs staged | create archive-adoption child commit | adoption commit | Git commit/tree/parent/thirteen-path delta | `WR0A_BLOB_IMPORT_BLOCKED` |
| adoption commit | verify WR0 and unchanged WR0A manifests | WR0 source matched | both digests and stable records | `WR0A_CONTRACT_SOURCE_BLOCKED` |
| sources matched | verify exact canonical packet | canonical evidence matched | 14-file roster and 13 hash entries | `WR0A_EVIDENCE_ADOPTION_BLOCKED` |
| canonical evidence matched | diagnostic-only frozen fixture check | diagnostic live matched | stdout-only bounded result; session absent | `WR0A_LIVE_READBACK_BLOCKED` |
| diagnostic matched | renderer opens marker-bound session and performs its own retained inspection | retained live matched | exact live/archive/candidate join in renderer transaction | `WR0A_LIVE_READBACK_BLOCKED` |
| retained live matched | same transaction renders/validates/publishes eight evidence files | evidence staged | hashes/schema/redaction/publication/session cleanup | `WR0A_RECONCILIATION_EVIDENCE_BLOCKED` |
| evidence staged | create evidence-only third commit | final evidence commit | final identities/path set reproduce | `FINAL_HEAD_SOURCE_MISMATCH` |
| final commit | independent audit | audit clear | exact-head audit | `PRE_PR_AUDIT_REPAIR_REQUIRED` |
| audit clear | technical-lead merge | implementation merged | merge readback | `WR0A_IMPLEMENTATION_MERGE_BLOCKED` |
| implementation merged | separate two-path closure | closed | no-active card and append-only decision | `WR0A_STATUS_CLOSURE_BLOCKED` |

### Forbidden transitions

- Any implementation state before `DESIGN_AUTHORITY_MERGED`.
- Running any reconciliation mode from untracked or uncommitted tool source.
- Any state directly to implementation acceptance without exact audit and merge.
- Importing archive `CURRENT_SLICE.md` in any state.
- Recreating “equivalent” source rather than importing exact archive blobs.
- Updating the live environment to make it agree with repository state.
- Restoring the older merged environment identity.
- Changing an unchanged governed source path and recalculating a new digest.
- Editing archive evidence to mention adoption.
- Combining adoption and status closure in one authority transition.
- Selecting a successor before `STATUS_CLOSED`.

### Restart/retry reconciliation

Before the tool commit, no reconciliation mode may run. After that commit, a
marker-bound adoption transaction records the exact tool commit, worktree,
original clean index, thirteen old entries, expected archive entries, nonce,
and phase. On interruption, recovery reads the marker plus physical
worktree/index state; it restores only entries whose states are exact old or
expected archive identities. Unknown content is preserved and blocks cleanup.
The tool commit and adoption commit remain reachable parents of the evidence-
only commit and are the source/adoption restart points. At final head, every
manifest is reproduced rather than inferred from an in-memory phase. After
push, branch/PR readback owns the final head. After merge, main owns acceptance
and only a separate closure may change status. Unknown worktree/index/cache
paths fail closed and are never deleted or overwritten.

## 7. Fallible-operation and mutation ledger

| # | Mode/owner | Preconditions | Fallible mutation or observation | Exact readback/recovery | Next state or blocker |
|---:|---|---|---|---|---|
| 1 | independent reviewer | committed immutable v3 | append v3 review only | exact v3/review blobs | reviewed clear or return to gate |
| 2 | approval recorder | exact clear review | create approval receipt | binds unchanged v3 and review | approval recorded |
| 3 | authority recorder | exact approval | update only current-slice authority | exact implementation fields/path diff | merge candidate |
| 4 | technical lead | exact authority PR | merge design authority | main merge commit/tree | merged or `DESIGN_AUTHORITY_MERGE_BLOCKED` |
| 5 | Git setup | exact merged authority | create implementation branch/worktree | exact HEAD/tree/clean status | implementation branch created |
| 6 | implementation owner | clean branch | create exactly two WR0A tool paths | no mode/path extras | tool files ready |
| 7 | implementation owner | only two tool paths changed | create tool-only commit | exact parent/tree/two-path delta; clean status | tool commit created |
| 8 | source verifier | clean tool commit | build two-file WR0A manifest | schema/digest readback | WR0A source matched |
| 9 | committed `negative-tests` | clean tool commit | create synthetic Git/session fixtures | production helper coverage; exact cleanup | bootstrap ledger passed |
| 10 | committed `verify-authority` | exact tool commit | read source/design/DG0/archive objects | no mutation; bounded JSON | verified or basis/source blocker |
| 11 | committed provenance helper | verified exact archive object | compare-and-set exact local provenance ref | ref/object/tree/parent readback; refuse wrong existing ref | provenance held through closure |
| 12 | committed `plan` | verified authority and provenance | derive fixed adoption plan | no mutation; exact manifest | verified or `ADOPTION_PLAN_MISMATCH` |
| 13 | `stage-adoption` transaction | exact plan/clean index | create unique marker-bound adoption session | file and parent fsync; exact readback | staging transaction active |
| 14 | `stage-adoption` preflight | owned session | inspect all destinations/index/modes | no source mutation on mismatch | writes permitted or refuse |
| 15 | production blob writer | exact old entries | write each of thirteen archive blobs | per-path old/archive physical classification | all bytes written or exact recovery |
| 16 | production mode setter | exact written blobs | apply each exact archive mode | per-path mode/blob readback | all modes exact or recovery |
| 17 | production index updater | exact worktree set | update thirteen literal index entries | per-entry mode/blob readback | index updated or recovery |
| 18 | production final-stage verifier | expected worktree/index | read complete index/worktree/roster | exact staged set | archive blobs staged or recovery |
| 19 | production adoption recovery | any pre-commit staging failure/restart | restore exact thirteen old entries and original clean index | unknown objects preserved; session cleanup only after proof | pristine tool commit or recovery blocker |
| 20 | implementation owner | exact staged set | create thirteen-path adoption commit | exact parent/tree/delta and clean status | adoption commit created |
| 21 | `verify-candidate`/source verifier | clean adoption commit | verify import/WR0/WR0A/design/DG0 identities | all manifests exact | sources matched |
| 22 | packet verifier | clean adoption commit | verify canonical archive packet | roster/hash/blob equality | canonical evidence matched |
| 23 | `inspect-live` session owner | guards clear | create unique diagnostic session | marker/nonce/containment/fsync/readback | diagnostic session active |
| 24 | `inspect-live` dispatcher | fixed call graph | bounded live reads; stdout diagnostics only | before/after equality; no retained handoff | diagnostic matched or blocker |
| 25 | `inspect-live` session owner | diagnostic complete/failure | remove exact diagnostic leaf only | root/leaf/source-tree absence checks | clear or cleanup blocker |
| 26 | `render-evidence` session owner | candidate/guards exact | create unique retained-evidence session | marker/nonce/phase fsync/readback | render transaction active |
| 27 | renderer-owned dispatcher | exact render session | perform sole retained live inspection | before/after equality stored only in owned session | retained readback matched |
| 28 | renderer | retained snapshot exact | render and validate eight-file packet in session | exact roster/hashes/schema/redaction | packet ready or rollback |
| 29 | publisher helper | validated session packet; destination absent | publish exact eight-file directory and stage literals | partial-set physical reconciliation; unknown preserved | evidence staged or publication blocker |
| 30 | render session owner | publish success/failure reconciled | remove exact marker-bound session leaf | leaf absent; fixed cache root not broadly deleted | clear or cache blocker |
| 31 | committed `negative-tests` | adoption commit | rerun applicable production tests | adoption/recovery/import/call-graph cases exact | ledger passed |
| 32 | implementation owner | exact eight-file stage | create evidence-only third commit | tool/adoption commits remain parents; 23 paths cumulative | final evidence commit |
| 33 | final verifier/`negative-tests` | final head | reproduce identities and rerun applicable safe tests | no source mutation; sessions cleaned | final matched or `FINAL_HEAD_SOURCE_MISMATCH` |
| 34 | publisher | exact final head | push branch/open ordinary non-draft PR | remote head/base/state readback | audit pending |
| 35 | independent auditor | exact PR head | read-only pre-PR audit | exact-head verdict | clear or `PRE_PR_AUDIT_REPAIR_REQUIRED` |
| 36 | technical lead | exact clear head | merge implementation | merge commit/tree readback | implementation merged |
| 37 | closure owner | merged result | two-path status closure and provenance readback | no-active card + append-only decision; exact ref/object through closure | status closed |

### Commit points

```text
after tool-only commit:
    the committed reconciler, not untracked session code, is the production
    owner; its two-file source manifest is measured before any archive import.

after staged-adoption start but before adoption commit:
    only the thirteen fixed worktree/index entries and the exact marker-bound
    session may differ; recovery is Git-local and the live environment is never
    a rollback target.

after adoption commit but before evidence:
    the reachable tool commit owns reconciler source provenance and its clean
    thirteen-path child owns exact archive adoption provenance.

after evidence-only third commit but before merge:
    tool, import, contract, authority, and governance manifests must reproduce;
    the final PR head is external Git/PR readback and is not self-recorded
    inside its evidence.

after technical-lead merge:
    main contains the adopted final repair; rollback requires a separately
    authorized Git revert, never live-environment mutation.
```

No WR0A commit point transfers or destroys live environment authority.

## 8. Process, thread, callback, and topology model

| Role | Created by | Parent/owner | Required thread/process | Lifetime | Exit/cleanup owner |
|---|---|---|---|---|---|
| Git/design implementation agent | operator-approved task | WR0A authority | ordinary user process | bounded task | agent |
| Git commands | implementation agent | exact worktree | ordinary child processes | command deadline | agent |
| Read-only WR0 inspector | implementation agent | exact accepted source | ordinary Deck user process; no Proton/Wine spawn | bounded inspection | agent |
| GitHub branch/PR | implementation agent after approval | repository control plane | external durable object | until merge/closure | technical lead |
| Proton/Wine/Windows/Bitwig/Serum | nobody in WR0A | out of scope | must be absent | zero | not created |

### Actual expected topology

```text
operator-approved repository agent
  -> exact Git worktree at merged design-authority basis
      -> exact two-file reconciler commit and source measurement
      -> committed read-only Git object/blob verification
      -> marker-bound exact allow-listed index/worktree import
      -> exact thirteen-path adoption commit
      -> diagnostic-only bounded read-only environment inspector
      -> renderer-owned retained inspection and sanitized evidence transaction
      -> evidence-only third commit
      -> remote branch and PR
```

There is no callback, IPC, real-time, Wine, Proton, or DAW topology. Every Git
command has a bounded normal command deadline. Process matching before live
readback uses the accepted forbidden-family classifier; no termination is
performed. If a forbidden workload appears, inspection stops.

## 9. Identity and authorization ledger

| Operation | Required identity | Freshness/readback | Refusal cases |
|---|---|---|---|
| select implementation basis | exact future design-authority merge commit/tree + technical-lead merge readback | immediately before branch creation | selection basis, unmerged design head, stale ref, different tree, dirty worktree |
| read archive | commit + tree + parent | before every import/verification phase | missing object, wrong parent/tree, branch name alone |
| import a path | repository-relative path + exact old mode/blob + exact archive mode/blob + allowlist membership | index/worktree readback immediately after import | unexpected path, mode, blob, symlink, conflict, unmerged entry |
| accept contract source | schema + sorted ten path/mode/blob entries + digest | clean adoption commit and final head | commit SHA alone, dirty/staged governed path, new roster |
| accept workload | exact archive/current blob plus SHA-256 | candidate tree | basename/path alone |
| replace canonical evidence | exact 14-file roster + archive blobs + hash manifest | candidate tree and final head | regenerated bytes, missing/extra file, invalid hash |
| accept live environment | schema/status + transaction + environment digest + runner/source/workload + prefix identity + bounded roster/registries + receipts + retired record | fresh read-only inspection | path existence alone, classification-only difference, sibling/process contamination |
| preserve decision register | exact design-authority-basis blob | candidate and final implementation trees | any implementation-PR edit |
| preserve DG0 | exact design-authority-basis path/mode/blob roster | candidate/final tree | archive versions, broad doc rewrite |
| publish branch/PR | exact final commit/tree, approved branch/base, non-draft state as specified later | remote readback | unknown remote head, wrong base, merge action |

No friendly name, branch name alone, path suffix, timestamp, in-memory flag, or
“same contents by inspection” is sufficient identity.

## 10. Data, durability, recovery, and migration

| Data class | Owner | Location | Sensitive? | Durability | Backup/rollback | Migration law |
|---|---|---|---|---|---|---|
| Main/merged history | Git | repository object database/GitHub | no | immutable commits | Git history | never rewritten |
| Archive final source | immutable archive commit | Git object database/GitHub archive branch | no | immutable commit/tree | multiple exact object copies | exact blob import only |
| Archive provenance ref | WR0A implementation/closure | exact local `refs/wr0a/provenance/archive-final-repair-52be943` | no | compare-and-set ref through closure | refuse wrong existing ref; never move it | post-closure material verification uses retained ledgers |
| Live WR0 environment | archive transaction | `<HOME>/.local/share/linux-vst-bridge/environments/wr0-proton11` | yes | existing marker/receipts/registry/prefix | no WR0A mutation | none |
| Canonical final WR0 evidence | archive packet | `evidence/wr0-proton-bootstrap/` | sanitized | Git blobs + hash manifest | old packet in history | exact superseding blob set |
| Reconciliation evidence | WR0A | `evidence/wr0a-final-repair-reconciliation/` | sanitized | Git commit + hash manifest | Git history | schema v1; no raw live state |
| DG0 governance | design-authority basis | repository docs/prompts/templates | no | Git blobs | Git history | byte-identical through implementation; separate post-merge closure owns status/decision |
| Candidate worktree/index | implementation agent | isolated Git worktree | no | transient until commit | restore exact candidate changes | no adoption from unknown state |
| Adoption/evidence sessions | committed reconciler | unique marker-bound leaves under fixed WR0A cache root | sanitized | marker/file/parent fsync plus physical phase readback | exact owned rollback/restart; unknown preserved | absent after completed mode |

Atomicity is Git-object and marker based: the tool-only commit establishes the
owner; exact archive blobs are staged under an owned transaction, committed,
and read back in an adoption tree; the evidence packet is rendered and
published under a separate owned transaction and committed as the third step.
The archive commit and exact provenance ref are source records through closure.
No environment migration or predecessor retirement occurs. A retry rederives
state from Git trees, exact session markers, worktree/index state, and the live
marker rather than cached booleans. Unknown objects or dirty paths are
preserved and refused.

## 11. Security, privacy, licensing, and proprietary material

### Threats

| Threat | Boundary | Prevention | Negative proof |
|---|---|---|---|
| credential leakage during source access | remote Git | exact local objects/technical-lead receipt; no token/origin/key forwarding | remote config unchanged; no credential in evidence |
| private environment disclosure | live readback | accepted allow-list inspector; placeholders; no raw registry/prefix | redaction and forbidden-literal scans |
| malicious/unknown archive path | Git import | exact 13-path list and mode/blob roster | reject any extra path, symlink, mode change, or archive CURRENT_SLICE |
| historical erasure | canonical evidence replacement | immutable old commit named in reconciliation evidence | old blob reachability and commit verification |
| governance regression | archive import | DG0 blob protection and narrow diff | exact protected-path comparison |
| proprietary/runtime redistribution | evidence/adoption | source contains metadata only; no runner/prefix/vendor binary import | binary/size/path scans and blob allowlist |
| false compatibility claim | status/evidence | exact claim ceiling and nonclaims | wording scan and independent audit |

### Sensitive/proprietary exclusions

Never retain private home paths, hostname/IP, account identifiers, credentials,
tokens, raw registry or prefix content, machine GUID/SID, raw command lines or
process environments, PIDs, Steam account data, Serum state/binaries, runner or
Runtime binaries, proprietary installers, license material, or raw session
directories.

### Network and authorization

Implementation may use the configured authenticated repository channel only to
push its exact approved branch and open/update a PR. It must not fetch alternate
source, change `origin`, access vendor services, authorize software, or perform
runtime network activity. Read-only source authority remains the exact local
objects plus the technical-lead receipt.

### Third-party licensing

No new dependency or third-party payload is added. Git transfers exact project
blobs already present in the immutable archive. Proton, Wine, Steam Runtime,
VST3 SDK, vendor software, and proprietary content are neither run nor
redistributed. Existing clean-room and license boundaries remain unchanged.

## 12. Approved code topology

| Component/file | Owner responsibility | Public interface | Must not own |
|---|---|---|---|
| `tools/wr0a-reconciliation/reconcile.py` | exact authority/delta verification, fixed 13-blob staging, candidate/live/evidence verification, rendering, negative tests | seven fixed modes | WR0 runtime, live mutation, approval, merge, closure |
| `tools/wr0a-reconciliation/README.md` | human contract for the fixed reconciler | exact mode/claim/failure descriptions | executable policy or alternate identities |
| `docs/WR0_RUNNER_LOCK.md` | exact final source/runner contract prose | archive blob `94471e...` | active slice authority |
| `tools/wr0-proton-bootstrap/launch.py` | accepted WR0 production behavior | archive blob `215718...` | WR0A adoption control |
| `tools/wr0-proton-bootstrap/README.md` | accepted tool contract documentation | archive blob `d90180...` | governance authority |
| canonical WR0 evidence packet | final WR0 empirical truth | 14-file archive packet/hash manifest | reconciliation narrative |
| WR0A reconciliation packet | source/delta/live/governance adoption proof | schema and hash manifest | new WR0 execution evidence |
| `docs/DECISION_REGISTER.md` | post-merge accepted reconciliation decision | separate closure only | candidate self-acceptance |
| `CURRENT_SLICE.md` | merged implementation authority, then separate closure | DG0 phase fields | imported archive authority or implementation-PR edits |

No new product component or reusable adoption framework is authorized. The two
tracked files above are the single rerunnable WR0A production owner; ephemeral
adoption, inspection, evidence, or negative-test scripts are not authority.

### Fixed reconciliation modes

```text
verify-authority
plan
stage-adoption
verify-candidate
inspect-live
render-evidence
negative-tests
```

`reconcile.py` uses only the Python standard library. Normal mode hardcodes
the accepted design-authority implementation basis supplied after merge; the
merged WR0 and archive commit/tree/parent; the complete 25-path envelope,
fourteen differences, thirteen imports, eleven identical paths, and excluded
archive `CURRENT_SLICE.md`; every old/archive mode/blob recorded in
`RECONNAISSANCE.md`; the exact archive 14-file evidence roster and 13-file hash
manifest; WR0 contract/workload identities; DG0 and merged design-authority
path/mode/blob sets; and exact live identities. Normal mode accepts no arbitrary
commit, ref, path, blob, environment, evidence destination, or call target.
Test mode is confined beneath one canonical no-symlink user-cache test root.

### Committed-tool bootstrap law

The two tool paths are the entire first implementation change. They are
committed in `RECONCILIATION_TOOL_COMMIT_CREATED` before any tool mode runs.
That commit must have the exact future design-authority merge as parent, an
exact two-path delta, and a clean worktree/index. The committed
`reconcile.py`, addressed from that commit's worktree, then measures
`linux-vst-bridge-wr0a-implementation-source/v1` and runs its production
negative tests. Only after those proofs pass may the same committed tool run
`verify-authority`, `plan`, or `stage-adoption`.

The archive adoption is a second commit whose parent is the exact tool commit
and whose delta is exactly the thirteen import paths. Reconciliation evidence
is a third commit whose parent is the exact adoption commit and whose delta is
exactly the eight WR0A evidence paths. The reconciler source manifest must be
identical at all three commits. No untracked copy, stdin script, generated
module, or implementation-session equivalent may perform production adoption.

The exact archive evidence manifest hardcoded by normal mode is:

```text
02b354f57f66d91a107136bbfaf1661a7c4f5f14bf97620c1a1bb64d3a422af8  BASIS.md
99882e178da936b372f2898ebe6d76951b197d94d835613dd0e94b9966cf48e9  ENVIRONMENT.md
0ba0fd3324a2b834ff9277cfe2b9a76d0ee59f9ab71b672cdd6d63d6a24eab2f  EXIT_PROPAGATION.md
5978db510a421637673f1f4bc8d5bda9d14620862fdd5ec0297ca1e1e1582325  FINDINGS.md
3457eda68560ae91ea79fae13dc0c5a5dd098bdc601089310b0c699f0d0fd5dc  LAUNCH_CONTRACT.md
343be77fa265cbf3752986f6b5546d9f68bff3e059f3237d30c4b66f129a4497  NEGATIVE_TESTS.md
609be19026560b5a7cff1f481a902806365f0a0039ee930478f7849a49eabaf5  PRESERVATION.md
938580080015db06083db202e6122a53f14a72e21cd5328f244c14bbb5c5f45f  PROCESS_GUARDS.md
60f297ae9e8a8776cbda515b3bc4088eae02cb7ccec66fb4ba82d32697578ca1  RUNNER_LOCK.md
db68c8577e2c9263ea46fb81c6fd0fd0f7b8a91c26a3aeb2f519da03c0bbff77  RUN_1.md
b1807d204c1155778c3436d083498784c2ec9b00a03a459c8c3a699e9ae9c2a3  RUN_2.md
be718e1c493431a53f92d483bba2a73916794e20bbb729eafccb2c0754258f08  SANITIZATION.md
338ad6b2f4a812dac53350e5e3ee75459f9f8c3636dce6c126088b539ab85b74  fixture.json
```

Those thirteen files plus exact archive `hashes.sha256` blob
`5369c4cb3e55314fd2c2589c9394518b40f5023f` are the complete fourteen-file
canonical roster. The complete 25-path old/archive ledger is the immutable
table in `RECONNAISSANCE.md` section 5 and is copied verbatim into tool
constants; a reference to that table is not runtime authority.

### Exact Git adoption algorithm

`stage-adoption`:

1. executes only from the exact clean tool commit whose parent is the accepted
   design-authority implementation basis;
2. verifies the measured two-file WR0A implementation-source identity;
3. requires a clean canonical worktree and stage-zero index;
4. creates and fsyncs a unique marker-bound adoption session containing the
   exact tool commit, nonce, canonical worktree identity, original clean index
   tree, thirteen old entries, thirteen expected entries, and current phase;
5. verifies every current-main old mode/blob;
6. verifies every archive mode/blob;
7. verifies the complete thirteen-path import allowlist;
8. verifies the complete eleven-path already-identical set;
9. verifies the exact excluded archive `CURRENT_SLICE.md` mode/blob;
10. reads each archive blob by exact object identity;
11. writes only the thirteen exact destination files through one production
    helper with a per-path old/archive physical-state readback;
12. applies and verifies each exact mode through the production mode helper;
13. stages only those literal thirteen paths using explicit `--` separators
    through the production index helper;
14. recomputes every staged Git blob and mode;
15. requires exact staged/archive equality and no additional staged,
    modified, or untracked path;
16. records `archive_blobs_staged` in the fsynced session and returns a bounded
    receipt; and
17. refuses unknown destination, index, marker, or cache state rather than
    overwriting it.

Broad archive checkout, cherry-pick, directory replacement, archived
`CURRENT_SLICE.md`, hand-edited equivalent source, reset, and clean are
forbidden. A partial failure changes only the implementation worktree/index.
Recovery is owned by the same production helpers, not a parallel test state
machine. It classifies every governed worktree file and index entry as exact
old, exact archive, absent, or unknown. Only old/archive states are eligible
for recovery. It atomically writes all thirteen exact old blobs, restores all
old modes, restores the thirteen exact original index entries, and proves the
entire worktree/index equals the clean tool commit. Unknown objects are left
untouched and produce `WR0A_BLOB_IMPORT_RECOVERY_BLOCKED`; the live environment
is never a rollback target.

The adoption session remains through the external exact thirteen-path commit.
`verify-candidate` accepts either (a) a staged transaction at the tool commit or
(b) the exact clean child adoption commit named by the session. In case (b) it
verifies parent/tree/delta/manifests, marks `adoption_committed`, then removes
only the exact session leaf. A restart uses the same marker and physical state;
it never treats a familiar cache path or an in-memory flag as ownership.

Test-only failure injection is available only beneath the canonical WR0A test
root and only through fixed enumerated seam names. Production-helper tests
inject after each Nth blob write, each Nth mode update, each Nth index update,
and the final index readback. Every injected case must restore all thirteen old
blobs/modes and the original clean index, preserve unrelated unknown paths,
remove no unknown object, and leave no partial import. Ordinary modes expose no
arbitrary callback, seam, path, commit, or failure selector.

### WR0A implementation-source identity

```text
schema: linux-vst-bridge-wr0a-implementation-source/v1
path_set:
  - tools/wr0a-reconciliation/README.md
  - tools/wr0a-reconciliation/reconcile.py
representation: stable sorted repository-relative path, tracked mode, Git blob
```

Five identities stay distinct: (1) the 13-import/11-identical/excluded-card
WR0 archive-import manifest; (2) the existing WR0 contract source
`c6543004...`; (3) the new two-file WR0A implementation source; (4) the exact
merged design authority; and (5) exact DG0 governance. Measure WR0A source at a
clean tool-only implementation commit before any archive mode acts. Its
thirteen-path adoption child and eight-path evidence-only grandchild must
reproduce the identical two-file representation and digest. The adoption
commit adds the WR0 import/contract identities; the final commit adds only the
reconciliation packet. A changed WR0A tool invalidates its manifest at either
later commit, while exact archive adoption and evidence-only additions preserve
it. Production negative tests run at the tool commit, again after adoption for
import/session boundaries, and at final head for all safe source-stability and
packet checks.

### DG0 protected basis roster

| Path | Exact mode/blob |
|---|---|
| `AGENTS.md` | `100644` / `4ca16746d4c53341bb5aa330448283bd3e15f791` |
| `GOVERNANCE.md` | `100644` / `1918ce7582c436d2f9c13d07acd32cedeb39c502` |
| `README.md` | `100644` / `0eabc8b65822444646db7a5ea6323e4f29279df7` |
| `docs/DEVELOPMENT_PROCESS.md` | `100644` / `162d9b106a22119946312becae8ec0d4b87cea68` |
| `docs/DECISION_REGISTER.md` | `100644` / `a3645c00586b8bd1546e7caca4dd0f34280bb251` |
| `docs/prompts/ADVERSARIAL_DESIGN_REVIEW.md` | `100644` / `a78bd60967594f8c5ebdf6fb0d463e9cae50b433` |
| `docs/prompts/CHOOSE_NEXT_SLICE.md` | `100644` / `cf0a180c2913a839f541baf7335c2ff0a1973a64` |
| `docs/prompts/PRE_PR_IMPLEMENTATION_AUDIT.md` | `100644` / `8aecb1898c126e6600a0c6a551d9e90a50afc887` |
| `docs/templates/DESIGN_APPROVAL_RECEIPT.md` | `100644` / `69bda7d4b32cfbc837b9e7f0df0843bb40e60034` |
| `docs/templates/IMPLEMENTATION_DESIGN_CARD.md` | `100644` / `7bc5d812552d55519c2bdb9954c51dcde035403f` |
| `docs/templates/SLICE_SELECTION_RECEIPT.md` | `100644` / `e10b4abf338e3d929752e4f7f2bc693b89e0fec8` |

The implementation prompt supplied after the authority merge adds exact merged
selection/recon/v3/review/approval/current-slice mode/blob identities. It does
not alter the DG0 roster.

### Frozen read-only live-inspection call graph

`inspect-live` first verifies candidate
`tools/wr0-proton-bootstrap/launch.py` is exactly
`100755/215718bb641765da9163779c0e2145bd02d3198a`. Before any module loader is
constructed, the dispatcher sets `sys.dont_write_bytecode = True`; it also
requires the process-level no-bytecode posture, uses fixed non-`__main__`
module name `_wr0a_readonly_wr0_launch_215718bb`, and imports only that exact
local file through a fixed `importlib.util` loader. Its allowlist is:

```text
process_guard()
verify_runner_lock()
contract_source_manifest(require_clean=True, repository=<exact candidate root>)
environment_snapshot(<fixed live path>, require_ready=True,
                     contract_source=<verified manifest>)
environment_receipt_snapshot(<fixed live path>, <exact archive fixture>)
read_replacement_commit_record(<fixed live path>)
capture_fixture()
```

WR0A-owned read-only helpers perform canonical containment, exact
transaction-sibling enumeration, archive JSON loading, and before/after live
identity capture. The dispatcher must not invoke `main`, `live`, `run_live`,
`run_preflight`, `ensure_cache_layout`, evidence/sanitizer publication,
environment creation, `atomic_write*`, `fsync_directory`, rename, deletion,
cleanup, replacement, workload launch, or process termination.

Before and after, compare exact content hashes plus declared stable sizes,
modes, mtimes, ctimes, inodes, prefix device/inode, marker, host/internal
receipts, selected registries, bounded roster, retired record, siblings, and
process guards. Access time is non-authoritative where reads may update it. Any
content/declared-metadata change blocks.

The dispatcher snapshots the candidate source tree and Git status before the
load and after module removal from `sys.modules`. It requires no
`__pycache__`, `.pyc`, or other new/changed source-tree object and no index or
worktree change. A production dispatcher test loads the exact fixture through
the real loader, proves the fixed module name and no-bytecode posture, and
rejects a loader/call target that could write bytecode or invoke a non-
allowlisted function before invocation.

### Diagnostic versus retained live readback

`inspect-live` is diagnostic only. It creates one owned session, invokes the
frozen dispatcher, emits bounded sanitized JSON to stdout, and removes that
session. It never writes repository evidence and its output is never an input
to `render-evidence`.

`render-evidence` performs the sole retained live inspection itself. In one
transaction it opens a new session, runs the same dispatcher, keeps the
sanitized snapshot only within that session, renders and validates all eight
WR0A evidence files from that exact snapshot plus Git facts, publishes those
files, stages only their literal paths, and removes the session. It neither
consumes diagnostic stdout nor silently calls the standalone mode.

### Marker-bound session and publication law

The permitted transient root is
`<HOME>/.cache/linux-vst-bridge/wr0a/`. The root is canonical, user-owned,
symlink-free, mode-bounded, and never recursively deleted merely because its
fixed path is familiar. Each operation exclusively creates one leaf named
`session-<32-lowercase-hex-nonce>` with mode `0700`. Its atomically written,
file-fsynced, parent-fsynced marker uses schema
`linux-vst-bridge-wr0a-session/v1` and binds:

```text
nonce
mode: inspect-live | render-evidence | stage-adoption
implementation_basis_commit/tree
tool_commit/tree/source_digest
adoption_commit/tree when available
canonical worktree identity
fixed target path or null
created_at
phase
```

No private path is retained in Git. The live path is fixed in source and is
never recorded raw in the packet. The session has explicit count, byte, and
deadline caps. Exact session physical states are:

```text
SESSION_ABSENT
SESSION_MARKER_COMMITTED
LIVE_READBACK_COMPLETE
PACKET_RENDERED_AND_VALIDATED
PUBLICATION_IN_PROGRESS
PACKET_PUBLISHED_AND_STAGED
SESSION_CLEANED
```

For `render-evidence`, the final evidence directory must be absent and the
eight paths absent from the index before publication; the marker binds that
original clean index state. Publication writes only the eight fixed files.
Before each write, the marker records `PUBLICATION_IN_PROGRESS`; after each
write and literal index update, the file's exact expected hash/mode/index entry
is read back. A failure or restart may remove a partial directory and unstage
those paths only when every present worktree/index entry is a fixed roster
member with the exact transaction-rendered bytes and the marker binds the same
tool, adoption, nonce, target, and phase. It then proves prior worktree and
index absence. An unknown
entry, wrong byte, wrong mode, missing/forged marker, symlink, alternate target,
or multiple session leaf is preserved and refused. Once all eight files and
their staged index entries verify, the marker records
`PACKET_PUBLISHED_AND_STAGED`; cleanup removes only the exact leaf after marker
and payload classification, then proves leaf absence. The fixed root remains.

On restart, the tool enumerates the root with a fixed count cap. Zero leaves is
clean. One exact owned leaf is reconciled from marker and physical state:
diagnostic sessions are discarded only after their bounded contents verify;
render sessions before complete publication roll back exact partial output;
an exact fully published packet is revalidated and retained. Unknown or
multiple leaves produce `TRANSIENT_CACHE_CLEANUP_BLOCKED` without deletion.
Failure before publication returns
`WR0A_RECONCILIATION_EVIDENCE_BLOCKED` after exact rollback. Cleanup failure
after exact publication preserves the staged packet but blocks completion as
`TRANSIENT_CACHE_CLEANUP_BLOCKED`. No live-environment path is writable in any
session state.

### Archive-object lifecycle and provenance

The archive object is an implementation-time source prerequisite, not a
permanent runtime dependency. After the tool commit and source measurement,
the committed authority/provenance helper creates this exact repository-local
provenance ref only if it is absent:

```text
refs/wr0a/provenance/archive-final-repair-52be943
  -> 52be94316664f88a19df630e164105a0ca50b875
```

Creation uses compare-and-set semantics. An existing ref at any other object is
preserved and refused. The object, tree, and parent are reverified immediately;
the ref is never moved or pushed by WR0A and must remain exact through
`STATUS_CLOSED`. The technical-lead-controlled GitHub branch
`archive/wr0-final-repair-52be943` remains the external provenance record
through closure. Neither ref substitutes for commit/tree/parent verification.

Mode lifecycle is exact:

- pre-adoption `verify-authority`, `plan`, and `stage-adoption` are one-shot
  modes and require the exact archive object plus provenance ref;
- `verify-candidate` at the adoption commit and the pre-PR final verifier still
  require that object through implementation acceptance and status closure;
- `inspect-live` and `render-evidence` read the adopted current blobs, fixed
  retained ledgers, and live fixture; they never fetch or mutate the archive;
- post-closure source/packet verification operates from current main's exact
  adopted blobs and the retained 25/13/11/exclusion/evidence manifests, while
  archive-object availability is reported separately as provenance
  availability rather than treated as product correctness.

After adoption, all required source and evidence bytes are present on main and
the reconciliation packet retains commit/tree/parent and complete mode/blob
ledgers. The archive branch is therefore not a build, runtime, or future
verification dependency. A clean main-only clone can verify adopted material
and retained ledgers, but it cannot truthfully re-read or prove reachability of
commit `52be943...` if neither archive ref nor object was fetched. V3 makes no
such claim. Removal of the local provenance ref after closure is outside WR0A
and requires separate authority; leaving it present is harmless provenance,
not a source dependency.

## 13. Changed-path and external-mutation envelope

### Allowed tracked paths for the future WR0A implementation PR: exactly 23

```text
docs/WR0_RUNNER_LOCK.md
tools/wr0-proton-bootstrap/README.md
tools/wr0-proton-bootstrap/launch.py
evidence/wr0-proton-bootstrap/BASIS.md
evidence/wr0-proton-bootstrap/ENVIRONMENT.md
evidence/wr0-proton-bootstrap/EXIT_PROPAGATION.md
evidence/wr0-proton-bootstrap/LAUNCH_CONTRACT.md
evidence/wr0-proton-bootstrap/NEGATIVE_TESTS.md
evidence/wr0-proton-bootstrap/PROCESS_GUARDS.md
evidence/wr0-proton-bootstrap/RUN_1.md
evidence/wr0-proton-bootstrap/RUN_2.md
evidence/wr0-proton-bootstrap/fixture.json
evidence/wr0-proton-bootstrap/hashes.sha256
tools/wr0a-reconciliation/README.md
tools/wr0a-reconciliation/reconcile.py
evidence/wr0a-final-repair-reconciliation/BASIS.md
evidence/wr0a-final-repair-reconciliation/SOURCE_DELTA.md
evidence/wr0a-final-repair-reconciliation/ARCHIVE_AUDIT.md
evidence/wr0a-final-repair-reconciliation/LIVE_READBACK.md
evidence/wr0a-final-repair-reconciliation/GOVERNANCE.md
evidence/wr0a-final-repair-reconciliation/FINDINGS.md
evidence/wr0a-final-repair-reconciliation/fixture.json
evidence/wr0a-final-repair-reconciliation/hashes.sha256
```

These are thirteen exact archive imports, two WR0A tool files, and eight WR0A
evidence files. The implementation agent may not edit `CURRENT_SLICE.md`,
`docs/DECISION_REGISTER.md`, the selection receipt, reconnaissance, design,
review, or approval. Those immutable inputs already exist in the merged
implementation basis.

The exact archive import subset is only these 13 existing paths:

```text
docs/WR0_RUNNER_LOCK.md
tools/wr0-proton-bootstrap/README.md
tools/wr0-proton-bootstrap/launch.py
evidence/wr0-proton-bootstrap/BASIS.md
evidence/wr0-proton-bootstrap/ENVIRONMENT.md
evidence/wr0-proton-bootstrap/EXIT_PROPAGATION.md
evidence/wr0-proton-bootstrap/LAUNCH_CONTRACT.md
evidence/wr0-proton-bootstrap/NEGATIVE_TESTS.md
evidence/wr0-proton-bootstrap/PROCESS_GUARDS.md
evidence/wr0-proton-bootstrap/RUN_1.md
evidence/wr0-proton-bootstrap/RUN_2.md
evidence/wr0-proton-bootstrap/fixture.json
evidence/wr0-proton-bootstrap/hashes.sha256
```

### Permitted external mutation after separate implementation approval

```text
Git worktree/index/commit objects for the exact implementation branch
exact local ref refs/wr0a/provenance/archive-final-repair-52be943
<HOME>/.cache/linux-vst-bridge/wr0a/session-<32-hex> (exact marker-bound leaves only)
configured repository branch push
ordinary non-draft implementation pull request metadata
```

The live WR0 environment and every external fixture are read-only.

### Protected state

- live transaction/environment/runner/source/workload/retired-record identities;
- no WR0 transaction siblings and zero forbidden workloads;
- immutable archive commit/tree/parent and all non-adopted archive blobs;
- the 11 already identical WR0 paths;
- `AGENTS.md`, `GOVERNANCE.md`, `README.md`, `docs/DEVELOPMENT_PROCESS.md`,
  `docs/ARCHITECTURE.md`, `docs/DESIGN_DOSSIER.md`, all DG0 prompts/templates;
- exact design-authority-basis `docs/DECISION_REGISTER.md` bytes throughout
  the implementation PR;
- HP0, HP1, SR0, Bitwig, Flatpak, `.wine`, Steam compatdata, Proton, Runtime,
  Serum, and unrelated repository/worktree state.

### Prohibited mutation

No archive `CURRENT_SLICE.md`, source outside the 13-blob subset, build file,
dependency, test framework, compatibility profile, runtime path, environment,
receipt, registry, prefix, DAW setting, Steam setting, runner, vendor asset, or
adjacent repository may change.

### Separate post-merge status closure

Only after technical-lead merge may a separate closure change:

```text
CURRENT_SLICE.md
docs/DECISION_REGISTER.md
```

That closure returns to no active slice and appends the accepted WR0A result,
naming the reviewed implementation head, merge commit/tree, final WR0 source,
archive/live identity, and claim ceiling. The implementation PR never calls
itself accepted.

## 14. Proof matrix

| # | Fact | Production owner/helper | Fixture | Positive proof | Negative/fault proof | Retained evidence | Claim ceiling |
|---:|---|---|---|---|---|---|---|
| 1 | selection basis commit/tree | `verify-authority` | real Git | exact object/tree | missing/wrong object refused | `BASIS.md`, JSON | selection only |
| 2 | archive commit/tree/parent | `verify-authority` | real Git | all three exact | any mismatch refused | `BASIS.md` | immutable source only |
| 3 | merged WR0 head/tree/merge | `verify-authority` | real Git | head/tree/merge exact | substituted history refused | `BASIS.md` | historical fact |
| 4 | exact 25-path WR0 envelope | `plan` | real Git | complete roster exact | missing/extra path refused | `SOURCE_DELTA.md`, JSON | envelope only |
| 5 | exact fourteen-path difference set | `plan` | real Git | exact diff | missing/fifteenth diff refused | `SOURCE_DELTA.md` | delta only |
| 6 | exact thirteen-path import set | `plan`, `stage-adoption` | real candidate | allowlist exact | missing/extra import refused | import manifest | adoption only |
| 7 | exact eleven-path identical set | `verify-candidate` | real candidate | modes/blobs unchanged | any change refused | import manifest | preservation only |
| 8 | archive current-slice exclusion | `plan`, `verify-candidate` | real + synthetic | exact excluded blob, authority preserved | staging/import refused | `SOURCE_DELTA.md` | no authority import |
| 9 | wrong archive path refusal | `plan` | synthetic Git | literal roster accepted | alternate path refused | test ledger | path authority |
| 10 | missing archive path refusal | `plan` | synthetic Git | complete roster accepted | omitted path refused | test ledger | completeness |
| 11 | extra archive path refusal | `plan` | synthetic Git | complete roster accepted | extra path refused | test ledger | completeness |
| 12 | wrong old blob refusal | `stage-adoption` | synthetic Git | exact old blob accepted | dirty/substituted blob preserved/refused | test ledger | overwrite safety |
| 13 | wrong archive blob refusal | `verify-authority` | synthetic Git | exact blob accepted | same path wrong blob refused | test ledger | content source |
| 14 | wrong file mode refusal | `stage-adoption` | synthetic Git | exact mode accepted | mode mismatch refused | test ledger | mode identity |
| 15 | unknown dirty destination refusal | `stage-adoption` | synthetic worktree | clean destination accepted | staged/unstaged/untracked unknown preserved/refused | test ledger | Git-local safety |
| 16 | exact archive blob staging/index | `stage-adoption` | real candidate | staged mode/blob equals archive | duplicate/unmerged/mismatch refused | `SOURCE_DELTA.md`, JSON | index proof |
| 17 | final WR0 contract digest | manifest verifier | real clean/final commits | exact `c654...` | governed change invalidates | `SOURCE_DELTA.md`, JSON | WR0 source |
| 18 | workload digest | `verify-candidate` | real blob | exact SHA-256 | changed workload refused | JSON | workload bytes |
| 19 | canonical WR0 evidence roster | packet verifier | real archive/candidate | fourteen files exact | missing/extra refused | `ARCHIVE_AUDIT.md` | roster only |
| 20 | canonical WR0 evidence hashes | packet verifier | real archive/candidate | thirteen entries exact | byte/hash mismatch refused | `ARCHIVE_AUDIT.md` | packet bytes |
| 21 | historical evidence reachability | `verify-authority` | real Git | merged blobs reachable | missing object blocks | `BASIS.md` | history retained |
| 22 | stale merged evidence/live inequality | live join | real read-only fixture | expected old/live difference exact | unknown/equality blocks chronology claim | `LIVE_READBACK.md` | chronology only |
| 23 | archive evidence/live equality | live join | real read-only fixture | all identities exact | any mismatch blocks | `LIVE_READBACK.md` | no workload rerun |
| 24 | guarded rename/first-fsync closure | archive source auditor | real archived source | same guarded operation verified | old control flow refused | `ARCHIVE_AUDIT.md` | source audit |
| 25 | ten final-repair helper tests | archive test auditor | archived tests | production helpers/seams called | name-only parallel model refused | `ARCHIVE_AUDIT.md` | retained test design |
| 26 | retained archive ledger 80/80 | packet verifier | archived fixture | exact counts/classes | wrong count/class refused | `ARCHIVE_AUDIT.md` | historical run |
| 27 | exact DG0 governance protection | authority manifest verifier | real candidate/final | modes/blobs exact | any changed protected path refused | `GOVERNANCE.md` | governance only |
| 28 | exact immutable design authority | authority manifest verifier | real candidate/final | six-path merged authority exact | selection/design/review/approval/current-slice mutation refused | `GOVERNANCE.md` | authority basis |
| 29 | exact read-only live call graph | dispatcher | instrumented synthetic + real | only seven fixed calls | any forbidden call fails before invoke | tests, `LIVE_READBACK.md` | read-only interface |
| 30 | no live environment content mutation | before/after helper | real read-only fixture | hashes/declared metadata exact | injected mutation detected | `LIVE_READBACK.md`, JSON | bounded selected identity |
| 31 | no forbidden workload | accepted `process_guard` | real Deck + synthetic | all forbidden counts zero | contaminant refused | `LIVE_READBACK.md` | observation window |
| 32 | no transaction sibling | WR0A sibling guard | real Deck + synthetic | roster empty | stage/previous/retiring/journal refused | `LIVE_READBACK.md` | immediate siblings |
| 33 | WR0A source at tool commit | implementation-source manifest | real tool-only commit | two-file digest recorded before any mode acts | missing/extra/dirty/uncommitted tool refused | `SOURCE_DELTA.md`, JSON | reconciler source |
| 34 | WR0A source at final head | final verifier | real final commit | same manifest | changed tool blocks | `FINDINGS.md`, JSON | source stability |
| 35 | adoption/evidence addition stability | final verifier | synthetic Git + real three-commit chain | all five identities stable at applicable states | governed edit refused | test ledger | source transitions |
| 36 | changed reconciliation source invalidates | implementation-source manifest | synthetic Git | unchanged verifies | README/Python change changes/refuses digest | test ledger | identity sensitivity |
| 37 | reconciliation evidence JSON/hash/roster | evidence verifier | real final candidate | eight files, JSON, seven payload hashes | bad/missing/extra refused | packet + hashes | sanitized evidence |
| 38 | no private/proprietary content | sanitizer | real + synthetic | UTF-8/NUL/size/redaction/binary scans pass | forbidden fixture refused | `FINDINGS.md` | allow-listed metadata |
| 39 | exact implementation path envelope | final verifier | real candidate | 23 paths exact | twenty-fourth/wrong path refused | `FINDINGS.md`, JSON | implementation PR |
| 40 | ordinary non-draft PR state | publisher/readback | GitHub | exact base/head/open/non-draft | wrong base/head/draft/merged blocks | external PR readback | publication only |
| 41 | implementation agent does not merge | publisher policy | GitHub | PR remains open/unmerged | merge action forbidden | external PR readback | no acceptance claim |
| 42 | Decision Register unchanged before acceptance | governance verifier | real candidate/final | exact basis blob | any implementation edit refused | `GOVERNANCE.md` | pre-acceptance only |
| 43 | separate post-merge status closure | closure verifier | post-merge Git | two-path closure after merge | pre-merge/candidate closure refused | later closure record | acceptance after merge |
| 44 | current selection-receipt identity and lineage | authority manifest verifier | real design authority | current `100644/4f3131...` and SHA exact; original operator blob named | stale `fe7b3...` as current authority refused | `BASIS.md`, `GOVERNANCE.md` | selection authority only |
| 45 | exact six-path authority-finalization envelope | authority manifest verifier | real design merge | only six fixed paths present/changed as authorized | seventh path or missing approval/review path refused | `GOVERNANCE.md`, JSON | design authority only |
| 46 | committed reconciler before action | Git/tool bootstrap verifier | real tool commit | exact two-path commit precedes every mode receipt | untracked/uncommitted invocation refused | `BASIS.md`, `SOURCE_DELTA.md` | production owner |
| 47 | tool-commit topology | Git topology verifier | real three-commit implementation chain | authority merge -> two-tool commit -> thirteen-import commit -> eight-evidence commit | combined/reordered/extra commit delta refused | `BASIS.md`, JSON | provenance topology |
| 48 | WR0A source at adoption commit | implementation-source manifest | real adoption child | exact tool-commit manifest reproduced | changed tool in adoption commit refused | `SOURCE_DELTA.md`, tests | source stability |
| 49 | production tests at lifecycle commits | committed `negative-tests` | real tool/adoption/final commits | applicable fixed ledger passes at each state | running uncommitted or wrong-tree tests refused | `ARCHIVE_AUDIT.md`, `FINDINGS.md` | tested source identity |
| 50 | diagnostic readback has no evidence handoff | `inspect-live` dispatcher | synthetic + real read-only fixture | stdout-only result and exact session cleanup | renderer input from diagnostic output refused | tests, bounded diagnostic receipt | diagnostic only |
| 51 | renderer owns sole retained live readback | `render-evidence` transaction | synthetic + real read-only fixture | live snapshot, packet, and publication share one nonce/marker | missing/reused/foreign snapshot refused | `LIVE_READBACK.md`, JSON | retained observation only |
| 52 | unique marker-bound session ownership | session helper | synthetic cache | exclusive leaf/marker/fsync/readback exact | forged, missing, duplicate, symlinked, or wrong-phase session preserved/refused | test ledger | transient ownership |
| 53 | partial evidence publication rollback | publication helper | synthetic worktree/cache | exact partial fixed-roster bytes restore prior absence | unknown/wrong entry preserved and blocks | test ledger | Git-local rollback |
| 54 | session restart and cleanup | session reconciler | synthetic interrupted phases | exact owned diagnostic/render/adoption states reconcile and leaf disappears | multiple/unknown leaf produces cleanup blocker | test ledger | transient cleanup |
| 55 | no bytecode/source-tree side effect | production module loader | synthetic + exact archive `launch.py` | fixed non-main load with no `__pycache__`/`.pyc`/Git change | cache-writing or alternate loader posture refused | tests, `LIVE_READBACK.md` | loader non-mutation |
| 56 | failure after Nth archive-blob write | production adoption helper | synthetic Git for N=1..13 | all old blobs/modes and clean index restored | residual partial import fails test | test ledger | write-boundary recovery |
| 57 | failure after Nth mode update | production adoption helper | synthetic Git for N=1..13 | all old blobs/modes and clean index restored | wrong/residual mode fails test | test ledger | mode-boundary recovery |
| 58 | failure after Nth index update | production adoption helper | synthetic Git for N=1..13 | original thirteen index entries and worktree restored | residual staged entry fails test | test ledger | index-boundary recovery |
| 59 | final index-readback failure | production final-stage verifier/recovery | synthetic Git | exact original worktree/index restored | stale staged set fails test | test ledger | readback-boundary recovery |
| 60 | unknown-path preservation during recovery | production adoption recovery | synthetic Git | known thirteen entries restored while unrelated unknown remains byte-identical | deleting/overwriting unknown fails test | test ledger | non-destructive recovery |
| 61 | archive provenance through closure | provenance-ref verifier | real Git + external receipt | exact local ref/object/tree/parent and external branch named through closure | wrong/moved/missing pre-closure ref blocks | `BASIS.md`, closure readback | provenance availability |
| 62 | post-adoption archive independence | post-adoption verifier | synthetic main-only clone + real final tree | adopted blobs/retained ledgers verify without archive object | claiming archive-object reachability without object refused | `SOURCE_DELTA.md`, `FINDINGS.md` | material independence, not object reachability |

## 15. Failure and blocked-result taxonomy

| Result code | Exact owner/stage | Preserved state | Cleanup | Next lawful action |
|---|---|---|---|---|
| `WR0A_SOURCE_IDENTITY_BLOCKED` | main/archive object verification | Git/live state unchanged | remove only owned temp transfer/worktree state | technical lead supplies exact source authority |
| `WR0A_DESIGN_REPAIR_PREFLIGHT_BLOCKED` | exact v1 design-repair preflight | all fixtures/known work unchanged | none | resolve authority externally |
| `WR0A_DESIGN_V3_PREFLIGHT_BLOCKED` | exact v2-to-v3 design-repair preflight | all fixtures/known work unchanged | none | resolve authority externally |
| `WR0A_ARCHIVE_LIVE_MISMATCH` | read-only live comparison | live state untouched | none | return to design gate; do not repair |
| `RETURN_TO_DESIGN_GATE` | defect/owner/state/proof finding | no implementation | retain bounded recon facts | revise and independently rereview design |
| `DESIGN_AUTHORITY_MERGE_BLOCKED` | design/review/approval/current-slice merge | implementation not started | none | repair authority transition |
| `IMPLEMENTATION_BASIS_MISMATCH` | implementation branch preflight | live and unknown worktree state preserved | remove only empty owned worktree/ref if exact | recreate from exact merge basis |
| `ARCHIVE_PROVENANCE_BLOCKED` | exact local archive-ref/object lifecycle | repository worktree/live state preserved | do not move or replace unknown ref | restore exact provenance authority before adoption |
| `RECONCILIATION_TOOL_BOOTSTRAP_BLOCKED` | tool-only commit/source preflight | archive/live/current source unchanged | remove only exact uncommitted two-path attempt if safe | recreate committed owner from exact authority basis |
| `ADOPTION_TOOL_SOURCE_MISMATCH` | WR0A two-file source manifest | candidate/live preserved | no live cleanup | repair within envelope or return to gate |
| `ADOPTION_PLAN_MISMATCH` | fixed 25/14/13/11/exclusion plan | main/archive/live unchanged | none | resolve exact ledger; do not import |
| `WR0A_ARCHIVE_DELTA_BLOCKED` | path/mode/blob audit | main/archive/live unchanged | none | resolve exact source delta |
| `WR0A_BLOB_IMPORT_BLOCKED` | candidate index/worktree import | live and archive unchanged | restore only exact candidate Git changes if safe | retry from clean merged design-authority basis |
| `WR0A_BLOB_IMPORT_RECOVERY_BLOCKED` | physical adoption rollback/restart | live/archive/unknown objects preserved | retain marker and refuse broad cleanup | inspect exact thirteen-path/index physical state |
| `WR0A_CONTRACT_SOURCE_BLOCKED` | manifest mismatch | candidate commit retained; live unchanged | no live cleanup | inspect exact blob mismatch; no reimplementation |
| `WR0A_EVIDENCE_ADOPTION_BLOCKED` | canonical archive packet | old evidence/history and live preserved | restore only candidate evidence changes | correct exact blob import |
| `READ_ONLY_CALL_GRAPH_VIOLATION` | inspect-live dispatcher | live untouched | exact transient cleanup only | return to design gate |
| `TRANSIENT_CACHE_CLEANUP_BLOCKED` | WR0A cache cleanup | live/candidate preserved | preserve unknown object; no broad deletion | inspect exact owned cache state |
| `WR0A_LIVE_READBACK_BLOCKED` | fresh fixture join | candidate commit and live state preserved | none | return to design gate if material |
| `WR0A_GOVERNANCE_DRIFT_BLOCKED` | DG0/current authority comparison | candidate/live/archive preserved | restore candidate governance changes only if exact | technical-lead review |
| `WR0A_RECONCILIATION_EVIDENCE_BLOCKED` | packet rendering/sanitization | tool/adoption commits and live state preserved | reconcile only exact owned packet/session state | regenerate from retained Git/live facts |
| `FINAL_HEAD_SOURCE_MISMATCH` | final five-identity reproduction | final candidate/live preserved | none | repair evidence-only transition |
| `PRE_PR_AUDIT_REPAIR_REQUIRED` | independent exact-head audit | PR open/unmerged; live unchanged | none | repair within approved design or return to gate |
| `WR0A_IMPLEMENTATION_MERGE_BLOCKED` | technical-lead merge/readback | PR open/unmerged; live unchanged | none | resolve external merge state |
| `WR0A_STATUS_CLOSURE_BLOCKED` | separate post-merge closure | accepted implementation preserved | none | repair closure only |

## 16. Material-discovery stop conditions

Implementation stops and returns to the design gate if any observation changes:

```text
[x] owner map
[x] state or transition set
[x] mutation root
[x] durability/commit/recovery boundary
[x] process/thread/callback topology
[x] protocol direction or payload law
[x] identity/authorization law
[x] security/privacy/licensing posture
[x] exact fixture
[x] primary claim or claim ceiling
[x] changed-path envelope
[x] proof matrix
```

Slice-specific stops:

- either exact Git object/tree/parent differs or disappears;
- the current clarified selection receipt is not exact
  `100644/4f3131c9...` or its operator-selection lineage changes;
- the reconciler cannot be committed and source-measured before any mode acts;
- more/fewer/different WR0 paths or blobs are needed;
- archive source does not close the guarded rename/first-fsync boundary;
- the 80-case ledger does not exercise the production helpers as recorded;
- live environment no longer exactly matches archive evidence;
- archive evidence cannot remain byte-identical at canonical paths;
- a governed source blob must be edited after import;
- archive `CURRENT_SLICE.md` would be required;
- DG0 or immutable design authority would change in the implementation PR;
- module loading creates bytecode or another source-tree side effect;
- adoption recovery cannot restore every old blob/mode and original index
  without touching an unknown object;
- retained evidence cannot be rendered and published from one exact marker-
  bound transaction with truthful restart/cleanup;
- the exact archive provenance object/ref cannot remain auditable through
  status closure;
- any Proton/Wine/workload execution or live-environment mutation appears
  necessary;
- status closure or successor selection is proposed inside implementation.

The agent may retain bounded read-only evidence but may not patch forward.

## 17. Implementation sequence

```text
1. Freshly review the exact immutable v3 blob.
2. Record separate operator approval and implementation-phase current-slice
   authority without editing v3; finalize only the exact six authority paths.
3. Merge the design-authority branch into main.
4. Create the implementation branch from that exact merge commit/tree.
5. Add exactly the two WR0A tool files and create the clean tool-only commit.
6. Measure the WR0A implementation-source identity and run all bootstrap-safe
   production negative tests from that committed tool.
7. Run committed verify-authority, create/verify the exact local provenance
   ref, and run plan; then use the marker-bound adoption
   transaction to stage exactly thirteen archive blobs.
8. Verify the staged candidate and create the exact thirteen-path adoption
   child commit; finalize/clean the adoption session.
9. Reproduce the WR0 import, WR0 contract, WR0A implementation, design-
    authority, and DG0 identities; verify the canonical WR0 packet and rerun
    adoption/session negative tests.
10. Run standalone inspect-live only as bounded stdout diagnostics and prove
    its exact session cleanup and no bytecode/source side effect.
11. Run render-evidence, which performs its own sole retained live inspection,
    renders/validates/publishes/stages the eight-file packet in one marker-
    bound transaction, and removes only its exact session leaf.
12. Create the evidence-only third implementation commit so both clean parent
    commits remain reachable.
13. At final head, reproduce all identities, exact archive evidence, live
    readback, 23 paths, zero forbidden processes, zero siblings, zero source-
    tree/cache artifacts, exact provenance ref, and a clean worktree; rerun all
    applicable safe production tests.
14. Push the approved implementation branch and open an ordinary non-draft,
    unmerged PR targeting main.
15. Obtain an independent exact-head pre-PR audit. The implementation agent
    does not merge.
16. After technical-lead merge, perform the separate two-path status closure
    while the exact archive provenance ref/object remains readable.
```

The reachable tool commit is required because WR0A's production owner must be
committed and measured before it acts. Its adoption child and evidence
grandchild separate code authority, exact imported material, and retained
evidence. The final head must reproduce every manifest without executing the
workload.

## 18. Adversarial design review disposition

The review file records the supplied v1 and v2 `DESIGN_REPAIR_REQUIRED`
verdicts against their exact commit/tree/blob/SHA identities, including GitHub
review `5080009434` for v2. A fresh independent reviewer may append a v3
`DESIGN_CLEAR` or repair verdict to that external file. The review file, not
this card, owns later disposition. This card remains byte-identical and is not
implementation authority.

## 19. Approved design identity

```text
card_path: docs/slices/WR0A/IMPLEMENTATION_DESIGN.md
card_git_blob: to be measured at the committed design head and named by the separate approval receipt
card_sha256: to be measured at the committed design head and named by the separate approval receipt
design_revision: wr0a-design-v3
design_status: proposed_for_adversarial_review
approval_receipt_path: docs/slices/WR0A/DESIGN_APPROVAL.md
approval_receipt_identity: absent; required before implementation
implementation_authorized: false
```

The card cannot contain its stable self-hash. Fresh review and later approval
record its external Git blob and SHA-256. The approval receipt also binds the
final clear-review-file identity. Implementation authority is owned only by the
merged approval/current-slice records; the card never approves itself.

## 20. Non-circular evidence, archive independence, and v3 validation

The eight-file WR0A packet may retain the merged design-authority basis, the
reachable tool and adoption commit/trees, all source/import/authority
manifests, renderer-owned bounded live readback, exact session/publication
classification, and a seven-payload hash manifest excluding `hashes.sha256`.
It must not contain its own final Git commit as authoritative data. Final PR
head/tree belong to branch/PR readback, independent audit, and technical-lead
exact-head review. Merge commit/tree and accepted result belong to merge
readback and the separate status closure.

Current main must not depend on future archive-branch availability after
adoption. Through status closure, the exact local provenance ref/object and
external archive branch remain required audit inputs. The packet
retains archive commit/tree/parent, the complete 25-path ledger, thirteen
imported and eleven identical entries, excluded current-slice mode/blob, final
contract manifest, exact archive evidence manifest, and chronology. Exact
source/evidence bytes exist in main after adoption; the archive remains
provenance only. A main-only clone verifies current material and ledgers, not
the reachability of an unavailable non-ancestor archive commit.

V3 is valid only when:

- exactly five design/review paths differ cumulatively from selection basis;
- no product source, accepted evidence, or live fixture changed;
- the review record binds exact v1 and v2 identities, GitHub review
  `5080009434`, and v3 names v2 as superseded;
- v3 binds the exact current selection receipt blob/SHA and freezes the six-
  path authority-finalization envelope;
- authority merge precedes implementation branch creation;
- implementation basis is the future exact authority merge, not `b517f96...`;
- v3 requires no post-review mutation;
- the tool-only commit precedes every production mode, followed by separate
  thirteen-path adoption and eight-path evidence commits;
- diagnostic and retained live readback have distinct exact owners;
- module loading is bytecode-cache-free and source-tree side-effect-free;
- marker-bound adoption/evidence sessions have physical recovery and restart
  laws, including Nth-boundary production-helper tests;
- archive-dependent and post-adoption mode lifecycles are explicit;
- the two-file owner, read-only call graph, WR0A source identity, separate
  implementation/closure envelopes, 62 proof rows, 37-row mutation ledger,
  and blocked taxonomy are exact;
- Decision Register acceptance occurs only after implementation merge; and
- `implementation_authorized=false` remains exact.

This sequence is design, not an implementation prompt. No implementation,
approval receipt, archive adoption, evidence replacement, workload execution,
or live mutation is authorized.
