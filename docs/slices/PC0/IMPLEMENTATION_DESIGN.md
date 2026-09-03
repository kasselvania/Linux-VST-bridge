# PC0 design revision v3 — Durable Failed-Batch Diagnostic and One Corrective Deck Batch

```yaml
slice: PC0
design_revision: pc0-design-v3
design_status: proposed_for_adversarial_review
implementation_authorized: false
product_claim_changed: false
design_repair_basis_commit: 1c0c31c4ab69a40303cd00b155ca30626323c451
design_repair_basis_tree: 192d2af4b5d83d94264510eab7c7729b5de1a9b5
v2_design_blob: ec0683fc66028239d7481640ba72e2dd9a060a2c
v2_design_sha256: 20e653a6b1fad720ea5fc888a8d531bda44c338840f5eb96e488612d197de992
v2_review: 5105712590 / PC0_DESIGN_V2_CLEAR
runtime_discovery: PR 37 comment 5533164226 / PC0_RUNTIME_DISCOVERY
owner_count: 1
stable_state_count: 9
unique_call_operation_count: 4
positive_call_count: 11
proof_row_count: 16
blocked_outcome_count: 10
source_configuration_path_count: 14
evidence_path_count: 5
additional_windows_builds: 0
additional_workflow_dispatches: 0
additional_positive_deck_batches: 1 maximum after later explicit authority
live_negative_exercises: 0
```

V3 is a bounded amendment to immutable pc0-design-v2. The V2 card is Git blob
`ec0683fc66028239d7481640ba72e2dd9a060a2c` with raw SHA-256
`20e653a6b1fad720ea5fc888a8d531bda44c338840f5eb96e488612d197de992`.
All V2 product, owner, lifecycle, normalization, call-order, shutdown, privacy,
and nonclaim laws remain binding unless this card explicitly changes a
proof-transaction failure-evidence or corrective-budget rule. An implementation
preflight must read and hash both cards. This proposed card does not authorize
implementation or live execution.

## 1. Authority, unchanged claim, and sole repair purpose

The design-repair basis is current remote `main` commit
`1c0c31c4ab69a40303cd00b155ca30626323c451`, tree
`192d2af4b5d83d94264510eab7c7729b5de1a9b5`. It contains the approved V2
authority. Technical-lead review `5105712590` returned
`PC0_DESIGN_V2_CLEAR` for exact V2 head
`996ee557d33ea55d6acf7ef2242f703c2c63f262`, tree
`bb1fe157bef42854812ebeb0b73b1a5332c93cf6`.

Merged PR #37 comment `5533164226` is the runtime-discovery record. Failed
implementation source is
`7ac6095a488d0077fcc78fedc5abd870b5ffb1cb`, tree
`ad4230a713a2bb644476be8be9d374a1feb36b06`, on
`refs/heads/codex/pc0-windows-vst3-pre-setup-processing-contract`.

The product claim remains exactly:

> On the exact accepted AGain lifecycle, while the processor remains in the
> Initialized state, the supervised Windows host performs one bounded read-only
> census of all audio/event buses, all existing `BusInfo` records, current audio
> speaker arrangements, and `kSample32`/`kSample64` support, retains one exact
> normalized pre-setup processing contract, and completes the accepted
> interface/component/factory/module shutdown without mutating processing state.

Source inspection established one material failure-evidence gap:

1. `supervise.py` returns `raw_exit=99` with
   `classification=output_publication_failed` for scanner output/publication
   failure.
2. A Python supervision or output-stream exception returns
   `classification=supervision_failed` and retains the observed `raw_exit`,
   including null when no exit is observable.
3. `run.py` currently discards that structured run before raising the blocker.
4. The Mac therefore retained only `PC0_EVIDENCE_BLOCKED` for the spent batch.

V3 changes only durable failed-run publication, strict Mac admission, historical
cost accounting, exact V3 source authority, and one corrective reservation.
It adds no product behavior.

## 2. Read-only failed-transaction reconnaissance

The supplied local files were read without modification. Both are canonical
JSON without duplicate keys.

| Input | Raw SHA-256 | Bytes |
|---|---|---:|
| `DX0_TRANSACTION_STATE.json` | `06473755eb7ccfa2529522bb29e3fb44a5a4a67ea38fd0e797d198939e1ed966` | 6589 |
| `PC0_OPERATOR_RECOVERY.json` | `870039310c0f6ee4d0bf4044d629e6f6e7d92d2f901b7b40a60f9724bb84f9e5` | 1256 |

The exact admitted historical facts are:

- transaction/operation nonce
  `a14bcc65d15e9fbdf15810a658b2ee87`;
- source/ref/tree
  `7ac6095a488d0077fcc78fedc5abd870b5ffb1cb` /
  `refs/heads/codex/pc0-windows-vst3-pre-setup-processing-contract` /
  `ad4230a713a2bb644476be8be9d374a1feb36b06`;
