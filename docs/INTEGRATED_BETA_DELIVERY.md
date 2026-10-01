# Integrated self-service beta delivery

Selected by the operator on 2026-09-30 from source
`ca95d1541a41a1aa7497a58752e26fc1f893b237`, tree
`28c295cc0d8b45c8e0504c7ec0c88e56d65781b9`.

## Outcome and basis

A declared candidate completes install, vendor authorization, discovery,
publication, musical use, save/reopen, reboot/reopen, recovery, populated update
and predecessor restoration through ordinary product controls. Passing journeys
require zero maintainer repairs.

Basis: AGENTS.md Mission/Core product invariants/Slice discipline and the current
working rules; ARCHITECTURE.md 5.1 Manager core, 5.3 Environment manager,
5.4 Installer supervisor, 5.10 Host publisher, 6.4 Thread affinity,
6.5 Component/controller/state, 7 Transport architecture, 15 Test architecture;
SELF_SERVICE_BETA_TODO.md tasks 1–5; SELF_SERVICE_DELIVERY.md Runtime setup,
Prebuilt publication and Package selection dependency; FAILURE_CLASSES.md and
SUPPORT_MATRIX.md for exact predecessor observations. Current operator scope
supersedes the earlier tasks-1/2-only selection. Historical evidence remains as
recorded.

## Implementation scope

Manager ownership/readback, frontend Setup/Library/recovery/navigation,
non-real-time admission and keeper preparation, host/proxy lifecycle and state,
phase-attributed audio observations, package generation/adoption/rollback,
distributable first-party fixtures, runtime intake/recovery, catalogue and release
tooling, and customer instructions are in scope. Rust remains the product
language; SDK C++ and bounded existing supervisor interfaces retain their roles.
Dependencies may change only where required for this outcome and with their
exact identities and notices retained. Profiles remain declarative.

Retain working generation components side by side. A changed host/proxy must not
silently replace a publication's exact pairing. Software restoration must not
restore or erase user projects or vendor authorization. No customer compiler,
SDK, preinstalled Wine/Proton or Steam-client prerequisite for the supported
prebuilt catalogue. No invented empty vendor state, licensing bypass, arbitrary
process termination, root filesystem unlock, or unrelated visual rewrite.

## Platforms and catalogue

Ubuntu 26.04.1 x86-64 and CachyOS 260809 x86-64 have clean and populated disposable
baselines. Use the same candidate source and component manifest with target
package/dependency metadata. Debian 13 continues package/startup coverage.
SteamOS 3.8.16 / Bitwig 6.1 on the working Deck requires a protected-system
delivery route, staged readback, retained predecessor and controlled regression.
VM signal capture establishes functional processing; physical audio/GPU claims
require the corresponding hardware test.

Track exact Pure LoFi, Pigments, FRAGMENTS, Serum 2 and Serum 2 FX builds.
The initial useful beta subset must include an instrument and effect from at
least two vendors with a common qualified Ubuntu/CachyOS subset. Exact versions,
module/class identities, runner, host, proxy, license channel and capability
results determine the matrix. BEAM-specific graphics work is nonblocking for
other qualified combinations. Unmatched module bytes receive a useful catalogue
explanation without a source-build fallback.

## Acceptance method

Develop with first-party stateful instrument/effect and held/partial/crashing
installer fixtures through the delivered paths. Commercial acceptance uses a
small authorized set whose DAW and vendor configuration permits saving.
Save-disabled demos cannot establish recall; normal vendor login/EULA/device
selection is allowed. A paid activation on every development build is not needed.

The assembled qualification run uses one fixed candidate. Before it begins,
retain baseline IDs, complete component identities, exact audio settings, start
and end phases, and the tested subset. Predeclare three cold loads (including
reboot), five warm loads, three editor close/reopen cycles per product, and two
repetitions of each supported recovery case. Run ten minutes of steady
processing for each audio workload after explicit processing readiness, with
startup and retirement counters retained separately. Do not discard a startup
failure by moving the measurement window or dismiss unexplained active gaps.

Initial functional mixing workload: 48 kHz, 1024-frame DAW blocks, one instrument
and one effect individually and then together. Hardware live-use workload:
48 kHz, 512-frame blocks, with total measured/reported latency retained. Larger
buffers must be presented with their latency tradeoff. Cold launch must finish
within 30 seconds and warm launch within 5 seconds for a responsive beta claim;
slower bounded results remain limited. Active processing must have no unexplained
missing frames for the declared supported workload. These are prospective
criteria, not a reclassification of internal30 observations.

Installer cases: normal completion, held exact operation stopped in visible
Setup while general readiness is unavailable, closed parent with owned helpers,
partial installation, interrupted frontend, and retry after confirmed retirement.
Continue from the same Setup card to discovery or a useful retry; backend fresh
operation ownership remains mandatory.

Recall: recognizable parameter/preset changes and automation, save, DAW close,
reopen, machine reboot and reopen; verify restored behavior and exact identities.
Populated update: test the same retained project before/after a changed component
generation and after visible predecessor restoration, without changing account.
Keep project data independent from software rollback.

