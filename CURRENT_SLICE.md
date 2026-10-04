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

## Active repair boundary: zero-frame sample payload

Base `ce17477a74378c22c56f2623b6a8edc11ee5836d`, tree
`1ef69c36df2863860b4fa573cf19037aa95a9864`. An N=0 operation carries events,
parameters and completion identity, but exposes no audio buses to the Windows
processor. The existing native and Windows owners nevertheless initialize, copy
and validate prepared sample planes. Separate actual sample payload from prepared
capacity in those owners; preserve the real vendor call and exact returned results.

Scope is native Session/mapping, Windows mapped/SDK processing and focused tests.
No wire-layout, runtime, scheduling, buffering, allowance or lifecycle policy change.
Keep all packet/identity checks, START ordering, cancellation, final deadline refusal
and guards for every exposed nonzero plane. Do not clear input after the receive
operation has populated it or needlessly shift nonzero preparation into service time.

A source regression must fail on the old actual processing path and pass with no
sample-plane access for N=0, including returned event/parameter results. Exercise
N=1 and multiple outputs, malformed completion, output guards/tails, allocation and
unpaced START-to-N=0 ordering. Build matching Windows/native artifacts, then repeat
the declared installed workload once with diagnostics off and once on, retaining
whole output, state, timing and retirement. Restore the exact predecessor normally.
The claim is removal of unnecessary zero-frame sample work, not attribution of
historical misses or dependable audio. Broader completion acceptance remains open.

The original source regression fails on the old actual Session path (four mapped
writes and reads for N=0). Repaired source `cc41a030` passed 142 Linux backend tests,
Windows SDK execution, five applicable CI workflows, 13 native SDK tests and paired
package rebuild. Review closed the mutable vendor frame-count bypass by retaining
the admitted count and refusing mutation before publication. Fixed-size native
result initialization remains; bounded post-Done private output reset may delay
following work. The ACK-paced two-ended LC1 runtime test remains unperformed;
unpaced START/N=0 ordering is covered separately at native callback/queue level.

Its `0.12.0audio7` candidate is retained **uninstalled**. Compatibility review found
that a cached engine after rollback can be admitted against an older host
(FC-MGMT-008), whose first N=0 reads guards missing from a fresh zero-backed mapping.
The follow-up prepares those existing guards once during inactive mapping creation.
Its actual Session IPC15 regression fails at `cc41a030`, then passes fresh
N=0/N=1/N=0 against the source-matched old-reader oracle, with per-operation mapped
access counts (0,0)/(4,4)/(0,0). This is not execution of the old Windows binary.
Bounded independent review has no findings. Linux validation passes 143 backend
tests (one paired integration ignored), 18 client tests and strict affected lint.
The unchanged Windows artifact may be reused only with all declared inputs verified.

Source, original failures and candidate identities are retained in
[evidence](evidence/audio-recovery/2026-10-04-zero-frame-sample-payload-source.json).
The loaded-engine admission gap remains a separate required architectural repair:
bind the executing caller to its pair, give an inspectable stale-caller result and
preserve usable exact predecessor restoration. Mapping guard preparation does not
close that gap. No greeting/ABI redesign is included in this sample-payload change.

## Installed checkpoint and next completion boundary

Paired `0.12.0audio8` is frozen at source
`d110cbc412f7b284264993508ebdd5ad66bb6d5d`, tree
`9c3be25e582322795a0d611c561290150bcb8274`. Five applicable CI workflows,
13 native SDK tests and independent package rebuild pass. The reviewed Windows
artifact retains `cc41a030` provenance with all 64 source inputs unchanged. The
consumer, audit and reference plug-in bytes match audio6.

