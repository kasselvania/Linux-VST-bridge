# Audio recovery architecture review

Stage 1 of the [recovery roadmap](AUDIO_RECOVERY_ROADMAP.md), selected on
2026-10-02, is complete at the source-review and design level. The installed
path is mapped below; [architecture section 18](ARCHITECTURE.md#18-audio-recovery-and-portable-execution)
records the production decisions and [Integrated beta delivery](INTEGRATED_BETA_DELIVERY.md#acceptance-method)
records the reconciled acceptance contract. No product code or installed state
changed in this stage. Audio, portability and beta qualification remain open.

## Source and evidence boundary

Reviewed source: `a308fbf93e623ba2b8b5bc5e029f06f440ae390c`, tree
`adc4e748f8b92642a7b644574864e73cb4efe406`. It contains the note-off repair from
PR #201 and its physical evidence. The installed executable source is
`bad5c69cf5a1edbbec6a48bb29a5b734ba9b9569`; later changes were documentation only.
The frozen beta comparison remains `37b5d41e5ffc7876b4bab4ecc6e57737297233bd`.

The [retained physical receipt](../evidence/audio-recovery/2026-10-02-late-note-off.json)
records exact native/Windows/runtime/module identities, preserved originals,
matched SDK output, source validation and the short Bitwig smoke. Recovery1's
Pure LoFi publication and the unchanged internal57 FRAGMENTS publication are
different component pairs. The repair's two 7.68-second passes do not establish
continuous audio; earlier reference lifetimes have measured gaps. This review
did not reconnect to the Deck, rebuild software or run another audio experiment.

This is a focused review of the production path and consequential architecture
choices, not a line-by-line audit of every historical PR, vendor integration or
failure mode. Source observations below establish reachability and behavior;
they do not establish the cause of the residual physical gaps.

## Installed path and ownership

| Boundary | Observed production path and source |
| --- | --- |
| Build and selection | [Package assembly](../tools/pkg0/assemble.py) rebuilds the Rust backend with `registered`. [CommercialInstrumentBridge](../native-vst3-proxy/CMakeLists.txt) uses `AP3_PREVIEW` and `AP8_PREVIEW`; these historical names are active commercial code. [Native builder](../tools/mf3/native_builder.py) selects exact prebuilt bytes for schema 3 rather than invoking a customer compiler. |
| DAW setup and processing | [Processor](../native-vst3-proxy/source/processor.cpp) handles `setupProcessing`, `setActive`, `setProcessing` and `process`; `setupBuses` reaches `ap10_setup`, and processing reaches `if2_process` or `ap19_process_outputs`. Commercial preview configuration selects queued processing, including offline mode. |
| Prepared format and admission | [Queued backend](../native-vst3-proxy/backend/src/queued.rs) takes `installed_delay` from the binding when present, validates it against the DAW maximum, prepares outputs and configures the session. Its callback validates bounded data, admits positioned requests and presents earlier results. Protocol 14 carries whole host blocks; older negotiated paths adapt chunks. |
| Native worker and transport | The same queued worker handles ordered control and audio, then calls `Session::process_positioned` in [backend lib](../native-vst3-proxy/backend/src/lib.rs). Audio and bounded events use shared mappings; the [mailbox](../native-vst3-proxy/backend/src/mailbox.rs) carries one processing request/reply at a time. Existing socket handling retains lifecycle/state duties. |
| Windows execution | [MappedSession](../windows-factory-probe/source/mapped_processing.cpp) validates format, request identity and position; [offline_processing.cpp](../windows-factory-probe/source/offline_processing.cpp) runs vendor processing on the named `lvb-audio` thread. Despite its filename, this code is used for sustained external processing. [DeliveryMailbox](../windows-factory-probe/source/delivery_mailbox.h) receives requests and publishes completion. |
| Presentation and return | Native reply validation precedes result publication. `Callback::process_outputs` consumes results with epoch/position/output-pool checks, accounts for priming, emits silence for missing spans and expires late audio. Callback success does not imply complete signal delivery. |
| Control and notifications | State/configuration share the worker with barriers and special capture handling. The native processor's `AP10.poll` notice path updates total/vendor latency; [commercial controller](../native-vst3-proxy/source/commercial_controller.h) forwards `AP10.restart` to the DAW component handler. Latency notification exists; repeated format-change correctness still needs qualification. |
| Runtime and supervision | [Runtime delivery](../bridge-manager/src/runtime_delivery.rs) acquires exact GE-Proton/SLR artifacts; [session supervisor](../bridge-manager/runtime/session.py) prepares the selected environment and closed runner policies. The existing Deck comparison uses its retained Proton identity, not the managed GE runtime. Manager selection and native publication remain separate operations. |

## Component decisions

| Component | Decision | Reason and completion evidence needed |
| --- | --- | --- |
| Native/Windows split, explicit ABI and exact publication pair | Retain | They implement the product boundary. Verify callers and actual pairs when building successors; a newer manager must not silently substitute a host. |
| Shared audio memory, bounded event storage, output pool, epochs and retirement | Retain and validate | These protections already exist. Preserve them through delivery changes and negative tests for stale/late results, teardown and peer failure. |
| Queue, worker wakeup and request/reply completion | Repair from measured cause; replace mechanics if necessary | Native idle/control waits and native reply polling sleep for 50 microseconds; Windows mailbox polling requests the same interval through `NtDelayExecution`. Requested sleep is not a worst-case wake bound. No single wait has yet been proved causal. |
| Deadline versus containment policy | Separate explicitly | The worker's five-second processing-reply timeout bounds a stalled transport while callbacks can already be losing audio. It is not an acceptable callback completion budget. |
| State/control scheduling and busy guards | Review and repair on a legal-host reproducer | The C++ processor and Rust instance have nonblocking guards. A failed acquisition can reject audio. Control shares worker service, with some capture interleaving already implemented. Neither blanket concurrency nor an unbounded audio mutex is justified. |
| DAW format configuration and latency | Consolidate around one prepared configuration | Setup capacity, actual block, precision, rate, bridge delay and vendor latency must remain distinct. Preserve the existing notification route; prove repeated inactive reconfiguration, including changed latency, without publication changes. |
| Queued versus same-callback delivery | Restore queued continuity, then evaluate same-callback design | Keep `D >= M` in queued mode. The target low-latency path needs a measured event-driven handoff and bounded completion over existing ownership. Default promotion and exact wake primitive await stages 2 and 4. |
| Readiness and compatibility policy | Replace fixture matching as the general capability assessment | [Readiness](../bridge-manager/src/readiness.rs) recognizes an exact Deck/SteamOS/Bitwig envelope. Its graphics-driver fact has no probe; actual device rate and DAW maximum intentionally remain unknown in idle readback. Retain honest fixture qualification while adding capability observations and live instance facts. |
| Profile performance and runner choices | Evolve closed policy and effective readback | [Profiles](../bridge-manager/src/profiles.rs) have one performance variant, `Frames512Recommended256Unqualified`. Runner policy already has explicit graphics alternatives. Preserve exact identities and useful policies, but prove their applicability and effect on each target. |
| Runtime acquisition, prebuilt proxies, populated rollback | Retain and integrate independently | These are substantial delivery work. Complete cleanup/recovery and end-to-end user journeys around the verified engine; source/binary selection alone is insufficient. |
| Legacy AP9/AP10 comparison settings and obsolete implementations | Isolate, then retire after caller and test review | [Performance selection](../native-vst3-proxy/backend/src/performance.rs) forces mailbox with `registered`; `AP10-Work/delivery-mode` is not the installed selector. Installed delay takes precedence over the AP9 fallback. Their presence does not prove they caused the installed failure. Do not delete working callers by name. |

The table selects implementation direction. It does not establish that changing
a timing control will fix the current gap. Apply repairs through a reproducible
failure and physical comparison, with the evidence stated alongside each
disposition. Ordinary implementation choices within the selected work do not
require a new architecture approval cycle.

## Decisions settled and questions still requiring experiments

Settled: the existing managed native/Windows split remains; portability is based
on capabilities with separate qualification; one prepared process configuration
owns format and delivery; licensing identity remains stable; audio deadlines are
distinct from containment; low-latency delivery is evaluated within the existing
transport; comparison controls do not become ordinary product settings.

The exact wake primitive and remaining gap cause require stage 2 timing evidence.
The default delivery mode and completion budget require stage 4 output and timing
results, including burst/offline behavior. Per-platform graphics/synchronization/
scheduling policies require stage 3 effective readback and workload results.
Distribution rights, final release trust material and missing physical target
access must be resolved before their stage 5/6 claims. These are explicit open
implementation or release questions, not assumed passes or reasons to repeat the
architecture review before every routine repair.

## Next bounded implementation

Use the retained recovery1 Pure LoFi publication at the same 48 kHz / 512-frame
settings and qualified recorder. Reproduce one complete missing presentation span.
Correlate epoch, request sequence, sample position and host callback across native
admission, worker/control wait, transport send, Windows receive/DSP/response,
native validation/publication and presentation. Read effective thread policies
and quotas without changing them. Keep clock domains explicit; calibrated clock
correlation must not create false negative durations or blame SDK processing for
time outside its call.

Reuse existing bounded observations. Add only the missing preallocated stamps or
state needed to say where that result was when it was required. Do not perform
CPU-accounting queries or export logs inside the measured DSP interval. Compare
diagnostics enabled/disabled and retain whole-session audio and overflow counts.

Deliver one reproduced mechanism, a failing regression, the minimum justified
repair (including architectural change if necessary), and a matched physical
comparison. A failure to reproduce is reported as such. Do not change runtime,
buffer size, priority or queue topology together, or start the broader release
matrix to hide an unresolved cause.

## Review verification

The source paths and symbols above were read against the stated base. Current
and historical acceptance wording was checked for conflicting durations and
buffer claims; the current contract now has precedence without relabeling old
results. Documentation links and whitespace are checked before commit. No code
tests were rerun for documentation-only edits, and no new physical result is
claimed. The record is a lead source review, not an independent second review.
