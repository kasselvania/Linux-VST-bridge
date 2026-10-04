# Current work: coherent audio completion

Operator selected on 2026-10-03 after the installed configuration hardening.
Branch `codex/audio-completion-contract` starts at
`5813a65b9b0c680344262a4557e4c4bc26550038`, tree
`d50a53e911affeb4db1244510e4d1a84c5968ce2`, and will stack on draft PR #206.
That PR remains unmerged, above #204 and the inherited #200 integration.

## Outcome and basis

Deliver one prepared audio-execution contract through the native SDK, owned
transport, Windows render thread and normal manager selection. The primary claim
is that the selected delivery mode completes ordered work under its declared
real-time/offline bound and failure posture, including zero-frame operations,
mode switches, inactive reconfiguration and retirement. Existing buffering remains
an explicit compatibility choice; same-callback delivery targets D=0 with truthful
latency and no implicit preference change.

Basis: Architecture 18.3 “Prepared processing configuration,” 18.4 “Delivery and
deadline policy,” and 18.7 “Platform execution convergence”; accepted D-028;
roadmap order 2b and the platform assessment's shared delivery findings. No yabridge
code may be copied. Its documented architecture is comparison evidence, not our
implementation or a performance guarantee.

## Agreed implementation boundary

- Keep `Live`/`Shared`/`Session`, the bounded queues, mapped payloads, stream epochs,
  exact process/transport custody and retirement. Add no parallel host/lifecycle
  architecture. Notifications prompt inspection; they never authorize execution,
  sequence advancement, slot reuse or acceptance of malformed data.
- Isolate notification traffic from the existing state/control connection under
  the same Session ownership. Pair a dedicated loopback notification connection
  before activation using the authenticated bootstrap, exact session and a fresh
  capability. This is channel pairing, not kernel process identity. Preserve the
  supervisor's independent final-host custody. IPC minor 15 explicitly negotiates the
  change; shared-memory layout 3 and notification schema 1 are separate axes.
- Network I/O stays off both the native DAW callback and Windows vendor render
  thread. The native transport worker owns its endpoint. A narrowly owned Windows
  transport pump exchanges notification hints and signals prepared Win32 events;
  the render thread waits/signals those events and accesses mapped payloads.
  The pump also owns existing active lifecycle/control/error socket bytes through
  bounded prepared handoffs, including Started and control-flag consumption.
  There is no GUI-owner relay or render-thread writer mutex. Cancellation must wake
  and join the pump/render/worker before releasing their storage.
- Native worker wake conditions include requests, pending controls, control-flag 2
  consumption, bounded capture service, fault and quit. Preserve concurrent state
  capture without spinning on an unread stream or delaying audio behind large state
  replies. Backpressure cannot discard the last required wake.
- Prepare typed `Buffered` or `SameCallback` delivery through existing class
  preferences/admission. Existing records remain Buffered; remembered 256/512/1024
  buffering survives selecting/restoring SameCallback. Report effective D and
  vendor L separately, with D+L through the existing SDK latency-change route.
- Carry actual callback mode through the C ABI and IPC 15 process request. Permit
  RT/prefetch switches without setup; offline boundaries require inactive setup.
  Add exact audio-operation tickets for completion, including consecutive N=0,
  without replacing existing epoch/sample-position/result validation.
- Declare a single absolute real-time/prefetch local allowance N/Fs from C++
  callback entry; Every RT/prefetch N=0 operation has a 1-ms flush allowance and must return its
  exact results or explicit failure in that callback, in either delivery mode. Offline
  initially has a 60-second absolute operation bound shared by callback/worker,
  cooperative cancellation checks at intervals no longer than 4 ms. This does not
  interrupt a hung vendor call: retain storage until joins or supervisor containment.
  Service admitted state capture incrementally during offline completion waits,
  preserving both deadlines. Wakes never renew a deadline.
  These are policies to qualify, not whole-DAW scheduling guarantees.
- Offline failure/cancellation/timeout must return explicit failure, never successful
  timeout silence. SameCallback RT misses are separately identified and contained
  through existing retirement. Buffered RT retains its declared gap/expiry posture.
  Buffered offline retains its declared D after completing each submitted operation;
  the host provides latency/tail calls. The bridge does not invent drain calls.
  Publish cancellation before waiting for callback leases. No request replay.

## Scope and executable acceptance

Files/components: native audio-client protocol/bootstrap; native backend mailbox,
queue, completion/preparation and admission; native SDK processor/ABI; Windows
mapped processing, notification adapter and narrow SDK mode propagation; existing
manager performance/action/readback owners; first-party reference modules and
independent SDK consumer/build/package tests. Keep pinned runtime, toolchain,
licensed environments, SDK version and unrelated configuration policies unchanged.
No vendor campaign, graphics renderer repair, broad manager redesign or new runtime.

