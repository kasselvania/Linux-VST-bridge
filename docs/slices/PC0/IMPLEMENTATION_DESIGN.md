# PC0 implementation design v2

```yaml
slice: PC0
design_revision: pc0-design-v2
design_status: proposed_for_adversarial_review
implementation_authorized: false
owner_count: 1
stable_state_count: 9
unique_call_operation_count: 4
positive_call_count: 11
proof_row_count: 16
blocked_outcome_count: 10
source_configuration_path_count: 14
evidence_path_count: 5
closed_plan: pc0-pre-setup-processing-contract-v1
expected_windows_acceptance_producers: 1
expected_positive_deck_batches: 1
renderer_only_external_effects: 0
```

V2 supersedes V1 commit `f6938e7dd501a4c86a382243670d82d060d1b75a`,
tree `bacf308af5a7c993e1da45059497fb392a3b8845`, design blob
`0f2d1c26aea3d009ce93d1d5086b52ca03fc8ce1`, SHA-256
`29db8b0f884407ba9ea75c80cab6ddd290908f2449d73e4502dd61347c490e86`.
Independent technical-lead review `5105496167` returned
`PC0_DESIGN_V1_REPAIR_REQUIRED`; its two bounded findings are retained in
[the review history](ADVERSARIAL_DESIGN_REVIEW.md).

## 1. Authority, claim, and ceiling

Selection basis is commit `858c240b104e090aaed8bd23ace04fd9a0dfd20e`,
tree `8a560faf07b793de8faab91952ee6f34f15a1e73`. Original activation is
`5ec7ef4f2c5f19faffcf1f2b01656d698ac7cd7a`, tree
`fce2e4c4f65a90fcde6835e0cf4aba0169f81c2a`; revised selection authority is
its child `f00824e196d2a71667d80a9cc622c03ea2cef403`, tree
`d895a44b98264f47549d4187303b00e9e9f85982`. This card implements only the
revised selection in [the selection receipt](SLICE_SELECTION.md), informed by
[the bounded reconnaissance](RECONNAISSANCE.md).

Primary claim:

> On the exact accepted AGain lifecycle, while the processor remains in the
> Initialized state, the supervised Windows host performs one bounded read-only
> census of all audio/event buses, all existing `BusInfo` records, current audio
> speaker arrangements, and `kSample32`/`kSample64` support, retains one exact
> normalized pre-setup processing contract, and completes the accepted
> interface/component/factory/module shutdown without mutating processing state.

PC0 calls only `getBusCount`, `getBusInfo`, `getBusArrangement`, and
`canProcessSampleSize`. It does not call or prepare `setIoMode`, `activateBus`,
`setActive`, `setBusArrangements`, `setupProcessing`, `setProcessing`,
`process`, `getLatencySamples`, `getTailSamples`, or allocate audio/event
buffers, parameter queues, event lists, or process contexts. Controller,
connection-point, state, parameter, automation, preset, proxy, C ABI, IPC,
shared-memory, broker/service, real-time, GUI, Bitwig, Serum, packaging,
signing, runner-selection, other plug-in/DAW, and general compatibility claims
are outside PC0.

## 2. Accepted boundary and source result

PC0 consumes, without redesign, WC0 component/host ownership and shutdown, WA0
`IAudioProcessor` lease ownership and quiescence, and DX0 split identities,
accepted-fixture store, custody/handoff, one-command transaction, retained-result
admission, recovery, process/environment containment, and protected-state law.
Exact fixture identities are AGain module
`60aa9ff6b9918d4330449e7b3ab34b588dd93cba09f37413a3cd91f6e7d2e18f`,
14-record bundle manifest
`bfaa1dce4d2e189f89cee41493838824efe647e361e81436676b7b3a86ff5164`,
and Runtime/Proton digest
`2d64df1d36786ca2d0e955c553005423dc2b5bdd714bd0a17872622e33912547`.
Accepted WA0 source is `24b7e6da7e29a5bd358097a6b89c5c59b747c413`,
tree `d7c43098a14d86bf36b9c428e3836e1eb5353613`; its evidence is
`99478005e9f2675036100486c87952b76d411f84`, tree
`56e861611d0a0fdfeeafac517e40f8bb7ea9bb8f`, merged by
`5d084ba5032dc8a7ce93be51e7d75dfd0d37ee22`. Accepted DX0 source is
`be046dd2d44a7915ca408c01a06212639ccea51e`, tree
`1322e4eb7b5bd54244bd2fa7fc83327ea6a0c183`; its evidence is
`85840920844693f7611306492298817e16dd2a6c`, tree
`35ddde4e56a810ab1ce44970313bf2f42e585908`, merged by
`1f71487717eabdb3cd5285a4559df2ce2915c8d8`. Post-DX0 status closure is the
selection basis `858c240b104e090aaed8bd23ace04fd9a0dfd20e`.

