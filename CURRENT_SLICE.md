# Current work selection

## Graphics readback and editor failure containment implementation

On 2026-10-02 the operator authorized the two source follow-ups below while
physical demonstration is unavailable. Base: `ad7287440b7beed668e2a89fd8aa71add59f4439`,
tree `8b52c12ea8e5496812580e5f5a855e9639c81da0`. Basis: AGENTS.md real-time,
thread-affinity, evidence and privacy rules; ARCHITECTURE.md 6.3–6.6, 7.1 and 18;
GRAPHICS_RUNTIME_REVIEW.md “Next bounded jobs”.

The first claim is truthful requested-versus-observed graphics reporting, with
bounded, explicitly requested host diagnostics and unknown Windows renderer facts
until that exact runtime can be observed. The second is controlled retirement
after a reproduced editor-owner exception, with separate controller-failure
tests preserving state integrity. Scope: manager readback and its tests, existing
Windows SDK-edge processing/editor ownership and focused SDK fixtures/CI. No
dependency, runtime policy, audio queue model or installed artifact changes.

Acceptance: local parsing, missing/contradictory/software renderer and bounded
helper failure checks; native Windows SDK reproduction of the original failure
and verification of the repair, including cancellation and timeout containment.
Record exact source, actual test outcomes and outstanding physical acceptance.
No Deck access or changes, vendor state, deployment, new acceleration claim or
BEAM crackle attribution. Preserve the frozen audio worktree and existing
evidence. Source rollback is ordinary commit reversion; no installed rollback is
needed. Commit and push the reviewed changes to draft PR #204.

Result: these two bounded source jobs are complete at
`84d4c91f50c3ecfbc24ed4ed3b6fdd8fc8d94c1f`, tree
`6b54a2f1df6e3d0b4ae63b8bcc8b57145b696982`. The Windows CI merge checkout has
the identical tree. The owner regression reproduced exit 96 on the prior loop;
the repaired SDK fixture joins a cancellable worker and contains an unresponsive
one at the five-second wait. Five real-MappedSession controller cases, mailbox
cancellation, two state-wait cases and sixteen Windows socket cases pass. The
first Windows socket attempt and intermediate harness compilation failure remain
recorded. Linux validation passes 496 manager Rust tests (two existing ignored),
352 runtime tests, frontend checks and lint. Graphics observations remain scoped
to configured policy and an explicitly requested native diagnostic.

[Retained evidence and artifact hashes](evidence/graphics-runtime/2026-10-02-source-repair.json)
distinguish source/SDK results from the unperformed Windows-renderer probe,
commercial graphics/audio comparison and frozen audio interaction/soak. The Deck
and audio recovery worktree were not changed. Controller synchronization failure
still stops processing and refuses state capture; relaxing that safely is not
part of this repair. No beta or new physical support claim is made.

## Graphics, editor and runtime source investigation

On 2026-10-02 the operator requested investigation of the non-audio layers while
retaining the Deck for personal tests. Base commit:
`764af9ca5380f14a8b8967acb462dcb9814e7e17`, tree
`78d5081c79999360344625cc11bf15b5387d7f86`.

Primary claim: map the existing graphics selection, editor execution and runtime
capability boundaries to source and retained evidence, identify concrete gaps,
and select bounded follow-up experiments. This is source investigation, not a
graphics repair or physical qualification.

Basis: the operator's current instruction; AGENTS.md real-time, cross-boundary
and evidence rules; ARCHITECTURE.md sections 6.3–6.6, 7.1, 9 and 18.1/18.2/18.5;
the portable audio recovery D-022; AUDIO_RECOVERY_ROADMAP.md stage 3 and Execution.
Scope: manager readiness/profiles/runtime launch, native GUI transport, Windows
owner/editor/controller service, and retained BEAM/Blackhole/FRAGMENTS graphics
evidence. Dependencies and production behavior remain unchanged. The fixture is
this exact source tree and local generated tests; retained physical observations
keep their original artifact identities and limitations.

Acceptance: an anchored source/evidence review, focused local checks of existing
boundaries where executable, explicit failure propagation and test gaps, and
separate graphics, execution-isolation and combined-performance follow-ups.
No Deck access, remote observation, installation, configuration, publication,
process control or test instrumentation is authorized by this investigation.
No new GPU, audio, compatibility or beta-readiness claim follows. Preserve the
audio repair branch and artifacts; retain only sanitized source findings and
check results, commit and push the separate result. No deployed rollback is
needed because this work changes documentation only.

Result: the [source investigation](docs/GRAPHICS_RUNTIME_REVIEW.md) is complete.
Fifteen local Python checks, seven registered-backend Rust checks and the existing
C++ concurrent controller-update test passed. The review identifies missing
effective renderer/driver readback, controller-failure propagation into audio,
an owner-exception/thread-retirement hazard, and unmeasured owner/rendering cost.
These are source findings and proposed reproductions, not BEAM crackle attribution.
The next proposed jobs are graphics capability readback and production SDK
failure-isolation tests; physical comparisons remain deferred to the operator's
availability. Audio recovery stage 2 below is unchanged and still open.

## Active audio recovery stage 2: one attributed presentation gap

On 2026-10-02 the operator authorized stage 2 of the recorded recovery roadmap.
Base: `7be2a0b2ed81ff48e65f98701c8f58c77c7f472f`, tree
`8c0e0dc04af5816038a5665744428730ca48998d`. The first bounded claim is that
one reproduced physical audio gap is attributed to its responsible delivery
boundary and a causal repair passes a failing regression and a matched physical
captured-output comparison. Stage 2 as a whole remains open until the frozen
artifact passes the declared interaction and soak gates.

Current result: the focused callback-burst repair has passed source regression
and a captured-output physical SDK A/B on the Deck. Recovery6 also passed two
short Bitwig lifetimes. Longer interaction/soak and earlier unexplained failures
remain open. Exact reference application and native component bytes are restored;
the repair package and evidence remain retained. The next bounded job is the
frozen recovery6 interaction/soak, with no concurrent architecture or runtime
change. Detailed observations below retain earlier failures as failures.

Basis: the operator's recovery directive; AGENTS.md real-time and evidence laws;
ARCHITECTURE.md §18; decision D-022; AUDIO_RECOVERY_ROADMAP.md stage 2; and
INTEGRATED_BETA_DELIVERY.md acceptance method. Begin with the retained physical
Steam Deck, recovery1 Pure LoFi 1.0.0.6121, 48 kHz, actual/maximum block 512 and
bridge delay 512. Verify the exact installed identities against the retained
late-note-off receipt before running. Keep the selected runtime, licensed
environment, scheduling policy and delay fixed while reproducing and attributing
the failure. Once attributed, vary only the measured cause in the repair comparison.

