# WC0 implementation design v1

```yaml
slice: WC0
design_revision: wc0-design-v1
design_status: proposed_for_adversarial_review
implementation_authorized: false
```

## Authority and exact fixture

This card implements the selected boundary in [the WC0 selection
receipt](SLICE_SELECTION.md); it does not authorize implementation.

Selection basis is commit
`1f4b946178319887950bdb764114543a1a7b845b`, tree
`75b91e9f4d8b059aef76abeb9ff6518cf0c310bc`. Design activation is commit
`b6292a505666db8fc646fba35406dff7382d3e67`, tree
`63b09755f9870253d31487cce1311f786ddfa15f`.

The positive fixture is the accepted AGain module, SHA-256
`60aa9ff6b9918d4330449e7b3ab34b588dd93cba09f37413a3cd91f6e7d2e18f`:

- processor logical CID `84E8DE5F92554F5396FAE4133C935A18`, raw Windows TUID
  `5FDEE8845592534F96FAE4133C935A18`;
- requested `Steinberg::Vst::IComponent` logical IID
  `E831FF31F2D54301928EBBEE25697802`, raw Windows TUID
  `31FF31E8D5F20143928EBBEE25697802`;
- expected controller logical CID `D39D5B65D7AF42FA843F4AC841EB04F0`, raw
  Windows TUID `655B9DD3AFD7FA42843F4AC841EB04F0`.

The official VST3 SDK remains root commit
`3cdf9ca5d1f5b1b21e0a86832aa4abe55607bd96`, tree
`38343890fd1a0cedd48b7ec80ef17da15231b6c8`, with exact recursive submodules:
`base@fcf9da0bd27a16f7f03773a3a39822f28f5c8477`,
`cmake@054c9143cbb8d47fc4694e473f2ee3b4d951a8f5`,
`doc@8bfca19d3b76a61d093951ba9297047f544caea1`,
`pluginterfaces@4f547e8e102b47de4a8b8aaf343c73b700786372`,
`public.sdk@586dc5e6c8012c3e4b01c79389375cbe96bdb1da`,
`tutorials@33b73dfbb87f3fde3bce8c0a10cae934dc66ad34`, and
`vstgui4@5db272256172557818b6158cf0bb2c4410bddb25`.

The exact definition readback used these immutable blobs:

| Repository | Path | Git blob |
|---|---|---|
| `pluginterfaces` | `base/ipluginbase.h` | `859424bfc7f14b61df4b209a85c09d413513127a` |
| `pluginterfaces` | `base/funknown.h` | `3f6de83b104484e09097411417b37bcd28a46a1c` |
| `pluginterfaces` | `base/smartpointer.h` | `ca64ae8f6260abc4225869bcb55ef6b27f3f3abf` |
| `pluginterfaces` | `vst/ivsthostapplication.h` | `1818efe85a6674bf706fd8f8040e28cb329fbe27` |
| `pluginterfaces` | `vst/ivstcomponent.h` | `e20ef5f429349bdccf55367d45051fcdb0ff97c5` |
| `base` | `source/fobject.h` / `source/fobject.cpp` | `6d092acfcc7bf3be91f8f00bcb3e6c27184d53eb` / `a1da6cfffb23eb77d53d39da9429709f246955e6` |
| `public.sdk` | `source/main/pluginfactory.cpp` | `a50c5000c1215ced5ae920a07c12153edb9b2498` |
| `public.sdk` | `source/vst/vstcomponentbase.cpp` / `source/vst/vstcomponent.cpp` | `ccfde59797185b8a793b4a5bff2246c711faa18a` / `d1dced4a441d35b73717da26eea0ada97406a874` |
| `public.sdk` | `source/vst/vstaudioeffect.h` / `source/vst/hosting/hostclasses.cpp` | `818cc4f3357c15c6f9a5ba649dbc70e87ced688f` / `fd0e12498e7f9035e6d9f23c154cf50adfd8a6ab` |
| `public.sdk` | `samples/vst/again/source/again.h` / `again.cpp` | `061c0d4afb5f59410e75681d9998871f32fb1e72` / `4676454679e37f188b99c2ec6e6def6b825da173` |
| `public.sdk` | `samples/vst/again/source/againentry.cpp` / `againcids.h` | `13b920b4b7a74137301bf213cf048e96e82861d4` / `d32d1640ea187e718baba9cbb039d3a436d53cce` |

