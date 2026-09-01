# WR0A reconnaissance — Post-Merge Final WR0 Repair Reconciliation and Adoption

## Result

```text
WR0A_LOCAL_SOURCE_CLEAR
WR0A_DESIGN_PREFLIGHT_CLEAR
WR0A_ARCHIVE_LIVE_MATCH
ARCHIVE_DELTA_AUDITED
IMPLEMENTABILITY: IMPLEMENTABLE
authority_phase: reconnaissance_and_design
implementation_authorized: false
```

No Proton, Wine, Windows workload, Bitwig, Serum, validator, runner update,
environment write, environment rename, receipt regeneration, evidence import,
or product-source modification occurred during reconnaissance.

## 1. Source authority and local-object verification

The public HTTPS fetch requested by the initial WR0A prompt was attempted and
could not authenticate. The technical lead then supplied a superseding source
authority receipt and explicitly prohibited another fetch, credential request,
token, origin change, SSH agent forwarding, or alternate source.

```text
github_main_verified_by_technical_lead:
  commit: b517f96bafa388107ea28c9d2e91528fd37b00d9
  tree: 839cda6145f592642f06b5273855b04ffa0562ac

github_archive_verified_by_technical_lead:
  branch: archive/wr0-final-repair-52be943
  commit: 52be94316664f88a19df630e164105a0ca50b875
  tree: 21601814bd352114fe2e55833c88a81e47e13e41
  parent: 3deb414a54174cd95432c84e117a642f30c482fe

deck_local_object_verification:
  main_commit_object: exact
  main_tree: exact
  archive_commit_object: exact
  archive_tree: exact
  archive_parent: exact

network_fetch:
  attempted: true
  result: authentication_unavailable
  required_for_source_identity: false
```

The exact objects were made available to the Deck by a credential-free Git
bundle transferred from an operator-owned authenticated checkout after the
operator said to proceed. Bundle SHA-256:
`b059f91fe04b436774976e416c78bd8bc8fba85f9d98b2cd03e67e58b256d021`.
The bundle advertised only the exact main and archive objects, verified before
and after transfer, and was removed from both temporary locations afterward.
No remote URL or configured `origin` changed, and no credential crossed hosts.

## 2. Design pre-flight

| Check | Read-only result |
|---|---|
| Device | Exact Steam Deck / SteamOS / x86_64 fixture |
| Repository | Expected Deck checkout, recorded as `<HOME>/code/Linux-VST-bridge` |
| Current main object/tree | Exact `b517f96...` / `839cda6...` |
| Main authority card | `status: no_active_slice` before WR0A activation |
| Existing successor PR | None open |
| Ordinary worktree | Clean; no reset, stash, or rebase performed |
| Design branch | Created directly from exact `b517f96...` |
| Design worktree | Separate canonical user-owned sibling; exact basis |
| Forbidden workloads | Bitwig 0; validator 0; Wine/Proton 0; Runtime 0; UMU 0; yabridge 0 |
| SteamOS read-only | Enabled |
| WR0 transaction siblings | Stage 0; previous 0; retiring 0; journal 0 |
| WR0 owned workload | None running |
| Live environment | Present at the exact canonical owned path |

The actual hostname, Linux user, IP address, and private home path are not
retained.

## 3. Governing material and history read

The following were read in full from current main:

- `README.md`, `AGENTS.md`, `GOVERNANCE.md`, and `CURRENT_SLICE.md`;
- `docs/DEVELOPMENT_PROCESS.md`, `docs/ARCHITECTURE.md`,
  `docs/DESIGN_DOSSIER.md`, and `docs/DECISION_REGISTER.md`;
- all three DG0 templates and the adversarial-design/pre-PR audit prompts.

The complete merged WR0 source/evidence at `9228217...`, complete archived
final-repair source/evidence at `52be943...`, WR0 status closure, DG0
implementation, DG0 status closure, PR #7 body, all technical-lead reviews,
both PR conversation comments, and all four resolved inline threads were also
inspected. GitHub reports PR #7 merged and closed at `9228217...` with one
commit and 25 changed paths. Its later conversation comment records that the
final repair completed at `52be943...` after the merge froze the PR head.

Governing consequence: DG0's selection/design/independent-review/operator-
approval sequence is current authority. The immutable archive is source
material and historical evidence, not active-slice authority.

## 4. Reconciliation event

