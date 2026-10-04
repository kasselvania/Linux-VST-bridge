# Current work: coherent audio completion

Operator selected on 2026-10-03 after the installed configuration hardening.
Branch `codex/audio-completion-contract` starts at
`5813a65b9b0c680344262a4557e4c4bc26550038`, tree
`d50a53e911affeb4db1244510e4d1a84c5968ce2`. Draft PR #207 stacks on draft PR #206.
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
historical candidate is not a gap-free musical qualification. The current exact
predecessor and installed selection are recorded in the latest checkpoint below.

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

## Earlier checkpoints

Retain all original evidence. [Audio3](evidence/audio-recovery/2026-10-03-audio3-installed.json)
covers slow offline completion, explicit failure/timeout and abrupt-consumer cleanup,
both D0 offline roles and normal buffering restoration. Later timing failures remain
failed: [audio4](evidence/audio-recovery/2026-10-04-audio4-phase-comparison.json),
[audio5](evidence/audio-recovery/2026-10-04-audio5-zero-frame-failure.json),
[audio6](evidence/audio-recovery/2026-10-04-audio6-refusal-frontier.json) and
[audio8](evidence/audio-recovery/2026-10-04-audio8-zero-frame-comparison.json).
The [zero-frame source repair](evidence/audio-recovery/2026-10-04-zero-frame-sample-payload-source.json)
removes unnecessary sample work; its installed audio8 result did not establish a
timing cure. [The allowance audit](docs/RESEARCH_BASIS.md#2026-10-04-callback-allowance-and-audio8-scope)
keeps N/Fs local containment distinct from an observed device/graph deadline.

The polling reachability audit and Windows render-file formatting are complete.
The roadmap selects splitting #200 and landing retained capabilities in dependency
order after audio acceptance. Preserve the original stack and record its dependency
map before extraction; no merge/rebase or new runtime/graphics/vendor work starts here.

## START overlap checkpoint and remaining work

The [source and two-ended proof](evidence/audio-recovery/2026-10-04-start-overlap-source.json)
closes the native START/request publication dependency at production source
`84bc1c621a974c841580f38dc51790b93f8b6754`, tree
`66cbb10c71b1d26945caead615feae20be6ccaa5`. The old `ad1c57f6` worker cannot
publish first N0 while Started is held; the repaired worker does so without
granting readiness. Exact acknowledgement, state/control admission order,
repeated epochs, empty Start/Stop and refused-start containment pass. The fixture
successor `7b5a7287` changes only the test module; both failed observer attempts
remain retained. No production Windows change or larger callback allowance follows.
Linux client 21, backend 148 in each configuration, strict affected lint, 13 SDK tests,
all five candidate CI workflows and an independent paired-package rebuild pass.

[Installed audio9](evidence/audio-recovery/2026-10-04-audio9-start-overlap-comparison.json)
uses the same Windows host, runtime, reference module, SDK consumer and audit as
audio8. Normal update, refresh, preparation and publication pass. Both declared
Buffered512 off/on lifetimes return 107 calls, 198,672 exact float values, correct
state and confirmed retirement. First N0 takes 442,827/847,211 ns against 1 ms.
Off still exceeds the local N13 allowance: 311,965 ns against 270,833 ns; on takes
252,570 ns and has no overrun. **Overall timing remains failed.** Each unpaced
run represents 49,668 four-lane frames, 1.03475 seconds of audio, not endurance.
The N13 tail is not an isolated transport round trip; neither this ordinary-scheduled
VM nor one off/on pair establishes a physical deadline or diagnostic causation.

Normal rollback restores exact audio4 publication `7145b6f90323884c08db9a52f7b24c9c`
and its paired host, with all class registrations/preferences restored; manager
`0.12.0audio9` remains installed. Both project machines are stopped after normal
Ubuntu Power Off; Audiobookshelf is running, and Deck/Nibbi remain unchanged.

The publication-dependency repair is complete at its declared scope. The parent
audio-completion capability remains open. Keep the existing callback bounds and
failed lifetimes; do not tune against the VM's tiny-block allowance until it turns
green. The separate [loaded-engine admission gap](docs/FAILURE_CLASSES.md#fc-mgmt-008--loaded-native-engine-is-not-bound-during-admission)
remains required before promoting a new physical candidate. Physical DAW/serial-chain
completion, inactive reconfiguration, lower blocks and the musician interaction/soak
still need qualification on one frozen artifact. The device/whole-graph deadline
and bridge-local containment allowance must remain distinct in that work.
