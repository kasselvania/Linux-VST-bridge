# Current Work: AP2 — Host-loaded Native VST3 Offline Bridge

**Active implementation work order.** Issue #52; branch `codex/ap2-native-vst3-offline-bridge`; basis `a39760c071a95182b79faa9ee759bb271acc07b7`. AP1 is accepted through PR #51. AGENTS.md authorizes this whole task before merge; there is no intermediate design or activation handoff.

## Outcome and why it is next

Deliver a native Linux ELF `.vst3` bundle whose standard VST3 processing interface drives the actual Windows AGain through the accepted mapped-audio backend. A Linux SDK-based host loads the bundle, owns its buffers, sets up and runs the processor, reads its returned audio, and tears it down. The host must not call private proxy exports, manipulate AP1 memory directly, or run the AP1 recipe executable as a substitute for the plug-in interface.

This connects the working Windows/transport side to the interface a native DAW will load. It is **offline-only and processor-only**, not yet a usable realtime Bitwig insert. Do not put AP1's seconds-long waits on an audio-device callback. Native factory discovery alone or another standalone socket client is insufficient.

## Selected surface and ownership

- Add a separate native proxy target. Reuse `native-probe/` SDK/bundle techniques, not its local gain DSP, class IDs or installed publication. Preserve the accepted LabHostProbe and Serum installations. Bind the proxy's processor class to the retained Windows AGain processor FUID and module digest; labels/capability description must identify the limited offline bridge. Expose no controller/view class in this cut; return the SDK's unsupported result for unavailable controller/state/editor functionality. Do not advertise complete AGain interface coverage or successful preset persistence. Keep the bundle in private test staging, outside normal DAW search paths.
- The C++20 shell owns official module/factory/IComponent/IAudioProcessor objects, SDK references and conversions. Rust owns the reusable session/mapping/control endpoint, extracted narrowly from `native-audio-client`. Their small versioned C ABI uses opaque handles and bounded borrowed buffers/results; no C++/Rust objects, exceptions or panics cross it. Host buffers are borrowed only for the call and never retained after return. Use the native host's compatible libc/PIC build, not AP1's standalone musl executable as a loadable library.
- The existing external supervisor owns environments, launching and cleanup. It supplies one exact private session binding to the proxy process, and launches Windows when the endpoint is ready. The module does not execute Wine/Proton, select paths from a peer, modify prefixes, or become another broker. Discovery and initialize without activation must not launch Windows. Reuse AP1's authenticated loopback control, fixed shared mapping, single-slot ownership and no-replay law.
- Host lifecycle is authoritative: initialize/setup -> activate -> start -> process -> stop -> deactivate -> terminate/release. Reflect successful transitions at the Windows owner/processing threads; do not merely return success while an unrelated pre-scripted run executes. Preallocate during setup/activation. Windows processes only in the host-requested processing interval; stop is acknowledged before deactivation/release, with its processing thread quiescent/joined before unloading. A zero-call start/stop is valid. Necessary narrow lifecycle messages and a versioned successor mode are included; preserve AP1's existing mode/contract. One active instance at a time is supported. Refuse conflicting attachment rather than sharing state by class ID. Fresh reopen creates a fresh session/instance.

## Processing contract

Accept only `kOffline`, `kSample32`, 48000 Hz and one active stereo audio input/output, with negotiated maximum block size in 1..256. Reject realtime/prefetch, 64-bit processing, unsupported arrangements or state before dispatch. Report only capabilities actually implemented. Query interface/reference/lifecycle behavior follows the pinned SDK; supporting mandatory cheap queries is part of this cut, not another milestone. Do not claim transport waiting adds audio sample delay; report the actual processing latency/tail for this retained fixture.

For each nonempty `process`, validate the actual ProcessData against setup before touching backend state. Copy host inputs into the owned slot, delegate to AGain, wait for the matching response within a finite offline deadline, and copy actual samples and output silence flags back before returning success. Support normal separate buffers and same-channel in-place buffers by snapshotting inputs before output writes. Input silence flags are optional hints, not output predictions; preserve AP1's independent full-width output-mask and zero-sample validation.

Gain uses AGain ParamID 0, normalized finite [0,1], with at most one point at offset zero per block. Missing/empty queues retain the last accepted gain, initialized to the retained AGain default. Bypass remains off; a supplied bypass-off value may be admitted, but enabling it, other nonempty parameter queues, musical events or intra-block automation is explicitly unsupported. Reject malformed/unsupported requests without silently dropping them or changing the cached gain. For `numSamples == 0`, permit the SDK's parameter-flush form with zero audio buses/null audio arrays: update the validated pending gain without a backend audio request; the next nonempty call delivers it. This is parameter delivery, not saved plug-in state.

Retain the backend's declared 64 nonempty-block bound per activation for this first cut; reaching it must yield an explicit refusal, never restart the plug-in. Zero-frame flushes do not consume an audio sequence. No extra sample is generated locally; no native gain/bypass DSP fallback exists.

A timeout, disconnect, invalid response or processing failure latches the session unusable. Return an explicit VST3 failure and clear only validated writable output spans defensively; clearing is not a successful silent result. Never replay, reconnect silently, use last-good output or reuse an outstanding slot. Stop/terminate must still permit bounded cleanup. No host reference, Rust handle or mapping may be released while an owned worker can use it. Unknown containment blocks a new attempt. Use the existing finite setup/request/close watchdogs, documenting any necessary narrow change before measurement. No production security or realtime guarantee is inferred from this test.

## Proof that must run