```text
merged historical WR0:
  implementation: 9228217b2abf7314b9dfaecc5fc4323d5f3d7a89
  tree: 8da6817eba1f259d3565e377fcb50098ab8f3cf2
  merge: 8237b96ce7c885edcf4e7a0923f2ac78d05a928d
  environment: 447e6d4dfccfbebe7be44cc521e8ed192bfad4ab1fe1f16673d391161db73c24
  contract_source: 887b148b7862038fd8fc0af52146ecc7452a5fbf82c22c5af8eaec6fe47504b8

post-merge final repair:
  archive: 52be94316664f88a19df630e164105a0ca50b875
  tree: 21601814bd352114fe2e55833c88a81e47e13e41
  environment: d3ed38d7ed53e9a5973cbf22fc504f2479dbdd15616fe1bfc1d5e1cd0bf5f4c4
  contract_source: c6543004fbcd70252393f0e0cab4f9ad7a31e85f433ce3e903e690244cc79541
```

This is neither random fixture drift nor authority to restore the older live
environment. Current main contains the older source/evidence while the current
live environment is the exact durable result of the archived repair.

## 5. Exact archive scope and delta

The merged and archive commits each contain the same original 25-path WR0
envelope. Fourteen paths differ; all modes remain exact. Eleven paths are
blob-identical.

### Differing paths

| Path | Merged mode/blob | Archive mode/blob | Semantic classification | Contract source? | Future adoption |
|---|---|---|---|---|---|
| `CURRENT_SLICE.md` | `100644` / `3b2f868ac8ed28095113cafb7dae8ab6eb26cd32` | `100644` / `a4dafdd0e0a94bf15fe683453e733b9eb2383b84` | obsolete active WR0 authority | no | **must not adopt** |
| `docs/WR0_RUNNER_LOCK.md` | `100644` / `d2076de6c90f6611ea8ab55426b57a6edf5c8a4c` | `100644` / `94471e033951f735bd2eb792020c7197ce3536c8` | exact final-repair documentation | yes | exact archive blob |
| `evidence/wr0-proton-bootstrap/BASIS.md` | `100644` / `e7259699aa7c66885ca36f6c0d5ab97d8396fa01` | `100644` / `feafc43b8ecc4652b1a524dba68827ff91655ea4` | exact final-repair evidence | no | exact archive blob |
| `evidence/wr0-proton-bootstrap/ENVIRONMENT.md` | `100644` / `eb927c55940862df8d7f4d22e5f058327b4db4ef` | `100644` / `4bdec8355fff12b1edf5bd9739e9c8d14068dad9` | exact final-repair evidence | no | exact archive blob |
| `evidence/wr0-proton-bootstrap/EXIT_PROPAGATION.md` | `100644` / `cd15fd2b55437497a3fb39801f919ea7dfb0eca7` | `100644` / `af250cfb53006d55c4808ea7a5b63ca48c9eb4a0` | exact final-repair evidence | no | exact archive blob |
| `evidence/wr0-proton-bootstrap/LAUNCH_CONTRACT.md` | `100644` / `f155a0d8594f8045869c784eea1d3d96ac80f1a0` | `100644` / `16c207a098f2a813d2dfaec780a12a616174eaaf` | exact final-repair evidence | no | exact archive blob |
| `evidence/wr0-proton-bootstrap/NEGATIVE_TESTS.md` | `100644` / `ed5486fc3c328a7414218ad3c963751029b88a04` | `100644` / `72a93fbf8e1d2b7ede510329df5a3853be65aa22` | exact final-repair evidence | no | exact archive blob |
| `evidence/wr0-proton-bootstrap/PROCESS_GUARDS.md` | `100644` / `2e0cb17837cd319ca9e62305eb7be96c228c993a` | `100644` / `0194ba983cb004535752105b066e44df87b622e7` | exact final-repair evidence | no | exact archive blob |
| `evidence/wr0-proton-bootstrap/RUN_1.md` | `100644` / `7f2dce634201e9fd6117fe6622b5cdc28fd3fb87` | `100644` / `d6bbcdb3dfdcfd993b2ca4ba5cb2cda4aa652a3e` | exact final-repair evidence | no | exact archive blob |
| `evidence/wr0-proton-bootstrap/RUN_2.md` | `100644` / `b7b1be520334718c5f3964f183fba6d3bb47d1e5` | `100644` / `281379a881c1ea526d6e08d28cbc5ffcb2dd43cd` | exact final-repair evidence | no | exact archive blob |
| `evidence/wr0-proton-bootstrap/fixture.json` | `100644` / `6f056a66c53c32522875fd831c3bb910b2de30bb` | `100644` / `7919772c25b97da0791095abbb470925feffc58d` | exact final-repair evidence | no | exact archive blob |
| `evidence/wr0-proton-bootstrap/hashes.sha256` | `100644` / `6863f7415bb0a271b082f57795a3d30a53332d41` | `100644` / `5369c4cb3e55314fd2c2589c9394518b40f5023f` | exact final-repair evidence manifest | no | exact archive blob |
| `tools/wr0-proton-bootstrap/README.md` | `100644` / `4e2b71a7ae524b3ab76e57db6f15e8595de746d1` | `100644` / `d90180efb2812feb0e510e7f0fb697d08a20c466` | exact final-repair contract documentation | yes | exact archive blob |
| `tools/wr0-proton-bootstrap/launch.py` | `100755` / `2a5337be3c2d1a330a245289f694b5ca3a18ab66` | `100755` / `215718bb641765da9163779c0e2145bd02d3198a` | exact final-repair source | yes | exact archive blob |

