# Integrated self-service beta delivery

Originally selected by the operator on 2026-09-30 from source
`ca95d1541a41a1aa7497a58752e26fc1f893b237`, tree
`28c295cc0d8b45c8e0504c7ec0c88e56d65781b9`.

Updated on 2026-10-02 for the operator-selected [audio recovery roadmap](AUDIO_RECOVERY_ROADMAP.md).
The acceptance method below is the current beta contract. It supersedes the
earlier ten-minute, 512/1024-frame release criteria, not the historical results
retained later in this document. PR #200 remains a frozen draft. Implementation
ordering is controlled by CURRENT_SLICE.md and the roadmap.

## Outcome and basis

A declared candidate completes install, vendor authorization, discovery,
publication, musical use, save/reopen, reboot/reopen, recovery, populated update
and predecessor restoration through ordinary product controls. Passing journeys
require zero maintainer repairs.

Basis: the operator's 2026-10-02 recovery directive and roadmap; AGENTS.md and
GOVERNANCE.md; ARCHITECTURE.md 18 Audio recovery and portable execution,
5.1 Manager core, 5.3 Environment manager,
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
SDK, preinstalled Wine/Proton or Steam-client prerequisite for general preparation
within the implemented format/ABI boundary. No invented empty vendor state,
licensing bypass, arbitrary process termination, root filesystem unlock, or
unrelated visual rewrite.

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
other qualified combinations. This catalogue bounds tested compatibility claims;
it does not bound which unfamiliar modules users may install, assess and try.
The earlier rule accepting a catalogue explanation for unmatched module bytes
is withdrawn by the operator's 2026-10-02 correction. General publication uses a
reusable precompiled engine and locally prepared data, without a customer build.

## Acceptance method

These are prospective requirements. No source review, previous short run or
passing component suite closes them. Every released claim identifies its exact
platform, DAW, plug-in/module/class, license channel, runtime/profile, native and
Windows components, format, workload and observation duration.

### Candidate and fixtures

The final qualification uses one frozen release source and complete component
manifest with declared platform builds. Keep the exact predecessor, original
projects and licensed environment available. Normal product controls must select,
publish, update and restore the intended pair; no manual registry/proxy repairs.
An engineering comparison may use controlled instrumentation, but is not itself
the complete customer journey.

Develop with the existing first-party stateful instrument/effect and deliberate
installer failures. Qualify the recording path with an empty/native reference
and deterministic bridged reference before assigning captured losses to a vendor
plug-in. Exercise Pure LoFi alone, FRAGMENTS alone and their real chain on the
Deck, then the declared commercial catalogue. Use working project copies and
lawful save-capable configurations. Save-disabled demos cannot establish recall;
normal vendor login, EULA and device selection remain user actions.

### Unfamiliar plug-ins and advanced compatibility