Design and future implementation validation must resolve every named authority,
source, evidence, merge, tree, and blob as a real Git object; compare its type,
exact value, parent/tree relation, merge ancestry, and required containment to
this card. A lexically valid 40- or 64-character string is never identity proof.

Pinned SDK contracts permit all four selected methods on the Windows UI thread
while Initialized. AGain creates `(1,1,1,0)` audio-in/audio-out/event-in/event-out
buses. Its three records are `Stereo In` / 2 / `kMain`, `Stereo Out` / 2 /
`kMain`, and `Event In` / 1 / `kMain`; each has only `kDefaultActive` set.
Both audio arrangements are `0x0000000000000003` (`kStereo`), and AGain
returns `kResultTrue` for both selected sample sizes. These are source
expectations until the authorized implementation is executed.

## 3. Sole new owner and state machine

`PreSetupProcessingContractCensus` is the only new owner. It borrows the
accepted initialized `IComponent`, live `IAudioProcessor`, and synchronous event
writer. It owns fixed coordinates, bounded count/record storage, normalized
bus/arrangement/sample-size results, first blocker, later cleanup dispositions,
and immutable completion. It never owns an interface, component, host, factory,
module, process, environment, buffer, controller, or transport.

It lives in the existing component-session `.h/.cpp` pair. The accepted
`AudioProcessorInterfaceLease` acquires the interface, invokes this borrower,
then alone retires the lease. Nine closed states are:

1. `pre_setup_census_absent`
2. `bus_count_in_flight`
3. `bus_counts_validated`
4. `detail_call_in_flight`
5. `bus_info_complete`
6. `speaker_arrangements_complete`
7. `sample_format_call_in_flight`
8. `pre_setup_contract_complete`
9. `pre_setup_census_blocked`

Every in-flight state also binds the exact operation and coordinates. An
ordinary failure enters `pre_setup_census_blocked`; timeout/crash leaves the
last call in flight because no return may be invented.

## 4. Exact positive order and attribution

After accepted initialization and WA0 interface acquisition:

1. `get_bus_count`: `kAudio/kInput`, `kAudio/kOutput`, `kEvent/kInput`,
   `kEvent/kOutput`.
2. Validate all four counts before allocating or iterating.
3. `get_bus_info` for every retained bus, ordered media `kAudio`, `kEvent`;
   direction `kInput`, `kOutput`; then ascending index.
4. `get_bus_arrangement` for every audio input then audio output, ascending
   index.
5. `can_process_sample_size` for `kSample32`, then `kSample64`.
6. Freeze the contract, return to WA0, release the interface (expected count
   1), terminate and release the component (expected count 0), then complete
   accepted factory/module shutdown.

AGain therefore makes exactly 11 new calls: 4 + 3 + 2 + 2. Before each call,
the accepted writer synchronously flushes `call_started`; after an ordinary
return it immediately synchronously flushes paired `call_completed` before any
lifecycle event or next call. All execute synchronously on the accepted scanner
main/UI thread; no second call starts while one is in flight.

Coordinates are closed enums and bounded integers: count `(media,direction)`,
info `(media,direction,index)`, arrangement `(direction,audio_index)`, sample
size `(kSample32|kSample64)`. Evidence retains symbolic values plus fixed-width
numeric projections, never pointers, addresses, thread/process identifiers, or
unbounded plug-in text. An unmatched start identifies its exact operation and
coordinates.