- source parent `1c0c31c4ab69a40303cd00b155ca30626323c451`;
- proof-plan digest
  `501829c4bf88988afb13ad984d5220839b73315d1ba89c8ca2e77600e58dc248`;
- failed DeckExecutionInputIdentity
  `ae89ee61636074feac5c875bab9b8a9621e3a0c6f7fa83b6d33bc11b94e631a3`;
- one recorded ordinary driver invocation;
- one separately recorded operator-authorized recovery continuation, with
  disposition `continuation_stopped`;
- execute phase disposition `failed`, blocker `PC0_EVIDENCE_BLOCKED`, and
  monotonic transaction state `deck_batch_in_flight`;
- effects `windows_builds=1`, `artifact_downloads=1`,
  `custody_operations=1`, `artifact_transfers=1`,
  `source_transfers=1`, `deck_executions=1`, `evidence_renders=0`;
- no local result directory, success JSON, or result sidecar for the failed
  DeckExecutionInputIdentity at reconnaissance time.

Recovery authority provenance is the exact receipt hash and its recorded
`operator_authorized_recovery_continuation_count=1`, corroborated by runtime
discovery comment `5533164226`. The receipt contains no literal operator
approval text; this design does not claim otherwise.

The recovery receipt proves that, before its continuation, remote intent,
result, result sidecar, driver lock, source handoff, and source worktree were
absent. It does not prove their post-failure state. The journal proves a
preserved failed transaction state; neither supplied file identifies the
scanner/supervisor failure class or independently proves the current remote
inner/outer lock state. V3 must retain:

```text
historical_failure_classification: unresolved_v2_no_diagnostic
historical_remote_lock_disposition: requires_future_authorized_readback
```

No implementation may retroactively label the V2 failure
`output_publication_failed` or `supervision_failed`.

The existing local host-artifact store validates against:

```text
WindowsBuildInputIdentity:
575d3bd9183be1ec0fe0311cff48bc0107c4299a3355748c470e3d195d284849
workflow run / attempt: 33812659869 / 1
artifact: 9915439437
Actions artifact digest:
sha256:a78f8118f1a0b856ad6483375cd3a20aa3118047425a9ed53ad9424cfb986940
host artifact manifest:
d0e11c374b7b1cb99b357faaa109e9310edc265c159559bd59a2098148484e9c
host build receipt:
7de6884d12b3144ff725fe9bc2550c7884356cc4379ed86f6f68e1ed8c3b2419
Mac custody receipt:
e2a06cb0f5ae69570e070bf6654bf1715f1b26fbfeb646f669d4fefb55db4f84
```

GitHub readback reports the producer completed successfully at the failed
source and artifact `9915439437` remains unexpired. A future implementation
must revalidate the exact local and Deck stores; this design does not claim
future availability.

## 3. Preserved product envelope and nonclaims

V3 preserves one `PreSetupProcessingContractCensus` owner, its nine states,
four operation types, eleven positive calls, sixteen proof rows, ten blockers,
fourteen source/configuration paths, and five tracked evidence paths. The
blocked taxonomy remains exactly:

```text
PC0_DESIGN_PREFLIGHT_BLOCKED
PC0_DESIGN_SCOPE_BLOCKED
PC0_BUS_COUNT_BLOCKED
PC0_BUS_INFO_BLOCKED
PC0_BUS_ARRANGEMENT_BLOCKED
PC0_SAMPLE_FORMAT_BLOCKED
PC0_CONTRACT_INCOMPLETE
PC0_PROCESS_CLEANUP_BLOCKED
PC0_EVIDENCE_BLOCKED
RETURN_TO_DESIGN_GATE
```

The durable diagnostic is private proof-transaction data owned by the existing
Deck execution lock. It is not a second owner, state, VST3 operation, blocker,
proof row, tracked evidence path, or protocol family.

All V2 prohibitions remain: no `setIoMode`, `activateBus`, `setActive`,
`setBusArrangements`, `setupProcessing`, `setProcessing`, `process`,
`getLatencySamples`, `getTailSamples`, buffers, controller, parameters,
state, presets, proxy, C ABI, IPC, shared memory, broker, real-time work,
Bitwig, Serum, packaging, signing, runner selection, or general compatibility.

## 4. Exact implementation repair delta

The repaired source may differ from failed source `7ac6095a...` at exactly
these four paths:

| Path | Exact V3 responsibility |
|---|---|
| `tools/host-proof.py` | Admit V3 source authority, bind the exact historical receipts, apply the corrective predicate, prohibit every Windows/artifact fallback, retrieve and retain one failed diagnostic, and project truthful history. |
| `tools/wf0-factory-census/run.py` | Admit V3 source authority on the Deck and atomically publish one validated private failed-run diagnostic in the existing execution lock before raising. |
| `tools/wf0-factory-census/evidence.py` | Strictly validate the diagnostic/custody projection and render the revised cost schema with explicit corrective history after success. |
| `tools/wf0-factory-census/negative_tests.py` | Prove both failure classes, diagnostic rejection, exact corrective admission, no third batch, producer prohibition, identity equality, and historical accounting without external work. |

