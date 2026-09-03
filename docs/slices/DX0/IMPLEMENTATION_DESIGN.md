# DX0 implementation design v2

```yaml
slice: DX0
design_revision: dx0-design-v2
design_status: proposed_for_adversarial_review
implementation_authorized: false
```

V2 supersedes reviewed V1 commit
`17304ef3b22d44eee9ebb5bbbe16bb237e9bad96`, tree
`98b1e6cfa5b039eeada5aee1c2f88670a7af9b9e`, design blob
`b71024010e828fba9acc5c9a2f82546a5bb6e6c9`, SHA-256
`de40a98822bc7ea4fda8684047f77007177626e5b8a49cdfeda7bed2d845682e`.
GitHub review `5096569104` returned `DESIGN_REPAIR_REQUIRED`; its two bounded
findings are retained in [the adversarial review record](ADVERSARIAL_DESIGN_REVIEW.md).

## Authority, claim, and ceiling

This card refines [the DX0 selection](SLICE_SELECTION.md): basis
`859422be75f65da4dd7dc51394b84470ad594358`, tree
`7b9a721f3e691974dd1720c20d9ef030932978a9`; unmodified activation
commit `6631c502f11a8c0a2552fe333f1b99ea64c35b30`, tree
`be7a4766702a462f94451dcf5fa3f24c86c8e4de`, sole parent the basis. D-026
governs.

Primary claim:

> From one exact Mac-side command and one closed proof plan, the existing proof
> system derives a Windows-build-input identity separate from the complete
> implementation-source identity, reuses an accepted content-addressed AGain
> fixture without rebuilding it, produces or reuses the exact compatible
> Windows host artifact, performs source custody, Mac-to-Deck handoff,
> detached-worktree admission, one supervised Deck proof batch, cleanup, and
> evidence return, and proves that a non-build-only source change causes zero
> Windows builds.

DX0 adds no product capability, Windows C++ path, VST3 call, or fault module.
It does not perform another interface or method; buses, sample-size queries,
processing setup, audio/events, controller work, state/parameters, GUI,
proxy/C ABI/IPC, Bitwig, Serum, another plug-in/DAW, packaging, signing,
release suitability, runner selection, or general compatibility. A hosted
runner is not claimed immutable.

## Reconnaissance and measured defect

Accepted WA0 is merge `5d084ba5032dc8a7ce93be51e7d75dfd0d37ee22`.
Its final source/evidence commits are
`24b7e6da7e29a5bd358097a6b89c5c59b747c413` /
`99478005e9f2675036100486c87952b76d411f84`. Accepted artifact `9869994854`
has manifest SHA-256
`028228c6a8cc638b4aaf1f477b317359a22eb1dd84e90a12193bb3c5d9159070`.

One 17-path digest currently couples build, artifact, custody, handoff,
execution, and evidence; `run.py` renders on Deck. An `evidence.py`-only change
between `4a5ff302acc4927142003e29bd401368920b275b` and final source invalidated
all phases despite byte-identical checked build inputs. The producer also
builds AGain twice. Runner image
`20260830.290.1` produced AGain SHA-256 `9c65a11bd67fdfc2416cc509b363c7a9ad83998cc1db5fa3cf2640e2c7431da8`;
image `20260824.284.2` produced the accepted
`60aa9ff6b9918d4330449e7b3ab34b588dd93cba09f37413a3cd91f6e7d2e18f`.

Static inspection yields a 14-command WA0 baseline (push, lookups, custody,
bundle, transfers, admissions, Deck batch, evidence validation/placement, and
closure), nine manually propagated values, and nine live exercises. DX0's
intended steady state is one command, zero copied identifiers, and one live
regression. These are baseline/target counts, not measured savings; one-time
fixture seeding is separate.

## Three owners

1. **`WindowsBuildInputIdentity`** owns the deterministic identity of inputs
   capable of changing the Windows host payload: exact repository records,
   SDK, build contract, toolchain constraint, and producer action identities.
   It owns neither orchestration nor an artifact.