### Blob-identical paths

| Path | Exact shared mode/blob | Classification | Contract source? |
|---|---|---|---|
| `evidence/wr0-proton-bootstrap/FINDINGS.md` | `100644` / `a29ffdbbe9124153d827aceb43389176a87c77a7` | unchanged final claim ceiling | no |
| `evidence/wr0-proton-bootstrap/PRESERVATION.md` | `100644` / `5ecca0e22809d1fa3dfe68fa0c5591df7fcba3ba` | unchanged preservation evidence | no |
| `evidence/wr0-proton-bootstrap/RUNNER_LOCK.md` | `100644` / `fbd74d5706d0eb47afe44f690e2c99d04b89bfce` | unchanged runner evidence | no |
| `evidence/wr0-proton-bootstrap/SANITIZATION.md` | `100644` / `913c955d71c99223717fcd6b40cfcdbdfa968e0e` | unchanged sanitization evidence | no |
| `tools/wr0-proton-bootstrap/common.sh` | `100755` / `2e382985402e19b09d71ceaf85078a77f52bab57` | unchanged governed wrapper | yes |
| `tools/wr0-proton-bootstrap/environment.sh` | `100755` / `b5389c36e0d75484b2cdad5f159f8a9cbd1d73e6` | unchanged governed wrapper | yes |
| `tools/wr0-proton-bootstrap/inspect-runner.sh` | `100755` / `e157340eff629127332d94650d4f369ad8688d94` | unchanged governed wrapper | yes |
| `tools/wr0-proton-bootstrap/negative-tests.sh` | `100755` / `b1882ddb3f903a441145866bfb24650ff046a20e` | unchanged governed wrapper | yes |
| `tools/wr0-proton-bootstrap/preflight.sh` | `100755` / `13bceffe6c75830ef6ed0828d32dcd7d7938068b` | unchanged governed wrapper | yes |
| `tools/wr0-proton-bootstrap/sanitize.sh` | `100755` / `44fd2d9ecf369030ed58b7898cdcfb87c2075cfb` | unchanged governed wrapper | yes |
| `windows-fixtures/wr0-probe/wr0-probe.cmd` | `100644` / `04c8707c01c92cfd007fe7f7a72a8adbd4de3588` | unchanged tracked workload | yes |

Adoption therefore requires 13 exact archive blobs: the 14 differing paths
minus obsolete `CURRENT_SLICE.md`. No equivalent reimplementation, cherry-pick
of the archive commit, or broad prefix import is justified.

## 6. Independent contract-source and workload verification

The archive's stable sorted path/mode/blob manifest was independently
reconstructed as:

```text
schema: linux-vst-bridge-wr0-contract-source/v1
file_count: 10
sha256: c6543004fbcd70252393f0e0cab4f9ad7a31e85f433ce3e903e690244cc79541
```