`supervise.py` is sufficient and must remain byte-identical at Git blob
`7bcd5f0ad93b0acf919af2ce9c17081d9ef1addb`. It already returns every fact
needed to distinguish scanner exit 99 from a supervision failure. If
implementation establishes that its returned run is insufficient, stop with
`RETURN_TO_DESIGN_GATE`; V3 does not authorize changing it.

The ten unchanged members of the fourteen-path source envelope, including
`supervise.py`, must remain byte-identical to the failed source:

```text
.github/workflows/wf0-windows-msvc-build.yml|100644|c4bbcb03d2bc9b464433bdb37931fb3d5a0f169c
tools/wf0-factory-census/artifacts.py|100644|78f03c49cf313ac761578fed9287678d40178ec9
tools/wf0-factory-census/build.py|100644|b81de04c2fd7dd6af29f27b7ef736ab9f76b075f
tools/wf0-factory-census/common.py|100644|dc5e2e2309733bd77a4f528e43940bdc571ce67d
tools/wf0-factory-census/normalize.py|100644|6efeeb4e4b568d623358841e1b4bba34eb8c9464
tools/wf0-factory-census/supervise.py|100644|7bcd5f0ad93b0acf919af2ce9c17081d9ef1addb
tools/wf0-factory-census/verify.py|100644|589ef7594303ebd5e8741d5303c56d1ee17348e8
windows-factory-probe/source/component_instance_session.cpp|100644|30ddecfdbc245990211acaea7d8326e35b45713e
windows-factory-probe/source/component_instance_session.h|100644|3f15fa52792ca75e4238c8f707e2154424a4f910
windows-factory-probe/source/main.cpp|100644|c37b4ca23787de515b9c256ea2215ae7f10d2c1c
```

The four repaired paths are outside the Windows-build roster. Every record in
that roster must equal the failed producer exactly:

```text
.github/workflows/wf0-windows-msvc-build.yml|100644|c4bbcb03d2bc9b464433bdb37931fb3d5a0f169c
CMakeLists.txt|100644|b6573f22f2931f24f0453c67accd514430cd525f
cmake/WF0DependencyLock.cmake|100644|3312c93653621dd8ac7a1a9027f57614666cdbdd
tools/wf0-factory-census/build.py|100644|b81de04c2fd7dd6af29f27b7ef736ab9f76b075f
tools/wf0-factory-census/common.py|100644|dc5e2e2309733bd77a4f528e43940bdc571ce67d
tools/wf0-factory-census/verify.py|100644|589ef7594303ebd5e8741d5303c56d1ee17348e8
windows-factory-probe/CMakeLists.txt|100644|c07fdab2fe5a2814cbbcd9847619e651f6383510
windows-factory-probe/include/linux_vst_bridge/wf0_probe/census.h|100644|fa7bc69b9408847617da68b4b32e0d136ee8084d
windows-factory-probe/include/linux_vst_bridge/wf0_probe/events.h|100644|ba19cf5f378df32a3b31b3a7e97503f8cfdecfe6
windows-factory-probe/source/component_instance_session.cpp|100644|30ddecfdbc245990211acaea7d8326e35b45713e
windows-factory-probe/source/component_instance_session.h|100644|3f15fa52792ca75e4238c8f707e2154424a4f910
windows-factory-probe/source/factory_census.cpp|100644|bf5a14cf1941a48f3e3aea513fe9901ae97ce57b
windows-factory-probe/source/factory_census.h|100644|a5757268ed6c91cd60d51dd4202ecd5d5e21e36d
windows-factory-probe/source/main.cpp|100644|c37b4ca23787de515b9c256ea2215ae7f10d2c1c
windows-factory-probe/source/win32_module.cpp|100644|9095817720550ff858b10cb233341d51b022399a
windows-factory-probe/source/win32_module.h|100644|0ae6eaaa54f68b8399f22c5b2cc99035a87f8bfd
windows-fixtures/wf0/CMakeLists.txt|100644|9f2c019e658f3789d3a698b67ffbb61aa51db99a
```

The identity algorithm and every non-record field remain unchanged. This exact
roster reproduces
`575d3bd9183be1ec0fe0311cff48bc0107c4299a3355748c470e3d195d284849`.
Any differing record or identity is `RETURN_TO_DESIGN_GATE`. The implementation
must not dispatch a producer to repair a mismatch.

`common.py` retains the V2 basis constants because it is a Windows-build input.
The V3 source parent is therefore admitted by three narrow, PC0-only validators
in `host-proof.py`, `run.py`, and `evidence.py`. Each must require the exact
future V3 authority merge commit/tree, exact fourteen-path direct-child
topology, V3 design/review/approval identities, and the existing PC0 ref.
Deterministic parity tests must prove that all three derive the same
`linux-vst-bridge-pc0-complete-source/v1` value. They may reuse the existing
canonical source-role hashing function after the strict V3 preflight. They
must not weaken or replace the inherited DX0/WA0 validator.