First source regressions must establish the current slow-offline/mode failures.
Then verify notifications under publication races, delayed/duplicate/coalesced hints,
backpressure, crossed/stale pairing, endpoint loss and cancellation. Cover concurrent
and idle capture, control handoff, start/stop generations and exact retirement.

Verify both roles with actual N from 0 through M, non-power-of-two blocks, in-place
buffers, supported output buses and zero-frame event/parameter results. Include
unpaced bursts, first/final block fidelity, slow valid offline processing beyond the
old five-second worker timeout, explicit failure, RT/prefetch switches, legal rate/
block reconfiguration, state recall, deadline entry time and no callback allocation.
Use original first-party instrumentation; no vendor payloads in public evidence.

Build paired native/Windows/manager artifacts and test normal delivery selection
and buffering restoration on disposable Ubuntu. Capture whole SDK callback timing,
actual output, state and owner retirement on one frozen candidate. Evaluate serial
chains and lower actual blocks 256/128 then 64 without inflating delay or shortening
acceptance. Physical DAW/Deck qualification follows reviewed installed results;
source tests and SDK runs cannot establish musician endurance or beta readiness.
Retain every whole failed attempt and original baseline. Explicitly report an
unperformed physical gate if hardware cannot safely be used.

## Baseline, rollback and machine custody

[Configuration hardening evidence](evidence/preparation/2026-10-03-foundation-installed.json)
retains Ubuntu config7 at source 3fcd7bae: both complete reference workflows, 26
separate audio lifetimes/127,795,200 matching samples, normal GUI restoration and
unchanged original states. Earlier config4 audio loss remains unexplained. That
candidate is the paired rollback/comparison, not a gap-free musical qualification.

All project builders/VMs are stopped at task start. Use one at a time, two CPUs on
host 2–3 and 256 processes; reserve CPU 0–1/capacity for Audiobookshelf. Builder limit
4 GiB; expanded Ubuntu 6 GiB guest / 8 GiB container, combined outer memory/swap equal to
memory. Keep exact predecessor artifacts, normal product recovery and uncertain
ownership records. No forced VM cleanup or unrelated service/privilege changes.
The Deck remains at the previously recorded general5deck configuration; Nibbi's
editor remains unresolved. No Deck mutation occurs during source development.

Implementation agents: Sol 5.6 xhigh (operator cost update); reviewers: Astra 6 xhigh;
computer use: Sol 6.1 High. Do not reuse earlier implementation agents with their
previous Sol 6.1 Max settings for new implementation assignments.
Root owns architecture, integration, machine custody, installed acceptance and
commit/push. Workstream review and required gates remain active until completed.

## Current checkpoint

The source at `8d8327a295ed6b064d3f14e252e5f51d46510d8b` passes applicable
Linux/Windows CI. Paired package `0.12.0audio1` passed independent native/backend
rebuild verification and is installed on the bounded Ubuntu fixture. Normal Setup
installed, discovered, prepared and published the completion instrument and effect
for testing. Original publications/preferences were preserved at package adoption.

The [installed checkpoint](evidence/audio-recovery/2026-10-03-audio1-installed.json)
retains both passing buffered offline lifetimes: N=0..512, inactive M=257/Fs=96 kHz
reconfiguration, exact state and retirement, and 1,336,552 independently compared
float samples with zero mismatches. The first real-time zero-frame flush after an
offline-to-real-time transition then failed its 1-ms allowance. Fault progress
places it behind START acknowledgement; the processing-ready marker follows its
failed callback. A declared diagnostic comparison reproduced this boundary.
Neither failure submitted audio frames, and both retired positively.

The prepared render lifecycle repair at
`f57a4a4dd357b22c610daa20ab64be2cac634375` is installed as `0.12.0audio2`.
Its [installed comparison](evidence/audio-recovery/2026-10-03-audio2-installed.json)
completes the formerly failing first RT N=0 callback in 844,029 ns against the
unchanged 1-ms allowance. The complete mode-switch run returns 198,672 correct
float samples, with no rejected callbacks or missing output. One N=13 callback
still exceeds its whole-callback allowance by 67,759 ns, so timing qualification
remains failed. Both offline roles retain exact output/state/retirement, totaling
1,336,552 independently compared samples. This is a bounded functional comparison,
not an attribution of historical gaps or a dependable-audio claim.