Negative cases include stale recovery targets, mismatched components/module
bytes, unresolved cleanup, interrupted intake/update, insufficient customer disk
space, and unavailable services. Preserve the working predecessor and show an
actionable result. Hypervisor/viewer failures invalidate affected measurements;
customer disk exhaustion is a product recovery case.

## Audio phase measurement

The source successor keeps whole-session counters and separately counts startup
and processing delivery. A callback is processing only when the worker has
acknowledged START for that exact current epoch; classification occurs once at
the host-block boundary. Each group records admitted, missing, gap, expired,
delivered and declared priming frames. Priming stays visible, and unexplained
missing frames after readiness still fail the declared workload. No late choice
of a favorable measurement window is allowed.

The callback adds one readiness load and six fixed atomic counter updates; the
production callback test measured zero allocations, reallocations and frees
over 1,100 host blocks. The control worker retains up to 128 fixed phase records
and counts omitted records explicitly. It writes those records after processing
ends, followed by retired or retirement-unconfirmed status. Intermediate atomic
observations are independent, not a simultaneous queue snapshot; final totals
and confirmed retirement determine acceptance. All 19 queued-backend tests pass.
This is source instrumentation, absent from internal32. Earlier unphased runs
are not retroactively described as clean processing.

## Delivery and reporting

Internal42 selected coherent Pure LoFi and FRAGMENTS successors on the populated
licensed Deck through ordinary check/prepare/apply controls. The original saved
project reopened after a normal reboot, both editors completed three close/reopen
cycles during playback, and a separate updated project copy saved and reopened.
The original project digest stayed unchanged. Vendor state is recaptured after
restore; the new Pure LoFi readback is not byte-identical to its saved capture.
Visible preset, parameters, automation and musical behavior remain the recall
checks. These results do not establish universal state fidelity or update rollback.

The same fixed candidate failed audio acceptance at 48 kHz / 512 host frames:
Pure LoFi retained 13,056 missing processing frames in 35 gaps; FRAGMENTS retained
3,328 in eight gaps. Both startup phases show only the declared 512-frame priming,
and both owners confirmed retirement. Counters cover the complete processing-ready
lifetime, including transport-idle/editor time; the ten-minute musical loop is a
subset. Queue/reply delay and ordinary thread scheduling were observed, without
establishing a unique scheduler cause. Viewer recovery was infrastructure work,
so no cold launch latency qualification follows from that run.

Ubuntu42's ordinary reference-effect check refused before inspection at the
canonical action lock, preserving its installation. A later observation saw an
overview hold that lock for at least 5.1 seconds; the original refusal's holder
was not retained. The source correction keeps status observation off user-action
serialization while preserving registry/owner capture and exact final freshness
checks. Its regression succeeds with the action lock held and still refuses
changed watched source. The affected operator tests pass (74 passed, one ignored),
as does Clippy. Installed successor acceptance remains required. The
[installed receipt](../evidence/self-service-delivery/internal42-populated-recall-and-deadline-failure.json)
retains package manifests, publication predecessors, recall, phases and failures.

Internal43's ordinary Ubuntu reference-effect check acquired both canonical
locks immediately (28 and 26 microseconds) and completed preliminary inspection.
It then failed service restoration at the caller's 70-second reply timeout.
The service finalizer later restored readiness, while the refused request stayed
retained. That is a failed first restoration, not a completed customer journey.
The [internal43 receipt](../evidence/self-service-delivery/internal43-recovery-handoff.json)
also records normal populated application selection on the Deck, with the
registry and both original and updated-copy project digests unchanged. Project
recall after that manager update and rollback are not qualified by this receipt.

The restoration path was independently re-verifying the same runtime for each
registered environment while service startup verified it concurrently. The
successor uses the same process-owned launch preparation as native admission.
Preparation owns no registry reservation; subsequent readiness rechecks exact
registration, file identities and the retained execution pair. Changed bytes or
registration refuse readiness. The deadline remains bounded. This source
correction requires the next delivered frontend retest; it does not establish
the unique cause of all prior startup or audio failures.

Internal43 reopened the licensed Deck project copy and processed the musical
loop. Its whole processing-ready lifetimes still failed audio acceptance:
Pure LoFi lost 39,680 frames and FRAGMENTS lost 6,912. An exact, temporary
RTKit scheduling experiment promoted the four owned delivery/audio workers,
then normal project closure retired all workers and the diagnostic flag was
removed. FRAGMENTS retained another 512 missing frames after promotion. Pure
LoFi's first-32 trace retention was already exhausted, and the scheduling policy
was not sampled again during the later interval. Neither improvement nor a
scheduler-only cause is established. The
[diagnostic receipt](../evidence/self-service-delivery/internal43-audio-priority-diagnostic.json)
preserves those limits, phase counters, exact instance identities, state-operation
hashes and unchanged original project. Manual promotion is development work;
it cannot qualify a zero-intervention product journey.