Source inspection also found V2-only admission inside
`artifacts.create_source_handoff`, `artifacts._source_from_bundle`, and
`artifacts.verify_source_handoff`. The V3 PC0 call sites in `host-proof.py` and
`run.py` must use narrow V3 equivalents of those existing responsibilities.
They retain the exact `linux-vst-bridge-dx0-source-handoff/v1` key roster,
bundle naming, one advertised ref, zero prerequisites, 128 MiB cap, canonical
receipt, sidecar, and self-contained bare-repository readback. Only the exact
admitted V3 authority/basis tuple changes. The tuple has the existing keys
`reviewed_design_commit`, `design_blob`, `design_sha256`,
`technical_lead_review`, and `approval_blob`. Old producer P and old custody
continue through the unchanged V2 artifact validator.

This routing is explicit; runtime monkey-patching of `common.py` or
`artifacts.py` is forbidden. No new repository-local import is added to
`run.py` or `evidence.py`, so the seven-record Deck closure and two-record
renderer closure remain complete. V3-specific proof wording and cost-schema
selection live in the already permitted driver/renderer paths; the historical
V2 literals in `common.py` are not relabelled as V3 authority.

## 5. Durable private failure diagnostic

### Location and schema

For a corrective execution input `D` and proof-plan digest `Q`, the existing
Deck-side inner execution lock is:

```text
<Deck result parent>/.locks/<D>-<Q>/
```

A failed PC0 run publishes exactly these two direct children:

```text
PC0_FAILURE_DIAGNOSTIC.json
PC0_FAILURE_DIAGNOSTIC.json.sha256
```

The JSON schema is
`linux-vst-bridge-pc0-failure-diagnostic/v1`. Its exact top-level key roster is:

```text
schema
operation_nonce
phase_nonce
execution_source
execution_input_sha256
proof_plan_sha256
run_id
raw_exit
classification
primary_blocker
secondary_cleanup_blocker
last_lifecycle
last_in_flight_operation
durable_record_count
durable_records
call_counts
audio_processor_observer_state
audio_interface_quiescence
inherited_shutdown
cleanup
environment_retirement_disposition
stdout_sha256
stderr_sha256
stderr_bytes
protected_snapshot_sha256
runner_identity_sha256
```

There is no in-object digest. The sidecar is exactly:

```text
<lowercase SHA-256><two spaces>PC0_FAILURE_DIAGNOSTIC.json<LF>
```

### Exact value laws

- `operation_nonce` and `phase_nonce` are lowercase 32-hex and must equal the
  prepared intent.
- `execution_source` uses the existing six-key source-role roster:
  `identity_sha256`, `commit`, `tree`, `parent`, `ref`,
  `manifest_sha256`.
- The execution-input and proof-plan digests must equal the prepared intent.
- `run_id` is lowercase 32-hex.
- `raw_exit` is null when unobservable or an exact signed integer in
  `[-255,255]`.
- `classification` is one of
  `output_publication_failed`, `supervision_failed`,
  `supervision_and_process_cleanup_failed`, `process_cleanup_failed`,
  `call_timeout`, `stage_timeout`, `abnormal_termination_in_flight`, or
  `scanner_blocked`.
- Scanner `raw_exit=99` requires
  `classification=output_publication_failed`. A supervision exception requires
  `classification=supervision_failed` unless process cleanup also failed, in
  which case the compound classification is exact. No other classification
  may relabel those cases.
- `primary_blocker` retains the supervisor's first bounded blocker. Its
  closed value set is exactly the union of the values of `EXIT_BLOCKER` and
  `IN_FLIGHT_BLOCKER` in pinned `supervise.py` plus
  `PC0_PROCESS_CLEANUP_BLOCKED`. These are inherited diagnostic values, not
  additions to the ten PC0 outward outcomes.
  The outward PC0 blocker remains derived by the existing V2 precedence law;
  an inherited or otherwise unselected primary maps outward to
  `PC0_EVIDENCE_BLOCKED` without rewriting the retained primary.
- `secondary_cleanup_blocker` is null or
  `PC0_PROCESS_CLEANUP_BLOCKED`.
- `last_lifecycle` is null or one existing accepted WF0/WC0/WA0/PC0 lifecycle
  enum. `last_in_flight_operation` is null or one of the existing 26 operations.
- `durable_record_count` is an integer from 0 through 2048 and equals the list
  length.