The [installed comparison](evidence/audio-recovery/2026-10-04-audio8-zero-frame-comparison.json)
completed normal update, inventory refresh, inspection, preparation and publication.
The initial inspection was correctly disabled until refresh; no request was sent.
Existing publications/preferences survived package adoption, and the sibling was
unchanged by publication. Diagnostic-off returns all 198,672 expected samples,
state and confirmed retirement across 107 calls, but callback105 (N13) takes
327,208 ns against 270,833 ns. Diagnostic-on refuses first RT N=0 at 1,107,316 ns
against 1 ms, before measured audio/final recall. It also retires positively.
Neither whole lifetime qualifies timing. No acceptance rerun followed.

The traced exact-operation predicate is satisfied at refusal. Its native reply
observation is already 71,783–71,852 ns after the converted deadline; wait-end is
95,174–95,243 ns after it. Processing-ready is observed 443,045 ns after C++ entry,
then the reply 628,757 ns later. These are chronological observations, not vendor
DSP cost or a measured scheduling cause. They differ from audio6's pre-deadline
reply/post-deadline presentation. The source sample-work repair is established;
the installed timing failure remains. Native worker policy0/priority0 and differing
Windows readbacks do not establish scheduling or observer causation.

Next bounded audio work must separate the START/readiness prerequisite and exact
request service through native transport, Windows pump/render processing and reply
publication before choosing another timing change. Reuse existing ownership and
observations where sufficient; do not add a second lifecycle or change deadlines,
buffering, runtime or privileges to hide the failure. The caller-engine admission
gap remains a separate required repair. No new vendor/graphics expansion or
repeat-until-green campaign follows this failed comparison.

Normal `ordinary_rollback` restored exact audio4 instrument publication
`7145b6f90323884c08db9a52f7b24c9c`, including its retained paired host. Manager audio8
remains selected. All prior class registrations and preferences match: instrument
Buffered512, sibling effect SameCallback D0 with remembered512. This is native
publication restoration, not whole-package rollback or a dependable fallback.
All attempts/output/receipts are durably retained. Ubuntu shut down through normal
GUI Power Off (exit0); builder is stopped, Audiobookshelf running, Deck unchanged.

Previous retained results, reused only within their stated scope:

| Candidate | Established result and open limit |
| --- | --- |
| [audio1](evidence/audio-recovery/2026-10-03-audio1-installed.json) | Both buffered offline roles pass output/state/reconfiguration; first RT N=0 fails behind START readiness. |
| [audio2](evidence/audio-recovery/2026-10-03-audio2-installed.json) | Prepared render lifecycle completes the first flush; complete output/state pass, but N13 whole-callback timing fails. |
| [audio3](evidence/audio-recovery/2026-10-03-audio3-installed.json) | Slow offline, vendor refusal, timeout and abrupt consumer-loss containment; D0 selection, both offline roles and normal buffering restoration pass. N13 timing remains failed; cold-start failure is retained separately. |
| [audio4](evidence/audio-recovery/2026-10-04-audio4-phase-comparison.json) | Exact audio/state and retirement in both runs; callback105 exceeds allowance in both. Phase observer localizes waiting but does not establish scheduling cause. |
| [audio5](evidence/audio-recovery/2026-10-04-audio5-zero-frame-failure.json) | Absolute monotonic waiting is source-repaired; installed off/on and exact audio4 control all fail an N=0 deadline. All retire. |
| [audio6](evidence/audio-recovery/2026-10-04-audio6-refusal-frontier.json) | Untraced output/state/timing passes; traced first N0 refuses after pre-deadline reply and post-deadline presentation. Both retire. |

The current control-acknowledgement polling audit, deliberate legacy paths and
Windows render-file formatting are complete. The roadmap selects splitting #200
and landing retained capabilities in dependency order after audio acceptance;
preserve the original stack and record its dependency map before extraction.
No merge/rebase or new runtime/graphics/vendor work starts here. Physical DAW,
serial-chain, lower-block, dependable-audio and full musician/soak gates remain open.
Both project machines are stopped; Audiobookshelf is running. Deck/Nibbi are unchanged.
