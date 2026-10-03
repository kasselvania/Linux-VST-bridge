# Graphics, editor execution and runtime investigation

Source assessment completed 2026-10-02 at commit
`764af9ca5380f14a8b8967acb462dcb9814e7e17`, tree
`78d5081c79999360344625cc11bf15b5387d7f86`. The operator requested this work
while retaining the physical Deck for personal testing. This investigation made
no Deck connection, observation, launch, installation or configuration change.
Production source and dependencies are unchanged.

The bridge already separates its GUI transport and Windows owner thread from
audio delivery. The unfinished work is effective graphics capability detection,
verification of selected rendering paths, narrower failure propagation and
measured coexistence under editor load. The retained BEAM rendering repair does
not establish that its editor-open crackling is fixed. No rendering backend,
CPU affinity or runtime replacement is selected by this review.

## Basis and claim boundary

The task is recorded in [CURRENT_SLICE](../CURRENT_SLICE.md). The relevant
architecture is sections 6.3–6.6 (reentrancy, threads and editor), 7.1 (transport
lanes), 9 (profiles), and 18.1/18.2/18.5 (ownership, capabilities and runtime
selection). The [roadmap](AUDIO_RECOVERY_ROADMAP.md) permits stage-3 investigation
alongside the frozen stage-2 audio comparison. This is an implementation and
evidence assessment under that architecture, not a replacement architecture.

Checks and exact source anchors are retained in
[the local result](../evidence/graphics-runtime/2026-10-02-source-review.json).
Local generated tests establish their particular source behavior; no Windows
SDK, Linux driver, GPU or commercial plug-in was executed in this investigation.

## Existing boundaries and dispositions

| Boundary | What the source actually does | Disposition |
| --- | --- | --- |
| Native UI versus audio | `queued.rs::gui_call` leases the instance/generation without taking the audio callback guard or control mailbox. `gui.rs` owns separate bounded UI queues. | Retain. Local queue saturation and malformed-message tests pass; this is not proof against vendor contention. |
| Windows UI versus processing | `offline_processing.cpp` runs `processor.process` on `lvb-audio`; the component/controller owner calls `MappedSession::service_owner` and pumps the editor. | Retain the thread ownership; investigate remaining failure propagation and service cost. |
| Parameter delivery | `ControllerUpdates` keeps latest display values in prepared atomic storage. The audio side publishes; the owner calls the controller. Sample-accurate processing events remain separate. | Retain. Concurrent coalescing passes locally. Do not move controller calls onto audio. |
| Editor queue and message service | `GuiChannel` has 512 slots per direction. Editor service handles up to 64 commands, throttles ordinary service to 4 ms, and uses the bounded-count Win32 pump. | Retain ordering, epochs and close handling. Count limits do not bound time inside vendor code. |
| Runtime graphics selection | `RunnerPolicy` selects exact retained graphics or touch policies. `session.py::environment` applies the DComp policy through Wine built-ins and `PROTON_USE_WINED3D=1`. | Preserve known rendering repairs. Evolve requirement/observation data rather than adding speculative switches. |
| Shared Proton execution | `NativeProtonSession` binds instances to the verified keeper's initialized runtime and forwards a closed variable set. | Retain; it repaired a real BEAM launch-topology failure. A GPU probe must observe the same execution context. |
| Readiness | Graphics facts cover desktop/session access; the graphics-driver value is explicitly absent. | Repair the missing capability observation. Desktop access is insufficient evidence for rendering support or acceleration. |
| Companion application renderer | Native Access has a separately scoped inherited/software-rendering comparison and explicit process ownership. | Keep application scope. Its fallback is not a plug-in-wide graphics policy. |

Source owners: [native queue](../native-vst3-proxy/backend/src/queued.rs),
[native GUI](../native-vst3-proxy/backend/src/gui.rs),
[Windows processing](../windows-factory-probe/source/offline_processing.cpp),
[Windows session](../windows-factory-probe/source/mapped_processing.cpp),
[controller updates](../windows-factory-probe/source/controller_updates.h),
[editor](../windows-factory-probe/source/editor_session.h),
[window pump](../windows-factory-probe/source/vendor_view.h),
[supervisor](../bridge-manager/runtime/session.py),
[profiles](../bridge-manager/src/profiles.rs), and
[readiness](../bridge-manager/src/readiness.rs).

## Concrete gaps

### 1. Requested graphics policy is not observed acceleration