- `durable_records` is the allow-listed projection already defined by
  `normalize.sanitized_timeline(run)["positive"]`. Each record keeps only that
  projection's exact keys. Admission revalidates sequence, event kind,
  operation/interface/tier/coordinate enums, call pairing, host-callback
  attribution, and lifecycle ordering to the last durable record. Unknown
  keys, duplicate JSON keys, gaps, extra records, mutable coordinates, or an
  impossible last-state/in-flight join block admission. The Deck reuses that
  existing projection; the Mac validator freezes its literal closed key/enum
  roster in `evidence.py` without importing `normalize.py` or `supervise.py`.
  Deterministic parity cases bind the two validators.
- `call_counts` has exactly the following 26 keys, each an integer 0 through
  2048, and it must reproduce the starts in `durable_records`:

```text
can_process_sample_size
count_classes
create_component
exit_dll
free_library
get_bus_arrangement
get_bus_count
get_bus_info
get_class_info_1
get_class_info_2
get_class_info_unicode
get_controller_class_id
get_factory_info
get_plugin_factory
init_dll
initialize_component
load_library
query_audio_processor
query_factory_2
query_factory_3
release_audio_processor
release_component
release_factory_2
release_factory_3
release_factory_base
terminate_component
```

- `audio_processor_observer_state` is one existing
  `AudioProcessorLeaseState` name; `audio_interface_quiescence` is boolean.
- `inherited_shutdown` has exactly `operations`,
  `clean_in_process_shutdown`, and `physical_containment_only`.
  `operations` has exactly the seven inherited shutdown operation names; each
  value has only `disposition` and `source` using existing supervisor enums.
- `cleanup` has exactly `owned_descendants_zero` and
  `process_group_empty`, both booleans.
- `environment_retirement_disposition` is exactly `retired`,
  `not_attempted_process_containment_unproved`, or `failed`.
- stdout/stderr/protected-snapshot/runner fields are lowercase SHA-256 values.
  `stderr_bytes` is an integer from 0 through 65536.
- Canonical diagnostic bytes are at most 2,097,152 bytes. Raw stdout, raw
  stderr, environment paths, usernames, hostnames, addresses, process IDs,
  credentials, plug-in binaries, license data, and proprietary state have no
  schema location.

### Publication order and failure law

`run.py` keeps the failed `run` value until environment-retirement disposition
is known. It constructs and validates the diagnostic before raising the PC0
blocker. Publication uses `write_atomic` for canonical JSON, then
`write_atomic` for the external sidecar; sidecar publication is the commit
marker. The pair is re-read and strictly validated before the exception is
raised. An already complete identical pair is reusable. A partial, symlinked,
oversized, conflicting, or multiply named publication remains fail-closed.

The inner lock is retained after every failure. A diagnostic never creates a
success result, changes the primary blocker, proves clean shutdown, authorizes
retry, or causes the lock to be removed. Diagnostic publication failure yields
`PC0_EVIDENCE_BLOCKED` and preserves the incomplete lock. No diagnostic is
published on the success path.

The historical V2 batch predates this schema and remains unresolved; no file
may be synthesized for it.

## 6. Mac retrieval, admission, and private custody

After the corrective remote command returns one exact PC0 blocker, the Mac
driver first performs the existing success-result recovery. Only when no
success result or sidecar exists may it inspect the exact inner lock derived
from `D` and `Q`. It retrieves only the two frozen diagnostic filenames.

Remote admission requires a regular non-symlink lock directory containing
exactly `prepared-intent.json` and the two diagnostic files, exact expected
prepared-intent bytes, exactly one diagnostic pair, no conflicting diagnostic
name, canonical JSON, exact sidecar bytes, the size bounds above, and every
schema/value/join law. Source, source role, operation nonce, phase nonce,
DeckExecutionInputIdentity, proof-plan digest, run ID, record sequence, call
counts, blocker precedence, protected-snapshot digest, runner digest, and
cleanup fields are all independently checked.

The Mac stages the pair beneath the active transaction, validates it before
promotion, then atomically promotes one private directory:

```text
<Mac transaction>/failure-diagnostic/
  PC0_FAILURE_DIAGNOSTIC.json
  PC0_FAILURE_DIAGNOSTIC.json.sha256
```

The `execute_deck_batch` phase output records exactly the outward blocker,
retained primary blocker, secondary cleanup blocker, classification,
`failure_diagnostic_sha256`, and custody disposition. No diagnostic bytes enter
tracked evidence. An invalid staging pair is not promoted.

Missing, ambiguous, partial, malformed, noncanonical, mismatched, oversized,
unsafe, or multiple diagnostics result in `PC0_EVIDENCE_BLOCKED` followed by
`RETURN_TO_DESIGN_GATE`. They never authorize another Deck launch. A valid
failed diagnostic also ends V3's corrective authority permanently.

## 7. Exact corrective-authority predicate

V3 replaces the blanket “any prior PC0 Deck execution blocks forever” check
with a single conjunction. It is not a numeric ceiling of two. Before
reserving the corrective batch, all of the following must be true:

1. The repaired source is one direct child of the exact merged V3 authority
   commit/tree and is on the unchanged PC0 implementation ref.