2. **`AcceptedFixtureStore`** owns one verified, private, content-addressed
   copy of the accepted AGain bundle on the Mac and one admitted Deck copy. It
   never compiles or silently replaces the fixture.
3. **`ProofTransactionDriver`** owns the one Mac command, planning, freeze,
   phase selection, existing mechanism invocation, bounded state receipt,
   resume, result retrieval, evidence rendering, and concise closure. It is
   not a daemon, database, task engine, or workflow language.

Supporting manifests are identities, not owners.

## Exact identity algorithm and domains

Manifest records are exactly `path`, `git_mode`, `git_blob`, unique and
raw-UTF-8 sorted. Canonical JSON uses UTF-8, sorted keys, compact separators,
ASCII escapes, and one trailing LF; identity is lowercase SHA-256 of those
bytes. Missing, non-blob, duplicate, unclassified, or dirty input blocks.
Membership is closed and may span domains.

| Domain and schema | Exact contents |
|---|---|
| `CompleteImplementationSourceIdentity`; `linux-vst-bridge-dx0-complete-source/v1` | Source commit/tree/parent/ref and exact 10-record implementation roster; any source commit change changes it. |
| `WindowsBuildInputIdentity`; `linux-vst-bridge-dx0-windows-build-input/v1` | The 17 records below; SDK root/seven submodules/no-patch state; checkout locks; toolchain/build options; action SHAs; producer contract. |
| `AcceptedFixtureIdentity`; `linux-vst-bridge-dx0-accepted-fixture/v1` | Exact 14-record bundle manifest, module hash, accepted WA0 artifact/custody coordinates, and store receipt. |
| `DeckExecutionInputIdentity`; `linux-vst-bridge-dx0-deck-execution-input/v1` | Host-artifact manifest, accepted-fixture identity, closed-plan identity, Runtime/Proton digest, and exact records for `artifacts.py`, `common.py`, `environment.py`, `normalize.py`, `run.py`, `supervise.py`, and `tools/wr0-proton-bootstrap/launch.py`. |
| `EvidenceRendererIdentity`; `linux-vst-bridge-dx0-evidence-renderer/v1` | Exact records for `evidence.py` and `common.py`, evidence schema/roster, and validator contract. |
| `ProofTransactionIdentity`; `linux-vst-bridge-dx0-proof-transaction/v1` | Role-labelled producer P, Deck execution E, and consumer C source identities; five narrower identities; operation nonce; artifact, fixture, plan, observation, renderer, and phase hashes. P/E/C may differ. |

The exact top-level key rosters are respectively: complete source — `schema`,
`commit`, `tree`, `parent`, `ref`, `record_count`, `records`; build input —
`schema`, `repository`, `record_count`, `records`, `sdk`, `build_contract`,
`action_commits`; fixture — `schema`, `fixture_id`, `bundle_manifest`,
`accepted_wa0_custody`; Deck input — `schema`,
`host_artifact_manifest_sha256`, `accepted_fixture_identity_sha256`,
`proof_plan_sha256`, `runtime_proton_sha256`, `record_count`, `records`;
renderer — `schema`, `record_count`, `records`, `evidence_schema`,
`evidence_paths`; transaction — `schema`, `operation_nonce`,
`artifact_producer_source`, `deck_execution_source`,
`evidence_consumer_source`, `windows_build_input`, `host_artifact`,
`accepted_fixture`, `deck_execution_input`, `proof_plan`,
`transaction_result`, `evidence_renderer`, `phase_receipts`.

The exact Windows repository roster is:

```text
.github/workflows/wf0-windows-msvc-build.yml
CMakeLists.txt
cmake/WF0DependencyLock.cmake
tools/wf0-factory-census/build.py
tools/wf0-factory-census/common.py
tools/wf0-factory-census/verify.py
windows-factory-probe/CMakeLists.txt
windows-factory-probe/include/linux_vst_bridge/wf0_probe/census.h
windows-factory-probe/include/linux_vst_bridge/wf0_probe/events.h
windows-factory-probe/source/component_instance_session.cpp
windows-factory-probe/source/component_instance_session.h
windows-factory-probe/source/factory_census.cpp
windows-factory-probe/source/factory_census.h
windows-factory-probe/source/main.cpp
windows-factory-probe/source/win32_module.cpp
windows-factory-probe/source/win32_module.h
windows-fixtures/wf0/CMakeLists.txt
```

This is a transitive build roster, not the DX0 change roster. The named
`wa0-positive-regression-v1` plan freezes it; future plans require reviewed
closed rosters, never caller-provided paths. The build receipt names its true
producer commit and build-input identity. A later consumer may reuse it only
after reproducing the same identity. The transaction preserves the true host
producer P, true original Deck execution source E, and current
renderer/consumer C without requiring equality. Ambiguous cache matches block
rather than select latest.

Path classification for the 10-path DX0 implementation is exact:

- workflow and `build.py`: complete + Windows build;
- `common.py`: complete + Windows build + Deck execution + renderer;
- `artifacts.py` and `environment.py`: complete + Deck execution;
- `run.py`: complete + Deck execution;
- `evidence.py`: complete + evidence renderer;
- `negative_tests.py` and `tools/host-proof.py`: complete;
- `README.md`: complete only.

Unchanged transitive and Deck/renderer records remain inputs.
`AcceptedFixtureIdentity` depends on accepted bytes/provenance, never code.
Transaction identity changes with its complete-source join. Classification
uses only these rosters, never extension/prefix/diff heuristics.

## Accepted fixture store

AGain's accepted 14-record bundle-manifest SHA-256 is
`bfaa1dce4d2e189f89cee41493838824efe647e361e81436676b7b3a86ff5164`;
module SHA-256 is
`60aa9ff6b9918d4330449e7b3ab34b588dd93cba09f37413a3cd91f6e7d2e18f`.
Its receipt binds WA0 artifact `9869994854`, manifest
`028228c6a8cc638b4aaf1f477b317359a22eb1dd84e90a12193bb3c5d9159070`,
workflow run `33689659595` attempt `2`, producer source
`24b7e6da7e29a5bd358097a6b89c5c59b747c413`,
wrapper digest `81f3d431a0cb00c4cc121799818ce5ac1b8487dfde583d03eb48b98adda7bf3b`,
payload digest `a9397ed53a070522230e6b75ce045e0aedaa3c0e73736cb8a901f203f09ef17c`,
build-receipt SHA-256
`fdf9aabcfcf7eaefcedb50bd15d54f3a93c2babe7525aeb18515b3f13d67fe1e`,
and Mac-custody-receipt SHA-256
`9d282701cde6f4576452044d58a13ab7380c19e4b0a5ccca0f950e0ebfcdef23`.

The canonical Mac root is
`<HOME>/Library/Application Support/Linux VST Bridge/proof/fixtures/by-manifest/<bundle-manifest-sha256>/`;
the Deck root is
`<HOME>/.local/share/linux-vst-bridge/fixtures/by-manifest/<bundle-manifest-sha256>/`.
Both require a new stage, archive/path bounds, all hashes,
`DX0_ACCEPTED_FIXTURE_RECEIPT.json` and its SHA-256 sidecar, atomic publish,
and read-only content. Normal `run` only verifies/reuses them.

Operator-only `seed-fixture`, outside `run`, may extract from retained WA0
custody or an authenticated exact-ID Mac download, verify every join, publish
the Mac copy, and admit Deck bytes by ordinary SSH. No binary enters Git or
depends solely on an expiring Actions object. Missing/corrupt/unsafe/unavailable
bytes return `DX0_ACCEPTED_FIXTURE_BLOCKED`; AGain is never rebuilt as repair.

## Host artifact and two validation lanes