The durable writer is part of call attribution. If `call_started` cannot be
written and flushed, the VST3 call has not been attempted and no in-flight call
or return is invented. If the call returns but `call_completed` cannot be
written and flushed, its scalar/output is unconsumable and the durable ledger
remains unmatched at the exact operation/coordinates. No interface/component/
factory release, `ExitDll`, `FreeLibrary`, or clean in-process retirement may
then be claimed; accepted physical containment owns retirement. Writer failure
is secondary to any earlier PC0 primary blocker and never erases it.

## 5. Count and output laws

Each `int32` count must be 0–32; the overflow-checked sum, accumulated only
after converting validated values to unsigned size, must be at most 64. Any
negative, per-domain excess, aggregate excess, or arithmetic inconsistency is
`PC0_BUS_COUNT_BLOCKED`; no storage, iteration, or later census call occurs.
Storage is fixed-capacity for 64 records.

Before `getBusInfo`, zero the whole `BusInfo` and retain its request. Only
`kResultTrue` makes output consumable. Returned media type and direction must
equal the request. `busType` is exactly `kMain` or `kAux`; unknown bits outside
`kDefaultActive|kIsControlVoltage` block. Control-voltage is false for event
buses. Audio channel count is 1–64; event channel count is 1–16. The normalized
flag projection contains explicit `default_active` and `control_voltage`
booleans plus the fixed-width raw value.

`String128` inspection is limited to 128 UTF-16 code units and requires a NUL
within the field. Strict conversion rejects unpaired or malformed surrogates;
it does not trim, case-fold, or normalize. Only code units before the first NUL
enter UTF-8 evidence; tail and padding never do. Empty but terminated names are
representable and do not establish identity. Any malformed field is
`PC0_BUS_INFO_BLOCKED`.

Before `getBusArrangement`, zero the complete 64-bit output and bind an index
already present in the retained audio roster. Only `kResultTrue` consumes it.
The returned bitset is retained as exactly 16 lowercase hexadecimal digits,
with its population count and a deterministic recognized-layout label when an
exact pinned constant matches. The population count must equal the associated
audio channel count. No standalone high-bit rejection is lawful: the pinned
SDK's `kAmbi7thOrderACN` uses all 64 bits. No arrangement is requested for an
event bus, and `setBusArrangements` is prohibited.

For each sample-size call, retain the raw `tresult` as signed fixed-width and
eight-digit lowercase unsigned hexadecimal projections. `kResultTrue` means
supported, `kResultFalse` means unsupported, and any other ordinary result is
`PC0_SAMPLE_FORMAT_BLOCKED`. Unsupported is a valid census value, although the
exact AGain positive claim requires both true.

The immutable value uses schema
`linux-vst-bridge-pc0-processing-contract/v1` and exact top-level keys
`schema`, `lifecycle_state`, `counts`, `buses`, `sample_sizes`, `call_count`,
`complete`, and `mutation_call_count`. `counts` has only the four canonical
coordinates. Each ordered bus has only `media_type`, `direction`, `index`,
`name_utf8`, `channel_count`, `bus_type`, `flags_u32_hex`, `default_active`,
`control_voltage`, and `speaker_arrangement`; the last is null for event buses
and otherwise contains `bits_u64_hex`, `channel_count`, and `recognized_layout`.
Each sample entry has only `symbolic_size`, `tresult_i32`,
`tresult_u32_hex`, and `supported`. Positive AGain requires
`lifecycle_state="Initialized"`, `call_count=11`, `complete=true`, and
`mutation_call_count=0`; latency and tail have no schema location.

## 6. Completion, quiescence, and failure precedence

A complete immutable contract requires four valid counts, exactly the implied
`BusInfo` and audio-arrangement rosters, both classified sample-size results,
the exact canonical call order, no in-flight call, and healthy bounded event and
output ledgers. Missing, duplicated, extra, reordered, partial, or mutable
output is `PC0_CONTRACT_INCOMPLETE`.

The first semantic/call blocker remains primary and stops all remaining census
calls. After an ordinary failure, temporary output is discarded; the census
returns control to the accepted WA0 lease owner, which may release the interface
and continue WC0/WA0 teardown only if existing interface/object quiescence is
proved. Later release, shutdown, cleanup, or evidence failures are secondary
and never erase it.