The first captured attribution is a 6.000040 ms preemption of the actual Windows
render thread, inside the SDK call and across the missing block's presentation
deadline. A scheduling-only experiment on that thread completed 19,480 Pure LoFi
blocks without missing frames; it is a test intervention, not an installed fix.
The bounded repair requests SCHED_RR priority 5 through existing RealtimeKit from
the owning supervisor, using a Rust control-plane operation. It binds the target
to existing PID/start custody, the exact mapped session status inode and the
named render thread. It preserves other policies and records actual readback or
unavailability. This is post-start capability acquisition, not a new processing
readiness gate or a promise that the policy was effective from the first sample.
The exact source `d5e710a243d5c990e33eb832103098ff4a14e6b3`, tree
`f00ca2178f65e33730fe7385de089a6fafc8d318`, was built and installed as
`0.12.0recovery3`. Both existing native publications and the Windows host/runtime
remained unchanged. The selected application's matching host/source pair routed
both sessions through the new supervisor. Actual render-thread readback confirmed
RR 5 with RESET_ON_FORK automatically in two separate project lifetimes.

**The installed candidate fails whole-session continuity.** In the untraced
interaction run Pure LoFi lost 512 frames in one gap over 40,076 blocks;
FRAGMENTS lost none over 39,687 blocks. The gap arose after the final clean live
read and before processing stopped, during the stop/save/close interval; the
responsible operation is not established. The whole recording also contains an
unexplained 512-frame silent span during playback while later bridge counters
were still clean. Do not attribute that separate observation to this bridge gap.
A traced reopen with two play/stop cycles and saves completed 27,828 / 27,435
blocks without gaps and retired cleanly. It does not erase the failed run.

The [sanitized result](evidence/audio-recovery/2026-10-02-owned-render-scheduling.json)
retains every comparison, source checks, exact artifacts and limitations. Normal
Setup and Updates controls restored the exact recovery1 application afterward;
the entire registry, original project and reference copy are unchanged. The
candidate and full private captures remain preserved. The builder is stopped.
Attribution and an installed scheduling capability are demonstrated; the primary
continuity-repair claim and stage 2 remain open. Next capture must cover the native
worker, Windows render and DAW callback through the remaining late-session gap,
and separately distinguish the monitor silence from native-reference behavior.

The next bounded comparison reproduced no gap on unchanged recovery3: the
traced lifetime completed 27,947 Pure LoFi / 27,556 FRAGMENTS blocks; the
diagnostics-disabled lifetime completed 45,502 / 45,120. Both used external
scheduler capture, whole speaker-monitor recording, copied-project saves and
two play/stop cycles; both retired cleanly. Neither clears the earlier failure
or proves that observation caused it. The native worker's exact per-session
identity was ambiguous in the first external collector and is not used for
causal attribution.

The [observation repair](native-vst3-proxy/host/PRESENTATION_GAPS.md) retains
16 bounded missing-span records without the optional observer. It exports the
presentation clock bracket, existing parent callback, worker identity/progress,
queue counters and control/readiness observations at close. It preserves the
audio result and all delivery protections. The new regression fails on the old
source; 22 queued tests pass on macOS, including callback allocation checks.
Linux backend validation passed 89 tests with one existing Windows-fixture test
ignored. The exact source `361442f42366e4be3f333d7622809e99273d8d87`, tree
`c32a848dc4a5d631e74a5d5a52ec587271600786`, was built and installed as recovery4,
including both matching native publications. With sample tracing off, a controlled
40 ms suspension of the owned Windows test process produced four retained spans
and exactly the corresponding 2,048 silent frames in captured SDK output. Audio
resumed and ownership retired cleanly. This validates installed observation;
the deliberately injected stall does not attribute the earlier natural failure.
A separate 120-second native Bitwig 1 kHz reference had no missing spans or signal
recurrence violations after capture startup. The earlier monitor silence remains
unattributed. [Retained comparison](evidence/audio-recovery/2026-10-02-residual-gap-observation.json).
This is diagnostic coverage, not the remaining causal audio repair.

The untraced recovery4 chain then reproduced 1,024 missing FRAGMENTS frames.
Both missing requests had not been consumed by the native worker. External
capture shows a 23.273160 ms native-worker preemption across both deadlines;
the collector omitted namespace mappings, so the exact numeric per-session TID
association is a stated gap. A subsequent exactly mapped intervention on both
native workers accumulated no further bridge gaps for about 630 seconds, but
Pure LoFi had already lost 512 startup frames and the monitor contains additional
unexplained silent spans. Neither whole lifetime passes.

The next bounded source repair prepares native-worker scheduling through the
existing authenticated supervisor, before servicing setup or audio. It preserves
DAW-wide limits and policies, verifies the exact namespace TID and thread start,
and independently reads back the effective result. The preparation contract is
[documented here](native-vst3-proxy/host/NATIVE_SCHEDULING.md). Source
`8ade872366ddd0cce59577c4af014cd580e76df4`, tree
`fb49cd38047cd98c4d15f6b47e1210810bf44466`, passed 93 Linux backend tests
(one existing fixture ignored), five Rust target-selection tests, five supervisor
tests and 16 ownership/retirement tests. The new supervisor regression fails on
the old source; it tests preparation, not a deterministic reproduction of the
physical audio loss. All ten package-build steps passed.

Installed recovery5 verified automatic RR 5 on both native workers before setup
and audio, and on both Windows render threads at later readback. Its first whole
lifetime completed 52,935 Pure LoFi / 52,544 FRAGMENTS blocks without bridge
misses, but the monitor recording contains an unexplained 512-frame silent span
during editor interaction. The second lifetime lost one startup block per
plug-in (23,638 / 23,249 total blocks). Both gaps precede the added direct recorder
and scheduler collector. Native scheduling was already effective; Windows policy
at the first callback is not established. Independent progress reads place each
missing position in the worker's processing operation, without a pending control
request; they do not identify SDK time versus reply wait or host callback timing.
The later direct recorder accumulated 43 stream errors and the scheduler helper
exceeded its requested file bound and timed out during cleanup. Those observer
failures are retained separately and cannot explain the earlier startup gaps.

The [physical result](evidence/audio-recovery/2026-10-02-native-worker-scheduling.json)
therefore remains failed. Normal historical candidate publication and application
selection restored all recovery1 component bytes and both reference native
binaries. New publication revisions and software location were created; the
registry and location are not byte-identical to the earlier selection. Other six
publications, runtime, environment, original project and reference copy are
unchanged. All DSP owners and experiment recorders retired; the builder is
stopped. The retained reference is functional, not qualified gap-free.

Next bounded target: locate the first missing startup result across native
admission, Windows render readiness/processing, reply and presentation before
changing another mechanism. Separately qualify the additional recorder before
attributing editor-interval output silence. Do not infer callback bursts from a
worker-ready timestamp or first-sample Windows policy from a later readback.
Stage 2 and its continuity-repair claim remain open.

The next short traced recovery5 lifetime reproduced a different, directly
attributed gap at the final Pure LoFi callback before close. Request admission
to presentation was only 1.340 ms; Windows SDK processing took 1.943 ms and
native publication followed presentation by 0.894 ms. Control work had finished
about nine seconds earlier. The first startup in this run was clean. The whole
speaker capture has no internal exact-zero span during its musical signal;
this identifies a delivery defect, not an audible dropout in that recording.

