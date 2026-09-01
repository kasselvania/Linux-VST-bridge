# Implementation Design Card — WR0A

## 1. Identity and authority

```text
slice_id: WR0A
title: Post-Merge Final WR0 Repair Reconciliation and Adoption
design_revision: wr0a-design-v1
design_status: review_required
repository: kasselvania/Linux-VST-bridge
basis_commit: b517f96bafa388107ea28c9d2e91528fd37b00d9
basis_tree: 839cda6145f592642f06b5273855b04ffa0562ac
selection_receipt_path: docs/slices/WR0A/SLICE_SELECTION.md
selection_receipt_identity: git-blob fe7b3f84f5f427cc48ce6b41752686812f678051; sha256 4cf4b4173aa99a0a62611f599359ab0f7416520b44484f4a582d2d9963810ed0
current_slice_path: CURRENT_SLICE.md
authority_phase: reconnaissance_and_design
implementation_authorized: false
prepared_by: repository reconnaissance and implementation-design agent
prepared_at: 2026-09-01T14:23:36Z
```

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
| Current main | `b517f96bafa388107ea28c9d2e91528fd37b00d9` / tree `839cda6145f592642f06b5273855b04ffa0562ac` | technical-lead receipt plus exact local object | Git implementation branch only after approval |
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

Implementation remains forbidden until fresh independent design review and a
separate exact design-approval receipt.

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
| Active authority | current `CURRENT_SLICE.md` plus selection/review/approval receipts | exact Git blobs on implementation branch | implementation agent | PR comments or archive source |

Physical Git/live state outranks in-memory booleans, branch names, prose, and
cached observations. No owner may mutate the immutable archive or live fixture
during WR0A.

## 6. State machine

### Legal states

| State | Authoritative facts | Physical/external reality | Entry proof | Exit operation |
|---|---|---|---|---|
| `MAIN_ACCEPTS_OLDER_WR0` | main contains merged WR0 source/evidence | live environment already newer | `b517f96...` tree plus live mismatch-to-main/archive match | identify exact archive |
| `ARCHIVE_FINAL_REPAIR_IDENTIFIED` | exact archive commit/tree/parent known | no repository/live mutation | local object and technical-lead receipt | bounded archive audit |
| `LIVE_ENVIRONMENT_MATCHES_ARCHIVE` | live identity equals archive packet | environment untouched | read-only exact comparison | path/blob delta audit |
| `ARCHIVE_DELTA_AUDITED` | 25-path roster, 14 differences, 13 adoptable blobs fixed | design records only | `RECONNAISSANCE.md` | independent design review |
| `ADOPTION_DESIGN_APPROVED` | exact design blob/revision approved | source/evidence still unmodified | review `DESIGN_CLEAR` plus operator receipt | create implementation branch/worktree |
| `FINAL_BLOBS_IMPORTED` | exact 13 archive modes/blobs in candidate index/worktree | live environment unchanged | index/worktree blob comparison | commit candidate |
| `CONTRACT_SOURCE_MATCHED` | clean candidate reproduces ten-file digest | immutable candidate commit exists | production Git manifest readback | validate canonical evidence |
| `FINAL_EVIDENCE_RETAINED` | exact archive 14-file WR0 packet current and hash-valid | old packet remains in history | exact blob roster plus `hashes.sha256` | read-only live readback |
| `LIVE_READBACK_MATCHED` | candidate source/evidence/live identities agree | live environment still untouched | fresh bounded inspector result | render reconciliation packet |
| `RECONCILIATION_EVIDENCE_FINALIZED` | WR0A packet sanitized/hash-valid; final source digest stable | final candidate head exists | final-head audit | PR review/acceptance |
| `WR0A_ACCEPTED` | technical lead accepts and merges exact head | current main contains final repair | merge commit/tree readback | separate closure |
| `STATUS_CLOSED` | no active slice; WR0A accepted fact retained | no product mutation | closure commit/tree | successor selection only |

### Legal transitions

```text
MAIN_ACCEPTS_OLDER_WR0
  -> ARCHIVE_FINAL_REPAIR_IDENTIFIED
  -> LIVE_ENVIRONMENT_MATCHES_ARCHIVE
  -> ARCHIVE_DELTA_AUDITED
  -> ADOPTION_DESIGN_APPROVED
  -> FINAL_BLOBS_IMPORTED
  -> CONTRACT_SOURCE_MATCHED
  -> FINAL_EVIDENCE_RETAINED
  -> LIVE_READBACK_MATCHED
  -> RECONCILIATION_EVIDENCE_FINALIZED
  -> WR0A_ACCEPTED
  -> STATUS_CLOSED
```

