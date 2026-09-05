# Current Work: AP4 — Windows plug-in state and Bitwig project recall

**Implementation authorized.** AP3 is accepted through PR #56. Basis: `8e60bace2f685b94fb5936919069103e3dc9a376`; branch: `codex/ap4-plugin-state-project-recall`; issue #57. AGENTS.md supplies the active ruling. One engineer completes implementation, necessary builds, focused tests and fresh verification before one PR; no intermediate planning/activation handoff.

## Product result

Change a control, save the disposable Bitwig project, close the DAW and all owned plug-in processes, then reopen that same saved project and recover the correct Windows AGain processing and displayed control without manually re-entering values. A byte serializer alone is not completion.

Extend the accepted queued preview, not a new bridge. Keep one reference instance, float32 stereo/48 kHz, supported 1–256-frame callbacks, 1024-sample added latency, existing offline behavior, owned supervision and explicit failure. Low-latency tuning, other formats/rates, instruments, vendor editors, full vendor-controller proxying, multi-instance operation, generic preset management and commercial compatibility are outside this cut.

## Selected state behavior

**Actual processor state, owned by the DAW.** Implement the native `IComponent::getState/setState` route to the real Windows component. Carry its complete bytes unchanged within a small versioned envelope binding the logical processor class, exact module digest, payload length and integrity. Cap payloads at 1 MiB and envelope overhead separately; handle short reads/writes, checked seek/size arithmetic and unsupported versions explicitly. Generic transport treats payloads as opaque. Do not put session IDs, handles, paths, tokens or current process state in the saved blob, or bind it to an incidental proof reservation. Keep class identity stable across save/reopen. The same module and supported envelope can reopen in a fresh authorized environment; missing/mismatched module is an error, not a substitute.

The current pinned AGain state is two little-endian float32 values (gain, gain reduction) and one int32 bypass value. A small reference-specific adapter may validate this exact format and synchronize the exposed gain; it must preserve the complete real payload. Do not manufacture saved bytes from `gain_`, rewrite a host stream into a test recipe, or let the current per-block gain/default-bypass messages overwrite restored state. Unsupported/nonfinite reference values are rejected, not normalized into a different state. VST stream attributes not implemented by this slice are honestly unexposed; never forward a private project path as a required state attribute.

**State and processing ownership.** State methods run on the SDK UI/owner thread on each side; the Windows audio thread must not call component state methods. Establish the initialized Windows component when a non-real-time state operation needs it, including before setup/activation, through the existing supervised session. Enumeration alone must not launch Wine. Save returns success only after an actual correlated Windows snapshot and a complete host-stream write. Restore returns success only after a valid blob is applied by that exact Windows component and local parameter synchronization is complete. Extend the existing handshake/lifecycle to permit this; do not report an unexecuted restore as completed.

A save is a coherent cut ordered after all audio/parameter requests accepted before its barrier, including parameter-only zero-frame updates; later requests may follow it. Flush such accepted changes to the real Windows processor using its normal processing interface before taking the snapshot. Service snapshots between processing calls with no concurrent access to the plug-in's mutable state. Support saving while the host continues callbacks: do not wait, allocate, log, reset the sample timeline, or reject an otherwise valid audio callback merely because the non-real-time save is waiting. Keep callback storage single-producer/single-consumer; a state request must not introduce a second producer to an audio queue. Do not hold the native registry/processor guard across blocking state I/O. Private mailbox/dispatch implementation is engineering latitude, not a new broker project.

Restore is supported after initialization before processing and while processing is stopped, including an active-but-stopped instance. Live state replacement during running processing is explicitly refused before mutation in AP4. Bitwig transport stop does not itself prove `setProcessing(false)`; honor the actual host lifecycle. After restore, empty parameter queues preserve the restored values. Stop/restart retains state; fresh creation with no supplied state gets the real default. Reject malformed streams before changing the current instance. A failed/uncertain remote restore can have changed the plug-in: latch that instance unusable, return failure and clean it up; do not claim rollback or continue with defaults. A failed save cannot be published as a successful snapshot. Keep bounded timeouts, original error retention and no blind retry/replay; the DAW remains responsible for durable project writes.

**Controller synchronization.** Keep the bridge-owned reference-gain controller, not a counterfeit vendor controller. Implement `setComponentState` so the host can query the restored gain via ordinary controller interfaces. Follow processor-first restore ordering, do not emit edit gestures during restore, and do not let separate controller state override processor gain. There is no independent GUI setting in this controller: use an explicit empty/versioned controller-state representation if the host requires it, not a second gain authority. Parameter updates still pass through host processing. Narrow ordinary lifecycle/connection integration necessary for this flow is included.

## Proof that matters

