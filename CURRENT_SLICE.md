# Current Work: AP3 — Sustained native audio and remote development access

**Implementation authorized on this branch.** Basis: `5890e862fc91a9fec96a142cbc72916849b19b12`; branch `codex/ap3-sustained-audio-and-remote-desktop`. AP2 is accepted through PR #53. Issue #54 tracks AP3; #55 tracks the supporting desktop tool. Read AGENTS.md's active ruling. These are implementation tasks, not requests for another plan.

## Deliverable

Make the existing native VST3 bridge sustain host-driven audio without blocking its host's processing callback; then demonstrate it in a disposable project in the existing Bitwig Flatpak. Separately establish Sunshine on the Deck / Moonlight on the Mac, including actual use by the permitted Mac-side Codex Computer Use. SSH remains the development/control path. A pending desktop permission must not block code or numerical testing.

The starting point is a working offline C++ VST3 shell, Rust session/transport and Windows AGain. Reuse them. This slice adds a measured, added-latency real-time preview for the reference effect, not a universal real-time guarantee, full AGain interface proxy or commercial-plug-in support.

## Selected audio behavior

**Do not run AP2's synchronous exchange from the host process callback.** Rust owns preallocated bounded single-producer/single-consumer queues and a dedicated transport worker. The callback snapshots incoming samples/parameter metadata into owned storage, takes already-completed output and returns. It never waits for Windows, allocates/deallocates, uses filesystem/network calls, launches a process or formats logs. Use fixed-size counters for faults. C++ remains the official SDK edge, connected by a small versioned borrowed-buffer/opaque-handle ABI. No cross-process Rust/C++ atomic-layout assumption is needed: the native queues are Rust-owned, and the existing Windows mapping/control path is reused by the transport worker.

Choose **1024 samples of fixed additional transport latency**, reported through VST3, at 48 kHz for this first preview. Preserve float32 stereo and actual call lengths 1..256. Absolute sample positions, sequence and activation epoch—not callback number alone—identify data. In an uninterrupted epoch, output position s is the processed input at s-1024; only the initial 1024 samples are deliberate startup zeros. A test drains the delayed tail by supplying sufficient trailing silence. Different callback lengths must not change alignment. Capacity includes both sample storage and enough request descriptors for supported small blocks; overflow is explicit failure, never implicit dropping. Document the actual finite capacities and atomic ownership before running. The 21.33 ms added latency is an intentional first-integration tradeoff, not the final low-latency product target.

The worker consumes requests in order and uses actual Windows AGain output; it cannot calculate replacement gain DSP. Windows must receive the selected real-time processing mode/setup, not run an offline mode mislabelled as realtime. Preserve AP2's explicit offline route separately. Remove the 64-call product ceiling in the sustained mode; finite duration/event-log limits belong to the test supervisor. Avoid per-block JSON/logging on the processing path; retain bounded summaries and useful first-failure details outside callbacks.

Gain remains normalized finite [0,1], with block-start updates, missing/empty-queue persistence, zero-frame flush and correct silence handling. Add only the small reference-gain controller required for ordinary host control, using an explicitly bridge-owned controller identity and verified fixture parameter definition. It must enqueue parameter changes through normal host processing; no controller-thread mutation of Windows DSP or fake vendor-controller coverage. No vendor GUI, state/preset claim or fabricated successful getState/setState. Unsupported automation offsets, events, formats and modes are explicitly refused. Snapshot in-place inputs before touching host outputs. The fixed transport delay is separate from any actual plug-in latency/tail.

A missing due result, invalid reply, timeout, queue overflow or dead worker latches a fault: clear validated output spans, return an explicit failure and expose a non-RT diagnostic. Do not replay, return old samples, silently reconnect or treat fault silence as correct audio. Queued storage is not reclaimed while its worker can use it. Stop/restart commands and epochs prevent old output entering a new processing interval; orderly stop drains already-dispatched work to quiescence but does not deliver it after stop. Restart begins a new delayed timeline, retains parameter state within the live instance and cannot silently recreate the plug-in. Fresh close/reopen resets the instance. Lifecycle control must respect the SDK's thread classes; particularly, setProcessing must not acquire AP2's seconds-long wait on an audio thread. Prepare resources outside callbacks, dispatch ordered control to the worker, and join/finish Windows ownership on the non-real-time teardown path. No processing after the acknowledged stop or release before quiescence.

## Consumer and verification

Use the actual SDK-loaded bundle for tests, not a new direct-transport fixture. First use local substitutes to verify delayed output/parameter alignment, variable callback lengths, in-place and silence cases, queue wrap/overflow, stop/restart/reopen, stalled/dead worker, stale replies and unload during failure. Instrument bridge-owned callback code for allocation, blocking/network/file/log activity; functional output alone does not prove callback safety.