| From | Operation | To | Durability/readback required | Failure classification |
|---|---|---|---|---|
| older-main | verify immutable archive | archive identified | commit/tree/parent exact | `WR0A_SOURCE_IDENTITY_BLOCKED` |
| archive identified | read-only live comparison | live matches archive | exact marker/snapshot/record and sibling/process guard | `WR0A_ARCHIVE_LIVE_MISMATCH` |
| live matched | raw path/mode/blob and semantic audit | delta audited | exact 25-path roster | `WR0A_ARCHIVE_DELTA_BLOCKED` |
| delta audited | independent review + operator approval | design approved | exact review/approval blob references | `RETURN_TO_DESIGN_GATE` |
| design approved | import exact 13 blobs | blobs imported | exact index/worktree modes/blobs; all other paths unchanged | `WR0A_BLOB_IMPORT_BLOCKED` |
| blobs imported | clean candidate commit and manifest check | source matched | Git commit/tree and digest `c654...` | `WR0A_CONTRACT_SOURCE_BLOCKED` |
| source matched | validate exact canonical packet | final evidence retained | 14-file roster and 13 hash entries | `WR0A_EVIDENCE_ADOPTION_BLOCKED` |
| evidence retained | fresh read-only fixture check | live readback matched | exact live/archive/candidate identity join | `WR0A_LIVE_READBACK_BLOCKED` |
| live matched | render/validate WR0A evidence; evidence-only amend | evidence finalized | final head/tree, packet hash, source digest unchanged | `WR0A_RECONCILIATION_EVIDENCE_BLOCKED` |
| evidence finalized | independent audit and merge | accepted | exact reviewed head and merge readback | `WR0A_IMPLEMENTATION_AUDIT_BLOCKED` |
| accepted | separate status closure | closed | main no-active-slice card | `WR0A_STATUS_CLOSURE_BLOCKED` |

### Forbidden transitions

- Any state directly to `WR0A_ACCEPTED` without exact review and proof.
- Importing archive `CURRENT_SLICE.md` in any state.
- Recreating “equivalent” source rather than importing exact archive blobs.
- Updating the live environment to make it agree with repository state.
- Restoring the older merged environment identity.
- Changing an unchanged governed source path and recalculating a new digest.
- Editing archive evidence to mention adoption.
- Combining adoption and status closure in one authority transition.
- Selecting a successor before `STATUS_CLOSED`.

### Restart/retry reconciliation

Before a candidate commit, discard only uncommitted exact WR0A Git changes
after verifying the worktree and restart from the approved basis; never touch
the live environment. After the clean candidate commit, its Git identity is the
restart point. After the evidence-only amend, verify the final head and source
digest rather than relying on an in-memory phase. After push, the branch ref
and PR head are authority. After merge, main is authority and only a separate
closure may change status. Unknown worktree/index paths fail closed and are not
deleted or overwritten.

## 7. Fallible-operation and mutation ledger