Timeout, crash, or missing completion fabricates neither return nor output and
forbids unsafe in-process continuation: no interface/component/factory release,
`ExitDll`, or `FreeLibrary` is claimed. Accepted process containment drains the
owned tree and retires the environment. Physical disappearance is not clean
in-process retirement. Failed containment is `PC0_PROCESS_CLEANUP_BLOCKED`.

## 7. Deterministic failure proof

Production-owner, event-decoder, normalizer, and DX0-planner tests cover:
negative/over-cap/aggregate-overflow counts; `getBusInfo` failure, coordinate
mismatch, malformed or unterminated UTF-16, channel/type/flag invalidity;
arrangement failure and channel mismatch; sample-size true/false/unexpected
classification; unmatched starts for all four operation types; first-primary
preservation; incomplete contracts; prohibited-call rejection; and
renderer-only zero-external-work planning. Injected streams exercise accepted
supervisor attribution. Tests make no GitHub, SSH, Proton, or fixture call.
There is no PC0 fault module or live negative family; the sole live exercise is
the positive AGain batch.

| Injected mutation/fault | Required disposition |
|---|---|
| Count negative, >32, sum >64, or checked-sum failure | `PC0_BUS_COUNT_BLOCKED`; no later census call. |
| Info failure, coordinate/enum/flag/channel/name defect | `PC0_BUS_INFO_BLOCKED`; output unconsumed. |
| Arrangement failure or popcount mismatch | `PC0_BUS_ARRANGEMENT_BLOCKED`; contract incomplete. |
| Sample result other than true/false | `PC0_SAMPLE_FORMAT_BLOCKED`; no support inference. |
| Missing/reordered/extra completion, including a failed durable `call_completed` write after ordinary return | Returned output is unconsumable; exact unmatched coordinates remain; `PC0_CONTRACT_INCOMPLETE` and physical containment apply. |
| Unmatched start, injected timeout, or crash | Exact in-flight coordinates retained; process containment only. |
| Renderer-only source mutation | P/E retained for C; all external invocation counters remain zero. |

## 8. Closed DX0 plan and identity contract

The only new registry entry is:

```json
{"accepted_fixture_id":"wa0-again-accepted-v1","deterministic_validation_set":"pc0-pre-setup-deterministic-v1","evidence_renderer":"pc0-five-file-renderer-v1","expected_result":"pc0-pre-setup-contract-complete-v1","host_mode":"host_only","live_deck_batch":"pc0-positive-only-v1","plan_id":"pc0-pre-setup-processing-contract-v1","schema":"linux-vst-bridge-dx0-proof-plan/v1"}
```

No caller-supplied command, path, environment value, or hook is admitted. The
ordinary invocation is exactly:

```text
python3 tools/host-proof.py run --source <commit> --plan pc0-pre-setup-processing-contract-v1
```

DX0 canonical JSON/hash, typed receipt, persisted intent/nonce, single-writer,
lost-acknowledgement recovery, and distinct P/E/C laws remain unchanged. PC0
uses the already-selected schemas `linux-vst-bridge-pc0-complete-source/v1`
(14 records), `linux-vst-bridge-pc0-transaction-result/v1`, and
`linux-vst-bridge-pc0-evidence-packet/v1`. It reuses the exact DX0 v1 Windows
build-input, accepted-fixture, Deck-execution-input, evidence-renderer,
proof-transaction, host artifact/custody, and source-handoff schema families;
their validators become plan-aware without weakening accepted WA0/DX0 data.

The future implementation branch/ref are frozen as
`codex/pc0-windows-vst3-pre-setup-processing-contract` and
`refs/heads/codex/pc0-windows-vst3-pre-setup-processing-contract`. The inherited
source-handoff wire identity remains schema
`linux-vst-bridge-dx0-source-handoff/v1`, bundle
`dx0-execution-source-<consumer-commit>.bundle`, sole advertised/Deck ref
`refs/handoff/dx0-source/<consumer-commit>`, and receipt filenames
`DX0_SOURCE_HANDOFF_RECEIPT.json` / `DX0_SOURCE_HANDOFF_RECEIPT.sha256`.
These are derived by the driver, never copied by the operator.

### Private retained result: P/E original-observation truth