The DComp policy selects `d2d1,d3d11,dxgi,dcomp=b`, `PROTON_USE_WINED3D=1`,
`PROTON_DISABLE_NVAPI=1` and Proton copy provisioning. This is an intentional
compatibility path. It is not evidence that DXVK is active; equally, selecting
WineD3D does not by itself establish CPU-only rendering.
[Valve documents](https://github.com/ValveSoftware/Proton#runtime-config-options)
the WineD3D option as choosing its OpenGL path over DXVK's Vulkan translation.
[Mesa's LLVMpipe](https://docs.mesa3d.org/drivers/llvmpipe.html) demonstrates why
an API name alone cannot distinguish hardware rendering from software rendering.

`readiness.rs` reports `Graphics driver: None` with the reason
`no profile-required driver probe`. Its graphics checks establish desktop/X11
access, not the renderer used by a Windows plug-in or its browser child.
`profiles.rs::Capabilities` has editor/lifetime/performance policies and an exact
runner match, but no general observed rendering contract. This is a product gap,
not a claim that no earlier development observer inspected loaded DLLs.

The missing readback should distinguish host capabilities, requested policy,
loaded Windows graphics implementation, actual rendering device/driver, software
fallback and observed workload result. Each needs exact runtime/session binding
and explicit unavailable values. A native GPU inventory alone cannot establish
what a sandboxed Windows editor actually uses. Normal plug-in selection should
not run a full graphics workload or historical diagnostic scan.

### 2. Controller synchronization can terminate processing

In `MappedSession::Impl::update_controller`, a failed owner-side
`setParamNormalized`/`EditorSession::host_value` sets `controller_update_failed`.
`MappedSession::next` checks that flag before accepting another audio request,
then routes refusal through the transport-error path. This is a confirmed source
coupling, not a reproduced BEAM cause.

Ordinary GUI backlog is different: `EditorSession::edit` suppresses nested
host-update echoes before checking GUI failure, and retained Windows tests
exercise successful controller updates after GUI overload. Do not erase that
protection or describe every editor error as an audio stop.

The missing test is a production-host comparison between controller update
refusal, a recoverable view/channel failure, and loss of processor integrity.
Determine the appropriate posture for each. Continuing audio must not fabricate
successful controller synchronization or state recall. Conversely, a display-only
failure should not silently become a dead processing instance. No blanket
ignore-error patch is selected.

### 3. The owner thread has finite work counts, not a service-time guarantee

The owner loop sleeps for 50 microseconds between `service_owner` calls.
Controller updates drain before editor service and before pending state service.
A drain can visit up to 8,192 configured parameters; each applied update calls
vendor code. Editor refresh has a 64-parameter batch bound. The Win32 pump caps
queued dispatch count but explicitly cannot bound sent calls or vendor handlers.
The requested sleep and the 4-ms editor throttle establish neither measured CPU
cost nor maximum service latency.

There is also an exception-lifetime hazard to reproduce: `service_owner` can
rethrow an editor exception while the local `std::thread worker` is joinable;
`worker.join()` is after that loop. Stack unwinding then reaches the joinable
thread destructor before the enclosing catch. This source path can terminate
the host rather than complete deliberate editor/audio retirement. It has not
been exercised by this investigation or attributed to a commercial failure.

Measure owner service time, pending-control age and actual scheduler delay, with
bounded observations exported off the processing path. Test a slow/throwing
controller and window handler in the existing SDK fixture. Preserve correct
thread affinity and state ordering. Neither arbitrary extra threads nor a global
priority/affinity change follows from this finding.

### 4. Rendering correctness and resource competition need separate evidence

The retained records below prove useful individual repairs, but none qualifies
general acceleration or editor-load isolation. Browser-backed editors also have
browser, renderer and helper processes; measuring only the Windows host misses
part of their load. [Microsoft's process model](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/process-model)
describes those roles; their presence alone is not proof of GPU utilization.

Acceleration can still create CPU work. [DXVK documents](https://github.com/doitsujin/dxvk#graphics-pipeline-library)
high CPU usage during some shader compilation. This is a possible measurement
dimension, not an attribution to BEAM's selected WineD3D path. Cold editor open,
warm reopen, idle animation and interaction therefore need separate observations.

Record visual correctness, response/frame timing, owner/browser CPU, actual GPU
activity where available, thread scheduling, memory and audio output separately.
Then compare their timelines. Do not infer a GPU defect from an audio gap, an
audio defect from a blank editor, or a useful acceleration change from average CPU.

## Retained physical evidence and its limits

| Fixture | Retained observation | What it does not establish |
| --- | --- | --- |
| BEAM 2.3.1 / BG1 V4 | DComp, embedded WebView2, coherent D3D11 capability advertisement and shared initialized Proton sessions repaired the previously blank editor. Managed Bitwig showed input and reopen; later operator touch use was recorded. | Efficient GPU rendering, editor-open continuity, current installed behavior, complete authorized controls or recall. The rendering receipt's empty-input session had missing-frame counters. The operator now reports earlier editor-open crackling; its cause remains unassigned. |
| Blackhole Immersive 1.4.4 | Exact coherent Wine graphics bytes supplied missing composition support; editor and Bypass worked, with operator-confirmed audio. | General graphics compatibility or measured uninterrupted performance. Preserve FC-GFX-001's exact scope. |
| FRAGMENTS / AP11 follow-up | An unnecessary focus-triggered refresh caused state capture and an attributed delivery stall. Removing that trigger passed its bounded comparison. | All remaining gaps, BEAM behavior or universal GUI isolation. Short cohort CPU samples were not an acceleration comparison. |
| Native Access 3.26.0 | Inherited → software → inherited gave white → visible → white for the exact application. | A GPU root cause, plug-in rendering policy, authorization or general usability. The selected operation-scoped fallback did not change defaults. |

BEAM provenance is retained Git commit
`82bdf081c05ffb2b8d8e3cf7c9b7c1b469c74d2e`, paths `docs/BG1.md`,
`evidence/bg1/managed-v4-rendering.sanitized.json` and
`evidence/bg1/operator-touch-2026-09-26.md`. These private-history records are
not imported wholesale or relabeled as current physical tests. Their source
identities are included in the local result. The public
[BG1 runtime owner](BG1_RUNTIME_OWNER.md) explains the narrower imported owner.
Other references: [Blackhole](BLACKHOLE_EDITOR.md),
[AP11 comparison](AP11_REVIEW_FOLLOWUP.md), and
[Native Access A/B/A](../evidence/naui2/restored-inherited-observation/README.md).

## Next bounded jobs

These are proposed experiments, not completed changes or permission to access
the Deck while it remains with the operator.

1. **Effective graphics readback.** Extend the existing capability/profile owner
   with separate requested and observed renderer facts. First exercise parsing,
   missing capabilities, software fallback and bounded probe failure using local
   generated inputs. A later Linux/Windows reference probe must run through the
   selected runtime context; record unsupported paths honestly. No renderer
   changes or automatic fallback are part of this first job.
2. **Editor/controller failure isolation.** Extend the existing Windows SDK
   fixture to inject slow/refusing/throwing controller work while production
   processing runs. Reproduce the flagged-failure path and joinable-thread
   exception path separately. Assert explicit editor status, correct audio
   outcome, state-save refusal where necessary and bounded owned retirement.
   Select one minimal repair from the reproduced result.
3. **Graphics qualification, then coexistence.** When physical testing resumes,
   freeze one exact plug-in/runtime/host pair. Verify rendering, input, resize,
   cold/warm reopen and renderer identity. Then run matched closed/open/animated/
   interaction audio captures. Only after a baseline may one compatible renderer
   or scheduling policy change at a time, with exact rollback and diagnostics-off
   confirmation. Keep the recovery6 Pure LoFi/FRAGMENTS acceptance separate.

For process isolation, inspect actual processor/controller capabilities first.
[Steinberg's API documentation](https://steinbergmedia.github.io/vst3_dev_portal/pages/Technical%2BDocumentation/API%2BDocumentation/Index.html)
requires correct UI/audio thread use and limits distributing components to
plug-ins that support it. A separate editor process is not a universal retrofit.
Vendor-internal synchronization remains a measured compatibility constraint.

## Local verification and completion

- Fifteen Python tests passed: graphical endpoint/identity projection plus the
  closed graphics-policy selection test, using temporary generated fixtures.
- Seven Rust tests passed with the `registered` feature: GUI queues, malformed
  messages, generation/close behavior, stalled-GUI audio separation and sparse
  host blocks across GUI/state changes with allocation checks.
- The existing C++ controller-update test passed, including 100,001 concurrent
  publications, final-value preservation and a refused application callback.

These checks preserve the useful existing boundaries; they do not reproduce the
newly identified Windows owner/controller failure paths. Windows SDK and physical
graphics tests remain unperformed here. The first source investigation is
complete. No production repair, hardware qualification or beta claim is added.