The producer is `workflow_dispatch` only. Before its API call the driver
verifies access, cache miss, exact remote ref/source, workflow path/blob, plan,
freeze, and build identity, then atomically persists `prepared` intent with
those inputs, `host_only`, and a stable 32-hex phase nonce. No warm-up build or
push fallback exists.

The pinned contract is `POST
/repos/kasselvania/Linux-VST-bridge/actions/workflows/wf0-windows-msvc-build.yml/dispatches`,
`Accept: application/vnd.github+json`, API version `2026-03-10`. The request
uses the frozen branch ref and closed inputs for source SHA, build identity,
host mode, and nonce. Accept only HTTP 200 with positive-integer
`workflow_run_id` and absolute `run_url`/`html_url`; that run ID is
authoritative. Branch eligibility is distinct from the exact checkout SHA,
which the job verifies before acquisition or compilation.

The run name/receipt carry the nonce. Readback requires repository, workflow
path/blob, event, ref, source SHA, nonce, run ID/attempt, build identity, host
mode, and runner/image/toolchain. The producer builds only
`wf0-factory-probe` in two clean roots, proves A/B and PE/import/export/
dependency closure, and uploads:
`dx0-host-payload.zip`, `DX0_WINDOWS_HOST_BUILD_RECEIPT.json`, and its SHA-256
sidecar. It invokes neither AGain nor any fault target. The receipt schema is
`linux-vst-bridge-dx0-windows-host-build/v1`; payload manifest schema is
`linux-vst-bridge-dx0-windows-host-artifact/v1`.

The one Actions artifact is named
`dx0-windows-host-<windows-build-input-sha256>-run-<run-id>-attempt-<run-attempt>`.
The Mac custody schema/file are `linux-vst-bridge-dx0-mac-host-custody/v1` and
`DX0_MAC_HOST_CUSTODY_RECEIPT.json`.
The payload contains only the scanner, build-identity core, host-artifact
manifest and sidecar, and the existing required SDK/license notices. It has no
AGain bundle, fault DLL, adapter, PDB, object, source, cache, or Git metadata.

The driver follows that run ID and reads its exact artifact list and ID object.
After a lost response, bounded exact workflow/ref/nonce/input reconciliation
must reattach to one running run, retrieve one completed valid run, or return
`DX0_HOST_ARTIFACT_BLOCKED`. Timeout, missing local completion, or a temporary
empty listing never proves non-dispatch or permits redispatch.

Mac custody retains the IDs and typed upload/REST/raw-wrapper digests under
`<HOME>/Library/Application Support/Linux VST Bridge/proof/host-artifacts/by-manifest/<host-artifact-manifest-sha256>/`.
Reuse requires one unique artifact joined to build identity/mode, run/attempt,
nonce, producer source, receipt, and typed upload/REST/wrapper digests. P may
differ from E/C. One post-freeze cache miss permits one producer; failure or
unknown outcome blocks without rerun or runner roulette.

Before freeze: Python compilation, deterministic identity/state/recovery,
manifest/schema/roster, renderer, static call-surface, fixture-manifest, and
dry-run tests. An optional one-root Windows compile check is non-acceptance:
host-only, with no upload/custody, fixture, Deck, or evidence.

## Closed plan and single-command transaction

The ordinary interface is:

```text
python3 tools/host-proof.py run --source <commit> --plan wa0-positive-regression-v1
```

`linux-vst-bridge-dx0-proof-plan/v1` has exactly these keys: `schema`,
`plan_id`, `accepted_fixture_id`, `host_mode`,
`deterministic_validation_set`, `live_deck_batch`, `expected_result`, and
`evidence_renderer`. Each value is a closed repository-owned identifier. The
plan permits one accepted fixture, one host mode, one ordered deterministic
set, zero or one live batch, one expected result, and one renderer. Paths,
shell, environment assignments, hooks, loops, and arbitrary commands are
invalid. DX0's live batch is one accepted positive WA0 regression; it adds no
fault exercise or VST3 call.