Run a fresh Deck batch with host-paced **30-second streams at 128 and 256 frames/48 kHz**, preserving changing input/gain and silence coverage. Inputs are host-selected after activation. Independently check all returned samples at the declared sample delay, startup zeros and drained tail; retain counts, digest/compact witnesses and maximum error, not millions of JSON sample rows. Use the existing exact binary-fraction recipes where appropriate. Record callback median/p99/max, period, overruns, underflows, queue high-water and cleanup. Acceptance requires exact audio and zero unexpected underflows/overflows/overruns for the declared measured streams; do not hide missed deadlines behind a good average. This is evidence for that fixture/load, not proof under arbitrary system scheduling.

Then use the same proxy/backend build in the installed Bitwig Flatpak. Record its actual version/runtime, sandbox and processing setup. Temporarily publish only the test bundle to an exact test search location; do not overwrite LabHostProbe, Serum, normal plug-in registrations or existing music projects. Use a disposable project, modest-level generated audio, one bridge instance, supported sample rate/block size and its basic gain control. Demonstrate scan/load, sustained playback, gain/mute while supported, stop and removal/reopen without hanging the DAW or leaving Windows processes. Preserve and restore only the settings changed for this test. A narrow compatible publication/supervisor launch binding is included; no broad Flatpak override or general broker is needed. If a required consumer behavior exceeds this contract, retain the exact Bitwig failure and the independent core result rather than manufacturing compatibility. AP3 is complete only after both the sustained core and controlled DAW result pass.

## Desktop tooling

Follow [DECK_REMOTE_DESKTOP.md](docs/DECK_REMOTE_DESKTOP.md). Record human streaming, injected input and actual agent control separately. Desktop setup/benign control is an operational change, not an AGain workload. Keep remote helpers separate from product processes and the disposable Windows prefix. Finish planned setup before establishing product-test protected baselines, list the changed paths/settings and do not bless unrelated drift.

Stream video/input only initially; do not change the audio device/route. Run timing measurements without an active stream first, then a short explicitly labelled stream-active check to expose interference. Sunshine encoder activity is part of machine load, not proof of bridge latency. GUI viewing can help operate Bitwig but compressed stream audio is never the numerical oracle.

## Execution and completion

One agent owns implementation, necessary native/Windows builds, narrow producer/input-roster/runner updates, source/artifact/plan binding and fresh verification. The same task permits routine diagnosis, focused repair and continuation. Limits: **6 Windows producers, 10 diagnostics in one AP3 campaign, 2 acceptance candidates of one batch each**. A batch may contain the two paced streams and controlled DAW session, with at most six supervised Windows instance launches and ten minutes of active test audio; it is not one reservation per audio block or UI click. Read-only preflight and local tests consume no live batch. The second candidate is available after a concrete tested implementation/review repair, including after a successful but incomplete first result. Preserve history; no blind retries or new identities to reset totals.

Reuse the pinned SDK/AGain and AP2's explicit Proton 11.0-2c/Runtime 4 selection. A completed official update in the same selected release line may be bound before a new unreserved candidate with old/new records and exact preflight/launch/post-run checks; no automatic trust of arbitrary bytes, runner-family switch or protected-seed migration. Reuse artifacts whose inputs are unchanged. The agent may bind the actual tested source and scoped authority once the adapter works; the default guard below is not an instruction to stop this authorized task.

Publish one reviewable implementation PR, with reusable code, focused tests, concise audio/GUI results and remote-access usage/reversal notes. Close #54 only for the completed audio/DAW result and #55 only for actual agent control. The two results can be delivered independently without a new planning cycle. Leave the PR unmerged. No additional governance framework, paid plug-in installation, vendor editor/state, arbitrary multi-instance engine or remote-control product is authorized.

## Targeted basis

- `docs/ARCHITECTURE.md` §§2, 5.7–5.10, 6–7 and 10: official SDK edge, Rust ownership, callback separation and Flatpak consumer.
- AP2 `CONTRACT.md`/`RESULT.md`, native proxy/backend and Windows mapped/offline processing: working baseline, not realtime evidence.
- [Pinned SDK processing/thread/latency contract](https://github.com/steinbergmedia/vst3_pluginterfaces/blob/4f547e8e102b47de4a8b8aaf343c73b700786372/vst/ivstaudioprocessor.h).

## Authority

Default command guard.

This mapping describes the default closed command and accepted registry. AP3's above work order authorizes implementation and scoped receipts authorize its exact executions; it does not claim an AP3 adapter already exists.

```yaml
status: no_active_slice
authority_phase: no_active_slice
change_class: PROOF_HARNESS_MAINTENANCE
maintenance_id: AP2
maintenance_status: complete
repository: kasselvania/Linux-VST-bridge
maintenance_implementation_authorized: false
product_implementation_authorized: false
live_execution_authorized: false
permitted_execution_class: none
classified_backend_core_ready: true
classified_backend_ready: true
production_adapter_registry: pc0_ap0_ap1_ap2_diagnostic_and_acceptance
accepted_product_frontier: AP2
current_product_target: AP3
pc0_status: accepted
ap0_status: accepted
ap1_status: accepted
ap2_status: accepted
diagnostic_campaign_authorized: false
acceptance_candidate_authorized: false
successor_selection_authorized: false
```