## Primary claim and ceiling

On the exact accepted Steam Deck fixture, the existing supervised Windows VST3
host path creates the exact AGain processor class as one `IComponent`, verifies
its declared controller class ID, initializes it with a minimal repository-owned
`IHostApplication`, terminates and releases it correctly, and then completes the
already-proven factory/module shutdown with no remaining object, process,
environment, or protected-state residue.

WC0 establishes only exact processor selection, one `createInstance` requesting
`IComponent`, result/output consistency, `getControllerClassId`, the minimal host
context, initialize, terminate, reference release, and exact failure attribution.
Its new object-model boundary stops after processor and host-object retirement;
only the already-accepted factory/module shutdown and physical cleanup follow.

It neither calls nor establishes edit-controller creation or initialization,
processor/controller connection, `IConnectionPoint`, `IAudioProcessor` census,
bus enumeration/activation, parameters, state, processing setup, `setActive`,
`setProcessing`, `process`, events, audio, GUI/editor behavior, a native proxy,
C ABI, IPC, shared memory, Bitwig or Serum execution, authorization, packaging,
runner selection, another plug-in, or general compatibility. AGain internally
creates bus records during its own `initialize` and removes them during its own
`terminate`; WC0 invokes and observes no bus method and makes no bus claim.

## Accepted WF0 prerequisites

The accepted WF0 identity is implementation merge
`e694cc84344394553c4a3eff6b13f34226b368ae`, source commit/tree
`8b76ab886fd75079c72e3f820781beb5d1b36ae9` /
`829aae74e221a169ccfbb46387004b5edb04ba37`, evidence commit/tree
`0096010a36ebf31a36149064d64059142ce7cfed` /
`58401f5b9d3caab9ffe53155fb2f0517d1801427`, and 26-record source-manifest
SHA-256 `03c3c017f7d6eb357ae657e992ef3c3988932f6870ac3a18192dfd9e343ee05f`.
Its accepted workflow is `.github/workflows/wf0-windows-msvc-build.yml`, Git
blob `95757dc1746bb2df59418b0ed06f5f40544dd3a3`, run `33601279364`, artifact ID
`9835459546`, artifact-manifest SHA-256
`217d38dddb5e8ae4ee6b60245cc03b3174710cf2692e2e6e1a4c4ea1c04b7684`, and
scanner SHA-256 `36643c2447811b52e1ad1455eb47e0ced9b5f5bf03849f9625d80979d7c5798f`.
Upload, REST, and raw-wrapper digest bytes join at
`7ccb8f53aee02748d98abaa641a53e479ebd6f1d7b8bf381c1c2f11644d44d53`.

WC0 reuses that exact Windows 2022/MSVC build plane, private Mac custody,
source-bundle and SSH handoff, detached Deck worktree, content-addressed artifact
admission, Runtime 4 / Proton 11 digest
`2d64df1d36786ca2d0e955c553005423dc2b5bdd714bd0a17872622e33912547`,
disposable environment, process ancestry/timeouts, factory census/release,
`ExitDll`, `FreeLibrary`, and protected-state comparison. Their schemas and
owners do not change. The WC0 positive transaction includes one invocation of
the accepted factory-census prefix and shutdown suffix as the sole inherited
WF0 regression; no WF0 fault family or 62-row matrix is replayed.

## New owners

Exactly two owners are added.

### `ComponentInstanceSession`

This session borrows the already-owned factory. It owns the fixed processor CID,
requested `IComponent` IID, null-initialized output slot, exact `tresult`, one
returned component reference when present, the 12-state lifecycle, controller
TUID, initialize/terminate/release results, immutable first-primary failure, and
ordered secondary cleanup failures. It never owns the module, factory, process,
environment, controller, buses, parameters, state, audio, or editor.