| # | Precondition | Fallible operation | Immediate physical result | Durability barrier | Exact readback | Next authority state | Failure owner | Recovery action |
|---:|---|---|---|---|---|---|---|---|
| 1 | approved basis object available | create isolated implementation branch/worktree | Git metadata/worktree only | branch ref creation | branch HEAD/tree and clean status | design approved | Git setup | remove only owned empty worktree/ref or stop |
| 2 | clean approved worktree | verify current destination blobs and archive objects | no mutation | none | exact current/archive modes/blobs | design approved | source preflight | stop; do not import |
| 3 | exact 13-path allowlist | import archive blobs into index/worktree | candidate repository files differ | none yet | index/worktree modes/blobs plus changed-path set | blobs imported | adoption staging | restore only allow-listed candidate Git changes; live untouched |
| 4 | exact imported blobs | write active implementation authority and narrow WR0A decision entry | repository text changes | none yet | path diff and existing-content preservation | blobs imported | governance staging | restore candidate changes or return to gate |
| 5 | exact candidate tree | create clean implementation commit | immutable candidate commit exists | Git object/ref update | commit parent/tree/path set | source candidate | Git commit | new commit/amend before push; no live rollback |
| 6 | clean candidate commit | run Git-backed contract-source verification | no external mutation | none | exact schema/path/mode/blob/digest | source matched | source verifier | stop with digest blocker |
| 7 | source matched | verify exact archive evidence packet | no external mutation | none | packet roster, blob IDs, SHA manifest, JSON | evidence retained | evidence verifier | stop; never regenerate archive packet |
| 8 | zero forbidden processes/siblings | bounded read-only live inspection | reads sensitive managed environment | none | allow-listed identity comparison only | live readback matched | fixture verifier | stop; do not repair or rerun |
| 9 | all proofs passed | create WR0A reconciliation evidence files | candidate Git files created | none yet | schemas, placeholders, hashes, redaction | evidence staged | evidence renderer | remove only uncommitted WR0A evidence or repair design-scoped renderer |
| 10 | sanitized evidence | amend the same adoption commit | final immutable candidate commit | Git object/ref update | final head/tree, parent, changed paths | evidence finalized | Git commit | amend/new exact repair before PR; source blobs remain exact |
| 11 | final head exact | push implementation branch/open PR | remote branch/PR changes | GitHub ref and PR persistence | remote head/base/draft/state | audit pending | publication | do not force unknown remote state; stop on mismatch |
| 12 | exact audited PR | technical-lead merge | main changes | GitHub merge commit | main commit/tree | accepted | technical lead | no agent merge without authority |
| 13 | accepted main | separate status closure | `CURRENT_SLICE.md` returns to none | closure commit/merge | exact main card | status closed | technical lead/operator process | separate repair; no product mutation |

### Commit points

```text
before clean candidate commit:
    only the implementation worktree/index changed; restoration is Git-local;
    live state is never a rollback target.

after clean candidate commit but before merge:
    the candidate commit is immutable source provenance; failures preserve it
    for audit or replace it only by a reviewed amended/new candidate.

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
  -> exact Git worktree at approved basis
      -> read-only Git object/blob verification
      -> exact allow-listed index/worktree import
      -> clean Git commit
      -> bounded read-only environment inspector
      -> sanitized reconciliation evidence
      -> remote branch and PR
```

There is no callback, IPC, real-time, Wine, Proton, or DAW topology. Every Git
command has a bounded normal command deadline. Process matching before live
readback uses the accepted forbidden-family classifier; no termination is
performed. If a forbidden workload appears, inspection stops.

## 9. Identity and authorization ledger

| Operation | Required identity | Freshness/readback | Refusal cases |
|---|---|---|---|
| select main basis | commit + tree + technical-lead receipt | immediately before branch creation | stale ref, different tree, dirty shared worktree |
| read archive | commit + tree + parent | before every import/verification phase | missing object, wrong parent/tree, branch name alone |
| import a path | repository-relative path + exact old mode/blob + exact archive mode/blob + allowlist membership | index/worktree readback immediately after import | unexpected path, mode, blob, symlink, conflict, unmerged entry |
| accept contract source | schema + sorted ten path/mode/blob entries + digest | clean candidate commit and final head | commit SHA alone, dirty/staged governed path, new roster |
| accept workload | exact archive/current blob plus SHA-256 | candidate tree | basename/path alone |
| replace canonical evidence | exact 14-file roster + archive blobs + hash manifest | candidate tree and final head | regenerated bytes, missing/extra file, invalid hash |
| accept live environment | schema/status + transaction + environment digest + runner/source/workload + prefix identity + bounded roster/registries + receipts + retired record | fresh read-only inspection | path existence alone, classification-only difference, sibling/process contamination |
| change decision register | exact current blob + append-only WR0A record | diff review | rewriting prior accepted entries |
| preserve DG0 | exact current-main path/mode/blob roster | candidate/final tree | archive versions, broad doc rewrite |
| publish branch/PR | exact final commit/tree, approved branch/base, non-draft state as specified later | remote readback | unknown remote head, wrong base, merge action |

No friendly name, branch name alone, path suffix, timestamp, in-memory flag, or
“same contents by inspection” is sufficient identity.

## 10. Data, durability, recovery, and migration

