# WA0 implementation design v1

```yaml
slice: WA0
design_revision: wa0-design-v1
design_status: proposed_for_adversarial_review
implementation_authorized: false
```

## Authority, basis, and exact fixture

This card refines only the boundary selected in [the WA0 selection
receipt](SLICE_SELECTION.md). It is design authority for independent review,
not implementation authority.

The exact selection basis is commit
`68f52e5b9678d87b13547d3bb37078a0e6a1255c`, tree
`776a01340eca9ec456b9c063cf9201d31bd9f3e3`. The unmodified design activation
is commit `5f647b949d429403e3c6e5f2ff91a1eb0104da6a`, tree
`0fa2adda90dbbd68d6862240061604ddc0d1c396`, with that selection basis as its
only parent.

The positive fixture is the accepted AGain module at SHA-256
`60aa9ff6b9918d4330449e7b3ab34b588dd93cba09f37413a3cd91f6e7d2e18f`.
Its processor logical CID is `84E8DE5F92554F5396FAE4133C935A18`; its raw
Windows TUID bytes are `5FDEE8845592534F96FAE4133C935A18`. The existing
component interface is `Steinberg::Vst::IComponent`, logical IID
`E831FF31F2D54301928EBBEE25697802`, raw Windows TUID
`31FF31E8D5F20143928EBBEE25697802`.

The admitted interface is `Steinberg::Vst::IAudioProcessor`, logical IID
`42043F99B7DA453CA569E79D9AAEC33D`, raw Windows TUID
`993F0442DAB73C45A569E79D9AAEC33D`. Logical FUID text and the 16 raw Windows
TUID bytes are separate typed values. Pointer equality is neither required nor
retained: C++ multiple inheritance may adjust the interface pointer.

## Primary claim and absolute ceiling

On the exact accepted Steam Deck fixture, the existing initialized AGain
processor component exposes the mandatory `Steinberg::Vst::IAudioProcessor`
interface through the exact interface query, the repository-owned Windows host
acquires and retires exactly one such interface reference without calling any
audio-processor method, and the accepted WC0 component, factory, module,
process, environment, and protected-state shutdown remains exact.

WA0 establishes only exact IID selection, one interface query, result/output
consistency, one bounded interface lease, one release to the component-owner
baseline, deterministic failure attribution, and the corresponding extension
of WC0 quiescence.

WA0 never calls `setBusArrangements`, `getBusArrangement`,
`canProcessSampleSize`, `getLatencySamples`, `setupProcessing`,
`setProcessing`, `process`, or `getTailSamples`. It does not create or
initialize an edit controller; query or connect `IConnectionPoint`; enumerate
or activate component buses; touch parameters or state; configure processing;
allocate audio buffers; handle audio, events, timing, automation, or presets;
open a GUI/editor; publish a native proxy or C ABI; introduce IPC/shared
memory; run Bitwig or Serum; authorize software; package, sign, or select a
runner; test another plug-in or DAW; or claim general VST3, Windows, Linux, or
product compatibility. It adds no preparatory implementation for those later
boundaries.

## Accepted WC0 and WF0 prerequisites

WA0 consumes, without redesign, the accepted WC0 implementation merge
`cb831c38e1be88f4bb6a0ab6f2fca2d94164891b`, source commit/tree
`9c0096930df86fc5b171cdebec40b306a198316a` /
`e60286740a3aff7589e0dc9bb3b278e68a23374e`, evidence commit/tree
`77bb40dbf35e19fd93b9f79b5286a43d56b9cc21` /
`43fab0b6341e2549775b7f4223965031521ff338`, and 19-record source-manifest
SHA-256 `38699a1d2026cb1078a569dc1997122b0111c78e608f294777dcf4afc49c8b25`.
The accepted scanner and artifact-manifest/cache SHA-256 values are
`51b899b7936921b24265ead9ff12180f14249d49b532ead9a18414559f5f83f7`
and `25bd47471f01ef57b06b3c8232bb6cc7e40c767f281c30186fa5426130f8ae82`.