The Deck publishes `linux-vst-bridge-pc0-transaction-result/v1`. Its exact
top-level keys are `schema`, `operation_nonce`, `artifact_producer_source`,
`deck_execution_source`, `execution_input`, `host_artifact`,
`accepted_fixture`, `source_handoff`, `closed_plan`, `original_observation`,
`positive_result`, `call_facts`, `quiescence`, `shutdown`, `cleanup`,
`protected_state`, and `integrity`. `positive_result.processing_contract` is
the immutable `linux-vst-bridge-pc0-processing-contract/v1` value.

This private object binds the true artifact producer P and original Deck
execution E. It contains no evidence-consumer C: C does not yet exist as an
execution fact when the Deck publishes. Strict private-result admission
requires canonical JSON plus external sidecar; exact P/E and Deck-input joins;
host artifact, accepted fixture, handoff, and closed plan; complete original
observation identity/timestamps; exact positive contract/calls; interface and
object quiescence; clean shutdown; zero descendants and retired/absent
environment; equal completed original protected-state comparison; and hashes
of each bounded projection. A filename, schema, digest, or success flag alone
never qualifies, and admission never rewrites P or E for a later consumer.

### Tracked evidence packet: P/E/C projection

After strict admission, C locally renders `TRANSACTION.json` as the complete
`linux-vst-bridge-pc0-evidence-packet/v1` value. Its exact top-level key roster
is:

```text
schema
artifact_producer_source
deck_execution_source
evidence_consumer_source
observation_disposition
consumer_executed_on_deck
consumer_deck_state_freshly_inspected
admitted_private_result_sha256
result_admission
windows_build_input
deck_execution_input
host_artifact
accepted_fixture
source_handoff
closed_plan
original_observation
processing_contract
call_facts
quiescence
shutdown
cleanup
protected_state
proof_rows
renderer
integrity
```

`observation_disposition` is exactly `original_observation` for the initial
same-transaction rendering, otherwise `reused_original_observation`.
`consumer_executed_on_deck` is true only when the admitted E observation
actually executed the exact C source; `consumer_deck_state_freshly_inspected`
is true only for the initial observation transaction, never for later reuse.
A renderer-only correction may change C and renderer identity while preserving
P/E; it must use the reuse disposition and both booleans are false.

`result_admission` records the exact predicate/schema, pass result, admitted
result digest, Deck-input digest, and plan digest. `original_observation`
retains E's immutable observation ID and bounded timestamps from the admitted
private result. `proof_rows` has exactly 16 unique row IDs and dispositions.
`integrity` contains hashes of the admitted private result and named nested
projections (`processing_contract`, `call_facts`, lifecycle closure,
`protected_state`, `proof_rows`, and renderer), not a digest of the packet
containing itself. Hash order is nested projections, private-result canonical
bytes and external sidecar, evidence nested projections, evidence-packet
canonical bytes, then the external evidence-file ledger; no object includes
its own hash in its hashed contents.

`COST_AND_INVALIDATION.json` uses the inherited DX0 cost-schema family and has
only `schema`, `external_effect_counts`, `phase_dispositions`,
`invalidation_cases`, `renderer_only_reuse`, `ordinary_driver_command_count`,
`manually_copied_identifier_count`, and `fixture_accounting`. It owns no
transaction, cleanup, or protected-state truth. `BASIS.md` and `FINDINGS.md`
are human projections only.

The accepted inherited ledger has 22 paired calls; PC0 adds exactly 11, so a
positive retained result requires `started_count=completed_count=33`, no
in-flight operation, no ledger overflow, and new-operation counts `(4,3,2,2)`
in canonical order. Neither the result nor evidence admits latency/tail fields
or any state-mutating operation.

Identity invalidation is exact: a changed Windows roster record selects at most
one post-freeze producer; a Deck-roster-only change selects zero builds and at
most one positive batch; a renderer-only change over an admitted result selects
only local rendering (zero build/download/custody/transfer/Deck effects); a
Mac-driver-only change reuses narrower identities where equal. Ordinary PC0
never compiles or reseeds AGain.

## 9. Frozen source rosters and envelope

Records are exactly `path`, `git_mode`, `git_blob`, unique and raw-UTF-8 sorted.
The following memberships and modes are frozen; listed blobs are the exact
revised-authority-head baseline. Final source identity substitutes only the
final commit's blob at the same path/mode. Missing/extra paths, mode drift, or a
blob not read from the bound commit blocks.