Use the workflow in [architecture section 18.6](ARCHITECTURE.md#186-general-preparation-and-compatibility-experimentation).
These are required acceptance cases, not results:

- Build the bridge package before generating the independent test modules.
  With no module/class entry or exact profile in that package,
  install, discover, assess, prepare, publish, process and save/reopen it through
  normal controls. Repeat with an instrument and an effect. Capture engine
  hashes and verify that preparation invokes no compiler or remote proxy build.
- Update the module while retaining its logical class; refresh metadata and
  assessment, preserve the DAW class ID and exercise saved-project recall. A
  changed digest must invalidate stale observations without demanding a new
  bridge release. Restore the retained predecessor and check project behavior.
- Try a reversible graphics/runtime option using the same configuration owner.
  Compare requested/effective settings, editor behavior and captured audio with
  the editor closed and active; keep the trial and restore the predecessor.
- Exercise a missing dependency with a distributable reference installer. Show
  the reason, add the dependency through supervised installation, re-assess and
  process. Repeat an interrupted/failed attempt and verify recovery and affected
  sibling behavior. Separate DLL hints from demonstrated loader requirements.
- Select an alternate coherent runtime, including an imported unqualified
  fixture accepted by the existing adapter contract. Retain exact identities,
  show the trial as unqualified and prove keep/restore. A maintainer plug-in or
  runner catalogue entry must not substitute for the required capability checks.
- Reject malformed descriptor data, an unsupported required interface, stale
  trial inputs and incompatible ABI/protocol pairs with specific reasons. A
  missing optional editor, unknown product name or absent historical support
  profile is not an equivalent failure. Verify cleanup and interrupted trial
  recovery without losing the working predecessor.

Advanced controls must show scope, effective configuration and differences,
preserve explicit local overrides across profile refresh, and keep support claims
separate from local results. A machine capability refresh must preserve licensed
environment identity. No new physical audio/GPU claim follows from these source
or reference tests; commercial and platform acceptance below still applies.

### Audio and reconfiguration

Restore continuity at the declared 48 kHz / 512-frame Deck setting first. The
recovery targets then include actual host blocks of 256, 128 and 64 frames. A
1024-frame comparison or increasing bridge delay cannot replace those targets.
The live-use target is same-callback output with no added bridge presentation
delay; a small host block with a large queued delay does not qualify. Vendor
latency and actual processing/transport time remain separately measured.
Keep sample rate, setup maximum, actual block, precision, added bridge delay and
vendor latency separate in both evidence and UI. Any reduction in the agreed
platform, catalogue or latency goal is an explicit product scope decision.

Test every advertised rate/format bound and actual lengths below the negotiated
maximum, including non-power-of-two blocks, supported zero-frame event/parameter
handling, and repeated block/rate changes through inactive setup. Check notes,
automation, returned events, transport stop/start, pause, seek, loop and offline
rendering. Burst and offline consumers must not sleep between blocks to make the
bridge work. No reinstallation or publication change may be needed for ordinary
DAW format changes.

Before testing a delivery successor, declare its completion bound, timeout result
and latency for each processing mode. For paced real-time reference processing,
retain callback duration tails and maxima against `N / Fs`; this measurement does
not replace actual presentation-deadline and captured-output checks in the DAW.
Report bridge and vendor delay separately and notify the DAW when latency changes.
Compare yabridge only with matched supported host installations and the same
hardware, plug-in, preset and workload; disclose unmatched sandbox/runtime factors.

### Interaction and sustained operation

For each platform's declared release instrument/effect chain, the minimum
sustained gate is a 30-minute interaction run followed by a two-hour soak on the
same frozen artifact at its declared lowest live-use block. The two-hour duration
is the selected engineering baseline for this roadmap, not a universal reliability
claim. Declare and retain the settings and durations for individual plug-ins and
the additional block/rate matrix too; do not transfer a pass to an unexercised
combination.

Interaction includes editor open/close/resize, automation, transport changes,
state save and repeated supported buffer changes. Retain complete output and
native/Windows lifetimes, startup/priming and retirement counts, missing spans,
callback rejections, timing tails, effective thread policies and cleanup results.
Acceptance runs have diagnostics disabled; measure observer effects separately.
There must be zero unexplained missing frames, valid-input callback rejections,
stuck notes or corrupted state. Expected musical silence and declared priming
must be distinguishable from bridge failure. A clean subsection of a failed
lifetime is not a pass. A broken no-plug-in recorder invalidates the affected
comparison rather than proving a bridge regression.

### Customer workflow and failure recovery

Retain three cold loads including reboot, five warm loads, three editor
close/reopen cycles per product and two repetitions of each supported recovery
case. Cold launch must finish within 30 seconds and warm launch within 5 seconds
for a responsive beta claim; slower results remain limited. Test the common
Ubuntu/CachyOS catalogue and the controlled licensed Deck workflow. VM results
do not qualify physical latency or graphics on an untested machine.

Save recognizable changed parameters/presets and automation, close the DAW,
reopen, reboot and reopen again. Verify restored behavior with captured output
and exact identities; existence of a saved file or nonempty state blob is not
meaningful recall. Test the retained project before/after populated update and
after visible predecessor restoration. Software rollback must preserve project
data and vendor authorization.

Exercise installer completion, held-operation Stop while general readiness is
unavailable, closed parents with owned helpers, partial installation, interrupted
frontend and retry after confirmed retirement. Continue through the same Setup
card to discovery or an actionable retry. Also test stale recovery targets,
mismatched bytes, unavailable services, interrupted runtime intake/update,
insufficient disk space, failed host launch, peer crash and teardown. Refuse wrong
recovery, retain the predecessor, and confirm retirement before reuse. Fix and
rerun the two known runtime cleanup failures and required manager/native checks;
do not waive them because the audio comparison passes.

### Release completion

Review the actual production changes and required checks; rebuild and verify the
declared release artifacts, signatures/trust configuration, exact third-party
notices and source-offer/distribution obligations. Do not ship vendor binaries or
authorization material. Complete fresh and populated installation/update/rollback
through the ordinary product, including the SteamOS protected-system route.
Publish only the support matrix established by this candidate. Missing hardware,
release trust material or distribution authority remains an open release blocker.

Changes during qualification create a new candidate identity. Reuse unaffected
component results with their provenance, rerun affected checks and complete the
final musician workflow on the candidate actually proposed for release. Preserve
historical failures below without relabeling them under this contract.

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

Internal56's physical Deck update/save/reopen completed ordinary controls,
but audio remains failed. First traced recall lost 2,048 Pure LoFi frames before
its first note. A second reopen was clean for the ten-minute musical subset,
then ended with 2,560 Pure LoFi and 2,048 FRAGMENTS missing frames after editor
activity. A tracing-disabled comparison still lost 1,536 Pure LoFi frames.
Tracing is therefore not the sole cause. Retained request 66595 spent 1.175 ms
in FRAGMENTS SDK processing but 19.574 ms in native admission-to-publication;
CPU accounting queries polluted the broader diagnostic brackets. The source
successor removes those queries and reports the existing exact SDK duration
with each gap. This is a measurement correction, not a residual-audio fix.
See the [physical results and limitations](RESTART_VALIDATION_2026_10_01.md#internal56-physical-deck-update-recall-and-audio-failure).

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

Internal46 delivered and selected both reference successors through normal
Check, Prepare and Replace controls on the populated Ubuntu account. Ardour's
ordinary rescan/reopen loaded both. A loop ran for more than ten minutes, but
the effect's `0x107` refusal stopped its processing and state capture. The new
snapshot retained its previous effect chunk; file creation is not a recall pass.
The instrument lost 433,637 processing frames in 380 gaps and the effect lost
11,776 in 15. Both owners confirmed normal retirement, with the original
project unchanged. The [failed fixed-candidate receipt](../evidence/self-service-delivery/internal46-populated-reference-failure.json)
also retains the absence of detailed Windows per-gap timing and CachyOS's
unchanged authentication/access blocker. These remain product and fixture gaps,
not a complete installed journey.

The internal46 Ubuntu controls also showed an older "Bridge ready" header
beside a newer product offer with unavailable capacity. The source successor
includes capacity from the same manager capture in product detail and uses it
for both the selected controls and health display. Expired readback cannot
retain a ready claim. A new overview with different capacity invalidates the
cached detail. The manager/frontend wire generation is 15; candidate assembly
reads its declaration from the exact source instead of retaining a hardcoded
generation. The 84 frontend tests, 74 operator tests (one opt-in test ignored),
and both all-target Clippy checks pass. This correction is not yet installed;
internal46 remains a failed schema-14 installed candidate.

The next source successor replaces the current product's 256-frame splitting
and numeric curve reconstruction with one whole DAW processing block. Protocol
minor 14 and mapping generation 3 admit at most 1024 frames and keep the
incoming parameter queues, their original sample offsets and processing context
unchanged. The Windows plug-in supplies its implicit previous value, including
after GUI edits, state restore and seeks. No descriptor default or cached bridge
value substitutes for vendor state. Historical protocol fixtures retain the
legacy planner, explicitly excluded from the current product path.

This changes the paired native/Windows transport. The matching Windows host and
prebuilt proxies must be rebuilt and delivered together; older selected
publications and rollback sets remain immutable. The fixed queue has 512
descriptors, retaining the previous 524,288-frame ceiling. All mapping planes,
result storage and Windows processing buffers are allocated before activation.
Windows multi-output buffers live in owner storage rather than exceeding the
default thread stack. Notes still require offsets inside their block; an exact
DAW parameter endpoint is preserved rather than silently clamped.

Source checks pass: 11 native-client tests, 87 macOS backend tests, 88 Linux
registered backend tests (one opt-in Windows fixture ignored in each backend
run), warnings-denied all-target Clippy, and ten native SDK/X11 tests. The actual
processing entry point preserves whole 1008/1024-frame calls and a zero-frame
flush after GUI, state and seek changes with zero allocations, reallocations or
frees. A production decoder/SDK-queue regression exercises vendor-owned implicit
values and refuses mismatched protocol/layout and out-of-bounds notes. These
are source results. Windows CI, fixed-candidate installation, audio, save/reopen,
reboot and populated rollback are still required before accepting the repair.

The matching Windows build passed its whole-block processing regression and
produced host digest `a49c6be190f3e018ce5f253b93750701c10f93b877fccf7066cd55ebdfd6b17b`.
Internal48 assembled schema-15 manager/frontend packages and seven exact prebuilt
entries from source `e2ac76aa07bcda208c7794810a83d6bf83cbea3d`. CachyOS's ordinary
frontend installed, stopped its idle predecessor, selected the successor and
started the service with zero DSP/maintenance/keeper owners. Runtime acquisition
and its musical workflow remain untested.

Ubuntu's populated internal48 update failed in Setup before selecting the
successor: `package_service_not_clean_idle`. Its two keeper leases survived an
abrupt hypervisor OOM/restart without positive retirement receipts. Fresh capacity
showed zero DSP/maintenance, two keepers and unconfirmed cleanup. The original
saved project remained unchanged. No leases were manually deleted or marked
successful. The [candidate and interruption receipt](../evidence/self-service-delivery/internal48-candidate-and-interrupted-update.json)
retains exact release rosters and installer identities. Internal48 has no new
Ubuntu musical or recall result and remains a failed qualification candidate.

The source successor separates permission to stop an exact idle service from
permission to adopt a package. Fresh exact owner, unit, route and transaction
checks remain; unresolved retirement still blocks adoption. Each newly launched
session records its kernel lifetime before creating a lease or process. A later
kernel lifetime retires the exact lease with an explicit interruption receipt;
it never changes the original failed result or fabricates saved plug-in state.

Historical leases have no inferred launch lifetime. After the exact idle service
has stopped, the recovery owner may record their current observation and ask the
user to restart. Only a subsequent kernel restart supplies retirement proof.
Same-kernel recovery, malformed or conflicting identities and active DSP owners
remain blocked. Setup explains the restart and retains projects, vendor state and
the selected application. Internal49 exercised this path through the normal
Ubuntu installer, Setup and operating-system restart on the existing account.
Two exact interruption receipts confirmed retirement without marking either
session successful. The original project digest remained unchanged, and Setup
selected and started internal49 without manual lease changes or process kills.

Internal49 is still a failed qualification candidate: its subsequent ordinary
inventory refresh persisted current discovery but refused service resume before
all registered environments were ready. The second environment became ready
13 seconds after the refusal; later product-owned recovery restored the service.
No musical, persistence or rollback pass is inferred from that recovery.

On CachyOS, the same Ubuntu variant acquired and verified its pinned runtime
through Setup without installed Wine, Proton or Steam. One held first-party
installer cancellation worked while global readiness was unavailable: the
ordinary Stop control retired the exact operation, confirmed zero live owned
processes, reported no durable installation, and offered Retry on the same card.
The second repetition, completed installation and musical journey remain pending.
The [internal49 progress receipt](../evidence/self-service-delivery/internal49-delivery-progress.json)
retains both release rosters, installed readbacks and screenshots.

The source successor stages every exact environment before waiting for any
single environment's readiness, using one overall recovery deadline and fresh
ownership readback after lock acquisition. Readiness is acknowledged only after
every binding reports Ready and its registration/components still match.
It also names running setup operations in frontend feedback without inventing
download percentages or phases. All 193 manager-library, 274 manager-binary and
86 frontend tests pass, with one opt-in binary test ignored; both all-target
Clippy runs pass with warnings denied. Installed successor acceptance remains
required. The [source regression receipt](../evidence/self-service-delivery/multi-environment-resume-regressions.json)
keeps that limitation explicit.

CachyOS fixture access was recovered on its cleanly stopped disposable child
with private disk/firmware backups. Only the existing test account's password
and password-change date were rotated; account identity, permissions and other
accounts were retained, and the parent disk digest was unchanged. Normal GUI sign-in passed. This engineering fixture repair is separate
from a self-service product qualification run.

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