The immutable accepted WC0 design is revision `wc0-design-v2`, commit
`064db624056f0fdf4daeda3b5ae394ab6210bee2`, tree
`6d7baf83e29704e803a76106c36d346cdbf7be03`, blob
`df31b8467af9dcd97bc06b6afdf5b4b8d6be7018`, SHA-256
`ca68cde6f68b02320e3c950b445aca9db99fdac1301fa7dfb50d510f83c7d78c`.
Review `5092052158` returned `DESIGN_CLEAR`; its review and approval blobs are
`6a8e83b657a951eb55d18266af64f0336439b3aa` and
`b02e62816ee3f22167e190a459a9ced902080f5b`. The merged design-authority
readback is commit `333b66f6aa689f01bb5c025b587ab7e469780524`, tree
`d9aaae75d384c7b29313adde1a71049635615366`.

The accepted WC0 build ran workflow
`.github/workflows/wf0-windows-msvc-build.yml` at blob
`8e456d24ff4af1130cb3ed4e5cabca0e6715b724`, run `33666394555`, attempt `1`.
Artifact `9861033341` was named
`wc0-windows-build-9c0096930df86fc5b171cdebec40b306a198316a-run-33666394555-attempt-1`;
its upload, REST, and downloaded-wrapper digest bytes join at
`db23a2a1781e9eb88dc43fbe9599cb74ff14f7c4f67a2884ca83113a274ff642`.

WA0 reuses the accepted Windows Server 2022 / Visual Studio 2022 / MSVC build
plane, private Mac custody, source-bundle and ordinary SSH handoff, detached
Deck worktree, content-addressed artifact import, Runtime 4 / Proton 11 digest
`2d64df1d36786ca2d0e955c553005423dc2b5bdd714bd0a17872622e33912547`,
disposable environment, process ancestry/timeouts, factory census, component
lifecycle, factory release, `ExitDll`, `FreeLibrary`, and protected-state
comparison. The WA0 positive transaction is the one bounded regression of that
accepted path. No WF0 or WC0 negative matrix is replayed.

## Source reconnaissance

The official VST3 SDK remains root commit
`3cdf9ca5d1f5b1b21e0a86832aa4abe55607bd96`, tree
`38343890fd1a0cedd48b7ec80ef17da15231b6c8`, with accepted submodules
`base@fcf9da0bd27a16f7f03773a3a39822f28f5c8477` and
`pluginterfaces@4f547e8e102b47de4a8b8aaf343c73b700786372` and
`public.sdk@586dc5e6c8012c3e4b01c79389375cbe96bdb1da` (plus the unchanged
accepted `cmake`, `doc`, `tutorials`, and `vstgui4` pins).

Exact readback established:

- `pluginterfaces/vst/ivstaudioprocessor.h`, blob
  `2a5428ceb3fd532a1e509a4a6a6dae4cda5191a3`, SHA-256
  `6289b19c8300fb381da7688414fae52d5ca139371f910830532204b860bd6549`,
  declares `IAudioProcessor` mandatory and declares the logical IID above.
- `pluginterfaces/base/funknown.h`, blob
  `3f6de83b104484e09097411417b37bcd28a46a1c`, says a successful
  `queryInterface` must add one reference, the initial object reference count
  is one, and `release` returns the new count. Its Windows `INLINE_UID`
  representation yields the raw TUID above.
- `public.sdk/source/vst/vstaudioeffect.h`, blob
  `818cc4f3357c15c6f9a5ba649dbc70e87ced688f`, has `AudioEffect` inherit
  `Component` and `IAudioProcessor`, exposes `IAudioProcessor` through
  `DEF_INTERFACE`, and delegates refcount methods to `Component`.
- `public.sdk/source/vst/vstaudioeffect.cpp`, blob
  `2c3535be777ec777c5f066e42807092168019310`, SHA-256
  `997ff3d44bb9927def01d26c23a2aa11331d974921ee4a78b02e7345449e705b`,
  adds no separate audio-interface reference owner.
- `public.sdk/samples/vst/again/source/again.h` / `again.cpp`, blobs
  `061c0d4afb5f59410e75681d9998871f32fb1e72` /
  `4676454679e37f188b99c2ec6e6def6b825da173`, show AGain derives directly
  from `AudioEffect`; its factory creator returns the new object through an
  `IAudioProcessor` view. The implementation does not override the inherited
  interface-query/refcount law.

