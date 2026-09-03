# DX0 implementation design v1

```yaml
slice: DX0
design_revision: dx0-design-v1
design_status: proposed_for_adversarial_review
implementation_authorized: false
```

## Authority, claim, and ceiling

This card refines only [the DX0 selection](SLICE_SELECTION.md). The exact
selection basis is commit `859422be75f65da4dd7dc51394b84470ad594358`, tree
`7b9a721f3e691974dd1720c20d9ef030932978a9`. The unmodified activation is
commit `6631c502f11a8c0a2552fe333f1b99ea64c35b30`, tree
`be7a4766702a462f94451dcf5fa3f24c86c8e4de`, with that basis as its sole
parent. D-026 is the governing accepted decision.

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

The current producer embeds one 17-path source digest into build core,
artifact manifest, Mac custody, Deck cache, source handoff, execution, and
evidence. `run.py` additionally renders evidence on the Deck. Thus a change
only to `evidence.py` between source `4a5ff302acc4927142003e29bd401368920b275b`
and final source invalidated every later phase although checked build inputs
were byte-identical. The producer also builds AGain twice. Runner image
`20260830.290.1` produced AGain SHA-256 `9c65a11bd67fdfc2416cc509b363c7a9ad83998cc1db5fa3cf2640e2c7431da8`;
image `20260824.284.2` produced the accepted
`60aa9ff6b9918d4330449e7b3ab34b588dd93cba09f37413a3cd91f6e7d2e18f`.

The minimum accepted-WA0 procedural call graph requires 14 operator
operations: source push; run lookup; upload-result lookup; Mac custody; source
bundle creation; source transfer; Deck source admission; artifact transfer;
Deck artifact admission; Deck batch; evidence transfer; Mac handoff validation;
packet placement; final identity closure. It propagates nine values by hand
(source commit, run ID/attempt, artifact ID/URL/digest, source-manifest digest,
artifact-manifest digest, and evidence destination). WA0's batch contains nine
live exercises. DX0 reduces the ordinary boundary to one command, zero copied
identifiers, and one live regression exercise.

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

Every repository manifest record is exactly `path`, `git_mode`, `git_blob`.
Records are unique and sorted by raw UTF-8 path bytes. Manifests are canonical
JSON: UTF-8, sorted keys, compact separators, ASCII escapes, and one trailing
LF. An identity is lowercase `SHA-256(canonical_manifest_bytes)`. Missing,
non-blob, duplicate, unclassified, or dirty input blocks before external work.
Membership is a closed bitset; a path may affect several domains.

| Domain and schema | Exact contents |
|---|---|
| `CompleteImplementationSourceIdentity`; `linux-vst-bridge-dx0-complete-source/v1` | Consumer commit/tree/parent/ref plus the exact 10-record implementation roster below. Any consumer commit change changes this identity even when a narrower identity is stable. |
| `WindowsBuildInputIdentity`; `linux-vst-bridge-dx0-windows-build-input/v1` | The 17 repository records below, SDK root tree and seven submodule commits, no-patch state, pinned checkout/source locks, generator/platform/toolset/SDK/configuration/static-runtime/options, pinned action SHAs, and producer contract. |
| `AcceptedFixtureIdentity`; `linux-vst-bridge-dx0-accepted-fixture/v1` | Exact 14-record bundle manifest, module hash, accepted WA0 artifact/custody coordinates, and store receipt. |
| `DeckExecutionInputIdentity`; `linux-vst-bridge-dx0-deck-execution-input/v1` | Host-artifact manifest, accepted-fixture identity, closed-plan identity, Runtime/Proton digest, and exact records for `artifacts.py`, `common.py`, `environment.py`, `normalize.py`, `run.py`, `supervise.py`, and `tools/wr0-proton-bootstrap/launch.py`. |
| `EvidenceRendererIdentity`; `linux-vst-bridge-dx0-evidence-renderer/v1` | Exact records for `evidence.py` and `common.py`, evidence schema/roster, and validator contract. |
| `ProofTransactionIdentity`; `linux-vst-bridge-dx0-proof-transaction/v1` | Producer and consumer complete-source identities, all five narrower identities, host artifact, fixture, plan, retained result, renderer, and phase-receipt hashes. |

The exact top-level key rosters are respectively: complete source — `schema`,
`commit`, `tree`, `parent`, `ref`, `record_count`, `records`; build input —
`schema`, `repository`, `record_count`, `records`, `sdk`, `build_contract`,
`action_commits`; fixture — `schema`, `fixture_id`, `bundle_manifest`,
`accepted_wa0_custody`; Deck input — `schema`,
`host_artifact_manifest_sha256`, `accepted_fixture_identity_sha256`,
`proof_plan_sha256`, `runtime_proton_sha256`, `record_count`, `records`;
renderer — `schema`, `record_count`, `records`, `evidence_schema`,
`evidence_paths`; transaction — `schema`, `producer_complete_source`,
`consumer_complete_source`, `windows_build_input`, `host_artifact`,
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
after reproducing the same identity; both producer and consumer remain in the
transaction. Ambiguous cache matches block rather than select latest.