### `MinimalHostApplication`

This is one repository-owned `IHostApplication` object with canonical
`FUnknown` identity, exact name `Linux VST Bridge WC0`, initial owner reference
count 1, and a fixed 64-record callback ledger. It owns `queryInterface`,
`addRef`, `release`, `getName`, and `createInstance` behavior and its own final
retirement. It exposes only `FUnknown` and `IHostApplication`; it creates no
message, attribute list, component, controller, view, or other host object.

## Lifecycle and reference law

The component state is one of exactly 12 values:

```text
component_absent
component_create_in_flight
component_created
controller_id_in_flight
controller_id_verified
host_context_ready
component_initialize_in_flight
component_initialized
component_terminate_in_flight
component_terminated
component_release_in_flight
component_released
```

The success path is:

```text
accepted module, census, and factory ready
  -> null output; create_component(exact processor CID, exact IComponent IID)
  -> require kResultOk and non-null; own one IComponent reference
  -> get_controller_class_id before initialize
  -> require kResultTrue and exact controller raw TUID
  -> construct host at reference baseline 1
  -> initialize_component exactly once; require kResultOk
  -> require host reference count 2
  -> terminate_component exactly once; require kResultOk
  -> require host reference count returned to baseline 1
  -> release_component exactly once; require return 0
  -> recheck host baseline 1; retire owner reference; require return 0
  -> inherited factory release 2, 1, 0; ExitDll; FreeLibrary; physical cleanup
```

The pinned SDK makes both zero expectations exact. `FObject` starts at 1.
AGain's factory creator returns that reference, `queryInterface(IComponent)`
adds one, and the factory drops its temporary reference, leaving exactly one
returned component reference. No WC0 action adds another, so final component
`release` must return 0. `ComponentBase::initialize` stores the context in an
`IPtr` and calls host `addRef` (1 to 2); `terminate` clears it and calls host
`release` (2 to 1). AGain delegates those operations and makes no host-object
request. The host owner's final local release then returns 0.

Each stable state plus the immutable failure latch states physical truth. An
ordinary failed controller-ID call returns to `component_created`; a failed
initialize returns to `host_context_ready`; a failed terminate leaves
`component_initialized`. Each then transitions through `component_release_in_flight`.
An in-flight crash or timeout has no invented transition.

Create accepts only `(kResultOk, non-null)`. Failure/null and success/null are
blocked with no component call. Failure/non-null is a contradictory physical
reference: it latches `WC0_COMPONENT_CREATE_BLOCKED`, calls no controller or
plugin-base method, and attempts exactly one cleanup `release_component`; that
cleanup cannot replace the create failure.

## Minimal host contract and attribution

`IHostApplication` is logical IID `58E595CCDB2D49698B6AAF8C36A664E5`, raw
Windows TUID `CC95E5582DDB69498B6AAF8C36A664E5`; `FUnknown` is
`0000000000000000C000000000000046` in both forms. `queryInterface` first nulls
a valid output. It returns the same canonical identity, calls `addRef`, and
returns `kResultOk` only for those two IIDs; all other IIDs return null and
`kNoInterface`.
`getName` writes the exact NUL-terminated name to `String128` and returns
`kResultOk`. Every `createInstance` request is outside WC0: it nulls the output,
records the CID/IID, latches `WC0_HOST_CONTEXT_BLOCKED`, and returns the pinned
SDK host's unsupported result `kResultFalse`. No exception may cross the SDK
ABI. On Windows, the retained result encodings are `kResultOk`/`kResultTrue` =
`00000000`, `kResultFalse` = `00000001`, and `kNoInterface` = `80004002`.

The scanner's initial Windows main thread is the only host-to-plug-in call
thread. Raw thread IDs are compared internally and normalized to
`scanner_main_thread`; none is retained. There is no worker, callback transport,
or reentrancy protocol.