2. The source parent contains the exact reviewed V3 design blob/SHA,
   `PC0_DESIGN_V3_CLEAR` review identity, and an exact V3 operator-approval
   receipt that authorizes one corrective batch and no Windows work.
3. The historical journal is canonical and hashes to
   `06473755eb7ccfa2529522bb29e3fb44a5a4a67ea38fd0e797d198939e1ed966`.
4. The historical recovery receipt is canonical and hashes to
   `870039310c0f6ee4d0bf4044d629e6f6e7d92d2f901b7b40a60f9724bb84f9e5`.
5. Those files join to transaction
   `a14bcc65d15e9fbdf15810a658b2ee87`, failed source `7ac6095a...`,
   failed tree `ad4230a...`, unchanged PC0 ref, exact proof plan, and failed
   Deck input `ae89ee...`.
6. Their admitted effect history is exactly one Windows build/download/custody,
   one host-artifact transfer, one source transfer, one Deck execution, zero
   evidence renders, one ordinary driver invocation, and one separately
   recorded authorized recovery continuation.
7. The historical execute phase is failed with
   `PC0_EVIDENCE_BLOCKED`, there is no admitted prior PC0 success result or
   sidecar, and its diagnostic class remains
   `unresolved_v2_no_diagnostic`.
8. The Windows build identity equals `575d3bd9...` and the existing Mac host
   store strictly validates run `33812659869` attempt 1, artifact
   `9915439437`, manifest `d0e11c...`, build receipt `7de6884d...`, and custody
   receipt `e2a06c...`. No producer, workflow dispatch, download, or custody
   fallback is selectable.
9. After later explicit execution authority permits Deck contact, the existing
   Deck host-artifact store and accepted AGain fixture store validate exactly.
   Absence or mismatch blocks. No artifact transfer, fixture seed, or AGain
   build fallback is selectable.
10. Future authorized readback finds no historical success result/sidecar and
    resolves the expected historical intent and failure-lock objects without
    contradiction. This design makes no current remote-lock claim.
11. Across the proof root there is exactly the one named historical
    transaction plus the current exact V3 transaction for this ref; no other
    PC0 Deck effect, corrective reservation, or completed corrective result
    exists.
12. The current V3 transaction has no `execute_deck_batch` reservation or Deck
    effect. The existing Mac execution-input single-writer lock is acquired
    before the corrective reservation is persisted.

Only after all twelve predicates pass may the driver durably reserve one
corrective execution. The current V3 journal increment and execute-phase
`prepared` receipt are the reservation. The same-source transaction key,
Mac single-writer lock, exact historical scan, and existing remote locks
jointly prohibit duplicate launch. Once reserved, acknowledgement loss,
command failure, valid diagnostic, invalid diagnostic, or success all count as
consumed. No V3 code path can authorize a third batch.

The additional effect ceiling is:

| Effect | V3 additional maximum |
|---|---:|
| Windows builds / workflow dispatches | 0 / 0 |
| artifact downloads / custody operations | 0 / 0 |
| fixture seeds / AGain builds | 0 / 0 |
| host-artifact transfers | 0 |
| repaired-source handoff/transfers | 1 |
| positive Deck batches | 1 |
| live negative exercises | 0 |
| evidence renders | 1 after success; otherwise one private diagnostic retrieval |

If the repaired source handoff already validates exactly, transfer count may
be zero. Any host/fixture cache miss is `RETURN_TO_DESIGN_GATE`, not permission
to reconstruct or transfer it.

The ordinary developer interface remains one command:

```text
python3 tools/host-proof.py run --source <repaired-commit> --plan pc0-pre-setup-processing-contract-v1
```

That steady-state interface does not erase earlier invocations.

## 8. Truthful historical cost and P/E/C roles

Producer P remains failed source
`7ac6095a488d0077fcc78fedc5abd870b5ffb1cb` because that exact source built
artifact `9915439437` under WindowsBuildInputIdentity `575d3bd9...`.
Successful execution E is the repaired V3 source. Consumer C is the final
evidence consumer and may differ only under the accepted renderer-only reuse
law.

`COST_AND_INVALIDATION.json` keeps the V2 top-level roster:

```text
schema
external_effect_counts
phase_dispositions
invalidation_cases
renderer_only_reuse
ordinary_driver_command_count
manually_copied_identifier_count
fixture_accounting
```

The V2 validator fixes incompatible V1 value constraints, including per-effect
maximum one and a one-invocation interpretation. V3 therefore freezes the new
schema value
`linux-vst-bridge-pc0-cost-and-invalidation/v2`. Publishing corrective history
under the V1 schema is forbidden.

`external_effect_counts` and `ordinary_driver_command_count` describe the final
V3 transaction only. `invalidation_cases` keeps its existing entries and adds
one exact `corrective_history` object with this roster:

```text
schema
v2_failed_execution
v2_operator_recovery
v3_corrective_authority
v3_corrective_execution
cumulative_external_effect_counts
cumulative_orchestration_counts
```