1. Through the actual SDK-loaded bundle, capture state after nondefault gain 0.25 and after a zero-frame update to mute; verify real Windows get/set calls, byte length/digest/readback, and failure reporting. Fully unload native and Windows sessions; reload using only the host-supplied saved stream, with a new session/mapping. Supply fresh audio with no initial gain resend and independently compare every returned sample at the declared delay. Verify controller value, subsequent gain editing, and state re-save. Include a source-backed reference state with a nonzero gain-reduction field and a bypass-state case through standard component methods, with actual Windows readback, so storing only the exposed gain cannot pass. Synthetic state seeds are labelled fixtures; captured results must come from the plug-in.
2. Focused local tests exercise the real proxy/state boundary for truncated/oversized/wrong-version/wrong-module streams, partial I/O, state-call failure, stale/lost responses, and quiescent teardown. Malformed input must not mutate the prior good state; ambiguous remote mutation must fail the instance. Include a save overlapping callbacks and pending/zero-frame parameter updates; retain callback instrumentation and exact delayed output. Reuse existing tests and supervision rather than another framework. Run one fresh 30-second paced stream at 48 kHz/256 frames including coherent saves, numerical checks and zero unexpected callback effects/overruns/queue faults. Earlier AP3 evidence remains unchanged; no need to repeat its entire measurement matrix.
3. In the installed Bitwig Flatpak, temporarily publish only this build and create a new disposable project. Set a clearly nondefault gain, save normally, record native state calls/digests, quit and verify all owned sessions have ended. Reopen the unchanged saved project with fresh processes and without an agent/sidecar/automation re-entering gain; observe restored controller value and playback before touching controls. Change gain to mute, save, fully close and reopen again; confirm mute remains and a later deliberate edit restores output. Keep the saved project as the carrier between runs; do not regenerate it for reopen. Correlate real Windows restoration, native state/processing facts and GUI results; meters/streamed audio are not numerical evidence. Preserve the small private test project; restore original preferences and remove only the temporary publication afterward.

This proves reference-effect state recall on the tested consumer, not every possible VST state stream or arbitrary save/restore timing. If actual Bitwig needs additional ordinary initialization/reactivation sequencing within these semantics, implement it in this task. Do not hide an incompatible lifecycle by changing the consumer or bypassing its standard interfaces.

## Execution and delivery

Use the established Moonlight/SSH setup and current Bitwig fixture; no remote-desktop installation project. Keep permissions and pairing private. Commands remain on approved SSH, not a streamed terminal. Use modest audio levels and only disposable projects. Test project files and planned publication/preferences are explicit fixture changes outside prior immutable evidence; establish the new baseline before measurement and never bless unrelated drift.

The task authorizes necessary native/Windows builds, state-call roster and versioned protocol/C ABI changes, narrow owner-thread servicing, producer/adapter/evidence integration and mechanical exact source/plan/candidate binding. Use the existing classified execution path and useful checkpoints; no new transaction system. Repair routine code/harness problems and continue, rather than stopping at an adapter or approval request. Source, executable/runtime identity, original failures and cumulative spending remain truthful.

Allowance: **6 Windows producer attempts, 10 diagnostic batches in one AP4 campaign, 2 fresh acceptance candidates of one batch each**. Each batch may use up to eight serial supervised Windows instances for state/fresh-process/Bitwig stages, with at most ten minutes of active test audio and existing bounded per-stage GUI/control waits. This is not one reservation per state call or GUI action. Local substituted-peer tests and read-only preflight consume no live batch. The second candidate is for a specific tested repair, including a review finding after a successful first run; no resets or blind repetition. Stop live spending if retention breaks; fix it locally. Prior AP3 unused allowance does not renew.

Reuse the retained SDK/AGain and explicit Proton 11.0-2c / Runtime 4 composition. A completed official update in that selected release line may be rebound before a new, unreserved candidate using actual versions/digests and exact preflight/launch/readback checks. Retain earlier bindings. No runner-family switch, arbitrary downgrade, seed migration or unchanged artifact rebuild is required. Do not weaken checks to admit new bytes.

Publish one non-draft PR closing #57 only when actual state, restored audio/controller and Bitwig project recall all pass. Leave it unmerged. Lead the report with restored values/sample comparisons, saved-state readback, fresh-process project recall and cleanup, then source/head and actual costs. Preserve all previous accepted evidence and keep AP3 as the accepted frontier pending review.

## Targeted basis

- `docs/ARCHITECTURE.md` §§5.7–5.10, 6.4–6.5, 7 and 10; existing AP3 proxy/worker and `docs/DECK_REMOTE_DESKTOP.md`.
- [Pinned IComponent state/thread surface](https://github.com/steinbergmedia/vst3_pluginterfaces/blob/4f547e8e102b47de4a8b8aaf343c73b700786372/vst/ivstcomponent.h).
- [Pinned actual AGain state and processing](https://github.com/steinbergmedia/vst3_public_sdk/blob/586dc5e6c8012c3e4b01c79389375cbe96bdb1da/samples/vst/again/source/again.cpp): retain gain, reduction and bypass; do not replay the old forced-bypass default over restored state.
- [SDK persistence/controller synchronization](https://steinbergmedia.github.io/vst3_dev_portal/pages/Technical%2BDocumentation/API%2BDocumentation/Index.html#persistence).

## Authority — default command guard

This mapping describes the closed default command and accepted registry, not a revocation of the AP4 work order above. The engineer is authorized to create exact scoped diagnostic/acceptance receipts after implementing and testing the route. It does not claim the AP4 adapter already exists.

```yaml
status: no_active_slice
authority_phase: no_active_slice
change_class: PROOF_HARNESS_MAINTENANCE
maintenance_id: AP3
maintenance_status: complete
repository: kasselvania/Linux-VST-bridge
maintenance_implementation_authorized: false
product_implementation_authorized: false
live_execution_authorized: false
permitted_execution_class: none
classified_backend_core_ready: true
classified_backend_ready: true
production_adapter_registry: pc0_ap0_ap1_ap2_ap3_diagnostic_and_acceptance
accepted_product_frontier: AP3
current_product_target: AP4
pc0_status: accepted
ap0_status: accepted
ap1_status: accepted
ap2_status: accepted
ap3_status: accepted
diagnostic_campaign_authorized: false
acceptance_candidate_authorized: false
successor_selection_authorized: false
```