The accepted WC0 evidence independently proves the created component owner has
one reference before final retirement and its final `IComponent::release`
returns zero. Therefore, on the exact positive object, the successful audio
interface query raises the shared count from one to two, the one
`IAudioProcessor::release` returns one, and the later WC0 component release
returns zero. This relationship is source- and evidence-bound; pointer-address
equality is irrelevant and prohibited.

The existing `ComponentInstanceSession` already owns the only legal interval
for this operation: after successful initialize and before terminate. The
existing event writer is operation-generic and synchronously flushed, and the
supervisor already maps an unmatched operation to timeout/crash containment.
No second executable, event protocol, supervisor, process owner, or runtime is
needed.

## New owner: `AudioProcessorInterfaceLease`

WA0 adds exactly one owner, implemented within the existing
`component_instance_session.h/.cpp` boundary. It borrows the live
`IComponent`; it never owns the component, factory, module, host context,
process tree, or environment.

It owns:

- the exact logical and raw `IAudioProcessor` IID;
- a null-initialized output slot and the one query result;
- the returned interface reference whenever the output is non-null, including
  the anomalous failure/non-null tuple;
- the lease state, query/release attempt identities, and first WA0 failure;
- exactly one permitted cleanup release of an owned interface reference;
- the ordinary release count, local pointer clearance, and the
  `audio_interface_quiescence` result.

It exposes only a bounded result back to the existing component session. It
does not call an `IAudioProcessor` method, compare interface addresses, perform
generic COM discovery, retain arbitrary interfaces, or become a reusable
interface registry.

## Closed lifecycle and positive order

The closed state set has exactly eight values:

1. `audio_processor_absent` — no query has run and no lease exists.
2. `audio_processor_query_in_flight` — the exact query attempt is unmatched.
3. `audio_processor_query_returned_without_lease` — an ordinary return proved
   a null output; result and consistency remain separately recorded.
4. `audio_processor_lease_acquired` — a non-null output is owned, whether the
   result was success or the anomalous failure/non-null tuple.
5. `audio_processor_release_in_flight` — the sole release attempt is unmatched.
6. `audio_processor_lease_retired` — release returned ordinarily with count
   one and the local interface pointer was cleared.
7. `audio_processor_retirement_incomplete` — release returned ordinarily with
   any count other than one; the pointer is cleared and unusable, but correct
   object ownership is unproved.
8. `audio_processor_ownership_unknown` — query or release timed out/crashed, or
   its paired completion/pointer disposition cannot be proved; only process
   containment may follow.

The positive sequence is exact:

```text
accepted WC0 component_initialized
    -> null-initialize one void* output slot
    -> call queryInterface(exact IAudioProcessor raw TUID)
    -> require kResultOk and non-null output
    -> own exactly one AudioProcessorInterfaceLease
    -> call release_audio_processor exactly once
    -> require ordinary return 1
    -> clear and prohibit all later use of the interface pointer
    -> prove audio_interface_quiescence=true
    -> accepted WC0 terminate exactly once
    -> accepted host reference returns to baseline and owner release returns 0
    -> accepted IComponent release returns 0
    -> accepted reverse factory releases, ExitDll, and FreeLibrary
    -> accepted process drainage and environment retirement
```

The two and only two new closed plug-in call operations are
`query_audio_processor` and `release_audio_processor`. Each runs synchronously
on the existing declared Windows scanner main thread. Immediately before the
call, the scanner emits and flushes `call_started` with exact operation, object
role, and typed IID where applicable. After an ordinary return it immediately
emits and flushes the paired `call_completed` before any lifecycle event or
later call attempt. The total accepted closed call roster becomes 22: 15 WF0,
five WC0, and two WA0 operations.

No host callback is expected or introduced by these two operations. The
existing bounded callback ledger remains the separate WC0 owner. Its count and
health must be unchanged across the WA0 interval; a callback record attributed
to either WA0 operation is an output-contract failure, not a reason to design a
general callback or reentrancy protocol.

## Query result/output ownership matrix

The output slot is set to null before the sole query and is never reused.

| Ordinary query result | Output | Ownership and disposition |
|---|---|---|
| success | non-null | Exactly one lease is acquired. Continue only through its one release. |
| failure | null | No lease exists. Retain `WA0_INTERFACE_QUERY_BLOCKED`; WC0 teardown may continue if all other quiescence facts remain provable. |
| success | null | Inconsistent tuple, no lease. Retain `WA0_INTERFACE_QUERY_INCONSISTENT`; WC0 teardown may continue because absence is proved. |
| failure | non-null | Inconsistent tuple and one returned reference. Own it as cleanup state, retain `WA0_INTERFACE_QUERY_INCONSISTENT`, and release it exactly once before any WC0 teardown. |