| Path | Mode | Git blob |
|---|---:|---|
| `docs/WR0_RUNNER_LOCK.md` | `100644` | `94471e033951f735bd2eb792020c7197ce3536c8` |
| `tools/wr0-proton-bootstrap/README.md` | `100644` | `d90180efb2812feb0e510e7f0fb697d08a20c466` |
| `tools/wr0-proton-bootstrap/common.sh` | `100755` | `2e382985402e19b09d71ceaf85078a77f52bab57` |
| `tools/wr0-proton-bootstrap/environment.sh` | `100755` | `b5389c36e0d75484b2cdad5f159f8a9cbd1d73e6` |
| `tools/wr0-proton-bootstrap/inspect-runner.sh` | `100755` | `e157340eff629127332d94650d4f369ad8688d94` |
| `tools/wr0-proton-bootstrap/launch.py` | `100755` | `215718bb641765da9163779c0e2145bd02d3198a` |
| `tools/wr0-proton-bootstrap/negative-tests.sh` | `100755` | `b1882ddb3f903a441145866bfb24650ff046a20e` |
| `tools/wr0-proton-bootstrap/preflight.sh` | `100755` | `13bceffe6c75830ef6ed0828d32dcd7d7938068b` |
| `tools/wr0-proton-bootstrap/sanitize.sh` | `100755` | `44fd2d9ecf369030ed58b7898cdcfb87c2075cfb` |
| `windows-fixtures/wr0-probe/wr0-probe.cmd` | `100644` | `04c8707c01c92cfd007fe7f7a72a8adbd4de3588` |

The tracked workload was independently hashed from the archive blob:

```text
sha256: 4518ca37b8d7e005f01b441de5273447b70fa7c8c9c3d2b9c194a91782cfecac
```

## 7. Read-only live environment comparison

The accepted production inspector was used in read-only mode. Its observed
classification (`passed`) was normalized against the retained historical
classification (`created_by_slice`); classification describes observation
context and is not an identity field. All substantive identity fields agreed.

| Fact | Archive evidence | Live readback | Result |
|---|---|---|---|
| Marker schema/status | `linux-vst-bridge-wr0-environment/v1` / `ready` | exact | passed |
| Transaction | `wr0-20260901T053317Z-183afd5bbf0736e4` | exact | passed |
| Environment identity | `d3ed38d7ed53e9a5973cbf22fc504f2479dbdd15616fe1bfc1d5e1cd0bf5f4c4` | exact | passed |
| Runner binding | `2d64df1d36786ca2d0e955c553005423dc2b5bdd714bd0a17872622e33912547` | exact | passed |
| Contract binding | `c6543004fbcd70252393f0e0cab4f9ad7a31e85f433ce3e903e690244cc79541` | exact | passed |
| Workload binding | `4518ca37b8d7e005f01b441de5273447b70fa7c8c9c3d2b9c194a91782cfecac` | exact | passed |
| Prefix device/inode | `66312` / `5866359` | exact | passed |
| Bounded top-level roster | six declared entries | exact | passed |
| `system.reg` hash/size | `c8d9fffd43fb5a442652ab71821b0cdba169fcc22f01d46930fef265ffe3fc1b` / `4085559` | exact | passed |
| `user.reg` hash/size | `9811dd3f1e1b6b0fef1001861e8f0afc013a12e027effa454dad8be5eb9b53ca` / `107055` | exact | passed |
| `userdef.reg` hash/size | `0ab8c8dd98a9b94d6bd4b6e4588dc9c5478908c71e7f1cb04fd81b3b2602378f` / `4190` | exact | passed |
| Host run receipts | three | exact | passed |
| Internal Run 2 receipt | `42b31ae82585537c43a7bdf1b5cdb0ad2f4335cf53209b34343a8c8c26430453` | exact | passed |
| Retired record | `917c939c605f457376b0bdd53e06d252d4aecee4b39728850b2e1a61aa102b3e` | exact | passed |
| Transaction siblings | zero | zero | passed |
| Owned workload processes | zero | zero | passed |

The archive `hashes.sha256` verified all 13 payload files in the 14-file WR0
packet. JSON parsing, packet roster, and retained sanitization classifications
were exact.

## 8. Final-repair defect audit

The archive closes the known predecessor-backup rename/fsync defect in the
actual production helper:

1. `predecessor_backup_context()` canonicalizes and binds the environment
   parent, canonical destination, transaction-derived backup, transaction ID,
   and complete predecessor snapshot.
2. `move_predecessor_to_backup()` verifies the canonical exact predecessor and
   exact backup absence before movement.
3. Destination-to-backup rename, the first parent-directory fsync, and complete
   backup verification are inside one `try` recovery boundary.
4. Any post-rename exception invokes the production physical-state recovery
   helper before the original exception is rethrown.
5. `recover_precommit_predecessor()` inspects actual destination and backup
   state, verifies the complete predecessor before restore, removes only an
   exact new-transaction destination, and refuses unknown/forged objects.