The driver first derives all identities and prints canonical `REUSE`,
`PRODUCE`, `TRANSFER`, `EXECUTE`, or `RENDER` dispositions. It then freezes a
clean committed source and exact remote ref; every later phase rechecks it.
GitHub auth stays on the Mac. SSH always uses `ForwardAgent=no`; the Deck has
zero GitHub operation. Derived run/artifact/bundle/ref/worktree/cache names are
never caller inputs. Source handoff uses schema
`linux-vst-bridge-dx0-source-handoff/v1`, bundle
`dx0-execution-source-<consumer-commit>.bundle`, and sole advertised/Deck ref
`refs/handoff/dx0-source/<consumer-commit>`. Its files are
`DX0_SOURCE_HANDOFF_RECEIPT.json` and
`DX0_SOURCE_HANDOFF_RECEIPT.sha256`.

Each phase receipt uses one closed outcome disposition: `prepared`,
`in_flight`, `recovered_in_flight`, `completed`, `reused`, `failed`, or
`outcome_unresolved`. Only a validated `completed` or `reused` output advances
the transaction. `outcome_unresolved` blocks the phase and never means that
the remote effect did not occur.

The closed state set has 10 values: `transaction_absent`, `planned`,
`source_frozen`, `fixture_verified`, `host_artifact_verified`,
`handoff_admitted`, `deck_batch_in_flight`, `transaction_result_retained`,
`evidence_rendered`, `transaction_complete`.

The 12 material operations are: `derive_identities`, `plan_external_work`,
`freeze_source`, `verify_fixture`, `reuse_or_produce_host`,
`custody_host_artifact`, `create_source_handoff`,
`transfer_and_admit_deck_inputs`, `execute_deck_batch`,
`retrieve_and_retain_result`, `render_and_validate_evidence`, and
`close_transaction`. Skipped operations remain explicit.

One bounded canonical state receipt is atomically written under
`<HOME>/Library/Application Support/Linux VST Bridge/proof/transactions/by-id/<operation-nonce>/`.
Its schema is `linux-vst-bridge-dx0-transaction-state/v1`.
The driver creates the operation nonce once at `planned`; resume uses it and
never silently creates a replacement transaction. Before each workflow
dispatch or Deck launch it creates one phase nonce once and atomically commits
a `prepared` intent containing the phase name, exact phase-input identity,
closed operation, and expected output. Only then may the remote effect start.
Each completed phase records exact inputs, outputs, hashes, and disposition.

The hash dependency is acyclic: canonical input manifests first; operation
and phase nonces plus prepared intents second; remotely produced outputs
third; completion receipts over earlier input/output hashes fourth; immutable
retained result and its external SHA-256 sidecar fifth; evidence members and
packet hash sixth; final transaction manifest and its external sidecar last.
The initial operation nonce names the directory and is never represented as
the final transaction digest. No manifest or receipt contains its own hash in
the bytes being hashed.

Source bundles and Deck worktrees remain exact-commit, detached, and clean.
Existing archive, SSH, Runtime/Proton, environment-marker, process, cleanup,
protected-state, publication, and sanitization primitives are composed, not
redesigned.

### Reusable private result contract

The immutable result uses schema
`linux-vst-bridge-dx0-transaction-result/v1`, is keyed by
`DeckExecutionInputIdentity` under `.../proof/results/by-execution-input/`.
Its exact top-level keys are `schema`, `operation_nonce`,
`artifact_producer_source`, `deck_execution_source`, `execution_input`,
`host_artifact`, `accepted_fixture`, `source_handoff`, `closed_plan`,
`original_observation`, `positive_result`, `call_facts`, `quiescence`,
`shutdown`, `cleanup`, `protected_state`, and `integrity`.