Use the pinned SDK's module/hosting helpers for a small independent offline test host, or a selected validator extension. It must discover the ELF bundle, instantiate/query through standard VST3, pass standard ProcessData/parameter queues, and inspect its own returned buffers. The orchestration supplies private launch binding out of band; the audio test knows no shared-memory offsets, private C ABI or transport recipe. Full stock-validator success is not required for this declared subset; do not fake unsupported capabilities to obtain it.

In a persistent activation, have the host select fresh reproducible signed binary-fraction inputs after activation, exercise the ten AP1-A2 cases through VST3, and independently compare every sample at zero tolerance. Include changing block lengths/gains, zero gain, whole/one-channel silence, an empty gain queue after a change, a zero-frame gain flush before real audio, and same-channel in-place buffers. Retain actual host request/return words, masks, maximum error and matched backend call counts. A correct Windows-side report without correct host-owned output is not success.

Prove clean close/unload and a fresh reopen with no inherited gain/session/handle state. Local tests through the actual loaded proxy/C ABI cover wrong setup/order/shape, double/invalid ownership, unsupported realtime calls, backend disconnect/timeout/invalid replies and failure teardown. Prove backend unavailability cannot fall back to local gain calculation. Use local peers/substituted platform effects for faults, not a new hostile-plug-in campaign. Keep prior regression tests; add tests for changed code, not a certification framework.

Fresh Deck verification runs the Linux SDK host and real Windows plug-in, with exact native/Windows artifacts, runtime selection, thread/lifecycle observations, mapping/process retirement and unchanged protected state. A batch may contain the bounded positive activation and one clean reopen; it is not a reservation per VST3 call. Offline CPU time is not evidence of meeting a realtime deadline.

## Execute and deliver

One engineer owns implementation, target-compatible Linux builds, necessary Windows builds, artifact delivery, focused tests, source/plan binding, diagnostics and fresh verification. Native CMake/SDK/FFI integration, narrow supervisor/adapter changes and stale build-branch/input-roster repairs are included. Reuse unchanged Windows artifacts when possible; do not rebuild merely to bind new Linux code. Do not import historical execution authority or rewrite accepted evidence. No separate adapter, governance or status-only PR is required.

Task ceilings: **six Windows producer attempts, ten diagnostic batches in one AP2 campaign, two acceptance candidates with one batch each**. Local builds/tests are ordinary development. The second candidate is authorized after a specific tested implementation/review repair, including a defect found after a successful first observation; preserve the first and all cumulative counts. No blind retries, reset or redundant successful runs. Reporting-only repair reuses its retained observation. Source/artifact/plan rebinding within this contract is delegated through the existing exact/atomic paths after local tests; do not pretend backend readiness exists before it does.

Use AP1's explicit Proton 11.0-2c/Runtime 4 selection and unchanged SDK/AGain. If Steam completes another official update in that same selected release line during AP2, the engineer may explicitly bind it for a new unreserved candidate, with old/new records and strict preflight/launch/post-run checks. Do not auto-trust arbitrary installed bytes, change runner family, rewrite historical locks or migrate protected seeds. Runtime-only changes do not force an endpoint rebuild. Ordinary in-scope repair requires no new approval; genuine ownership/claim changes, unknown containment, changed protected state or exhausted allowance stop live work, not local diagnosis.

Open one non-draft PR closing #52 with implementation, focused tests, a compact ABI/usage note and the actual host-returned sample/cleanup results. Leave it unmerged for review; AP1 remains accepted until then. No Bitwig launch/publication, audio device, real-time synchronization/optimization, arbitrary plug-in support, controller proxying, state/presets, editor, multi-instance broker or runtime-management project belongs here.

## Targeted basis

- `docs/ARCHITECTURE.md` §§2, 5.7–5.9, 6–7: SDK edge, Rust ownership, host/supervisor distinction and separate control/audio.
- `native-probe/source/processor.cpp` and root `CMakeLists.txt`: accepted local effect/build reference, not the bridge implementation.
- AP1 `CONTRACT.md`, `RESULT_A2.md`, native Rust client and Windows mapped/offline processing sources: accepted transport and processing basis.
- [Pinned processing interface](https://github.com/steinbergmedia/vst3_pluginterfaces/blob/4f547e8e102b47de4a8b8aaf343c73b700786372/vst/ivstaudioprocessor.h): offline/realtime distinction, buffer rules and lifecycle/thread requirements.
- [SDK validator](https://steinbergmedia.github.io/vst3_dev_portal/pages/What%2Bis%2Bthe%2BVST%2B3%2BSDK/Validator.html): standard host/test integration. Keep the repository's existing SDK revision; no SDK migration is needed.

## Authority — default command guard

This mapping continues to refuse unspecified live commands. The active AP2 work order/ruling authorizes implementation; its exact scoped receipts authorize tested AP2 executions. Registry readiness below describes accepted adapters, not an already-implemented AP2 adapter.

```yaml
status: no_active_slice
authority_phase: no_active_slice
change_class: PROOF_HARNESS_MAINTENANCE
maintenance_id: AP1
maintenance_status: complete
repository: kasselvania/Linux-VST-bridge
maintenance_implementation_authorized: false
product_implementation_authorized: false
live_execution_authorized: false
permitted_execution_class: none
classified_backend_core_ready: true
classified_backend_ready: true
production_adapter_registry: pc0_ap0_ap1_diagnostic_and_acceptance
accepted_product_frontier: AP1
current_product_target: AP2
pc0_status: accepted
ap0_status: accepted
ap1_status: accepted
diagnostic_campaign_authorized: false
acceptance_candidate_authorized: false
successor_selection_authorized: false
```