A non-null output is never discarded because the result failed. Query is never
retried. The first material failure remains primary even if cleanup later
fails.

## Interface retirement and extended quiescence

`audio_interface_quiescence=true` has one exact predicate:

```text
no query or release call remains in flight
and (
  an ordinary query returned a null output and no lease was ever acquired
  or
  exactly one non-null lease was acquired,
  exactly one release returned ordinarily with reference count 1,
  and the local audio-processor pointer was cleared and is unusable
)
```

The result/output consistency flag is not part of physical quiescence: a
failure/null or success/null path can prove absence while retaining its WA0
primary blocker, and a failure/non-null path can prove retirement after its
one cleanup release returns one. Success still requires success/non-null.

Accepted WC0 terminate and all later in-process retirement are gated by that
predicate. The accepted WC0 nine-fact object-quiescence predicate gains
`audio_interface_quiescence=true` as a tenth required fact.

An ordinary release result of zero is invalid because it contradicts the
separate live `IComponent` owner; a result above one does not return to its
baseline. Either reaches `audio_processor_retirement_incomplete`. A missing
paired completion, unexpected count, release timeout/crash, remaining in-flight
call, or unproved pointer disposition makes interface quiescence false or
unknown. The local pointer is never called through after any ordinary release,
and release is never retried.

When interface quiescence is false or unknown, the scanner does not call WC0
terminate, does not release `IComponent`, does not release factory interfaces,
and does not call `ExitDll` or `FreeLibrary`. Every suppressed operation is
recorded as `not_attempted_audio_interface_quiescence_unproved`; the first WA0
blocker is preserved. The accepted process owner drains the exact process tree
and retires the exact disposable environment. Physical disappearance is not a
clean interface, component, factory, or module retirement claim.

## Failure precedence, timeout, and crash law

An accepted WC0 failure before `component_initialized` remains its existing
owner and WA0 does not run. Once WA0 starts, precedence is:

1. first query failure or result/output inconsistency;
2. release failure if no earlier query failure exists;
3. evidence/output failure;
4. process/environment cleanup failure as a secondary cleanup fact, unless no
   earlier failure exists;
5. protected-state drift, which always blocks acceptance and is never repaired.

Failure/non-null keeps the query inconsistency primary even if the required
cleanup release fails. A cleanup failure never erases an earlier query failure.

For an unmatched `query_audio_processor`, output ownership is unknowable; no
release or WC0 in-process teardown is invented. For an unmatched
`release_audio_processor`, retirement is unknowable; no later in-process call
is attempted. The supervisor maps the exact unmatched operation to query or
release ownership, distinguishes timeout from abnormal process termination,
retains the last in-flight operation, drains the process tree, and claims only
physical containment. No post-crash state is fabricated.

## Focused fault and mutation ledger

The unchanged AGain module is the sole positive fixture. The existing
source-owned fault module gains only eight WA0 modes:

| Fixture | Controlled mutation | Required outcome |
|---|---|---|
| `wa0-query-failure-null` | exact IID returns failure/null | query blocker; no release; permitted WC0 teardown |
| `wa0-query-success-null` | exact IID returns success/null | inconsistency blocker; no release; permitted WC0 teardown |
| `wa0-query-failure-nonnull` | returns failure plus one owned interface ref | inconsistency remains primary; exactly one cleanup release |
| `wa0-query-hang` | exact query does not return | unmatched query; no later in-process call |
| `wa0-query-crash` | exact query terminates the process | unmatched query; containment only |
| `wa0-release-unexpected-count` | interface release returns two | retirement incomplete; all WC0/module teardown suppressed |
| `wa0-release-hang` | interface release does not return | unmatched release; containment only |
| `wa0-release-crash` | interface release terminates the process | unmatched release; containment only |

The fault component implements the interface only so the exact query and
ownership tuples can be produced. Every `IAudioProcessor` method body is a
stage-local forbidden-call tripwire; none is invoked. Deterministic owner tests
also reject release counts zero and greater than one, without adding more live
fixtures. No WC0 host-context, initialize, terminate, component-release,
factory, module, archive, custody, or handoff fault family is replayed.