The nested schema is `linux-vst-bridge-pc0-corrective-history/v1`. Its closed
sub-rosters are:

| Object | Exact keys |
|---|---|
| `v2_failed_execution` | `transaction_id`, `journal_sha256`, `source`, `deck_execution_input_sha256`, `proof_plan_sha256`, `execute_phase_disposition`, `transaction_state`, `primary_blocker`, `failure_classification`, `local_result_disposition`, `remote_preflight`, `effect_counts`, `driver_invocation_count` |
| `v2_operator_recovery` | `receipt_sha256`, `continuation_count`, `original_driver_invocation_count`, `original_failure_boundary`, `disposition`, `runtime_discovery_comment_id` |
| `v3_corrective_authority` | `merge_commit`, `merge_tree`, `design_blob`, `design_sha256`, `review_id`, `approval_blob`, `prior_journal_sha256`, `prior_recovery_receipt_sha256`, `additional_positive_deck_batches_maximum`, `additional_windows_builds_maximum` |
| `v3_corrective_execution` | `transaction_id`, `journal_sha256`, `execution_source`, `evidence_consumer_source`, `driver_invocation_count`, `reservation_count`, `effect_counts`, `result_sha256` |

Every `source` uses the existing six-key source role. Every `effect_counts`
uses the existing seven effect keys. `remote_preflight` has exactly `result`,
`result_sidecar`, `inner_lock`, and `outer_lock`, with values `absent`,
`absent`, `present_intent_matched`, and `present_intent_matched` after the
later authorized readback. A missing or mismatched historical lock blocks the
corrective reservation; these values are not claims made by this draft.

`cumulative_external_effect_counts` has the seven existing effect keys plus
`workflow_dispatches`, `again_builds`, `fixture_seeds`, and
`live_negative_exercises`. `cumulative_orchestration_counts` has exactly
`v2_driver_invocations`, `operator_recovery_continuations`,
`v3_driver_invocations`, and `total_orchestration_entries`.

It binds both historical file hashes; V2 transaction/source/input/phase;
unresolved classification; observed future lock preflight; producer
run/artifact/build identity; each transaction's exact effect counts and
journal hash; V2 driver invocation count; recovery continuation count; V3
authority identities; V3 driver invocation and reservation count; repaired E;
final C; and final result or failure-diagnostic disposition.

On a successful corrective batch, expected cumulative history is:

```text
windows_builds: 1
workflow_dispatches: 1
artifact_downloads: 1
custody_operations: 1
artifact_transfers: 1
source_transfers: 1 plus the actual V3 value in [0,1]
deck_executions: 2
evidence_renders: 1
again_builds: 0
fixture_seeds: 0
live_negative_exercises: 0
v2_driver_invocations: 1
operator_recovery_continuations: 1
v3_driver_invocations: 1
total_orchestration_entries: 3
```

The final renderer must calculate these values from admitted journals and
receipts. It may not use constants as substitutes for file admission. The
tracked evidence must state that the product's clean steady-state interface is
one command while actual implementation history used two ordinary driver
invocations, one additional authorized recovery continuation, and two Deck
executions. No success packet is rendered if the corrective batch fails.

## 9. Branch replacement, archive, and final topology

The draft V3 design branch is:

```text
refs/heads/codex/pc0-durable-failure-diagnostic-design-v3
```

No implementation history changes during design. After V3 review, explicit
operator approval, authority finalization, merge, and exact `main` readback:

1. Create and push immutable archive ref
   `refs/heads/codex/archive/pc0-v2-failed-7ac6095a488d` at exact failed source
   `7ac6095a488d0077fcc78fedc5abd870b5ffb1cb`.
2. Read back the archive ref and refuse replacement if it differs.
3. Create one repaired source commit directly above the exact V3 authority
   merge. Its diff against that parent is the same fourteen source/configuration
   paths. Relative to failed source, only the four repair paths in section 4
   may differ.
4. Prove the complete Windows roster and WindowsBuildInputIdentity equal the
   failed producer exactly.
5. Replace
   `refs/heads/codex/pc0-windows-vst3-pre-setup-processing-contract` only with
   force-with-lease expecting old tip
   `7ac6095a488d0077fcc78fedc5abd870b5ffb1cb`. Any lease mismatch stops.
6. After the one authorized corrective success, create one five-path
   evidence-only child.

Final implementation topology is exactly:

```text
V3 authority merge
  -> one 14-path repaired source commit
     -> one 5-path evidence-only commit
```

The archived failed source is retained as historical evidence and is not an
ancestor of the repaired two-commit implementation branch. A merge, rebase
chain, third implementation commit, missing archive readback, or unreviewable
force update is `RETURN_TO_DESIGN_GATE`.

## 10. Sixteen-row proof matrix

No seventeenth row is added.