Current IPC 15 control acknowledgement polling was replaced with a reviewed
notification at `06820906`, now included in the installed audio3 comparison. Next work
retains the same completion contract through failure/slow offline and delivery
selection checks. The first slow-offline run returns correct
audio and retires, but its full-envelope state oracle incorrectly requires replay
of transient fixture control ID31. Vendor opaque state and durable values are
unchanged; the current controller mirror legitimately reports the later operation.
The reviewed observer correction at `88451fb8` was rebuilt separately and tried
against unchanged audio2 after a VM restart. That attempt stops before audio:
manager launch verification expires, native state becomes `Failed`, and the later
bus call refuses that phase. No consumer audio transport was created; this is not
a transport-retirement failure or a result from the corrected state oracle.
A separately declared single warm comparison on unchanged audio2 then passes the
corrected oracle, 4,148 exact audio samples, a 12-second offline callback with
concurrent state capture, and exact retirement. The cold-start failure remains
failed; no failed lifetime is relabeled by that comparison.

The shared preparation regression is reproduced and repaired: successful bounded
byte observations now survive a late completion, while the expired caller remains
refused. Later admission retains fresh file-identity and expected-digest checks.
The focused regression, shared preparation checks and 243 manager library tests
pass; scoped independent review has no findings. No startup/audio budget, runtime
or persisted authorization policy changed. Paired `0.12.0audio3` is frozen at
`29e820402851a3d5f05395f6f4f923456a5afa0c`, tree
`fd75321eaa8289ffc968bd1c9ffabdfd52e06bae`, with all five applicable CI workflows
and independent native rebuild verification passing. It includes the reviewed
control acknowledgement notification. The unchanged Windows host retains its
`f57a4a4` provenance; all 64 declared Windows inputs match the new freeze.
Normal installation and both successor publications preserve prior settings and
predecessors. After a normal shutdown/restart, its first slow-offline consumer
passes audio, state and retirement without an intervening consumer warmup. That
one installed success does not prove the automatic warm task crossed its deadline;
the source regression establishes the late-observation defect separately.
The [installed lifetimes](evidence/audio-recovery/2026-10-03-audio3-installed.json)
also pass explicit vendor failure, original offline timeout and abrupt native
consumer disappearance, with confirmed process/transport retirement. Timeout
returns explicit failure 78,796 ns beyond 60 seconds; no strict whole-callback
wall-clock guarantee follows. Normal manager actions select SameCallback D=0 for
both roles while retaining remembered512. Both complete offline consumers pass
787 callbacks and 664,180 exact samples each, state and retirement. Independent
evidence review has no findings for those six lifetimes. Normal GUI restoration
then returns only the instrument to Buffered512; the effect remains D0 with
unchanged preference bytes. Restored offline output/state match audio2 exactly.
The matched modes comparison also preserves exact output/state/retirement, but
callback105 (RT, N13) takes 280,722 ns against 270,833 ns. Both historical and new
whole-callback timing remain unqualified. Eight whole lifetimes are retained;
local source/record analysis identifies callback105 as the N13 residual tail,
which may need the preceding N512 completion. Buffered nonzero deadlines bound
completion waiting; subsequent presentation and C++ result delivery remain outside
that enforced wait bound. The outer SDK bracket includes unquantified audit costs.
Existing aggregate counters cannot attribute this callback's overrun.

The bounded native phase observer is installed as `0.12.0audio4` at source
`1124c1b32637432ab9b0cf3a31b46cbb0820fcbc`, tree
`f649b1399b16d8466e4c8edfa3fc4f3cf1a3f11f`. All five applicable CI workflows,
13 Linux SDK tests, three phase regressions, legacy native builds and independent
package rebuild pass. Explicit invocation-only `LVB_AP23_PHASE_TRACE=1` prepares
128 records while inactive and exports them after quiescence. Reached phases and
valid clocks differ; inferred deadlines retain conversion brackets; final C++ is
pre-return. Ordinary disabled callbacks retain their ABI and no phase export.

The [installed off/on comparison](evidence/audio-recovery/2026-10-04-audio4-phase-comparison.json)
uses the same reference module, settings, Windows executable and independent
consumer. Normal rescan, inspection, preparation and successor publication retain
the predecessor and all performance preferences. Both 107-call runs preserve exact
audio/state bytes from audio3 and confirmed retirement, but each has one timing
overrun. Callback 105 takes 283,345 ns off and 294,484 ns on against 270,833 ns. The traced
wait interval spans 285,056 ns; its post-return end stamp is 18,405–18,467 ns beyond the
conversion bracket. C++ result delivery spans 595 ns. This localizes the interval,
not the cause: notification timing, kernel waiting, pre-syscall and post-return
descheduling remain indistinguishable. The initial offline state-setting flush is
one additional native record outside the 107 measured Exercise calls; all 108 are
retained and correlated using unique SDK clock brackets. No acceptance rerun or
budget change follows from correcting the analyzer's initial count assumption.