| Data class | Owner | Location | Sensitive? | Durability | Backup/rollback | Migration law |
|---|---|---|---|---|---|---|
| Main/merged history | Git | repository object database/GitHub | no | immutable commits | Git history | never rewritten |
| Archive final source | immutable archive commit | Git object database/GitHub archive branch | no | immutable commit/tree | multiple exact object copies | exact blob import only |
| Live WR0 environment | archive transaction | `<HOME>/.local/share/linux-vst-bridge/environments/wr0-proton11` | yes | existing marker/receipts/registry/prefix | no WR0A mutation | none |
| Canonical final WR0 evidence | archive packet | `evidence/wr0-proton-bootstrap/` | sanitized | Git blobs + hash manifest | old packet in history | exact superseding blob set |
| Reconciliation evidence | WR0A | `evidence/wr0a-final-repair-reconciliation/` | sanitized | Git commit + hash manifest | Git history | schema v1; no raw live state |
| DG0 governance | current main | repository docs/prompts/templates | no | Git blobs | Git history | unchanged except narrow WR0A decision/current authority |
| Candidate worktree/index | implementation agent | isolated Git worktree | no | transient until commit | restore exact candidate changes | no adoption from unknown state |

Atomicity is Git-object based: exact blobs are staged, committed, and read back
as a tree before publication. The archive commit is the durable source record;
the final candidate commit is the adoption record. No environment migration or
predecessor retirement occurs. A retry rederives state from Git trees and the
live marker rather than cached booleans. Unknown objects or dirty paths are
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
| exact-blob adoption procedure | verify/import the fixed 13 archive blobs | path/mode/blob allowlist | source redesign, live fixture mutation |
| `docs/WR0_RUNNER_LOCK.md` | exact final source/runner contract prose | archive blob `94471e...` | active slice authority |
| `tools/wr0-proton-bootstrap/launch.py` | accepted WR0 production behavior | archive blob `215718...` | WR0A adoption control |
| `tools/wr0-proton-bootstrap/README.md` | accepted tool contract documentation | archive blob `d90180...` | governance authority |
| canonical WR0 evidence packet | final WR0 empirical truth | 14-file archive packet/hash manifest | reconciliation narrative |
| WR0A reconciliation packet | source/delta/live/governance adoption proof | schema and hash manifest | new WR0 execution evidence |
| `docs/DECISION_REGISTER.md` | append-only accepted reconciliation decision | narrow WR0A entry | rewriting DG0/WR0 decisions |
| `CURRENT_SLICE.md` | active authority/status | DG0 phase fields | imported archive authority |

No new product component or reusable adoption framework is authorized. A
bounded validation script may be embedded in the implementation command/session
or reconciliation evidence generation, but no new tool path is permitted by
this design.

## 13. Changed-path and external-mutation envelope

### Allowed tracked paths for the future cumulative WR0A implementation PR

```text
CURRENT_SLICE.md
docs/DECISION_REGISTER.md
docs/WR0_RUNNER_LOCK.md
docs/slices/WR0A/SLICE_SELECTION.md
docs/slices/WR0A/RECONNAISSANCE.md
docs/slices/WR0A/IMPLEMENTATION_DESIGN.md
docs/slices/WR0A/ADVERSARIAL_DESIGN_REVIEW.md
docs/slices/WR0A/DESIGN_APPROVAL.md
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
evidence/wr0a-final-repair-reconciliation/BASIS.md
evidence/wr0a-final-repair-reconciliation/SOURCE_DELTA.md
evidence/wr0a-final-repair-reconciliation/ARCHIVE_AUDIT.md
evidence/wr0a-final-repair-reconciliation/LIVE_READBACK.md
evidence/wr0a-final-repair-reconciliation/GOVERNANCE.md
evidence/wr0a-final-repair-reconciliation/FINDINGS.md
evidence/wr0a-final-repair-reconciliation/fixture.json
evidence/wr0a-final-repair-reconciliation/hashes.sha256
```

The implementation agent may not edit the accepted selection, reconnaissance,
design-review, or design-approval records; they appear in the cumulative PR
only as immutable authority inputs. Any design repair occurs before approval
and produces a new reviewed design revision.

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
- all prior `docs/DECISION_REGISTER.md` content, permitting only an append-only
  WR0A reconciliation entry;
- HP0, HP1, SR0, Bitwig, Flatpak, `.wine`, Steam compatdata, Proton, Runtime,
  Serum, and unrelated repository/worktree state.

### Prohibited mutation