| # | Focused proof |
|---:|---|
| 1 | Every V2/V3 authority and Git relationship resolves; the repaired source is one exact fourteen-path child and all identity rosters reproduce. |
| 2 | The sole `PreSetupProcessingContractCensus` still borrows resources and adds no product owner. |
| 3 | Corrective AGain execution emits exactly the canonical eleven-call sequence and 33 paired lifecycle calls. |
| 4 | Count bounds and checked arithmetic remain exact. |
| 5 | Every `BusInfo` remains coordinate-consistent, bounded, and strictly normalized. |
| 6 | Both speaker arrangements retain exact bits and channel consistency. |
| 7 | Sample-size true/false/other classification remains exact; AGain is true/true on success. |
| 8 | Only the full immutable roster reaches contract complete. |
| 9 | Ordinary failure preserves the first blocker and only proven teardown. |
| 10 | Writer/supervision failures durably publish and strictly admit the exact bounded diagnostic; exit 99 and supervision failure remain distinguishable. |
| 11 | Static and event ledgers contain no prohibited setup, latency, tail, activation, or processing call. |
| 12 | Invalidation proves all Windows-build records and WindowsBuildInputIdentity remain exact while only the four non-Windows repair paths differ. |
| 13 | Accepted AGain and host-artifact stores are reused with zero build, seed, producer, download, custody, or host-artifact transfer. |
| 14 | One clean corrective driver command is distinguished from the admitted V2 driver and recovery history; the exact predicate admits one reservation only. |
| 15 | Producer P from failed source joins repaired execution E through unchanged build identity; strict P/E result admission and C renderer reuse remain exact. |
| 16 | Final machine-readable evidence proves cleanup/protected-state closure plus failed V2 disposition, corrective authority, two-batch cumulative history, and exact final costs. |

Rows 3, 13, 14, and 16 require the later authorized positive observation.
All other refinements are deterministic/static or combine deterministic
validation with already admitted immutable identities. A failed corrective
batch yields only private diagnostic custody and no PASS packet.

Deterministic tests must prove, without GitHub, SSH, Proton, Wine, fixture, or
DAW execution:

- raw exit 99 publishes and admits
  `classification=output_publication_failed`;
- injected supervision failure publishes and admits
  `classification=supervision_failed` with exact nullable raw exit;
- wrong sidecar, duplicate keys, noncanonical bytes, extra/missing keys,
  oversized values, unknown enums, record-sequence defects, mismatched
  source/input/plan/nonces, altered call counts, multiple diagnostics, and
  symlinks are rejected;
- the exact historical journal and receipt plus later V3 authority admit one
  corrective reservation;
- no corrective authority blocks a second batch;
- a consumed corrective reservation blocks a third batch, including after
  lost acknowledgement or failed diagnostic retrieval;
- producer, dispatch, download, custody, host-artifact transfer, seed, and
  AGain-build paths are unreachable in corrective mode;
- the exact seventeen-record roster reproduces
  `575d3bd9183be1ec0fe0311cff48bc0107c4299a3355748c470e3d195d284849`;
- final cost/evidence validation rejects hidden V2 execution, hidden recovery,
  false one-invocation history, false one-batch history, or changed P/E/C roles;
- the deterministic proof count remains sixteen and external effects remain
  zero.

## 11. Stop laws, privacy, and approval boundary

Stop with `PC0_V3_RECONNAISSANCE_BLOCKED` if the exact local journal/receipt
bytes or admitted facts differ from section 2 before design authority is
accepted. `PC0_V3_RECONNAISSANCE_BLOCKED` is a design-process return label,
not an additional PC0 runtime blocked outcome.

Return `RETURN_TO_DESIGN_GATE` if truthful implementation requires a
Windows-build-input change; `supervise.py` change; fifth repair path; another
owner/state/call/blocker/proof row/evidence path; changed claim/fixture/runner;
new producer/download/custody/host transfer; live negative; AGain build/seed;
generalized retry framework; or broader protocol.

A failed diagnostic is private, bounded, allow-listed, and contains hashes
instead of raw streams or protected snapshots. It never enters Git. The five
tracked evidence paths remain exactly:

```text
evidence/pc0-windows-vst3-pre-setup-processing-contract/BASIS.md
evidence/pc0-windows-vst3-pre-setup-processing-contract/COST_AND_INVALIDATION.json
evidence/pc0-windows-vst3-pre-setup-processing-contract/FINDINGS.md
evidence/pc0-windows-vst3-pre-setup-processing-contract/TRANSACTION.json
evidence/pc0-windows-vst3-pre-setup-processing-contract/hashes.sha256
```

The draft design PR authorizes no implementation or external execution. A later
technical-lead review must return `PC0_DESIGN_V3_CLEAR` against an exact
head/tree/design-blob/SHA. A later exact operator approval must be retained in
repository authority and explicitly authorize only one corrective positive
Deck batch with zero Windows work. Until that merged authority is read back,
`implementation_authorized=false`.