The shared absolute-wait repair is committed at
`2847d4b9343fa91f54110ee310746fc89a25b625`, tree
`47bdcf07afb7b25ef964f209210600d30de271c7`.
The [Linux source regression](evidence/audio-recovery/2026-10-04-absolute-completion-wait.json)
first fails against the relative wait, then all 133 backend tests pass with the
reviewed absolute monotonic wait. Checked conversion, early timeout reinspection,
publication races, interruption and cancellation retain the original deadline.
This fixes deadline arming; it does not guarantee a descheduled caller returns by
that deadline. All five applicable CI workflows, 13 native SDK tests and the
independent package rebuild pass for paired `0.12.0audio5`.

The [installed comparison](evidence/audio-recovery/2026-10-04-audio5-zero-frame-failure.json)
**fails**. Diagnostic-off and diagnostic-on both refuse the first RT N=0 call:
1,130,203 ns and 1,072,008 ns against 1 ms. Neither reaches measured audio or final
state recall. Both confirm process and transport retirement. The traced operation
predicate remains unsatisfied; its enclosing wait is 1,055,160 ns. The processing-ready
marker appears 789,283 ns after SDK entry. This marker and the post-wait stamp do
not isolate worker scheduling, transport, render-thread or SDK duration.

One declared same-boot control restores exact audio4 native publication through
normal `ordinary_rollback`, retaining audio5 manager, identical Windows host/runtime,
consumer and all preferences. It passes the first RT N=0 in 679,060 ns, compares
16,380 samples correctly, then refuses callback12 (prefetch N=0) in 1,066,883 ns.
It also retires positively. None of these three lifetimes passes. The earlier
engine's failure prevents assigning the new first-call failures to the absolute
wait alone; it does not prove timing equivalence. The exact audio4 native predecessor
is left selected, with audio5 manager and all sibling registrations/preferences
unchanged. Preserve this explicit mixed selection; do not call it a whole-package
rollback or a known dependable fallback.

Next bounded work is the shared START/readiness and queued zero-frame completion
path, including unpaced backlog and the native/Windows wake chain. Establish where
the required operation is before its deadline before another production change.
Native worker readback is policy0/priority0 with realtime budget unavailable;
Each audio5 run has two unavailable Windows scheduling requests and no effective
policy readback. The predecessor control has one unavailable request and one effective
policy1073741826/priority5 readback. These differing post-start observations confound
an isolated engine timing comparison and do not prove continuous scheduling policy.
Do not substitute a launcher policy for actual-thread measurement. Keep existing
budgets, buffering, runtime and privilege scope unchanged. Keep `setProcessing`
lightweight: it may execute on the processing thread; do not move a blocking START
acknowledgement wait there.
Physical/serial-chain/lower-block/musician acceptance remains open.

The observation change starts from `637c40b431e64492884025e5b0ace409f6f95f24`,
tree `d4d4c0a5c921d429bdb21b327a19c43ee4a20f4a`. It reuses AP12's existing mapped status:
prepare a retained read-only view and capture one bounded snapshot when the first
exact-deadline refusal is recorded, under explicit invocation diagnostics. It observes
the refusal interval, not an atomic cross-process instant at the original deadline.
Keep per-lane stability, generation/epoch/position/sequence matching and clock domains
separate; a stable stale row is not current work. A coherent diagnostic worker tuple
binds protocol sequence to callback/ticket, including repeated zero-frame calls at
the same position. Changed or absent correlation stays unknown. Export only after
quiescence.
This is one snapshot of existing stages, not a second trace stream or a new transport.
Cover an advancing-after-refusal row, stale/unstable/absent data, map lifetime,
first-failure preservation and callback allocation. Disabled operation performs no
snapshot reads. Separately preserve identity-checked scheduling observations after
an unsuccessful capability request instead of discarding them in a generic error.
Scope: backend `fault_status.rs`, `lib.rs`, `queued.rs`, and manager
`audio_scheduling.rs`, plus required focused tests. No Windows/protocol, budgets,
RTTIME/privilege, lifecycle or buffering changes. The
[source validation](evidence/audio-recovery/2026-10-04-deadline-status-source.json)
passes 140 Linux backend tests, 12 scheduling tests, final focused tests and clippy;
independent review has no remaining findings. Recovery export has helper-level
tests and source-reviewed integration, not a new end-to-end recovery proof. The
next installed off/on pair uses one frozen package and unchanged reference workload.
Failure-time observation alone is not an audio repair.

The polling audit distinguishes current control acknowledgement from deliberate
legacy paths. Windows mapped-processing formatting was completed separately at
`12ee4383`, preserving all tokens/comments and preprocessor records. The roadmap
selects splitting #200 and integrating retained capabilities in dependency order
after audio acceptance; preserve the original stack and record the exact dependency
map before that later extraction. No rebase or merge starts in this slice. Ubuntu was
shut down normally after this checkpoint; both project machines are stopped.
Audiobookshelf remains running and the Deck remains unchanged.