Five new closed operations exist: `create_component`,
`get_controller_class_id`, `initialize_component`, `terminate_component`, and
`release_component`. Each emits and flushes `call_started`, binds object role
and operation, calls synchronously, then immediately emits and flushes the paired
`call_completed` after an ordinary return and before another lifecycle event.

Host callbacks are separate `host_callback` records tied to the enclosing call
attempt and synchronously flushed before callback return. Recording is bounded,
non-throwing across the ABI, and distinguishes `queryInterface`, `addRef`,
`release`, `getName`, and `createInstance`. The positive AGain ledger requires
one component-originated `addRef` during initialize, one component-originated
`release` during terminate, and zero other callbacks. The owner-retirement
release is labeled local, not a plug-in callback. Overflow, wrong-thread access,
unsupported interface access, or callback-output failure latches
`WC0_HOST_CONTEXT_BLOCKED`.

Supervisor timeout/crash attribution is exact: each unmatched operation maps to
its corresponding create, controller-ID, initialize, terminate, or release
blocked result. Process cleanup owns physical containment after an abnormal
return; no later in-process release, terminate, factory release, exit, or unload
is claimed.

## Failure and cleanup law

- Unknown CID, wrong IID, create failure, or null output calls no controller-ID,
  initialize, or terminate operation. Any actual returned reference follows the
  single anomalous release rule above; then inherited cleanup runs when safe.
- Controller-ID failure or mismatch forbids initialize and releases the component.
- Initialize failure forbids terminate, releases the component immediately as
  required by `IPluginBase`, checks/retire the host, then continues inherited
  cleanup.
- After successful initialize, terminate is attempted exactly once before release,
  even when a host callback has already latched a primary failure.
- Terminate failure remains primary; one release attempt and inherited cleanup
  follow. A nonzero ordinary final release is blocked and is never retried.
- Host reference equality is checked after terminate or, when terminate is
  prohibited, after component release. A mismatch is a host-context failure; one
  owner-reference retirement is attempted, never repeated to force zero.
- A hang or crash leaves its operation in flight. The accepted supervisor drains
  the owned process tree and retires only the exact environment.
- The first create, controller-ID, mismatch, host-context, initialize, or terminate
  failure is immutable. Release, factory, exit, unload, process, and evidence
  failures are ordered secondary facts and never erase it.

## Focused fault family

Two named exercises use AGain directly: unknown processor CID and unsupported
test IID. The existing source-owned fault-fixture file adds exactly this 15-module
roster:

```text
wc0-create-failure-null
wc0-create-success-null
wc0-create-failure-nonnull
wc0-controller-id-failure
wc0-controller-id-mismatch
wc0-initialize-failure
wc0-initialize-hang
wc0-initialize-crash
wc0-terminate-failure
wc0-terminate-hang
wc0-terminate-crash
wc0-release-hang
wc0-release-crash
wc0-host-object-request
wc0-host-reference-leak
```

Their other `IComponent` methods are tripwires. No old WF0 fault module is built
or rerun.

The existing loader adapter gains deterministic owner tests for the host
interface, exact name, unsupported object request, state transitions, result/
pointer tuples, reference counts, first-failure precedence, and forbidden-call
tripwires. It is not a second component-lifecycle executable.

## Exact proposed implementation paths

Future implementation is bounded to exactly 33 tracked paths: 19 source/config
paths and 14 evidence paths.