6. A restoration fsync exception is reconciled by exact physical readback; an
   exact restored destination with absent backup is accepted without a second
   destructive recovery.
7. The caller sets its event flag and advances to `predecessor_backed_up` only
   after the structured successful receipt returns.
8. The outer pre-commit handler invokes physical-state recovery regardless of
   the stale boolean event.
9. The existing durable record still commits the new environment before
   predecessor retirement. Post-commit exceptions cannot call pre-commit
   rollback or remove the authoritative new environment.

No parallel synthetic state machine substitutes for these owners.

## 9. Production test audit

The archive reports 80/80 retained passing cases: 75 non-live production cases
and five actual live cases. The ten final backup cases were traced to calls of
the production `move_predecessor_to_backup()` and
`recover_precommit_predecessor()` helpers with narrow test-only operation
injection:

1. rename then first-fsync failure restores the exact predecessor;
2. backup-verification failure restores the exact predecessor;
3. restore-fsync failure is reconciled through exact outer readback;
4. physical rename before event recording is recovered;
5. false event plus exact backup is recovered;
6. unknown backup remains byte-identical and is refused;
7. forged WR0-like backup identity is refused;
8. destination and backup both absent fail closed;
9. unknown destination plus exact backup is refused without overwrite;
10. successful guarded receipt is the only basis for caller state advance and
    supports exact pre-commit rollback.

The assertions check physical paths and identities, not only names or expected
exceptions. The first parent fsync is explicitly asserted to be inside the
guard. Existing source-manifest, command-vector, causal handshake, process-
topology, durable-commit, retirement, evidence-finalization, malformed-gate,
and live-held-cleanup cases remain present.

## 10. Ownership findings

| Fact | Owner | Consequence |
|---|---|---|
| Merged WR0 historical truth | commit `9228217...` and merge `8237b96...` | immutable historical fact; never rewritten |
| Final-repair source truth | archive commit `52be943...` | exact path/mode/blob import source only |
| Current live environment truth | transaction `wr0-...053317...` marker and retired record | read-only; archive-owned, not restored to old state |
| Current repository truth | current `main` beginning at `b517f96...` | incomplete until separately approved adoption merges |
| Adoption delta | exact 13-blob allowlist | no broad checkout/cherry-pick |
| Final WR0 source identity | ten-file manifest `c6543004...` | must reproduce byte-for-byte after adoption |
| Final WR0 environment identity | `d3ed38d7...` | must match read-only after adoption |
| Historical WR0 evidence | Git history at `9228217...` | remains recoverable and named as superseded |
| Superseding WR0 evidence | archive canonical packet | exact final packet replaces stale canonical paths |
| Reconciliation evidence | future `evidence/wr0a-final-repair-reconciliation/` packet | records sources, delta, live readback, and governance preservation |
| DG0 governance | current-main DG0 blobs | byte-identical protected authority |
| Active reconciliation authority | `CURRENT_SLICE.md` plus WR0A receipts | cannot be imported from archive |

## 11. Evidence representation decision

Two strategies were considered:

- retaining the archive packet only under a new superseding path would leave
  the canonical `evidence/wr0-proton-bootstrap/` path stale and ambiguous;
- replacing the ten differing canonical evidence blobs with their exact archive
  versions, while adding a separate reconciliation packet, makes current
  repository truth unambiguous and preserves old evidence through immutable Git
  history.

The second strategy is selected for design. It retains all final archive hashes
and internal links, names both old and new identities, and teaches future
readers that the older environment/evidence is superseded. It does not rewrite
archive content.

## 12. Implementability verdict and remaining gates

```text
IMPLEMENTABLE
```

Exact source objects, path/mode/blob delta, contract identity, workload digest,
live/archive agreement, defect closure, production-helper tests, and governance
boundary are all established. No implementation ambiguity requires another
reconnaissance mutation.

Implementation remains unauthorized. Required next steps are:

1. fresh independent adversarial review of the exact design card;
2. repair and new revision if that review finds a material gap;
3. separate operator approval naming the exact accepted design identity;
4. only then, bounded exact-blob adoption under the approved envelope.

## 13. Nonclaims

This record does not adopt source, replace evidence, mutate or rerun the WR0
environment, establish a new Windows execution result, load a VST3, inspect or
launch Serum, launch Bitwig, process audio, implement a proxy/bridge/IPC/shared
memory/manager/broker, select a product runner, prove real-time safety, or
generalize beyond the exact archived Deck fixture.