The [bounded completion repair](native-vst3-proxy/host/COMPLETION_DEADLINE.md)
lets the prepared protocol-14 callback wait for already-due output within one
actual-block-duration budget. It retains the queued delay, ownership and epochs;
this is not the stage-4 zero-added-delay path. A burst regression loses 7,680
frames on recovery5 and passes after the repair with zero callback allocations.
Timeout, expiry, control exclusion and notification-race tests accompany it.
The final source `85c444ff2a1271f5d616ab3f3192ff2d36fb111e`, tree
`0ef273cacf66d45f38b9b7ce895728e1bd6f15bf`, passed 96 Linux backend tests
(one existing fixture ignored) and all ten package-build steps. Installed
recovery6 passed two short copied Bitwig lifetimes: 9,095 / 8,702 traced and
8,399 / 8,014 untraced Pure LoFi / FRAGMENTS blocks, with zero missing,
rejected or expired frames, no internal exact-zero captured span, and clean
retirement. Neither run exercised the completion wait.

A separate diagnostics-disabled physical SDK comparison deliberately paused
only the owned consumer for 40 ms, producing catch-up callbacks while a note
was held. With identical application, Windows host, runtime, plug-in state and
consumer, recovery5 replaced 1,536 captured frames with exact zeros; recovery6
used three completion waits and delivered every block, with no internal exact-zero
span or expiry. Maximum full SDK processing-call duration was 1.526402 ms.
Both arms captured identical opaque state before/after and retired cleanly.
Native scheduling remained SCHED_OTHER in this SDK host because it had no finite
realtime budget; Windows RR 5 was verified in both. Bitwig independently verified
native RR 5. The deliberate pause proves the burst boundary, not a device-output
deadline or the cause of every historical failure.

The [retained result](evidence/audio-recovery/2026-10-02-queued-completion-deadline.json)
therefore completes the focused burst-delivery repair. Stage 2 remains open.
Next: qualify the frozen recovery6 artifact through the declared interaction
and soak, covering startup, stop/restart/save/close and editors, and separately
attribute any captured silence without bridge misses. Do not claim the older
startup or editor-interval failure is fixed from this focused comparison.
Normal publication and installer/Setup controls restored all eight reference
application component hashes, both reference native binaries and their host
pairs. New selection/publication locations were created. Other six publications,
runtime/environment, original and reference projects are unchanged; service is
ready, owners and recorders are retired, tracing is off and the builder is stopped.
Audiobookshelf was running at final readback. The reference remains functional,
not qualified gap-free.

Scope: the actual callback, queue, worker, transport, Windows render and
presentation path; bounded preallocated telemetry; independent SDK consumer;
focused regression and evidence. Dependencies change only if required by the
measured cause. Preserve ownership, epochs, strict wire validation, original
projects and rollback artifacts. Do not weaken the queued D >= M contract or
fold runtime selection, manager redesign, new platforms or lower-latency claims
into this repair.

Capture every run in full, including failures, actual output, callback timings,
trace coverage, component hashes, settings and confirmed retirement. Locate the
missing result at its presentation deadline. Compare observer effects explicitly;
diagnostics remain off for acceptance. Test the failure and the repaired behavior,
including cleanup and stale-result rejection where affected. Use project copies
and the existing publication/selection mechanism for any installed successor.
One VM or builder at a time must leave Audiobookshelf resource headroom. Retain
private captures outside Git; publish only sanitized evidence. Commit and push
the bounded result without promoting or merging the frozen PR #200.

## Completed audio recovery stage 1

On 2026-10-02 the operator selected the [six-stage recovery roadmap](docs/AUDIO_RECOVERY_ROADMAP.md)
and asked to begin stage 1: settle the production architecture and one beta
contract. Base: `a308fbf93e623ba2b8b5bc5e029f06f440ae390c`, tree
`adc4e748f8b92642a7b644574864e73cb4efe406`. The inherited repair PR remains
independently reviewable; this assignment changes repository documentation only.

Primary claim: the actual installed processing path is mapped to its owners and
build features, consequential components are classified for retention or change,
and the recovery directive controls one consistent release contract. Scope is
the native proxy, Rust transport, Windows SDK host, runtime/profile/host assessment
and delivery boundaries needed for that decision. Basis: the operator's recovery
directive and roadmap; AGENTS.md, GOVERNANCE.md; ARCHITECTURE.md sections 4 through
10 and 15; applicable product decisions D-001 through D-009.

Completion requires a source-grounded review, explicit decisions and unresolved
questions, traceable local references, and acceptance criteria that distinguish
source tests, installed behavior and physical qualification. No Deck, runtime,
licensed environment, publication, project, VM or builder is changed in stage 1.
No audio defect, platform or release gate is closed by a document. Retain the
physical evidence and frozen beta branch unchanged; documentation is reversible
through Git.

Stage 1 is complete at the source-review and design level. The
[review](docs/AUDIO_RECOVERY_REVIEW.md) maps the installed call path and component
dispositions. Architecture section 18 and decision D-022 record the selected
direction; Integrated beta delivery now owns the reconciled current acceptance
criteria. Source paths, local links, whitespace and preservation of historical
result sections were checked. No executable tests were rerun for these document
changes. The next code deliverable is stage 2: attribution and repair of one
physical presentation gap under the retained settings. It has not begun in this
documentation assignment.

## Completed first repair: late note release; audio continuity still open

The operator stopped beta expansion and selected the audio-recovery directive on
2026-10-02. Recovery starts from `37b5d41e5ffc7876b4bab4ecc6e57737297233bd`, tree
`d4ce205d5a3722888d35ac5e0116b3018bca2a16`. PR #200 remains a frozen draft and
comparison source, not a beta candidate to promote.

Preserve the uncommitted internal57 receipt and private diagnostics outside the
checkout before intervention. Recover paired installed predecessors through
normal controls; keep licensed state, original projects and exact binaries.
Earlier usable receipts do not establish gap-free audio. Compare captured output
only after qualifying the recorder against an independent native signal.

The first repair claim is deliberately narrow: a host note-off whose timestamp
is already before the current nonempty block releases the identified note at
sample zero without killing subsequent processing. The retained internal56 SDK
input witness is offset -1661. Preserve valid offsets and note identity, other
event fields, strict wire validation, epochs and ownership. Negative note-ons,
future note-offs, zero-frame notes and invalid identities gain no new support.

Scope is the production native SDK input boundary, its focused regression and
an independent installed SDK consumer for physical comparison. The Windows
host, runtime, queue, buffer size, scheduler and manager behavior are unchanged.
Require a failing old-source regression, passing repaired callback with the
callback audit, and captured output from the exact installed pair. Keep physical
DAW continuity, reconfiguration and low-latency qualification separate and open.
Retain whole failed runs and restore the reference through normal rollback if a
successor fails. No source-only test may be reported as physical acceptance.

Basis: the operator's recovery directive; AGENTS.md “Keep the engineering
safeguards” and “Verification and review”; docs/ARCHITECTURE.md §7.5 “Events and
automation”; the existing event transport and installed publication boundaries.

The exact source `bad5c69cf5a1edbbec6a48bb29a5b734ba9b9569`, tree
`0edcc1b36cd4309785d7e37f91e567b884dd5bfd`, was built as internal-test package
`0.12.0recovery1`, selected through the installed GUI, and published for Pure
LoFi through normal product controls. FRAGMENTS retains its internal57 pair;
all other registrations are unchanged. The Windows host and selected runtime
are unchanged. Manager/frontend/Rust sources are unchanged, but rebuilt binary
hashes differ and are recorded explicitly.