| Source/configuration path | Reason |
|---|---|
| `.github/workflows/wf0-windows-msvc-build.yml` | Reuse the one workflow; bind the exact WC0 branch/source roster and changed payload. |
| `cmake/WF0DependencyLock.cmake` | Add exact interface/lifecycle source-blob readbacks without changing SDK identity. |
| `tools/wf0-factory-census/README.md` | Replace the now-false stop-before-instantiation description with the closed WC0 mode and retained WF0 boundary. |
| `tools/wf0-factory-census/artifacts.py` | Replace hard-coded WF0 26-record and `wf0-v7` source-handoff identity literals with WC0 identities; custody, archive, and transport algorithms stay exact. |
| `tools/wf0-factory-census/build.py` | Build only the probe, existing adapter, focused WC0 fixtures, and pinned AGain; retain two-build proof. |
| `tools/wf0-factory-census/common.py` | Bind WC0 authority, 19-record source roster, IDs, focused fixtures, and evidence roster. |
| `tools/wf0-factory-census/evidence.py` | Render and validate the focused WC0 packet. |
| `tools/wf0-factory-census/negative_tests.py` | Exercise only new lifecycle faults and deterministic owner laws. |
| `tools/wf0-factory-census/normalize.py` | Normalize component states, results, IDs, references, and callbacks. |
| `tools/wf0-factory-census/run.py` | Orchestrate one inherited regression, focused negatives, positive run, and evidence. |
| `tools/wf0-factory-census/supervise.py` | Admit five calls and callback records; map their timeout/crash owners. |
| `tools/wf0-factory-census/verify.py` | Replace WF0's no-instantiation check with the exact WC0 call-surface and PE/fixture checks. |
| `windows-factory-probe/CMakeLists.txt` | Compile the two new owners into the existing probe and adapter target. |
| `windows-factory-probe/include/linux_vst_bridge/wf0_probe/events.h` | Extend the accepted synchronous event writer with bounded callback records. |
| `windows-factory-probe/source/component_instance_session.cpp` | Implement both narrow new owners and their fixed lifecycle. |
| `windows-factory-probe/source/component_instance_session.h` | Declare both owner contracts and result records. |
| `windows-factory-probe/source/main.cpp` | Add one closed component-admission mode while retaining factory-census mode. |
| `windows-fixtures/wf0/CMakeLists.txt` | Define one focused `wc0-fault-fixtures` build target. |
| `windows-fixtures/wf0/source/fault_fixture.cpp` | Add the focused component faults and forbidden-method tripwires. |

The evidence root is `evidence/wc0-windows-vst3-processor-component-admission/`
with exactly: `BASIS.md`, `BUILD.md`, `BUILD_MANIFEST.json`, `ENVIRONMENT.md`,
`LAUNCH_AND_PROCESS.md`, `COMPONENT_SESSION.json`, `CALLBACK_LEDGER.json`,
`STAGE_TIMELINE.json`, `NEGATIVE_TESTS.md`, `PRESERVATION.md`, `FINDINGS.md`,
`SANITIZATION.md`, `fixture.json`, and `hashes.sha256`.

All other WF0 source—including root CMake, custody/transfer algorithms,
environment, factory census, module loading, and unload owners—remains
unchanged. `artifacts.py` is in scope only because its accepted implementation
literally requires schema `linux-vst-bridge-wf0-implementation-source/v1`, 26
records, and `refs/handoff/wf0-v7-source/...`; admitting the WC0 source identity
is impossible without changing those closed literals. No custody or transfer
owner, route, archive law, or authentication rule changes. A need for a 34th
path is material and returns to design.

## Focused proof matrix

`COMPONENT_SESSION.json` and `CALLBACK_LEDGER.json` retain positive object facts;
`STAGE_TIMELINE.json` retains paired calls and fault attribution;
`BUILD_MANIFEST.json` retains exact source/build/custody joins and the 25-row
disposition. Markdown summarizes but does not replace machine-readable proof.