The two source objects are P and E. Each contains `identity_sha256`, `commit`,
`tree`, `parent`, exact `ref`, and `manifest_sha256`. `execution_input` binds
the Deck-input, plan, Runtime/Proton, handoff ref/worktree, host manifest, and
fixture identities. `host_artifact` binds build input, run/attempt, artifact
ID, manifest, build receipt, and Mac custody. `accepted_fixture` binds fixture,
bundle, AGain module, and both store receipts. `source_handoff` binds bundle,
receipt, ref, and worktree. `closed_plan` binds literal
`wa0-positive-regression-v1`, its digest, expected result, and one-live-
exercise ceiling.

`original_observation` binds the original 32-hex Deck run ID, phase nonce,
event-stream digest, completion disposition, and UTC interval.
`positive_result`, `call_facts`, `quiescence`, and `shutdown` retain the typed
WA0 query/release result, exact fixture/interface, no-method fact, bounded
paired call ledger, last in-flight operation, interface/object quiescence,
component/factory releases, `ExitDll`, `FreeLibrary`, and scanner completion.
`cleanup` retains descendant count, process-group empty, environment retired,
and stage absent. `protected_state` retains original pre/post digests, equality,
and completed comparison. `integrity` hashes the bounded member projections;
the whole-result hash exists only in its sidecar.

SHA-256/Git IDs are exactly 64/40 lowercase hex; the operation nonce is 32
lowercase hex; IDs/counts are nonnegative JSON integers (artifact/run IDs are
positive); booleans/null are native JSON. Unknown, missing, duplicate, or
wrong-typed fields invalidate the result. P and E may differ.

The cached file alone is never a cache hit. The current transaction adds an
atomic result-admission phase receipt containing exactly the retained-result
SHA-256, P, E, current evidence-consumer source C, current
`EvidenceRendererIdentity`, disposition `reused_original_observation`, and
predicate result. C uses the same typed source object and may differ from P
and E. This receipt plus the immutable result is the reusable-result closure;
it preserves all three identities without rewriting the original observation.

Admission is true only when all types/hashes and artifact/custody, fixture,
handoff, execution-input, and plan joins validate; the original observation
completed; query succeeded non-null; interface release returned 1; no audio-
processor method ran; interface and object quiescence hold; component release
returned 0; reverse factory release, `ExitDll`, `FreeLibrary`, and scanner
completion succeeded; no call remains in flight; descendants are zero;
process group is empty; environment retirement and stage absence hold; and the
original protected-state comparison completed equal. Missing, inconsistent,
truncated, failed, or overflowed facts reject reuse. Filename, existence,
digest, schema label, or unaudited `success=true` never suffices.

Evidence rendered by C says that C reused E's original observation produced
with P's host artifact. It never claims that P equals E or C, that the Deck
executed C, or that current Deck state was freshly inspected.

### Remote outcomes and single writer

Existing filesystem primitives enforce one writer per expensive phase. The
Mac host-producer lock is keyed by `WindowsBuildInputIdentity + host_mode`; the
Deck execution lock is keyed by `DeckExecutionInputIdentity + proof_plan`.
Exclusive creation binds inputs, phase/operation nonces, and intent. A second
driver may wait for that owner or consume its valid atomic publication, never
start duplicate work. No daemon or database is added.

The Deck persists intent before Proton. After execution it completes cleanup,
environment retirement, protected comparison, and bounded hashing, then
stages, reads back, and atomically publishes result plus sidecar before its
acknowledgement. Lost acknowledgement/transfer resumes retrieval, never Proton.

Recovery first reconciles persisted intent: observe an attributable live
operation, validate its completed publication, or retain its failure. A stale
lock plus valid publication is marker-bound retired after admission; without
an attributable owner or complete output it is unresolved and returns the
existing host/Deck blocker. Partial/conflicting output is preserved, never
adopted. Unknown state is not deleted, unrelated processes are not killed, and
replacement work is not launched.

## Invalidation, failure, and reuse