Path classification for the 10-path DX0 implementation is exact:

- workflow and `build.py`: complete + Windows build;
- `common.py`: complete + Windows build + Deck execution + renderer;
- `artifacts.py` and `environment.py`: complete + Deck execution;
- `run.py`: complete + Deck execution;
- `evidence.py`: complete + evidence renderer;
- `negative_tests.py` and `tools/host-proof.py`: complete;
- `README.md`: complete only.

Unchanged transitive records above and the Deck/renderer rosters participate
in their named identities. `AcceptedFixtureIdentity` depends only on accepted
bytes and provenance, never a code blob. `ProofTransactionIdentity` changes
through its complete-source join whenever any implementation record changes.
Classification uses these exact rosters, not file extensions, prefixes, or
diff heuristics.

## Accepted fixture store

AGain is keyed by accepted bundle-manifest SHA-256
`bfaa1dce4d2e189f89cee41493838824efe647e361e81436676b7b3a86ff5164`.
Its 14 records include module SHA-256
`60aa9ff6b9918d4330449e7b3ab34b588dd93cba09f37413a3cd91f6e7d2e18f`.
The receipt binds WA0 artifact `9869994854`, artifact manifest
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
Both use a new empty stage, archive/path bounds, complete hash verification,
`DX0_ACCEPTED_FIXTURE_RECEIPT.json` plus
`DX0_ACCEPTED_FIXTURE_RECEIPT.sha256`, atomic publish, and read-only content.
Normal `run` only verifies/reuses these roots.

One explicit `seed-fixture` operation may extract only the accepted bundle
from already retained WA0 custody bytes or an authenticated exact-ID Mac
download, verify all accepted joins, publish the durable Mac copy, then admit
the verified Deck copy over ordinary SSH. It is operator-authorized maintenance,
not part of `run`. No binary enters Git; an expiring Actions object is never
the sole durable owner. Missing, corrupt, unsafe, or unavailable accepted bytes
return `DX0_ACCEPTED_FIXTURE_BLOCKED`; AGain is never rebuilt as repair.

## Host artifact and two validation lanes

The Windows producer becomes explicit `workflow_dispatch`, correlated by a
driver-generated 32-lowercase-hex transaction nonce in the run display name
and receipt. It does not run on push. Before compilation it reads back the
requested source/build-input identity and actual runner/image/toolchain. It
builds only `wf0-factory-probe` in the inherited two clean roots, verifies A/B
bytes and PE/import/export/dependency closure, and uploads a host-only envelope:
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

The nonce-bearing dispatch name lets the driver poll one exact workflow/run,
then read its one exact artifact list and exact artifact-ID object itself.
Mac custody retains those IDs and typed upload/REST/raw-wrapper digests under
`<HOME>/Library/Application Support/Linux VST Bridge/proof/host-artifacts/by-manifest/<host-artifact-manifest-sha256>/`.
Reuse requires one unique verified artifact for the exact build-input identity
and host mode.
The producer commit remains truthful even when the consumer differs. A cache
miss permits one acceptance producer after freeze; failure blocks without an
automatic rerun. There is no runner roulette.

Cheap development validation precedes freeze: Python compilation,
deterministic identity/state/resume tests, manifest/schema/roster checks,
evidence-renderer tests, static call-surface checks, fixture-manifest checks,
and dry-run planning. An optional one-root Windows compile check is explicitly
non-acceptance: host-only, no upload/custody, fixture, Deck, or evidence.

## Closed plan and single-command transaction

The ordinary interface is:

```text
host-proof run --source <commit> --plan wa0-positive-regression-v1
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
`<HOME>/Library/Application Support/Linux VST Bridge/proof/transactions/by-id/<transaction-id>/`.
Its schema is `linux-vst-bridge-dx0-transaction-state/v1`.
Each completed phase records inputs, outputs, hashes, and disposition. Failure
leaves the last verified immutable phase reusable; partial stages are never
adopted. Source bundles and Deck worktrees remain exact-commit, detached, and
clean. Existing archive, SSH, Runtime/Proton, environment-marker, process,
cleanup, protected-state, and sanitization algorithms are composed, not
redesigned.

The private bounded result schema is
`linux-vst-bridge-dx0-transaction-result/v1`, keyed by
`DeckExecutionInputIdentity` under `.../proof/results/by-execution-input/`.
It retains canonical normalized result, call/cleanup/protected-state facts,
input identities, and hashes—not unrestricted logs, paths, PIDs, credentials,
or binaries. It is sufficient to rerender and revalidate evidence without
custody, transfer, or Deck execution.

## Invalidation, failure, and reuse

| Change | Complete | Build | Deck | Renderer | External result |
|---|---:|---:|---:|---:|---|
| `evidence.py` only | yes | no | no | yes | 0 builds/custody/transfers/Deck; rerender retained result |
| Mac-only `tools/host-proof.py` | yes | no | no | no | reuse fixture, host, and result unless another identity changed |
| Deck orchestration | yes | no | yes | as classified | reuse host/fixture; at most one handoff and Deck batch |
| Windows/CMake host input | yes | yes | yes via new host | one if shared | one build after freeze; fixture reused; at most one Deck batch |
| documentation | yes | no | no | no | no external work |
| accepted fixture mismatch | yes/no | no | blocked | no | no build or automatic seed |

An evidence failure reuses the retained result and rerenders only. A Deck
failure reuses verified fixture, host custody, source bundle, and completed
admission; no failed/partial result is cached. A handoff failure reuses all
prior immutable Mac outputs and repeats only an unproved transfer/admission.
A host failure preserves source freeze and fixture but publishes no accepted
host artifact. A later failure never invalidates an earlier phase merely by
position. No hidden retry, rebuild, reseed, or widened plan is permitted.

| Blocker owner | Reusable verified output |
|---|---|
| design preflight/scope or `RETURN_TO_DESIGN_GATE` | No new phase; every pre-existing content-addressed store remains untouched. |
| build-input identity | Complete source and any independent accepted fixture remain; no external phase starts. |
| accepted fixture | Source/plan and any verified host cache remain; no build repairs the fixture. |
| source freeze | Existing fixture/host/result caches remain; no external phase starts. |
| host artifact | Source freeze and fixture remain; a failed producer publishes no accepted host entry. |
| handoff | Source bundle, fixture, host artifact, and Mac custody remain; only the exact unproved handoff/admission repeats. |
| Deck transaction | All verified Mac and Deck inputs remain; an incomplete live result is not reusable. |
| evidence | The verified retained result and every earlier phase remain; rerender only. |

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

The workflow becomes driver-dispatched and host-only. `host-proof.py` is the
single Mac owner; the README states its closed contract. `common.py` freezes
schemas/rosters. `build.py` separates host production from fixture input.
`artifacts.py` owns existing custody/handoff mechanics plus the two private
caches. `environment.py` joins verified host and fixture inputs into the
unchanged disposable layout. `run.py` emits one bounded private result instead
of rendering on Deck. `evidence.py` renders/validates locally from that result.
`negative_tests.py` gains only deterministic invalidation, cost, and resume
fixtures. Existing `verify.py`, `normalize.py`, `supervise.py`, CMake, C++,
fault modules, environment shape, Runtime route, and protected owners remain
unchanged.

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
| 3 | Evidence-only mutation changes complete/renderer identities and performs only one rerender. |
| 4 | Mac-driver-only mutation reuses host, fixture, and unchanged execution result. |
| 5 | Deck-orchestration mutation changes only execution identity and permits at most one live batch. |
| 6 | Host/CMake mutation changes build identity, rejects cached host, and permits one build after freeze. |
| 7 | Exact WA0 AGain is seeded/admitted by manifest; AGain target is never invoked. |
| 8 | Host reuse preserves distinct producer/consumer identities and exact custody. |
| 9 | Dry-run plan precedes work; dirty or changed frozen source blocks; no runner roulette. |
| 10 | Closed-plan validator rejects paths, shell, environment, hooks, unknown values, and extra keys. |
| 11 | One Mac command discovers IDs, uses no Deck GitHub, performs one positive live batch, cleans up, and returns evidence. |
| 12 | Injected late-phase failure resumes from verified phases without repeated build, seed, custody, or valid transfer. |
| 13 | Accepted supervision reaches zero descendants/environment residue and exact protected-state equality. |
| 14 | Private result rerenders five-file evidence; cost ledger and all six identity joins validate. |

Deterministic rows use temporary repositories, manifests, stores, and injected
failures; they never launch Proton. The sole live row is the accepted positive
WA0 regression.

The cost ledger uses operator-issued phase commands and values copied or
re-entered between them; automatic subprocesses do not count:

| Metric | Current WA0 minimum | DX0 cached/rerender | DX0 cold changed host |
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

Evidence-only change costs 0 builds, custody operations, downloads, transfers,
or Deck batches. Deck-code change costs 0 builds and at most one source/input
transfer and one Deck batch. Host-input change costs at most one build, one
custody/download, one required transfer, and one Deck batch. A late evidence
failure repeats zero earlier phases.

Future implementation requires one source commit directly above the merged
DX0 design-authority/readback basis and one evidence-only commit. It must pass
a fresh independent pre-PR audit and exact-head technical-lead review; the
implementation agent may not approve or merge it.

## Material stop law

More than three owners, 12 source/configuration paths, five evidence files,
16 proof rows, one acceptance build, or one Deck batch; any new C++ product
path, VST3 call, fault module, runner/build plane, remote artifact service,
database/daemon, generic workflow language, broad `wf0-*` rename, or product
capability returns `DX0_DESIGN_SCOPE_BLOCKED`. Evidence that the primary claim
requires changed ownership or architecture returns `RETURN_TO_DESIGN_GATE`.