Nested current/product projections were also discarding their outer request's
digest observations. They now retain the same process-local observations for
that request; every reuse still reopens and compares complete file identity.
An execution admission starts a fresh byte scope and cannot borrow readback
observations. The regression covers nested reuse, same-size mutation and fresh
launch isolation. The Linux library run passed 196 tests and refused one
historical-profile fixture owned by a different build user; that test then
passed against a correctly owned copy. All-target Clippy passes. Installed
response-time and frontend recovery acceptance remain required.

The next native diagnostic retains the first 16 and most recent 16 gap traces,
counting displaced records explicitly, and includes the exact mailbox reply's
Windows timing with each correlated gap. This avoids losing every later failure
when early misses exhaust the bounded history. Storage and the existing file
and line limits remain bounded; observation stays outside the audio callback.
The 83 native library tests pass with one Windows-fixture test ignored when run
serially, and all-target Clippy passes. An earlier concurrent test run retained
three `Full` refusals from tests sharing the production instance table. This
diagnostic is not an audio repair and is absent from internal44.

The internal40 reference effect's input witness captured a parameter point at
offset 1024 in a 1024-frame host block during saved automation. The prior bridge
treated this curve endpoint as an invalid sample and permanently refused later
callbacks. The Rust successor preserves linear segments at each 256-frame
transport boundary and carries the exact future endpoint to the next continuous
callback. A new explicit start, GUI edit, seek or Stop takes precedence. Native
instance storage is prepared before activation; the production callback test
observed zero allocations, reallocations and frees. Windows transport and note
offset validation remain strictly inside their respective block extents.

An implicit curve crossing a chunk boundary needs an actual left anchor. The
successor refuses an unavailable baseline before admitting any chunk (0x107);
descriptor defaults cannot stand in for current vendor state. This remains a
declared capability limit, not universal automation support. Interior points,
jumps, repeated positions, endpoint values, overflow and zero-frame flush have
source checks. Internal40 retained processing-phase gaps before the refusal;
this translation alone cannot qualify its audio. The
[installed failure and source receipt](../evidence/self-service-delivery/internal40-41-recovery-and-automation.json)
also preserves the separate DAW startup crash, cache rescan and failed Deck
service-restoration result. A coherent installed successor must repeat those
journeys.

Internal44 completed the effect's normal Check compatibility, Prepare and
Replace controls on the populated Ubuntu account. Ardour's normal rescan and
reopen loaded both references and restored their visible automation. Processing
still missed 4,352 effect frames and 7,168 instrument frames; the effect then
reported contained silence and refused state. Normal DAW quit confirmed owner
cleanup, and the original project stayed unchanged. The failed snapshot is not
recall evidence. The [installed receipt](../evidence/self-service-delivery/internal44-populated-reference-automation-failure.json)
retains those outcomes and separates the SDK's false terminal label from the
Windows owner's normal termination.

Source review found the automation refusal's 0x106 collision with IF2's terminal
result. The successor reserves 0x107 for a curve refusal and tests that it cannot
claim successful terminal silence. It configures the exact SDK parameter IDs
once before processing (8,192 maximum), with no inferred initial values. Accepted
last-sample values supply the SDK's implicit previous point at position -1,
including after a transport seek; a future endpoint remains a different fact.
GUI edits, state restore, Stop/restart and recovery invalidate those values.
Recovery retains the exact parameter census. No descriptor defaults or opaque
vendor-state interpretation seed the cache. The [official parameter queue contract](https://steinbergmedia.github.io/vst3_doc/vstinterfaces/classSteinberg_1_1Vst_1_1IParamValueQueue.html)
is the basis for that interpolation.

The extension's `ap22_backend.h` is native-only. The pinned Windows host's
shared AP10 header and exact source pair stay unchanged. The first assembler
refused the shared-header edit before producing a package; the separated header
then passed the same nine SDK/X11 tests. No Windows host was rebuilt or swapped.

The successor passes 86 macOS and 87 Linux registered Rust tests with one
Windows-fixture test ignored on each platform, all-target Clippy, and nine native
SDK/X11 tests. Sparse curves and the production entry point retain zero callback
allocations/reallocations/frees. These are source regressions; the fresh installed
candidate must still prove recall and audio. Internal45 retains the collision and
is unqualified. None of these repairs resolves the separate missing-frame failure.

Produce installable candidate artifacts and version/component manifest;
authenticated distribution, required notices/SBOM and customer install/update/
rollback/support instructions; exact platform × plug-in passed/limited/failed/
not-tested matrix; frontend recovery and persistence results; phase-attributed
audio; maintainer intervention accounting; release blockers and nonblocking
backlog. No customer release claim until these requirements pass.

Retain useful sanitized evidence under evidence/self-service-delivery/. Private
vendor binaries, accounts, authorization, prefixes, presets and project state
remain outside Git. Preserve failed runs and valid unchanged sub-results.
Rerun affected journeys on the corrected fixed candidate rather than assembling
a fictional pass across repaired builds.