The 14 changed source/configuration paths are:

| Path | Why |
|---|---|
| `.github/workflows/wf0-windows-msvc-build.yml` | Register the exact PC0 host-only dispatch/plan while preserving the producer. |
| `tools/host-proof.py` | Admit and drive the new closed plan through the existing one-command transaction. |
| `tools/wf0-factory-census/artifacts.py` | Bind PC0 source identity in existing custody/handoff validation. |
| `tools/wf0-factory-census/build.py` | Bind the PC0 build input and host-only receipt. |
| `tools/wf0-factory-census/common.py` | Freeze plan, schemas, rosters, coordinates, and identity derivation. |
| `tools/wf0-factory-census/evidence.py` | Strictly admit the retained result and render five files. |
| `tools/wf0-factory-census/negative_tests.py` | Exercise production validation, invalidation, and injected failures. |
| `tools/wf0-factory-census/normalize.py` | Decode the exact 11-call stream into the bounded contract. |
| `tools/wf0-factory-census/run.py` | Select PC0 execution and publish its canonical retained result. |
| `tools/wf0-factory-census/supervise.py` | Attribute the four new operations and coordinates. |
| `tools/wf0-factory-census/verify.py` | Permit exactly the selected read-only C++ call surface and forbid all others. |
| `windows-factory-probe/source/component_instance_session.cpp` | Implement the one bounded borrower and call sequence. |
| `windows-factory-probe/source/component_instance_session.h` | Declare its fixed states/results/caps. |
| `windows-factory-probe/source/main.cpp` | Select PC0 mode and close its session result. |

Their exact baseline records, in that order, are:

```text
.github/workflows/wf0-windows-msvc-build.yml|100644|a00125a058607c0f1958cc883e2f09dcf1535214
tools/host-proof.py|100644|beb57c62b94f3d99fb87f56db3b20b20553064c4
tools/wf0-factory-census/artifacts.py|100644|b6b54bd0ccd9f35d9f395f3162008015000eb4dd
tools/wf0-factory-census/build.py|100644|fc8850824808d3c635c2026a77877cb0eebf8c1e
tools/wf0-factory-census/common.py|100644|e80dee0906bd7cdd3fea3f01eada609a1b996b48
tools/wf0-factory-census/evidence.py|100644|41cb964c3f625ddb9626f35d8a7081f59df56ed2
tools/wf0-factory-census/negative_tests.py|100644|9d12c99b2af474ecfb7a179d5bfab9bfcff8f3a6
tools/wf0-factory-census/normalize.py|100644|39e840f51d7224a29e9f5d1e2eb9e13f84797551
tools/wf0-factory-census/run.py|100644|48201b1f7a088c035638ab122a4ee7c465d11dde
tools/wf0-factory-census/supervise.py|100644|aceb727371cd5a9405ec6664ba4394c78eaed4ad
tools/wf0-factory-census/verify.py|100644|20451e84e209be0d903b74a4c239e68de83e0fd0
windows-factory-probe/source/component_instance_session.cpp|100644|f1f4281b5484d38007fca22adbb40c7f35a827bc
windows-factory-probe/source/component_instance_session.h|100644|6c81005b178c4cc7fa9f59a37830095a7a98acc9
windows-factory-probe/source/main.cpp|100644|4047e91b3fef6365407c71e50b3398a05b280975
```

The inherited WindowsBuildInputIdentity roster remains the following exact
17-record raw-sorted roster:

```text
.github/workflows/wf0-windows-msvc-build.yml|100644|a00125a058607c0f1958cc883e2f09dcf1535214
CMakeLists.txt|100644|b6573f22f2931f24f0453c67accd514430cd525f
cmake/WF0DependencyLock.cmake|100644|3312c93653621dd8ac7a1a9027f57614666cdbdd
tools/wf0-factory-census/build.py|100644|fc8850824808d3c635c2026a77877cb0eebf8c1e
tools/wf0-factory-census/common.py|100644|e80dee0906bd7cdd3fea3f01eada609a1b996b48
tools/wf0-factory-census/verify.py|100644|20451e84e209be0d903b74a4c239e68de83e0fd0
windows-factory-probe/CMakeLists.txt|100644|c07fdab2fe5a2814cbbcd9847619e651f6383510
windows-factory-probe/include/linux_vst_bridge/wf0_probe/census.h|100644|fa7bc69b9408847617da68b4b32e0d136ee8084d
windows-factory-probe/include/linux_vst_bridge/wf0_probe/events.h|100644|ba19cf5f378df32a3b31b3a7e97503f8cfdecfe6
windows-factory-probe/source/component_instance_session.cpp|100644|f1f4281b5484d38007fca22adbb40c7f35a827bc
windows-factory-probe/source/component_instance_session.h|100644|6c81005b178c4cc7fa9f59a37830095a7a98acc9
windows-factory-probe/source/factory_census.cpp|100644|bf5a14cf1941a48f3e3aea513fe9901ae97ce57b
windows-factory-probe/source/factory_census.h|100644|a5757268ed6c91cd60d51dd4202ecd5d5e21e36d
windows-factory-probe/source/main.cpp|100644|4047e91b3fef6365407c71e50b3398a05b280975
windows-factory-probe/source/win32_module.cpp|100644|9095817720550ff858b10cb233341d51b022399a
windows-factory-probe/source/win32_module.h|100644|0ae6eaaa54f68b8399f22c5b2cc99035a87f8bfd
windows-fixtures/wf0/CMakeLists.txt|100644|9f2c019e658f3789d3a698b67ffbb61aa51db99a
```

The DeckExecutionInputIdentity roster is exactly:

```text
tools/wf0-factory-census/artifacts.py|100644|b6b54bd0ccd9f35d9f395f3162008015000eb4dd
tools/wf0-factory-census/common.py|100644|e80dee0906bd7cdd3fea3f01eada609a1b996b48
tools/wf0-factory-census/environment.py|100644|5299101704055e371a14a2f8cd70dd8eb1bace24
tools/wf0-factory-census/normalize.py|100644|39e840f51d7224a29e9f5d1e2eb9e13f84797551
tools/wf0-factory-census/run.py|100644|48201b1f7a088c035638ab122a4ee7c465d11dde
tools/wf0-factory-census/supervise.py|100644|aceb727371cd5a9405ec6664ba4394c78eaed4ad
tools/wr0-proton-bootstrap/launch.py|100755|215718bb641765da9163779c0e2145bd02d3198a
```

Its transitive repository-local import/execution closure must equal this
roster. The EvidenceRendererIdentity roster is exactly:

```text
tools/wf0-factory-census/common.py|100644|e80dee0906bd7cdd3fea3f01eada609a1b996b48
tools/wf0-factory-census/evidence.py|100644|41cb964c3f625ddb9626f35d8a7081f59df56ed2
```

The exact evidence roster is:

```text
evidence/pc0-windows-vst3-pre-setup-processing-contract/BASIS.md
evidence/pc0-windows-vst3-pre-setup-processing-contract/COST_AND_INVALIDATION.json
evidence/pc0-windows-vst3-pre-setup-processing-contract/FINDINGS.md
evidence/pc0-windows-vst3-pre-setup-processing-contract/TRANSACTION.json
evidence/pc0-windows-vst3-pre-setup-processing-contract/hashes.sha256
```

`TRANSACTION.json` alone owns the complete machine-readable tracked packet;
its exact `processing_contract` value nests the immutable contract schema.
`hashes.sha256` hashes the other four tracked files in lexical path order and
does not include itself. The evidence-renderer identity binds this exact roster
and packet contract.

## 10. Cost ledger, proof matrix, and blockers

Before PC0 the closed DX0 driver supports only WA0; after PC0 one command owns
plan selection, host reuse/production, fixture verification, custody/handoff,
one positive batch, result retrieval, and evidence. Required steady-state
ledger: one ordinary Mac command, zero copied identifiers, at most one Windows
producer after source freeze, zero AGain builds/seeds, at most one positive Deck
batch, zero live negatives. Renderer-only correction performs `(build,
download/custody, transfer, Deck) = (0,0,0,0)`.

The initial PC0 source necessarily changes Windows-roster C++ and verification
blobs, so its expected acceptance transaction is exactly one Windows producer
and one positive Deck batch. This is not a retry allowance.