The old-source regression failed; eight non-graphical native tests and one
filtered Rust event test passed. Two X11 panel tests were not run. On the
physical Deck, the same independent SDK consumer and -1661 input changed from
600 rejected callbacks, silent later notes and failed state capture to 720
successful callbacks, captured later-note audio, successful state capture and
confirmed retirement. Both repaired 7.68-second runs recorded zero missing
frames. This proves the bounded note-release repair, not dependable timing.

A copied Bitwig project exercised automation, both editors, save/close/reopen
and retirement. All four complete native lifetimes reported zero gaps and
rejections. Its speaker-monitor recording captures the first playback, not
resumed audio after reopen; meaningful unique-state recall remains unqualified.
The original project hash is unchanged. Earlier usable reference lifetimes
still contain gaps. See the [sanitized physical evidence](evidence/audio-recovery/2026-10-02-late-note-off.json).

The next bounded repair is one measured request-to-presentation gap on the
frozen physical artifact. Preserve queue epochs and ownership, locate the
result at its deadline, repair one cause and compare captured output with
diagnostics disabled. No lower-block, reconfiguration, installer, platform,
distribution or complete beta gate is closed by this first repair.

## Frozen predecessor: integrated self-service beta delivery

The operator selected the integrated delivery assignment and its refinements on
2026-09-30. Base: `ca95d1541a41a1aa7497a58752e26fc1f893b237`, tree
`28c295cc0d8b45c8e0504c7ec0c88e56d65781b9`.
Primary claim: one declared installed candidate completes the ordinary musician
journey for the exact qualified catalogue on Ubuntu and CachyOS, including
installer recovery, predictable first use, project recall, populated update and
rollback, with zero maintainer repairs in passing journeys.
The acceptance contract, workloads, release requirements and platform boundaries
are in [Integrated beta delivery](docs/INTEGRATED_BETA_DELIVERY.md).
Implement the necessary connections in this effort; do not seek a new scope
decision for each routine repair or treat component results as a completed beta.
Preserve application-owned runtime acquisition, prebuilt proxies, immutable
publications, paired components, vendor state and exact predecessors.
SteamOS delivery belongs to this effort. The working Deck remains selected until
a staged candidate and verified populated migration justify its controlled
update. No unrelated Deck/Pi installation or user project may be replaced.

## Retained predecessor: runtime and prebuilt proxy delivery

The operator selected implementation of tasks 1 and 2 and testing of task 3 in
[Self-service beta delivery](docs/SELF_SERVICE_BETA_TODO.md) on 2026-09-29.
Base commit: `13ed1d85e830d581ec297760e04e9f9433bfd671`.
One primary claim: the delivered package supplies its own exact compatibility
runtime and supported prebuilt proxies, and the normal managed installation
journey can use them without customer-installed Proton or development SDKs.
Use the disposable Ubuntu 26.04.1 x86-64 fixture first. Existing Deck/Pi
installations and user-owned environments remain under their existing custody.
Scope includes package construction/intake, runtime ownership, native
preparation/publication, normal frontend setup and focused acceptance tests.
Preserve exact identities, vendor-owned authorization, predecessor artifacts,
RT behavior and failures. Record notices for the exact redistributed contents.
Tasks 4 and 5 now belong to the integrated assignment above. No universal
platform or plug-in support claim follows from these predecessor results.
Record actual sound/editor/recall/reboot/recovery outcomes and any open gap.
The installed 1024/512 host-block refusal also selects the generic explicit
[larger-buffer configuration](docs/SELF_SERVICE_AUDIO_BUFFERING.md) repair.
It preserves the default and historical support envelopes; exercise the exact
successor through normal delivery and product controls.
The measured UI Automation null-provider removal fault also selects the exact
[preparation accessibility policy and isolated runtime correction](docs/RUNTIME_UIA_GUARD.md).
Apply the existing declared process policy to the measured module/runtime only;
retain its screen-reader limitation and candidate claim level. The isolated
Wine guard remains a construction/reference proof until separately delivered
and commercially retested. No existing bound runner is edited or replaced.

Internal30 has exercised the retained larger-buffer publication and exact
accessibility policy in the delivered Ubuntu package: normal insertion,
editor close/reopen, processed audio and normal instance cleanup. Cold preview,
state/project persistence, responsiveness and ordinary installer recovery remain
open. Tasks 1 and 2 are bounded Ubuntu delivery results; task 3 is not complete.

## Beta delivery: clean-machine portable first run

The current portability owner begins from post-PKG1 canonical main
`3dd598fb46a4fd3de909c658b289246e5e24b1f9` and has merged
post-product-controls main `de70bd29fed59140ff0396cfc1696cf56db7de75`.
Its one claim is an exact, explicitly adopted package generation that opens
the normal frontend and reports a truthful first-run state on a declared
clean graphical x86 Linux fixture. The [portability contract](docs/BETA_PORTABILITY.md)
and [Ubuntu receipt](evidence/beta-portability/ubuntu-first-run-2026-09-29.json)
record an internal-test Ubuntu 26.04.1 journey through GUI adoption, service
activation and reboot. The machine reported compatibility unqualified. This
is not a customer release, plug-in qualification or audio result. Debian's
target bytecode/package gate and CachyOS executable first run remain open.
No Deck installation or publication occurred.

## Completed source-only current plug-in controls without Diagnostics

The prior source owner began from post-first-run canonical main
`6545422c49b0cf3055b9e5f8a7ea0c7c465c4993` (tree
`b7c99d0458145e4af1c65afefc21883a80a2e06d`). Its one claim is that
Library can load the exact selected plug-in's current compatibility and
manager controls without waiting for the full historical Diagnostics
Snapshot. Admission checks the same current manager offer; the existing
mutation owner still verifies its exact physical target. See
[selected-product controls](docs/BETA_PRODUCT_CONTROLS.md) and the
[staged Deck receipt](evidence/beta-delivery/product-controls-2026-09-28.json).
It merged as `de70bd29fed59140ff0396cfc1696cf56db7de75`.

The exact fixture is the unchanged Steam Deck Desktop Mode, SteamOS 3.8.16,
Bitwig 6.1 installation, with Pure LoFi as the ordinary selected product and
BEAM/Serum 2 FX as experimental and effect readback checks. This source-only
change does not install a manager, publish a proxy, run Bitwig, qualify the
Push 3 note-release fix, or claim an Ubuntu/CachyOS musical workflow. The
installed private UI2 generation and all six user routes remain selected.

## Beta delivery: portable package and explicit first run

PKG1 merged normally into canonical main as
`3dd598fb46a4fd3de909c658b289246e5e24b1f9` (tree
`dd83bae6201060725ac76c0f7fdac34a7e465881`). The current beta
delivery effort builds from that authority. Its portable-package owner adds
one fixed system launcher, one Debian-family package format from the exact
PKG0/PKG1 roster, and an explicit guarded user-service activation step.
The exact package and evidence boundary is in
[Beta portability](docs/BETA_PORTABILITY.md).