| Change | Complete | Build | Deck | Renderer | External result |
|---|---:|---:|---:|---:|---|
| `evidence.py` only | yes | no | no | yes | rerender; zero external work |
| Mac-only driver | yes | no | no | no | reuse unchanged inputs/results |
| Deck orchestration | yes | no | yes | as classified | reuse host/fixture; ≤1 handoff/batch |
| Windows/CMake host | yes | yes | yes via host | if shared | ≤1 frozen build and Deck batch; fixture reused |
| documentation | yes | no | no | no | no external work |
| accepted fixture mismatch | yes/no | no | blocked | no | no build or automatic seed |

Evidence failure rerenders an admitted result. Deck failure preserves verified
inputs but its receipt cannot enter the reusable cache. Handoff failure keeps
Mac outputs and reconciles remote state; retry requires proven remote absence.
Host failure keeps freeze/fixture but publishes no artifact. Later failure
does not invalidate earlier verified output. No hidden retry, rebuild, reseed,
or widened plan exists.

| Blocker owner | Reusable verified output |
|---|---|
| design/scope/return | No phase; stores untouched. |
| build identity | Source and fixture remain; no external start. |
| fixture | Source/plan/host remain; no repair build. |
| source freeze | Fixture/host/result caches remain; no external start. |
| host artifact | Freeze/fixture remain; no failed publication. |
| handoff | Mac outputs remain; reconcile remote attempt before any retry. |
| Deck transaction | Inputs remain; retrieve atomic completion or block without launch. |
| evidence | Admitted result and earlier phases remain; rerender. |

The ten outcomes are: `DX0_DESIGN_PREFLIGHT_BLOCKED`,
`DX0_DESIGN_SCOPE_BLOCKED`, `DX0_BUILD_INPUT_IDENTITY_BLOCKED`,
`DX0_ACCEPTED_FIXTURE_BLOCKED`, `DX0_HOST_ARTIFACT_BLOCKED`,
`DX0_SOURCE_FREEZE_BLOCKED`, `DX0_HANDOFF_BLOCKED`,
`DX0_DECK_TRANSACTION_BLOCKED`, `DX0_EVIDENCE_BLOCKED`, and
`RETURN_TO_DESIGN_GATE`.

## Exact future path envelope

The implementation source/configuration roster is exactly 10 paths:

```text
.github/workflows/wf0-windows-msvc-build.yml
tools/host-proof.py
tools/wf0-factory-census/README.md
tools/wf0-factory-census/artifacts.py
tools/wf0-factory-census/build.py
tools/wf0-factory-census/common.py
tools/wf0-factory-census/environment.py
tools/wf0-factory-census/evidence.py
tools/wf0-factory-census/negative_tests.py
tools/wf0-factory-census/run.py
```

The workflow is host-only/driver-dispatched; `host-proof.py` is the Mac owner;
README defines its contract; `common.py` freezes schemas/rosters; `build.py`
separates host/fixture; `artifacts.py` composes custody/handoff/caches;
`environment.py` joins inputs; `run.py` emits the private result; `evidence.py`
renders locally; `negative_tests.py` owns deterministic identity, cost, and
recovery fixtures. Existing verification, normalization, supervision, CMake,
C++, fault, environment, Runtime, and protected-state owners stay unchanged.

The evidence roster is exactly five paths:

```text
evidence/dx0-split-build-identity-proof-transaction/BASIS.md
evidence/dx0-split-build-identity-proof-transaction/COST_AND_INVALIDATION.json
evidence/dx0-split-build-identity-proof-transaction/FINDINGS.md
evidence/dx0-split-build-identity-proof-transaction/TRANSACTION.json
evidence/dx0-split-build-identity-proof-transaction/hashes.sha256
```

The packet schema is `linux-vst-bridge-dx0-evidence-packet/v1`;
`TRANSACTION.json` and `COST_AND_INVALIDATION.json` use
`linux-vst-bridge-dx0-retained-transaction/v1` and
`linux-vst-bridge-dx0-cost-and-invalidation/v1`.

Evidence is canonical UTF-8, bounded, NUL-free, hash-closed, and allow-listed.
It retains public Git/Actions identities, content digests, phase dispositions,
counts, and sanitized proof facts; never credentials, usernames/home paths,
host/network IDs, PIDs, pointers, binaries, raw logs, private stages,
compatdata, or proprietary state.