| Operation | Before PC0 | PC0 accepted target |
|---|---:|---:|
| Supported ordinary PC0 driver commands | 0 | 1 |
| Manually copied cross-plane identifiers | not a lawful route | 0 |
| Windows acceptance producer | not selectable for PC0 | 0 on hit, otherwise 1 maximum after freeze |
| AGain build / fixture seed | 0 / 0 | 0 / 0 |
| Positive Deck batches | not selectable for PC0 | 0 on valid hit, otherwise 1 maximum |
| Live negative exercises | 0 | 0 |
| Renderer-only correction external effects | no PC0 result exists | 0 |

| # | Focused proof |
|---:|---|
| 1 | Every named Git authority resolves with its exact type and relationship; the 14-source envelope and all four narrower rosters reproduce. |
| 2 | Sole owner borrows but never acquires an interface/process resource. |
| 3 | AGain emits exactly the canonical 11-call sequence and paired records. |
| 4 | Negative, capped, aggregate, and overflow count laws stop before iteration. |
| 5 | Every `BusInfo` is zeroed, coordinate-consistent, bounded, and strictly normalized. |
| 6 | Both stereo arrangements retain exact bits and match two channels. |
| 7 | Sample-size true/false/other classification is exact; AGain is true/true. |
| 8 | Only a closed immutable full roster reaches contract-complete. |
| 9 | Each ordinary early failure preserves first blocker and permits only proven WA0 teardown. |
| 10 | Unmatched/timeout/crash and durable writer-failure attribution names exact coordinates, makes returned output unconsumable where required, and uses physical containment without a clean-retirement claim. |
| 11 | Static/event ledgers contain none of the prohibited setup, latency, tail, activation, or process calls. |
| 12 | Build-, Deck-, renderer-, and Mac-only mutations select the exact DX0 phase set. |
| 13 | Accepted AGain identity is reused with zero build and zero seed. |
| 14 | The real closed plan completes through one driver command with zero copied identifiers. |
| 15 | Strictly admitted private P/E result can be rendered for C; rerender changes C only, truthfully marks historical reuse, and performs zero external effect. |
| 16 | `TRANSACTION.json` machine-readably proves all 16 dispositions, contract, interface/component/factory/module closure, zero descendants/environment, and equal protected state. |

Exact blocked taxonomy (10): `PC0_DESIGN_PREFLIGHT_BLOCKED`,
`PC0_DESIGN_SCOPE_BLOCKED`, `PC0_BUS_COUNT_BLOCKED`, `PC0_BUS_INFO_BLOCKED`,
`PC0_BUS_ARRANGEMENT_BLOCKED`, `PC0_SAMPLE_FORMAT_BLOCKED`,
`PC0_CONTRACT_INCOMPLETE`, `PC0_PROCESS_CLEANUP_BLOCKED`,
`PC0_EVIDENCE_BLOCKED`, `RETURN_TO_DESIGN_GATE`.

## 11. Stop, topology, audit, and privacy

Return `PC0_DESIGN_SCOPE_BLOCKED` if implementation needs another owner,
eleventh state, nineteenth proof row, eleventh blocker, seventeenth source path,
seventh evidence path, live negative family, second producer/positive batch,
fixture rebuild/reseed, runner/build-plane change, broad rename, or generic
method framework. Return `RETURN_TO_DESIGN_GATE` for latency/tail, setup or any
mutation, activation/processing, buffers, controller/connection point, changed
fixture/claim, redesigned DX0 transaction, or another ownership domain.

After future authority merge/readback, implementation topology is exactly one
14-path source commit and one five-path evidence-only child. A fresh-context
pre-PR audit must verify immutable authority, source/build/Deck/renderer
rosters, transitive imports, canonical JSON/hash closure, private-result P/E
truth without C, tracked-packet P/E/C truth, the exact `TRANSACTION.json` key
roster, 16 proof dispositions, actual effect counts, writer-failure
containment, shutdown/cleanup/protected equality, exact two-commit topology,
and no binaries, SDK source, secrets, private locations,
host/network/process/pointer identifiers, or proprietary state. Evidence is
allow-listed UTF-8, bounded, NUL-free, and hashed. No approval or implementation
is implied by this proposed card.