This source owner does not select or install a Deck generation, adopt a user
generation on package installation, bundle Proton/SLR, publish a native proxy,
or qualify Ubuntu, Debian or CachyOS audio and graphics. The existing Deck generation, products,
publications, workspaces and rollback authority remain selected.

## Completed source-only package-kit authority — PKG1

PKG1 began from post-MIDI0 canonical main
`0385faaef3251e5c1036741b4e440ec5b66e5133` (tree
`87f3e9c5ccc446c85db77d17a1d774c748431fa0`) and merged forward
post-PB0-C0 canonical main `4a5b94e9f4c74cc997dba1fc8ed9b08087b8beb2`
(tree `21c29a264b36d36aaefa0b05b29d5f048d091bd5`). Its basis is
`AGENTS.md` “Keep the engineering safeguards” and “Verification and review”;
`GOVERNANCE.md` “What evidence means”; and `docs/ARCHITECTURE.md`
installed-software and rollback boundaries. The focused contract is in
[PKG1 native kit adoption](docs/PKG1_NATIVE_KIT.md).

**One claim:** a verified package may select one exact source-owned native
preparation kit in the same immutable software generation as its paired
manager/frontend, while retaining the previously selected kit and all product
state for exact rollback. Historical six-file PKG0 generations remain readable.
Selecting a kit does not create a managed proxy publication; UI2 must prepare
and test the exact successor candidate separately.

PKG1 was source-only and merged as recorded above. It did not install
a Deck generation, publish Pure LoFi, launch Bitwig, choose a release signing
key, or claim the Push 3 held-note symptom is physically resolved. PB0-C0 is
canonical source but remains uninstalled on the Deck. After PKG1 review and
merge, RC0 may integrate the paired schema-12 manager/frontend, this exact kit
and a separate Pure LoFi MIDI0 publication under one controlled release train.

## Completed source-only product overview — PB0-C0

PB0-C0 normally merged into canonical main as
`4a5b94e9f4c74cc997dba1fc8ed9b08087b8beb2` with tree
`21c29a264b36d36aaefa0b05b29d5f048d091bd5`. Its one claim is a fast
current-state Home and Setup overview for the exact Steam Deck, SteamOS and
Bitwig fixture, with four truthful readiness outcomes, one bounded next step
and an explicit sanitized local support export. The interactive path excludes
deep history; full Snapshot remains a separate Diagnostics route. The source
contract and earlier evidence are in [PB0-C0](docs/PB0_C0.md).