## Exact identity and custody ledger

Future implementation, if separately authorized, uses branch
`codex/wa0-windows-vst3-audio-processor-interface-admission` and ref
`refs/heads/codex/wa0-windows-vst3-audio-processor-interface-admission`.
Its source commit is one direct child of the exact merged WA0
design-authority/readback basis named by later approval.

The implementation-source schema is
`linux-vst-bridge-wa0-implementation-source/v1`, with exactly 17 records. Each
record contains only `path`, `git_mode`, and `git_blob`. The workflow remains
`.github/workflows/wf0-windows-msvc-build.yml`, triggers only for that exact
branch and this exact source roster, and uploads one artifact named
`wa0-windows-build-<source-commit>-run-<run-id>-attempt-<run-attempt>`.

The source bundle is `wa0-execution-source-<source-commit>.bundle`; its sole
advertised and Deck-local ref is
`refs/handoff/wa0-source/<source-commit>`. Source handoff uses:

```text
linux-vst-bridge-wa0-source-handoff/v1
WA0_SOURCE_HANDOFF_RECEIPT.json
WA0_SOURCE_HANDOFF_RECEIPT.sha256
```

Evidence handoff uses:

```text
linux-vst-bridge-wa0-evidence-handoff/v1
WA0_EVIDENCE_HANDOFF_RECEIPT.json
WA0_EVIDENCE_HANDOFF_RECEIPT.sha256
```

The Mac selects the exact run and artifact ID, never latest or name-only, and
binds repository, workflow path/blob, exact branch/ref, source commit/tree/
parent, 17-record source-manifest digest, run ID/attempt, artifact ID/name,
upload-action bare digest, REST `sha256:` digest, and downloaded-wrapper
SHA-256. The source bundle/receipt, Deck ref/detached worktree, artifact cache,
run receipts, and evidence handoff all reproduce that one source identity. No
evidence from different source commits, runs, artifacts, or worktrees combines.

These inherited WF0 wire identities and algorithms remain unchanged:

```text
linux-vst-bridge-wf0-windows-build/v1
linux-vst-bridge-wf0-artifact-manifest/v1
linux-vst-bridge-wf0-bundle-manifest/v1
linux-vst-bridge-wf0-mac-artifact-custody/v1
linux-vst-bridge-wf0-scan-environment/v1
linux-vst-bridge-wf0-artifact-cache-owner/v1
wf0-payload.zip
WF0_WINDOWS_BUILD_RECEIPT.json
WF0_WINDOWS_BUILD_RECEIPT.sha256
WF0_MAC_ARTIFACT_CUSTODY_RECEIPT.json
.wf0-artifact-owner.json
C:\wf0\bin\wf0-factory-probe.exe
C:\wf0\fixture\again.vst3\
C:\wf0\session\
.wf0-factory-census.stage-<32-lowercase-hex-run-id>
```

This retention is deliberate reuse, not a claim that WA0 is WF0. New receipts
must embed the exact WA0 source identity. No new workflow, custody system,
transfer system, supervisor, executable, service, manager, broker, or runtime
is introduced; the Deck performs no GitHub operation.

## Exact future implementation paths

The exact raw-UTF-8-sorted source/configuration roster has 17 paths:

```text
.github/workflows/wf0-windows-msvc-build.yml
cmake/WF0DependencyLock.cmake
tools/wf0-factory-census/README.md
tools/wf0-factory-census/artifacts.py
tools/wf0-factory-census/build.py
tools/wf0-factory-census/common.py
tools/wf0-factory-census/evidence.py
tools/wf0-factory-census/negative_tests.py
tools/wf0-factory-census/normalize.py
tools/wf0-factory-census/run.py
tools/wf0-factory-census/supervise.py
tools/wf0-factory-census/verify.py
windows-factory-probe/source/component_instance_session.cpp
windows-factory-probe/source/component_instance_session.h
windows-factory-probe/source/main.cpp
windows-fixtures/wf0/CMakeLists.txt
windows-fixtures/wf0/source/fault_fixture.cpp
```