| # | Claim or failure owner | Production proof and fixture |
|---:|---|---|
| 1 | Exact processor CID and `IComponent` IID | Positive AGain call record plus locked source IDs. |
| 2 | Create result/pointer consistency | Positive tuple and synthetic success/failure/null/non-null cases. |
| 3 | Controller CID before initialize | Positive call order and exact raw/logical comparison. |
| 4 | Minimal host interface boundary | Existing adapter tests only `FUnknown` and `IHostApplication`. |
| 5 | Host reference ownership | Positive 1 -> 2 -> 1 -> 0 ledger and leak fixture. |
| 6 | Initialize succeeds | Positive AGain paired call and `component_initialized`. |
| 7 | Terminate succeeds | Positive AGain exactly-once paired call and `component_terminated`. |
| 8 | Final component release | Positive AGain exactly-once return 0 and object-absence record. |
| 9 | Create failure cleanup | Unknown CID, wrong IID, and three create-tuple exercises. |
| 10 | Controller-ID failure cleanup | Focused module: no initialize, one release. |
| 11 | Controller-ID mismatch cleanup | Focused module: mismatch blocker, no initialize, one release. |
| 12 | Initialize failure; no terminate | Focused return-failure module and call ledger. |
| 13 | Initialize timeout | Unmatched `initialize_component`; accepted process containment. |
| 14 | Initialize crash | Same unmatched operation with abnormal termination. |
| 15 | Terminate failure then release | Primary precedence and later release attempt. |
| 16 | Terminate timeout | Unmatched `terminate_component`; no invented release. |
| 17 | Terminate crash | Same unmatched operation with abnormal termination. |
| 18 | Release timeout | Unmatched `release_component`; process containment. |
| 19 | Release crash | Same unmatched operation with abnormal termination. |
| 20 | Unexpected host-object request | Null plus `kResultFalse`, host blocker, no object created. |
| 21 | No controller creation | One-create ledger, controller-CID tripwire, static closed call surface. |
| 22 | No bus/parameter/state/process/audio/editor call | Component tripwires plus static closed call surface. |
| 23 | Inherited factory/module shutdown | Positive transaction's sole WF0 census/shutdown regression. |
| 24 | Zero process/environment residue | Every focused exercise drains and retires exact ownership. |
| 25 | Protected state exact | Before/after WR0, Runtime/Proton, Bitwig 6.1, and history equality. |

## Blocked results and material stops

WC0 defines exactly 13 new outcomes. Accepted WF0 prerequisite failures retain
their existing exact blocker rather than being relabeled.

- `WC0_DESIGN_PREFLIGHT_BLOCKED`: design basis, activation, branch, or authority differs.
- `WC0_DESIGN_SCOPE_BLOCKED`: the bounded card cannot own the needed design.
- `WC0_COMPONENT_CREATE_BLOCKED`: create result, pointer, CID, IID, or returned reference is invalid.
- `WC0_CONTROLLER_ID_BLOCKED`: the controller-ID call fails or produces invalid output.
- `WC0_CONTROLLER_ID_MISMATCH`: a successful call returns a different controller CID.
- `WC0_HOST_CONTEXT_BLOCKED`: host identity, callback, thread, reference, or retirement law differs.
- `WC0_COMPONENT_INITIALIZE_BLOCKED`: initialize fails, times out, crashes, repeats, or is misordered.
- `WC0_COMPONENT_TERMINATE_BLOCKED`: terminate fails, times out, crashes, repeats, or is misordered.
- `WC0_COMPONENT_RELEASE_BLOCKED`: release is missing, repeated, nonzero, timed out, or crashed.
- `WC0_PROCESS_CLEANUP_BLOCKED`: accepted process/environment cleanup does not reach absence.
- `WC0_PROTECTED_FIXTURE_DRIFT`: any protected projection differs; it is not repaired.
- `WC0_EVIDENCE_BLOCKED`: evidence roster, join, hash, normalization, or sanitization differs.
- `RETURN_TO_DESIGN_GATE`: a material owner, topology, path, fixture, or claim change is required.

Return to design for controller creation, connection points, audio-processor
census, bus/parameter/state/processing/event/editor access, native proxy, IPC,
another build/custody/transfer/runtime architecture, Bitwig, Serum, another
positive plug-in, a persistent service, a 34th path, or a widened claim. An
ordinary defect within the two owners, accepted event/supervisor extension,
focused fixture, normalizer, or evidence renderer is an implementation repair.

Evidence remains allow-listed, UTF-8, hash-closed, and free of raw process or
thread IDs, private paths, credentials, binaries, compatdata, proprietary state,
and vendor material. Bitwig 6.1 remains protected and unlaunched. No signing,
attestation, provenance, release, or product-runner claim follows.

`implementation_authorized=false`