## Focused proof and cost acceptance

The matrix has 14 rows:

| # | Required proof |
|---:|---|
| 1 | Exact authority, 10-record complete-source roster, and closed path classification. |
| 2 | Historical `4a5ff302...` and `24b7e6da...` differ completely but reproduce one Windows build-input identity. |
| 3 | Evidence-only C mutation admits exact prior P/E observation and locally rerenders: zero build, custody, transfer, or Deck work. |
| 4 | Mac-driver-only mutation reuses host, fixture, and unchanged execution result. |
| 5 | Deck-orchestration mutation changes only execution identity and permits at most one live batch. |
| 6 | Host/CMake mutation changes build identity, rejects cached host, and permits one build after freeze. |
| 7 | Exact WA0 AGain is seeded/admitted by manifest; AGain target is never invoked. |
| 8 | Valid distinct P/E/C reuse preserves artifact, fixture, handoff, input, observation, and custody joins and reports original-observation reuse. |
| 9 | Planning/eligibility precede work; dirty/changed source blocks; pinned dispatch supplies run ID; no warm-up, push fallback, or runner roulette. |
| 10 | Closed-plan validator rejects paths, shell, environment, hooks, unknown values, and extra keys. |
| 11 | One Mac command discovers IDs, uses no Deck GitHub, performs one positive live batch, cleans up, and returns evidence. |
| 12 | Injected lost dispatch ack, lost completed-Deck ack, and duplicate driver recover exact intent with no duplicate expensive operation. |
| 13 | Accepted supervision reaches zero descendants/environment residue and exact protected-state equality. |
| 14 | Typed result admission rejects incomplete/failed facts; valid P/E/C rerenders five files, validates six domains, and performs zero external work. |

Deterministic rows use temporary repositories, manifests, stores, and injected
failures; they never launch Proton. The sole live row is the accepted positive
WA0 regression.

The ledger counts operator commands and re-entered values, not subprocesses.
WA0 is a static call-graph baseline; DX0 columns are intended steady-state
acceptance results, not measured savings:

| Metric | Static WA0 baseline | Intended DX0 cached/rerender | Intended DX0 cold changed host |
|---|---:|---:|---:|
| manual commands | 14 | 1 | 1 |
| manually copied identifiers | 9 | 0 | 0 |
| Windows acceptance builds | 1 per source, plus observed retries | 0 | 1 maximum |
| artifact downloads | 1 | 0 | 1 maximum |
| artifact transfers | 1 | 0 | 1 only if Deck cache misses |
| source transfers | 1 | 0 | 1 only if live execution is required |
| live Deck proof batches | 1 | 0 | 1 maximum |
| live exercises | 9 | 0 | 1 |
| evidence renders | 1 | 1 local | 1 local |
| verified phases repeated after late evidence failure | up to 9 | 0 | 0 |

Evidence-only change performs zero external work; Deck-code change permits no
build and at most one transfer/batch; host change permits at most one build,
custody/download, required transfer, and batch. Late evidence failure repeats
no earlier phase. One-time `seed-fixture` is reported separately. No duration,
token, or monetary saving is claimed before measurement.

Future implementation requires one source commit directly above the merged
DX0 design-authority/readback basis and one evidence-only commit. It must pass
a fresh independent pre-PR audit and exact-head technical-lead review; the
implementation agent may not approve or merge it.

## Material stop law

More than three owners, 12 source/configuration paths, five evidence files,
14 proof rows, one acceptance build, or one Deck batch; any new C++ product
path, VST3 call, fault module, runner/build plane, remote artifact service,
database/daemon, generic workflow language, broad `wf0-*` rename, or product
capability returns `DX0_DESIGN_SCOPE_BLOCKED`. Evidence that the primary claim
requires changed ownership or architecture returns `RETURN_TO_DESIGN_GATE`.