The workflow and common/build/custody tools bind the WA0 branch, 17-record
manifest, focused targets, and receipts. `WF0DependencyLock.cmake` adds the two
new exact SDK source locks above. The README states the bounded operator
contract. Component-session source houses and sequences the one new lease
owner; `main.cpp` exposes only the WA0 mode and two operations. The existing
normalizer, supervisor, verifier, runner, and evidence renderer gain only WA0
states, attribution, proof, and identity joins. The existing fixture CMake and
source add the eight fault modes.

`windows-factory-probe/CMakeLists.txt` is unchanged because no source file or
target is added. `events.h` is unchanged because its accepted operation-generic
synchronous event contract already carries both new operation names.
`environment.py` is unchanged because the accepted marker/containment owner is
reused. A need to change any other source/configuration path returns to design.

The exact evidence packet path is
`evidence/wa0-windows-vst3-audio-processor-interface-admission/`; its 14 paths
are:

```text
evidence/wa0-windows-vst3-audio-processor-interface-admission/BASIS.md
evidence/wa0-windows-vst3-audio-processor-interface-admission/BUILD.md
evidence/wa0-windows-vst3-audio-processor-interface-admission/BUILD_MANIFEST.json
evidence/wa0-windows-vst3-audio-processor-interface-admission/ENVIRONMENT.md
evidence/wa0-windows-vst3-audio-processor-interface-admission/LAUNCH_AND_PROCESS.md
evidence/wa0-windows-vst3-audio-processor-interface-admission/AUDIO_PROCESSOR_LEASE.json
evidence/wa0-windows-vst3-audio-processor-interface-admission/COMPONENT_SESSION.json
evidence/wa0-windows-vst3-audio-processor-interface-admission/STAGE_TIMELINE.json
evidence/wa0-windows-vst3-audio-processor-interface-admission/NEGATIVE_TESTS.md
evidence/wa0-windows-vst3-audio-processor-interface-admission/PRESERVATION.md
evidence/wa0-windows-vst3-audio-processor-interface-admission/FINDINGS.md
evidence/wa0-windows-vst3-audio-processor-interface-admission/SANITIZATION.md
evidence/wa0-windows-vst3-audio-processor-interface-admission/fixture.json
evidence/wa0-windows-vst3-audio-processor-interface-admission/hashes.sha256
```

`AUDIO_PROCESSOR_LEASE.json` owns the new machine-readable claim.
`COMPONENT_SESSION.json` retains only the narrow accepted-WC0 regression,
including callback-ledger closure; the complete historical WC0 packet is not
duplicated.

## Focused proof matrix

The exact matrix has 20 rows:

| # | Claim or failure owner | Required proof |
|---:|---|---|
| 1 | Accepted prerequisite | Exact WC0 source/evidence/scanner/artifact and Runtime/Proton identities read back before execution. |
| 2 | Exact interface identity | Locked logical IID and raw Windows TUID; no pointer-equality evidence. |
| 3 | Exact ordering | Query follows `component_initialized`; interface retirement precedes WC0 terminate. |
| 4 | Query output law | Output was null-initialized and all four ordinary result/output tuples normalize exactly. |
| 5 | Positive acquisition | AGain success/non-null creates exactly one lease and no second query. |
| 6 | Positive reference law | Audio release occurs once and returns 1; later component release returns 0. |
| 7 | Extended quiescence | Pointer cleared, no WA0 call in flight, and audio quiescence is the tenth WC0 gate fact. |
| 8 | Claim ceiling | Static call surface plus fixture tripwires prove no `IAudioProcessor` method or controller path ran. |
| 9 | Failure/null query | No lease or release; exact query blocker; permitted WC0 cleanup. |
| 10 | Success/null query | Exact inconsistency blocker; absence proved; permitted WC0 cleanup. |
| 11 | Failure/non-null query | Returned reference owned and released once; inconsistency remains primary. |
| 12 | Query timeout | Unmatched `query_audio_processor`; no later in-process call; physical containment. |
| 13 | Query crash | Same exact attribution under abnormal termination. |
| 14 | Unexpected release count | Count 2 live fixture plus deterministic rejection of 0 and values above 1; retirement incomplete. |
| 15 | Release timeout | Unmatched `release_audio_processor`; no WC0/module teardown. |
| 16 | Release crash | Same exact attribution under abnormal termination. |
| 17 | Shutdown suppression | Every unproved-retirement case suppresses terminate, component/factory release, `ExitDll`, and `FreeLibrary`. |
| 18 | Bounded WC0 regression | Positive AGain retains host 1 -> 2 -> 1 -> 0, component release 0, reverse factory release, exit, and unload. |
| 19 | Physical cleanup | All nine live exercises drain owned descendants and retire exact disposable environments; blocked cases make no clean-shutdown claim. |
| 20 | Identity, preservation, evidence | One 17-record identity joins source/build/custody/Deck/evidence; protected state remains exact and the 14-file packet validates. |