No archive `CURRENT_SLICE.md`, source outside the 13-blob subset, build file,
dependency, test framework, compatibility profile, runtime path, environment,
receipt, registry, prefix, DAW setting, Steam setting, runner, vendor asset, or
adjacent repository may change.

## 14. Proof matrix

| Claim | Positive proof | Negative/fault proof | Real fixture required? | Production helper exercised? | Retained evidence | Claim ceiling |
|---|---|---|---|---|---|---|
| exact source authority | commit/tree/parent local verification plus technical-lead receipt | missing/wrong object refuses | no | Git | WR0A `BASIS.md` | no network-auth claim |
| exact adoption delta | 25-path path/mode/blob census; 13-blob subset | extra/missing/wrong mode/blob and archive CURRENT_SLICE refused | no | Git | `SOURCE_DELTA.md`, fixture JSON | no source redesign |
| archive closes defect | production helper control-flow audit | ten exact failure-injection implementations inspected | no rerun | accepted production helpers audited | `ARCHIVE_AUDIT.md` | no new runtime result |
| contract identity preserved | clean/final ten-file manifest equals `c654...` | any governed change changes/refuses digest | no | production Git manifest verifier read-only | fixture JSON | exact archive source only |
| final WR0 packet retained | exact 14-file roster/blobs and SHA manifest | regenerated/missing/extra bytes refused | no | packet sanitizer/read-only validator | canonical packet + WR0A packet | historical packet remains in Git |
| live matches adopted truth | exact bounded marker/snapshot/record readback | drift, process, sibling, registry/receipt mismatch blocks | yes, read-only | accepted environment inspector | `LIVE_READBACK.md` | no workload rerun |
| DG0 preserved | protected path/mode/blob equality; append-only decision diff | any other governance change blocks | no | Git | `GOVERNANCE.md` | no process redesign |
| no live mutation | before/after live identity and mtime-safe bounded equality; zero workload process | any difference invalidates | yes, read-only | accepted inspector/process guard | fixture JSON | observation only |
| reconciliation packet safe | hash/JSON/UTF-8/NUL/size/redaction/binary scans | private/raw/proprietary pattern refuses | no | repository validator or bounded exact checks | packet hashes | sanitized metadata only |

## 15. Failure and blocked-result taxonomy

| Result code | Exact owner/stage | Preserved state | Cleanup | Next lawful action |
|---|---|---|---|---|
| `WR0A_SOURCE_IDENTITY_BLOCKED` | main/archive object verification | Git/live state unchanged | remove only owned temp transfer/worktree state | technical lead supplies exact source authority |
| `WR0A_DESIGN_PREFLIGHT_BLOCKED` | basis/worktree/process/sibling/design authority | all fixtures unchanged | none | resolve authority externally; rerun preflight |
| `WR0A_ARCHIVE_LIVE_MISMATCH` | read-only live comparison | live state untouched | none | return to design gate; do not repair |
| `RETURN_TO_DESIGN_GATE` | defect/owner/state/proof finding | no implementation | retain bounded recon facts | revise and independently rereview design |
| `WR0A_ARCHIVE_DELTA_BLOCKED` | path/mode/blob audit | main/archive/live unchanged | none | resolve exact source delta |
| `WR0A_BLOB_IMPORT_BLOCKED` | candidate index/worktree import | live and archive unchanged | restore only exact candidate Git changes if safe | retry from clean approved basis |
| `WR0A_CONTRACT_SOURCE_BLOCKED` | manifest mismatch | candidate commit retained; live unchanged | no live cleanup | inspect exact blob mismatch; no reimplementation |
| `WR0A_EVIDENCE_ADOPTION_BLOCKED` | canonical archive packet | old evidence/history and live preserved | restore only candidate evidence changes | correct exact blob import |
| `WR0A_LIVE_READBACK_BLOCKED` | fresh fixture join | candidate commit and live state preserved | none | return to design gate if material |
| `WR0A_GOVERNANCE_DRIFT_BLOCKED` | DG0/current authority comparison | candidate/live/archive preserved | restore candidate governance changes only if exact | technical-lead review |
| `WR0A_RECONCILIATION_EVIDENCE_BLOCKED` | packet rendering/sanitization | clean candidate commit/live preserved | remove only owned uncommitted packet stage | regenerate from retained Git/live facts |
| `WR0A_IMPLEMENTATION_AUDIT_BLOCKED` | independent exact-head audit | PR open/unmerged; live unchanged | none | repair within approved design or return to gate |

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
- more/fewer/different WR0 paths or blobs are needed;
- archive source does not close the guarded rename/first-fsync boundary;
- the 80-case ledger does not exercise the production helpers as recorded;
- live environment no longer exactly matches archive evidence;
- archive evidence cannot remain byte-identical at canonical paths;
- a governed source blob must be edited after import;
- archive `CURRENT_SLICE.md` would be required;
- DG0 requires more than the approved narrow current-slice/decision changes;
- any Proton/Wine/workload execution or live-environment mutation appears
  necessary;