The public manager/frontend wire generation is schema 12; private schema-11
requests remain narrow historical recovery inputs. The final exact-head staged
read-only Deck gate recorded five ordinary release Overviews at a 1.569-second
median and 2.205-second maximum, and five Pulses at no more than 0.248 seconds.
All 16 products, four Setup rows and one FL workspace remained represented.
The 4,600-record manager metadata inventory and six routes were identical
before and after. The installed private UI2 generation remains selected; no PB0
generation was installed. The exact receipt and source identity are retained in
[PR #188](https://github.com/kasselvania/Linux-VST-bridge/pull/188). PB1 remains
outside the selected release train.


## Completed source-only native note-release correction — MIDI0

MIDI0 starts from canonical main `c920f2eca09bb27e92172b1f32e5899d45d2630b`
(tree `656049fe96aa9a0685c8326ca5a93058b3b0e08b`). Its basis is
`AGENTS.md` “Real-time laws”, “Slice discipline” and “Review standard”;
`GOVERNANCE.md` “What evidence means”; and `docs/ARCHITECTURE.md` §7.5
“Events and automation”. The exact source contract is in
[MIDI0 Push note release](docs/MIDI0_PUSH_NOTE_RELEASE.md).

**One bounded claim:** in a valid, admitted callback with at most 256 input
events on the supported bus, the native proxy skips poly pressure and note
expression value/text while preserving ordinary note-on and note-off events in
order. The source-only regression uses the production SDK `Processor::process()`
boundary; unknown event types and other invalid inputs still refuse. The
diagnostic counts callbacks containing recognized expression once each. The
operator's Push 3 held-note report motivates this repair, but the rejected
physical callback's exact input event type is not established.

The Steam Deck's selected manager, publications, runners, environments and
workspace remained unchanged during source validation. A physical
Push/Bitwig check on an exact installed successor with a new managed native
proxy publication is required before calling the observed held-note bug closed.
MIDI0 merged normally into canonical main as
`0385faaef3251e5c1036741b4e440ec5b66e5133` with reviewed tree
`87f3e9c5ccc446c85db77d17a1d774c748431fa0`. PB0-C0 includes that
source repair without claiming physical Push or audio acceptance.

## Completed read-only installed-state gate — PB0-R3

PB0-R3 starts from post-BG1-R0 canonical main
`62d556cfea57b17b567c5374a7a42b78fa22780c` (tree
`f0f9c430e303447ea32129cb85d2532b0174a816`). Its basis is
`AGENTS.md` “Keep the engineering safeguards” and “Verification and review”;
`GOVERNANCE.md` “What evidence means”; `docs/ARCHITECTURE.md` installed
software, runtime ownership and rollback boundaries; and the completed
[PB0-R2 owner audit](docs/PB0_R2.md) and
[BG1-R0 source contract](docs/BG1_RUNTIME_OWNER.md).

**One claim:** canonical source can read and project the exact installed Deck
state, verify the selected BG1 V4 history, and compute an exact PKG0 successor
predecessor plan without changing the installed generation or any managed
record. The bounded result and limits are in [PB0-R3](docs/PB0_R3.md).

No PB0, PKG0 or manager generation was installed by this slice. The current
private UI2 generation remains selected on the Deck. This completed gate
allowed a new PB0 integration on canonical main; old PB0 PR #179 was later
closed as superseded by the current draft PR #188.

## Completed source-only native command-session owner — BG1-R0

BG1-R0 starts from post-PB0-R2 canonical main
`d20d143b5e1beb8009d7ae9f4bc9b4bf18556204` (tree
`bbfd32cfed8007264ab8f361f7017da64dd0ba9d`). Its basis is `AGENTS.md`
“Keep the engineering safeguards”, “Shared failure-class check” and
“Verification and review”; `GOVERNANCE.md` “What evidence means”;
`docs/ARCHITECTURE.md` runtime ownership and update/rollback boundaries; and
the completed [PB0-R2 audit](docs/PB0_R2.md).

**One claim:** public source owns the selected BG1 V4 Lunacy runner's exact
native Proton command session, from pinned-component selection through keeper
readiness, instance launch and positive process-group retirement. It also
interprets the exact retained V1/V3/V4 transition and rollback records without
offering a new runner update or rollback operation. The source contract is in
[BG1 runtime owner](docs/BG1_RUNTIME_OWNER.md).

BG1-R0 was normally merged as `62d556cfea57b17b567c5374a7a42b78fa22780c`
with the reviewed tree `f0f9c430e303447ea32129cb85d2532b0174a816`.
The installed Deck generation, all publications, workspaces, installers,
environments, runners, authorizations and rollback files remain unchanged.
BG1-R0 does not import private renderer/build tooling, BG1 graphics experiments,
BETA0, RPR0, DIST0/CACHY0, ARM/Pi or PB0, and makes no new physical BEAM or
customer-package claim. The selected PB0-R3 continuation tests the canonical
Snapshot/Activity and PKG0 predecessor boundaries before PB0 may approach Deck
installation.

## Completed read-only installed-state audit — PB0-R2

PB0-R2 starts from canonical main `a35604781a703f35691bf193c8f301d94a02ed33`
(tree `189ae05f3b3917b653575add43b17362ee856ec6`), after the normal
source-only merge of PKG0 PR #184. Its basis is `AGENTS.md` “Keep the
engineering safeguards” and “Verification and review”, `GOVERNANCE.md`
“What evidence means”, `docs/ARCHITECTURE.md` installed-software and rollback
boundaries, and the reviewed UI1, UI2 and PKG0 contracts. The exact owner
audit and its limitation are in [PB0-R2](docs/PB0_R2.md).

**One claim:** determine whether current public source owns every selected
durable record and runtime needed to preserve the installed Steam Deck state
before any replacement. This slice performs read-only inspection only. The
installed private UI2 manager/frontend and all user-owned publications,
workspaces, installers, environments, projects, authorizations and rollback
generations stay untouched. PB0 PR #179 remains draft and cannot proceed to a
Deck replacement from this audit alone.

The read-only canary found a selected BG1 V4 Lunacy runner with a pinned native
command-session component. Public source recognizes its runner policy but lacks
the exact command-session launch and retirement owner in the installed runtime
scripts, nor the selected BG1 transition/rollback interpretation retained in
private history. That is an **independent canonical integration prerequisite**, not a
decoder uncertainty. PB0-R2 remains audit-only at this stop rule; the complete
canonical readback and package-preservation gates are not claimed. The older
PB0-R PR #180 is historical audit input and is not merge authority.

## Completed source-only package authority — PKG0

PKG0 began at public canonical main `1120efa349f84250fc9602503c28ea2fbd6fb8b9`
(tree `1956a0071f626ec7fa0808369b618520549d1825`) after the reviewed UI2
source merge. Its basis is `AGENTS.md` “Mission”, “Authority order”, “Core
product invariants”, “Slice discipline”, “Flatpak and SteamOS rules”, and
“Third-party and clean-room rules”; `GOVERNANCE.md` “What evidence means”;
`docs/ARCHITECTURE.md` installed-software and rollback boundaries; and the
selected [PKG0 contract](docs/PKG0.md).
The approved PR #184 head was `c5b7dc2a90719bd090f42ab2e25cc27808717ac1`
(tree `189ae05f3b3917b653575add43b17362ee856ec6`). It merged normally as
`a35604781a703f35691bf193c8f301d94a02ed33` with the same tree and parents
`1120efa349f84250fc9602503c28ea2fbd6fb8b9` and
`c5b7dc2a90719bd090f42ab2e25cc27808717ac1`.

**One claim:** a fixed installed package with one exact paired manager/frontend
generation and a complete declared release roster can be verified, copied to
immutable user-owned software, selected, updated without discarding a populated
catalogue, and rolled back to its exact predecessor. Interrupted route switches
have an exact recovery journal. Package uninstall retains user-owned software,
installers, environments, publications, compatibility history, workspaces,
projects, preferences and vendor authorization. The exact Proton/SLR runtime is
an external verified prerequisite; this package does not distribute it.

Scope is package assembly/verification/signing tools, fixed package intake,
immutable software generation and exact route switch/rollback. Source-owned
synthetic fixtures prove the owner boundaries. The existing Deck generation and
rollback remain untouched. PKG0 does not install on the Deck, qualify a clean
CachyOS/Ubuntu/Debian machine, merge private UI1/UI2/BG1/BETA0/RPR0/DIST0 work,
or claim a releasable Proton closure. Independent source review and PB0-R
installed-state convergence precede any Deck replacement. PB0 remains draft.
The repair binds root-owned package reads to one checked descriptor, verifies
Arch package metadata and removes the install hook, restores missing/stale
routes for the same generation, and retains the transition journal until the
effective user-service route has reloaded and matched the selected generation.

## Completed source-only canonical integration — UI2

UI2 starts from public canonical main `a9e3dab74465f8f39c180e454785261ad1a984ca`
(tree `2f8896efced382b8b9cb569d1007bf85dedc838a`), after the normal UI1
merge. It normally merged forward PR #183 via main
`bcad71d845b0dd899d23a55dcbfa90fd86f39121`. **One claim:** integrate guided compatibility checks, exact experimental
test publication, bounded operator results, and interrupted-result recovery into
public manager/frontend source. The paired operator wire generation is schema
10. Interrupted compatibility checks and test results now have manager-offered
continuations bound to their original immutable operations and fresh request
tokens. Existing durable installer, candidate, result and publication records keep
their owner-defined schemas; historical private schema-11 UI2 operation
requests remain readable for recovery, while current schema-11 requests refuse.
The [UI2 source contract](docs/UI2.md) names the bounded workflow and evidence.

The UI2 source integration did not install itself on the Deck or claim a new physical plug-in
result. The installed private UI2 generation remains Deck authority. It did
not import BETA0/package, BG1, RPR0, DIST0/CACHY0, PB0 or ARM work. Package
authority and remaining installed-state convergence require separate review
before PB0 can replace the installed manager.

## Completed source-only manager readback measurement

PR #183 merged as `bcad71d845b0dd899d23a55dcbfa90fd86f39121`.
One bounded full-snapshot fixture measured the repeated parse and validation of
installed profiles, then reused one exact profile set within each snapshot.
Fresh readback and mutation checks remain. The result and limits are in
[manager readback performance](docs/MANAGER_READBACK_PERFORMANCE.md). It makes
no Deck, larger-library, cold-start or user-visible navigation speed claim.

## Completed source-only canonical integration — UI1

Public canonical main after WD1 was `80e66bc050975a35a80c85bb2a1da3d7738f5c63`
(tree `585214bbbd44401a085c85cd9efcd0a2f7cdb979`). The UI1
[PR #181](https://github.com/kasselvania/Linux-VST-bridge/pull/181) reviewed head
before the final scan-order correction is `78a17f887fa0e04a44714489639cfe88715e87ee`
(tree `9c0d25939ee1ad917514459b134db5ca50683a6e`); the PR records the
submitted final head and tree. **One claim:** integrate guided native plug-in
onboarding and truthful grouped Setup presentation into public manager source.
Manager and frontend form one paired operator wire generation, schema 9. New
catalogues with onboarding runtime use schema 4; historical schema 3 remains
readable without a read-side rewrite. This is source-only work and does not
install itself on the Deck; the installed UI2 generation remains Deck authority.
No UI2, BETA0, BG1, RPR0, DIST0 or CACHY0 implementation was imported by UI1.
UI2 canonical integration now owns the separate guided-result source step above.
PB0 remains draft behind UI2 and separately reviewed package authority.
This source integration makes no new physical support claim.

The completed FN1/PW1 work below remains separate ARM history. PSL1 is a
proposed separate Pi slice and has not begun.

## Completed managed Windows DAW slice — WD1

WD1 [PR #169](https://github.com/kasselvania/Linux-VST-bridge/pull/169) adds
one manager-owned Serum 2.1.5 Windows VST3 installation inside the existing FL
Studio trial workspace. The retained [Deck result](evidence/wd1/deck-serum-direct-2026-09-25.md)
establishes FL discovery, audible piano-roll playback, a preset change,
parameter automation, normal close, and fresh-session playback. The FL
application and workspace identities, native bridge publications, runner, and
audio settings were preserved. The operator deferred WAV export; trial-mode
FLP save/reopen, licensed recall, ASIO, and physical MIDI remain unqualified.
WD1 is a bounded parallel lane, not a new native-bridge support claim.

## Completed Pi state-safe first-note prewarm — PW1

PW1 was developed separately on FN1
[PR #174](https://github.com/kasselvania/Linux-VST-bridge/pull/174) head
`1399a9655b6d1c8abb99f7058c8b34d28bcb76ab`, tree
`1e0a92909f0f94e7be136fe1fc4cbfc92edb8bdb`. FN1 merged first as
`64251ef1c328849997cda9fb9381ce2c08916f4c`; PW1 PR #175 was then
retargeted to canonical `main`. Its basis is the operator's
Pi repair direction, AGENTS real-time and slice laws, GOVERNANCE “What evidence
means”, ARCHITECTURE §§7 and 13, FC-AUDIO-001, and the
[FN1 physical attribution](evidence/fn1/serum-first-note-2026-09-25.md).

**One claim:** for the named Serum 2.1.5 default-state Pi fixture, opt-in
startup-only note-on/note-off processing followed by stop, deactivate, and an
exact second state restoration removes the repeatable first-user-note gap
while preserving the serialized state, ordinary subsequent output, and clean
retirement. The
normal callback, shared transport, protocol 12/map v1, 256-frame quantum,
512-frame reserve, 48 kHz/512 JACK setup, GE Proton/FEX runtime, governor,
scheduling, vendor settings, and installed selections remain fixed. Output
from the startup note is discarded before JACK activation. A restoration
mismatch or prewarm timeout refuses readiness. The experiment does not claim
a general Serum repair, a vendor-internal allocation result, arbitrary preset
equivalence, or Pi musical qualification.

Source scope is the standalone appliance startup sequence and focused
verification; physical scope is a private source candidate, the same state/module,
editor closed and the retained two-note MIDI fixture. Compare first-user-note
Windows process span, delivered and missing frames, output presence, state
readbacks and exact cohort retirement with the retained FN1 baseline. Retain
the actual result even if prewarming merely moves or fails to remove the gap.
Do not import BR1/BR1R recovery or install a new normal service as part of PW1.

The staged [PW1 physical result](evidence/pw1/serum-default-prewarm-2026-09-25.md)
contains a fresh unchanged FN1 baseline between three runs of the final PW1
build. The baseline again lost 1,280 first-note frames; all three PW1 runs
lost zero frames, retained byte-identical state readback and matching control
readbacks, delivered nonzero note audio, and retired cleanly. PW1 moved the
cold approximately 43 ms call into an approximately 136–140 ms startup
sequence. It does not establish vendor-hidden state equivalence, arbitrary
preset behavior, a CPU saving, or musical qualification. FN1 and PW1 were
reviewed separately. PW1 remains off by default, and no normal Pi generation
was installed.

## Completed Serum first-note attribution — FN1

FN1 starts from canonical `main` merge commit
`25a6cfc3824164d4fdde79abda758ffdca636998`, tree
`ed16e8d971dda8c9b844a4235d94dd1a58e14f57`, after PI-R
[PR #173](https://github.com/kasselvania/Linux-VST-bridge/pull/173)
merged. Its basis is the operator's 2026-09-25 first-note direction,
`AGENTS.md` real-time and evidence rules, GOVERNANCE “What evidence means”,
ARCHITECTURE §§7 and 13, FC-AUDIO-001, and the
[PI-R physical result](evidence/pi-reconciliation/serum-default-2026-09-25.md).

**One claim:** attribute the repeatable approximately 34 ms first-note
Windows `processor.process()` span on the canonical Pi source to caller
execution, worker dependency, scheduling, translation/JIT, faulting or
blocking, or state the exact remaining attribution gap. Bind the first slow
request and one later note request to epoch, sequence, position, MIDI identity,
native service stages, Windows wall and CPU counters, and Linux task activity.
Diagnostics remain optional and bounded; they do not decide whether audio
proceeds. Retain source/build identities, raw private records, sanitized
findings, observed first-note gap and clean shutdown.

The fixture remains the private Serum 2.1.5 module and captured default state
from PI-R, editor closed, on the Pi 5 with GE Proton/FEX, JACK 48 kHz/512,
protocol 12/map v1, 256-frame quantum, 512-frame reserve, ondemand governor,
normal scheduling/affinity, and the existing two-note fixture. No runtime,
plug-in state, vendor setting, queue, timeout, recovery policy, governor,
priority, affinity, quantum or reserve change is admitted. FN1 does not
prewarm, optimize, qualify musical usability or replace an installed service.

The retained [FN1 Pi result](evidence/fn1/serum-first-note-2026-09-25.md)
binds the repeatable 1,280-frame first-note gap to a roughly 34 ms Windows
call. The caller ran through the span, aligned user-space samples landed in
the FEX ARM64EC boundary, and new Serum guest-code map entries appeared during
that call. This supports a cold translation/JIT mechanism for the exact
default-state fixture. FN1 makes no repair or speed claim. PW1 is the
separate startup experiment selected from that result.

## Completed Pi source reconciliation — PI-R

The operator selected the minimal current-main Pi standalone reconciliation after
the shared-storage repair merged as PR #172. PI-R starts at canonical `main`
`f7668b6ffcb2ae1261d121514aaaed0c9b8986c7`, tree
`00495de1d8653a26e25b331fef1f93348d14606b`. Its basis is the operator's
Pi repair direction, the real-time and slice laws in `AGENTS.md`, GOVERNANCE
“Decisions” and “What evidence means”, ARCHITECTURE §§5.2, 5.7–5.9, 6.3–6.6,
7, 13 and 15, draft PR #171 at `6fe296bd1139b071c9ae6ffb7da628b519309bd2`,
and FC-AUDIO-001/002. PR #171 remains unmerged and is cited as evidence, not
repository-local authority.

**One claim:** the standalone ARM JACK/MIDI frontend and explicit Pi execution
adapter can run against the canonical shared bridge core and Windows host,
using protocol 12, map v1, a 256-frame processing quantum and the existing
512-frame Serum presentation reserve. PI-R may add the minimum Windows host
architecture check needed to preserve the cross-process mapping contract.
Source scope is `rpi0/standalone`, `rpi1/standalone`, source-owned `rpi2` binding
metadata, the bounded `rpi0` backend adapter, and that host handshake. The
commercial fixture is the existing private Serum 2.1.5 module on the Pi 5,
with JACK 48 kHz/512 and its pinned GE Proton/FEX runtime. Build artifacts,
private configuration and state stay outside the repository.

Acceptance requires ARM release build and focused source tests, exact
architecture/identity validation, normal MIDI/audio and state operation,
supervised lifecycle and clean retirement on the Pi. An incorrect layout,
module, runtime, state, or response must refuse explicitly. Retain the actual
physical observations, including any incomplete gate. The comparison arms are
the pre-AS1 `b76c9e2` base plus identical reconciliation R and the post-AS1
base plus R. Establish the runnable pre-AS1 arm before attributing any measured
effect to the storage repair.

This slice does not import BR1/BR1R recovery, change callback policy, runtime,
JACK configuration, governor, graphics, quantum, reserve, queue, timeout,
vendor settings, state identity or installed selections. It does not qualify
musical usability or a performance gain from a successful source build.

The staged [PI-R Serum default-state result](evidence/pi-reconciliation/serum-default-2026-09-25.md)
establishes a runnable current-main Pi source candidate and a matched
four-pair pre/post-AS1 comparison, including one reverse-order pair. Mean
completed-request service fell by 14–20 µs per pair, while the same
1,280-frame first-note gap remained in all eight runs and CPU ranges
overlapped. All sessions retired cleanly. This is
candidate evidence for [PR #173](https://github.com/kasselvania/Linux-VST-bridge/pull/173),
not approval of the old recovery policy, an installed change, or Pi musical
qualification. The next repair must be selected from the repeatable first-note
Windows processing span rather than folded into PI-R.

## Completed shared-audio source slice — AS1

The operator selected a focused, shared audio-execution storage repair. AS1
starts from canonical `main` commit
`b76c9e2270c63c78be2a51adf6562a3f838ffa7f`, tree
`74e13f5b1e10bd4a53d8b626657c403320fb62d2`. Its basis is the operator's
2026-09-25 instruction, AGENTS real-time and slice laws, GOVERNANCE “Decisions”
and “What evidence means”, ARCHITECTURE §§5.2, 5.7–5.9, 6.3–6.6, 7, 13 and 15,
[draft PR #171](https://github.com/kasselvania/Linux-VST-bridge/pull/171) at
`6fe296bd1139b071c9ae6ffb7da628b519309bd2` (its
`docs/ARM_PORTABILITY_REASSESSMENT.md`), FC-AUDIO-001/002 and the
SUPPORT_MATRIX exact processing conditions. PR #171 is not merged into this
source base.

**One source claim:** after session setup, the selected shared processing
request/reply path uses reusable, bounded per-instance bridge storage without
recurring allocation, reallocation or deallocation. The covered path is the
native transport worker's audio mapping, event/context request encoding,
mailbox or socket frame exchange and result decode, plus the Windows host's
request decode and result publication. Protocol minors 5 and 7–13, float32,
negotiated blocks 0–256 where admitted, up to 256 input events, 64 returned
events, 128 parameter points and 4096 returned payload bytes are the bounded
source contract. Protocol, quantum, reserve, queue, state, recovery policy,
checks and callback ownership are unchanged. SDK/vendor/runtime allocations,
setup/state/lifecycle/error paths and historical protocol minors 1–4 are not
part of this claim.

Changed implementation scope: `native-audio-client` mapping/frame/event and
endpoint helpers; `native-vst3-proxy/backend` session/context/mailbox storage;
Windows `ap1_protocol.h`, `delivery_mailbox.h` and `mapped_processing.cpp`;
focused tests and this slice entry. No runner, FEX, Wine, graphics, governor,
priority, JACK, capacity, timeout, policy, vendor setting, plug-in, frontend
or installed generation change is admitted. Acceptance requires exact wire
equivalence, first-block and repeated zero bridge allocation counts, maximum
valid traffic, zero/partial blocks, multi-output checks, malformed responses,
independent-instance ownership and a Windows production build. Failure must
remain explicit and must not release a mailbox slot before its reply is copied.

The exact Pi comparison fixture is a Raspberry Pi 5 at JACK 48 kHz/512 using
the existing supervised standalone path and Serum 2.1.5 module digest
`501e7bb3dd9cafe416b3412df3d4e084c01b7468201e5690b9009d7ecd4e5283`,
Windows host digest `4ab203ab8aa25555eebce6782cd52a11643a1b935abe8a2ce0cb04baeb453161`,
protocol 12, 256-frame processing quantum and 512-frame presentation reserve.
The BR1 gate's unnamed preset is not a reproducible fixture identity. No AS1
physical comparison may be attributed to this patch until a runnable
current-main/Pi source pair has been reconciled in a separate prerequisite,
built with matching options, and baselined before applying the AS1 change.
The Pi fork's map-v2/512-capacity specialization and current main's
map-v1/256-capacity plus protocol-13 multi-output support must be reconciled
without importing BR1/BR1R recovery policy. Keep all installed generations,
private state and experimental candidates in place. A source pass is not a
musical or performance qualification; record the physical result or exact
blocker in the implementation PR.

The native Linux DAW bridge remains the primary release-driving product.
WD0 merged into this canonical base through
[PR #163](https://github.com/kasselvania/Linux-VST-bridge/pull/163).
[WD1 PR #169](https://github.com/kasselvania/Linux-VST-bridge/pull/169) is
the active, separate managed-Windows-DAW lane; it is not an AS1 dependency or
part of this source change. No Deck-mutating work is selected by AS1.

## Completed native manager slice — UI0

UI0 [#160](https://github.com/kasselvania/Linux-VST-bridge/issues/160) is
merged and physically accepted on the Steam Deck. The installed immutable
manager/frontend generation is
`29e52e7537c2e150a68db7f66799621a94d39b7aac7b20e22aded0d03a28facf`;
its sanitized [Deck result](evidence/ui0/deck-acceptance-2026-09-24.md) records
ordinary and narrow navigation, one Serum 2 session, manager close/reopen,
continued Bitwig audio and clean retirement. The previous manager/frontend
generation remains a rollback target. Home, Plug-ins, Workspaces, Activity,
Setup and Diagnostics, including `ActivityCertainty` and exact session routing,
are accepted source and installed behavior. WD0 preserved that native manager
boundary.

The six selected native publications, Serum candidate D,
`x11_touch_routing_v2`, required exact Windows host/source pairs and runner
policies remain current product state. WD0 workspace work did not alter the
native bridge service, publication, proxy, capacity or audio path.

## Completed bounded source slice — WD0

WD0 [#158](https://github.com/kasselvania/Linux-VST-bridge/issues/158)
merged as [PR #163](https://github.com/kasselvania/Linux-VST-bridge/pull/163)
at the AS1 canonical base above. Its bounded result is a managed FL Studio
workspace with repeatable same-workspace installation after clean uninstall,
append-only operation history, a normal managed trial-mode launch, saved-file
presence, and confirmed cleanup. Stock-audio/export behavior and licensed
saved-project recall remain unqualified as recorded in
[the Deck result](evidence/wd0/deck-trial-2026-09-25.md). The original
workspace migrated from schema 1 to schema 2 and installed official FL Studio
`26.1.5.5618`; the earlier install/uninstall receipts remain. The installer
returned a nonzero outer status, retained as historical `needs_user_action`,
while the later managed launch completed with confirmed cleanup. Licensed
saved-project recall remains unqualified in trial mode. The broader issue
#158 remains open; its merged WD0 source slice is complete.