## Sanitization and privacy

Evidence is allow-list based, canonical UTF-8, size bounded, NUL-free, and
hash-closed. It may retain public commit/tree/blob hashes, source/artifact
digests, typed public Actions run/artifact identifiers, logical IDs, raw TUID
bytes, return codes/counts, bounded operation/state names, and sanitized fixture
facts. It retains no pointer/address/handle, OS process or thread identifier,
hostname, IP or MAC address, username/home path, credential/token/cookie/SSH
material, private staging path, binary, SDK source, source bundle, artifact ZIP,
compatdata, proprietary state, installer, or vendor material. A pointer is
represented only by `null`/`non-null` and later `cleared`/`unusable` facts.

## Blocked results and material discoveries

WA0 defines exactly 12 outcomes; accepted WC0/WF0 prerequisite failures keep
their accepted exact blockers rather than being relabeled.

- `WA0_DESIGN_PREFLIGHT_BLOCKED`: design basis, activation, branch, path, or authority differs.
- `WA0_DESIGN_BLOCKED`: a non-material design-environment problem prevents the exact card.
- `WA0_MATERIAL_DESIGN_DISCOVERY`: reconnaissance contradicts the selected owner, ordering, fixture, or claim.
- `WA0_SOURCE_IDENTITY_BLOCKED`: branch, source topology/manifest, bundle/ref, or Deck worktree identity differs.
- `WA0_BUILD_CUSTODY_BLOCKED`: exact workflow/build/artifact/custody identity or bytes differ.
- `WA0_INTERFACE_QUERY_BLOCKED`: the exact query fails, times out, crashes, is absent/repeated/misordered, or has an unmatched call record.
- `WA0_INTERFACE_QUERY_INCONSISTENT`: success/null or failure/non-null is observed; returned ownership still follows the matrix.
- `WA0_INTERFACE_RELEASE_BLOCKED`: release is missing/repeated/misordered, returns other than 1, times out, crashes, or lacks pointer retirement.
- `WA0_PROCESS_CLEANUP_BLOCKED`: accepted process/environment containment does not reach exact absence.
- `WA0_PROTECTED_FIXTURE_DRIFT`: any protected projection differs; no repair is attempted.
- `WA0_EVIDENCE_BLOCKED`: evidence roster, normalization, identity join, hash, or sanitization differs.
- `RETURN_TO_DESIGN_GATE`: implementation needs a material boundary change.

Return to design if implementation requires an `IAudioProcessor` method,
controller object, `IConnectionPoint`, bus/parameter/state/processing/audio/
event/editor work, IPC/proxy/C ABI, another owner or executable, build/custody/
transfer/runtime redesign, Deck reconnaissance, another fixture, another DAW,
Bitwig or Serum execution, a path outside the exact 17+14 envelope, or a wider
claim. An ordinary defect in the lease owner, existing event/supervisor path,
focused fixture, normalizer, or evidence renderer is an implementation repair.

## Future implementation audit and completion topology

Implementation remains prohibited until this exact card receives fresh
independent adversarial review, an explicit approval receipt, and an exact
merged authority/readback basis. If later authorized, completion has exactly
two substantive commits above that basis:

```text
merged WA0 design-authority/readback basis
    -> one source commit: exactly 17 source/configuration paths, zero evidence
    -> one evidence commit: exactly 14 evidence paths, zero source changes
```

The 31-path cumulative delta, source manifest, workflow/build/custody joins,
Deck source and artifact admission, nine focused executions, evidence hashes,
sanitization, clean worktrees, and protected-state equality require a fresh
independent pre-PR implementation audit at the exact evidence head. Only after
that audit may one ordinary non-draft implementation PR target `main`; it stays
open and unmerged pending exact-head technical-lead review. No status closure
or successor selection is implied.

`implementation_authorized=false`