- status closure or successor selection is proposed inside implementation.

The agent may retain bounded read-only evidence but may not patch forward.

## 17. Implementation sequence

```text
1. Verify exact approved design blob/revision and approval receipt.
2. Verify current main basis, immutable archive object/tree/parent, clean
   implementation worktree, no competing successor, zero forbidden workloads,
   SteamOS read-only, zero transaction siblings, and exact live fixture.
3. Freeze the exact current and archive path/mode/blob ledgers.
4. Import only the exact 13 archive blobs; explicitly assert archive
   CURRENT_SLICE.md was not imported and all 11 shared WR0 blobs remain exact.
5. Make only the approved WR0A current-slice and append-only decision changes.
6. Create one clean intermediate adoption commit.
7. From that clean commit, verify the exact ten-file contract-source manifest,
   workload hash, 14-file archive evidence packet, changed-path envelope, and
   protected DG0/source blobs.
8. Perform one fresh bounded read-only live environment comparison; do not run
   Proton, Wine, or the Windows workload.
9. Render the eight-file WR0A reconciliation packet from exact Git identities
   and the allow-listed live readback.
10. Validate JSON/hashes/UTF-8/NUL/size/redaction/binary/proprietary scans.
11. Amend the same adoption commit with reconciliation evidence.
12. At final head, reverify contract-source digest, archive evidence blobs,
   live readback agreement, governance protection, exact paths, clean worktree,
   and no forbidden process or transaction sibling.
13. Push only the approved implementation branch and open an ordinary,
   non-draft, unmerged PR targeting main.
14. Obtain independent pre-PR implementation audit and technical-lead exact-head
   review. The agent does not merge.
15. After merge, perform status closure in a separate authority transition.
```

The intermediate adoption commit is required because the operative WR0 source
identity must be measured from clean Git state before final evidence-only
amendment. The final head must reproduce the same path/mode/blob manifest
without executing the workload.

## 18. Adversarial design review disposition

```text
review_path: docs/slices/WR0A/ADVERSARIAL_DESIGN_REVIEW.md
review_identity: pending fresh independent review
review_result: pending; expected DESIGN_CLEAR or DESIGN_REPAIR_REQUIRED
findings_resolved: none yet
unresolved_findings: independent review not yet performed
```

This design card is not approved and must not be used as implementation
authority.

## 19. Approved design identity

```text
card_path: docs/slices/WR0A/IMPLEMENTATION_DESIGN.md
card_git_blob: to be measured at the committed design head and named by the separate approval receipt
card_sha256: to be measured at the committed design head and named by the separate approval receipt
design_revision: wr0a-design-v1
approval_receipt_path: docs/slices/WR0A/DESIGN_APPROVAL.md
approval_receipt_identity: absent; required before implementation
implementation_authorized: false
```

The card cannot contain a stable self-hash without circularity. Its committed
Git blob and file SHA-256 are external design identities and must be recorded
by the fresh review and separate approval receipt.

## 20. Implementation handoff

Any later implementation prompt must include:

- the exact accepted-main implementation basis commit/tree;
- this card path, revision, Git blob, and SHA-256;
- the exact `DESIGN_CLEAR` review identity;
- the exact operator approval receipt;
- the one primary claim and complete claim ceiling;
- immutable archive commit/tree/parent and exact 13-blob import ledger;
- exact live fixture and read-only-only law;
- exact cumulative changed paths and Git-only external mutation;
- the proof matrix, blocked taxonomy, and material-discovery stop law;
- ordinary non-draft PR target/state and explicit no-merge instruction.

The implementation agent must receive this bounded contract, not infer work
from PR #7 discussion, the archive's active WR0 card, or the full dossier.
