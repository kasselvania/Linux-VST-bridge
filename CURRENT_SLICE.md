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

Next work retains the same completion contract: replace the current IPC 15 control
acknowledgement polling with notification, and complete installed failure/slow
offline and delivery-selection checks. The first slow-offline run returns correct
audio and retires, but fails its exact component-state recall assertion; attribute
the saved-byte difference before repair or further installed tests. Same-callback,
serial-chain, lower-block, failure and physical acceptance remain open.

The polling audit distinguishes current control acknowledgement from deliberate
legacy paths. Windows mapped-processing formatting was completed separately at
`12ee4383`, preserving all tokens/comments and preprocessor records. An explicit
stack landing decision remains required before trunk integration. Ubuntu is the
sole active project machine; the builder is stopped and the Deck remains unchanged.
