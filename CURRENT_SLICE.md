# AP18 — Arturia Software Center to Pigments vendor pathway

## Authority and basis

AP18 is the active implementation slice.

- Integrated AP17 main: `2329706a6e797137e68d719edbbbe5cc1e0cdbf1`
- Active issue: [#94 — AP18: Arturia Software Center to Pigments vendor pathway](https://github.com/kasselvania/Linux-VST-bridge/issues/94)
- Branch: `codex/ap18-asc-pigments-vertical`
- Product setting: 512 added frames per proxy, selected/supported/recommended
- 256-frame setting: available but unqualified and outside AP18

Continue in one implementation PR against `main`. Work to the product outcome under [AGENTS.md](AGENTS.md); do not recreate the long AP17 campaign or add process ceremony that does not change the result.

AP17 is accepted and integrated. Its exact Steam Deck / Bitwig / Arturia envelope is six live DSP instances globally, at most three Pure LoFi and four Efx FRAGMENTS, four per loaded native class image, three qualified parallel tracks, serial bridged depth three, and two simultaneous direct editors exercised. Seven remains unqualified and eight was excluded for the tested workload. See [docs/AP17_CLOSURE.md](docs/AP17_CLOSURE.md).

Revision 10 is the accepted ordinary AP17 profile generation in source. The Deck was deliberately left on exact revision 7 after the final investigation. Activate revision 10 once through the existing bounded transition and verify physical readback before ASC work; this is installation housekeeping, not authority to replay AP17.

The repository remains publicly readable but proprietary. Preserve [COPYRIGHT.md](COPYRIGHT.md), [CONTRIBUTING.md](CONTRIBUTING.md), and the all-rights-reserved README notice.

## Private installer handoff

The operator has already downloaded one or more Windows Arturia Software Center installer files into the Steam Deck user's `$HOME/Downloads` directory. They have not been opened or executed.

Before first execution:

- identify the plausible regular files without scanning unrelated Downloads content;
- retain original filename, size, SHA-256, file type, and available Authenticode publisher/signature facts;
- let the operator resolve ambiguity if multiple distinct plausible installers remain;
- copy the selected exact bytes into a private manager-owned, content-addressed artifact location;
- leave the original file unchanged;
- never commit, upload, extract into the repository, or publish the installer bytes.

The operator states that the installer came from Arturia's official site. The exact selected bytes and later installed readback—not a remembered website version—are authority for this fixture.

## Product outcome

Complete one official vendor-managed acquisition-to-use path:

```text
exact ASC installer
→ ASC installed in the existing managed Arturia environment
→ user signs in through the real Arturia UI
→ Pigments is selected, downloaded, activated and installed through ASC
→ the manager discovers the exact installed module, classes and resource roots
→ one immutable Pigments ReviewCandidate and native proxy are produced
→ Pigments is published reversibly to Bitwig
→ its editor, preset, notes, automation, state, save/reopen and cleanup work
```

The goal is not merely to make `Arturia Software Center.exe` launch. The slice closes the gap from the official vendor application to a usable, exactly identified Pigments device in Bitwig.

## Reuse before invention

The repository already provides:

- exact persistent environments and pinned runners;
- `install ENV_ID INSTALLER_PATH SHA256`, which verifies and supervises one normal foreground installer;
- exclusive environment operation locking;
- supervised process ownership and cleanup;
- exact Windows VST3 module/class inspection;
- profile construction, native proxy generation, publication, rollback and readback;
- standalone vendor-editor access;
- the accepted AP17 capacity and failure laws.

Extend those owners only where the real ASC workflow demonstrates a missing capability. Do not start by designing a general package manager, vendor abstraction hierarchy, or full GUI framework.

## Account and authorization boundary

Arturia owns authentication and licensing; the operator owns the account interaction.

- The operator enters credentials directly in ASC.
- Pause automation at sign-in, license selection, purchase, consent, or other secret-bearing steps.
- Do not inspect, capture, log, store, transmit, or commit account email, password, cookies, tokens, serials, unlock codes, license payloads, or activation files.
- Do not intercept TLS, scrape undocumented Arturia APIs, replay vendor network calls, patch licensing, or manufacture authorization.
- Do not clone or run a copied authorized environment as another device.
- Do not update Pure LoFi or Efx FRAGMENTS merely because ASC offers an update.

The bridge may report non-secret posture such as `signed_in`, `activation_required`, `installed`, `update_available`, or `unknown` only when that state is directly observable without retaining secret material.

## Work sequence

### 1. Establish the safe baseline

With Bitwig closed, require zero DSP leases, no pending publication transaction, no stale tmpfs session, and a healthy service/keeper. Record exact software, environment, runner, profiles, publications, modules, resource roots, authorization posture, free space, and protected-project hashes.

Activate ordinary revision 10 through the accepted AP17 transition and verify it physically. Do not rerun capacity, reboot, or failure matrices.

Create a private non-running rollback snapshot or an equivalent exact backup of every environment location ASC may mutate. Do not launch a cloned authorized prefix. Stop before installation if safe rollback and sufficient disk headroom cannot be established.

### 2. Install and register ASC

Run only the exact admitted installer through the managed Arturia environment. Retain a bounded before/after transaction covering allowlisted filesystem and safe registry roots, process ownership, exit/cleanup, installed application identity, helpers/services/startup entries, and discovered download/resource/VST locations.

Do not infer successful installation from exit code alone. Confirm the installed ASC application exists, has an exact identity, and can launch again in the same environment.

### 3. Give ASC a reusable product-owned lifecycle

The manager must be able to launch or focus the exact registered ASC application without accepting an arbitrary executable path. It should report installed/running/needs-attention/completed/failed state, distinguish the visible launcher from continuing download/install helpers, preserve required vendor infrastructure, and clean up only owned operation processes.

The current one-hour installer owner is not automatically appropriate for a long-running software center or a large product download. Change its lifecycle only in response to actual ASC behavior.

Attempt the real ASC UI first. Build a minimal manager frontend only if a demonstrated focus, navigation, lifetime, or integration problem prevents the user from completing the flow. Any such surface remains a thin view over Rust manager truth: launch/focus, current operation, detected products, rescan, Pigments status, and errors. It must not reproduce Arturia authentication, licensing, catalogue, or download APIs.

### 4. Install Pigments through ASC

Let the operator sign in. Select only Pigments, use the normal licensed or vendor-provided demo pathway, and allow ASC to download and install it. Do not update unrelated Arturia products.

Prefer stable Windows `C:` locations inside the existing managed environment. Observe rather than assume the actual VST3, standalone application, shared Resources, presets/samples, app-data, program-data, preferences, databases, caches, and authorization locations. Avoid arbitrary `Z:` or external Linux paths unless real evidence requires them and rollback remains clear.

Close and relaunch ASC once after installation to confirm its own installed-product state persists.

### 5. Discover Pigments automatically

The manager must derive the candidate from observed installation facts:

```text
allowlisted installation delta
→ exact new/changed VST3 module candidate
→ supervised factory/class census
→ exact Pigments class and metadata
→ exact runner/environment/module/host/native identities
→ immutable ReviewCandidate profile
```

The operator must not transcribe hashes or class IDs. Product selection may use exact Arturia factory/class metadata after the module is identified; filename or display-name substring alone is not authority.

Build the native proxy through the existing source-owned descriptor route. Derive stable Linux processor/controller IDs from the exact VST3 class identity. Begin with a conservative Pigments class limit of one unless the actual work justifies more. Extend AP17's capacity policy explicitly rather than treating all Arturia products as identical.

Preserve LoFi and FRAGMENTS revision 10 unchanged. Ordinary activation remains verified-only. Exercise Pigments through one exact sealed AP18 engineering publication with revision 10 as the stable existing-product baseline.

### 6. Qualify the real Pigments result

At 48 kHz, float32, actual host maximum 512 and 512 added bridge frames:

- load one Pigments candidate in normally launched Bitwig;
- open and navigate the real vendor editor, including ordinary resize;
- load one factory preset from Pigments' own browser;
- play notes and retain nonzero local output;
- move one real control and verify Begin/Value/End plus Bitwig automation;
- replay the automation and confirm UI/sound follow;
- close and reopen the editor on the same DSP instance;
- save a protected project, quit normally, relaunch through Applications and reopen it;
- confirm intended preset/visible state/opaque state behavior and audio;
- exercise Pigments alongside one existing Arturia sibling;
- remove Pigments without disturbing the sibling;
- quit normally and require positive ownership retirement.

Retain all gaps and failures truthfully. Do not turn this into exhaustive Pigments, preset, MPE, multibus, rendering, or performance qualification.

## Existing-product preservation

ASC may modify shared Arturia resources or offer updates. Verify proportionately that Pure LoFi and Efx FRAGMENTS retain their accepted exact modules/profiles, direct editors, audio/automation, saved-project behavior, and cleanup.

If ASC changes an existing module, resource tree, or authorization state unexpectedly, do not silently bless or overwrite history. Retain the change privately, restore the safe baseline when necessary, and classify the update as a distinct exact transition.

## Evidence and implementation discipline

- Rust owns installer/application/product records, environment revisions, discovery, profile, publication, rollback, and canonical readback.
- C++ remains limited to VST3/Win32 SDK boundaries.
- Nothing involving installation, hashing, process census, files, registry, logging, or UI enters an audio callback.
- Public evidence uses stable aliases and excludes private absolute paths, raw local process/session IDs, account data, authorization material, installer/product bytes, presets, opaque state payloads, and sensitive vendor logs.
- Private diagnostics may retain what is necessary for recovery and debugging outside Git.
- Reuse accepted AP15–AP17 evidence when its owners do not change. Test changed owners and the new Pigments path; do not replay unrelated historical campaigns.

## Completion threshold

AP18 is complete when the single implementation PR demonstrates:

1. exact private ASC installer admission and provenance;
2. reversible installation into the managed Arturia environment;
3. reusable product-owned ASC launch/status ownership;
4. user-completed real sign-in/activation without credential capture;
5. Pigments installed through ASC;
6. exact installed application/module/resource/preset roots discovered;
7. automatic Pigments class census and immutable candidate construction;
8. exact native proxy build and reversible publication;
9. real Bitwig preset, notes/audio, automation, editor reopen, save/recall, sibling, removal, and cleanup behavior;
10. existing LoFi/FRAGMENTS preserved or any vendor update classified honestly;
11. exact final environment/software/profile/publication state and rollback;
12. no active DSP lease, pending transaction, orphan vendor operation, stale tmpfs session, debugger, or heavy trace at handoff.

512 remains selected/supported/recommended. 256 remains unqualified.

## Explicit non-goals

AP18 does not include Serum, another vendor, broad support for every software center, automatic purchasing/account creation, credential/license automation, updating all Arturia products, exhaustive Pigments/MPE/multibus/offline-render coverage, 256 qualification, residual delivery issue #90, lease-race issue #93 unless it directly blocks this operation, process pooling, another DAW, Wayland-native embedding, CLAP, consumer release packaging, or a speculative full manager UI.

## Handoff

Update one AP18 implementation PR with exact source and artifact identities, installer/ASC lifecycle, user-owned authorization boundary, installation changes, Pigments discovery/profile/publication, actual Bitwig result, preservation/rollback, focused tests, final physical state, and explicit nonclaims. Leave it open and unmerged for independent technical review.

### Current checkpoint

ASC's post-login fault is now attributed to the pinned runtime's
`UIAutomationCore!UiaDisconnectProvider` null-provider path. The exact
operation-scoped accessibility comparison reached the real ASC library. The
operator installed Pigments through ASC and reports completing activation.
Normal ASC close returned launcher exit 0 and positive empty-cgroup cleanup.
Historical failed attempts remain retained; see
[evidence/ap18/post-login/uia-attribution.json](evidence/ap18/post-login/uia-attribution.json).

The manager now has an inactive, nonpublishing `vendor-product scan pigments`
route. It nominates the installed module inside ASC's exact environment, invokes
the existing supervised module-level SDK census, refuses ambiguous classes and
checks actual factory/class metadata. It does not ask the operator for IDs,
hashes or paths. The real scan identifies Pigments 7.0.1.6772 with 4,446 parameters and successful
opaque state capture. Its generated native and corrected auxiliary-input Windows
host are built. The sealed candidate and new-class qualification route are implemented. Bitwig
discovers Pigments; its first revision-1 load refused auxiliary input activation.
The default-active advertisement is corrected and independently tested through
the production Processor/SDK fixture. Revision 1 remains immutable history;
revision 2 binds the corrected native. Actual loading and musical/editor
validation remain pending. Ordinary exact ASC
launch now selects the demonstrated operation-scoped accessibility mitigation.

LoFi and FRAGMENTS revision 10 remain valid and their module bytes unchanged.
512 remains recommended; 256 is unqualified. PR #95 remains draft and unmerged.


### AP18 sole auxiliary-input decision

The operator's focused input-bus ruling authorizes exactly one stereo transport
input: index-zero main input, or sole stereo auxiliary input with no main. Bus
media, direction, index, type, name, arrangement and supported default activation
remain SDK-derived. Bitwig alone selects a source. Inactive input is zero; active
input forwards its exact supplied buffers and validated silence hints. A main
plus auxiliary topology still transports only main and refuses auxiliary
activation. No role/product/name dispatch or general multibus support is added.

Preserve Pigments revisions 1 and 2 and the paused stash. New native/Windows
artifacts require immutable ReviewCandidate revision 3, sequential replacement
1 -> 2 -> 3, and actual no-source/routed-source/source-removal proof before
claiming the bounded input path. Existing ordinary products remain unchanged.

### Revision-3 real-host checkpoint

Revision 3 was built, sealed, staged and reversibly published. Bitwig accepted
the auxiliary bus, displayed its normal sidechain selector with No input, and
presented the real Pigments editor. The session then terminated after 18,852
processed blocks with `malformed or oversized process results`. The collector's
first exact refusal is not retained; neither capacity exhaustion nor invalid
vendor data is established. Two subsequent local output captures were silent.
Do not count load/editor visibility as musical or routed-input success.

Pigments is restored inactive, its history/artifacts retained. Both ordinary
revision-10 products and protected project hashes match the pre-test baseline;
service and keeper are active, zero DSP leases/pending transactions, tracing
off, CPUWeight unset/effective 100. The next bounded investigation belongs to
the exact returned-result refusal, before further source-routing qualification.
See `evidence/ap18/pigments/input-bus-live-result.json`. PR #95 remains draft.

### First returned-result diagnosis

The focused operator continuation authorizes a fixed POD, first-write-wins
collector rejection record without changing acceptance or capacities. Emit
scalar diagnostics after processing-thread join. Preserve revisions 1/2/3;
revision 4 is a diagnostic ReviewCandidate with the exact new Windows host and
unchanged revision-3 native. Run only Welcome/no-source/C3 until the first
refusal or a documented bound beyond 18,852 blocks. Select any repair only from
the exact refusal; preserve candidate history and restore Pigments inactive.

### Crash-surviving returned-result custody

The revision-4 attempt exited during vendor view release after successful
`removed()` and `setFrame(nullptr)`, before a Windows terminal record or
processing-thread join. The exact exception/termination cause is unknown. This
is separate from revision 3's collector refusal and the later Bitwig state-save
warning. No installation/save-path cause is established. Retain existing
evidence without repeating that attempt.

The next diagnostic uses an independently versioned, fixed scalar session mapping
written after `process()` returns and before error/teardown. The Linux owner
reads it before containment. Post-join JSON is secondary. Preserve revision 4;
changed host/reporting bytes require ReviewCandidate revision 5. After generated
and hosted validation, one bounded Welcome/no-input/C3 run leaves the editor
open until the first rejection, typed host exit or 30,000 completed blocks. No
save or manual editor close precedes the outcome. Restore Pigments inactive.

### Revision-5 custody result

The single permitted run retained `EventChannel` at request 360, callback 358,
position 91392, offset 144 in 256 frames. Pigments emitted a standard Note On,
bus 0/channel 0/pitch 60, against active output-event bus metadata declaring zero
channels. All prior output counts were zero. `process()` returned, custody
committed, and processing subsequently stopped/joined; readable JSON agrees.
The rejection preceded editor initialization, so the requested persistent
Welcome/open-editor interval was not reached. No second attempt was made.

Do not increase capacities or silently interpret zero as sixteen. The SDK also
requires used channel counts to be rechecked after arrangement negotiation;
the current layout retains earlier metadata. Whether this is stale metadata or
a vendor metadata/event conflict is not established. Keep revision 4's abrupt
view-release exit separate. Pigments is inactive again, ordinary revision10
publications/projects unchanged, zero DSP/pending/stale sessions, service and
keeper active, tracing off, CPUWeight restored unset/effective100. See
`evidence/ap18/pigments/result-custody-live.json` and
`evidence/ap18/pigments/result-custody-installed-final.json`. PR #95 stays draft.


### Event-output operational census (review 5183863039)

A separate nonprocessing inspection mode now captures the same component's raw
output-event BusInfo after initialization, arrangement negotiation, processing
setup, event-bus activation, component activation, and processing start. The
production inspection owner retains its normal admission lock, timeout and
positive cleanup. No editor or state capture is needed. The SDK fixture covers
unchanged zero, zero becoming sixteen, and an unchanged positive count.

The exact installed Pigments reported **zero at all six stages**, with one active
main event-output bus and unchanged stereo audio arrangements. All SDK calls and
cleanup completed. This excludes a positive post-negotiation channel count for
this fixture. No raw value was replaced, compatibility policy selected, or
result validator changed. Bitwig's event/parameter sink presence remains the
next required observation. Revisions 1–5 and both ordinary revision-10 products
remain unchanged; no revision-6 profile has been created.

See `evidence/ap18/pigments/event-output-operational-census.json`. PR #95 remains
draft and unmerged. This is census evidence, not successful Pigments processing.

### Native sink observation interrupted

The one authorized external callback snapshot found both Bitwig result sinks
non-null, output-event bus active and advertised channels zero, but it occurred
28.360 seconds after the supervisor's pre-containment snapshot of an already
failed Windows session. It therefore does not establish sink presence on a
healthy or rejected callback. Case B/C remains unresolved; no normalization or
revision 6 has been created. A replacement live snapshot requires renewed
operator authorization; the first authorized snapshot has been consumed.

The preceding session independently retained EventChannel at request 90385,
callback 90383, position 23137792, 256 frames, Note On bus0/channel0/pitch58,
offset32, zero input notes/parameters and zero prior output use. Processing
returned, stopped and joined; mapped custody and post-join JSON agree. The
outer launcher exited 5; the final editor stage was 215, before vendor view
release. Exact Windows termination cause remains unknown. The operator reported
a possible touchscreen UI close; this is context, not causal proof.

Pigments was restored inactive. Ordinary LoFi/FRAGMENTS physical publications,
software, protected project hashes and Flatpak permissions equal baseline.
Service/keeper active; zero DSP/pending/stale sessions, tracing off, no Bitwig or
debugger, CPUWeight unset/effective100. An earlier unexplained CPUWeight override
is excluded from performance evidence. See
`evidence/ap18/pigments/event-output-sink-interrupted.json`.

### Direct correction authorized after the interrupted observation

The operator explicitly removed the pre-repair sink-observation prerequisite.
Implement the exact-profile reported-zero/effective-sixteen interpretation in
the native descriptor, Windows setup comparison and Collector bus limit. Keep
the existing result-delivery path, all other validation and capacities. Preserve
revisions 1–5; build corrected revision 6 through the existing sealed route.
No further pre-repair live investigation or desktop control is authorized.
Use SSH/background tools for the corrected C3 check, then continue AP18 if it
plays. The attempted replacement debugger wait expired without finding an
instance or taking a snapshot.

### Corrected revision 6 built

The exact Pigments profile now declares the closed event-output policy. Its
native descriptor retains raw zero separately and advertises effective 16;
Windows preserves raw BusInfo while setup and Collector use effective 16.
Channels 0/15 pass unchanged; negative/16 remain rejected by the existing MIDI
field validator. No capacities, result encoding, input routing or sink-delivery
behavior changed. Revision 5 remains byte-identical in its historical path;
replacement extends only 5→6, never ordinary activation.

The Linux SDK/CTest lane and 37 Linux runtime tests pass. All four hosted source
lanes pass at eb593b3. Exact Windows and native identities are retained in
`evidence/ap18/pigments/event-output-correction.json`. Corrected C3 playback is
the next check; no live playback success is claimed yet.

### Corrected revision 6 operator audio and separate host exit

All four hosted lanes pass at 27543277. The operator reported audible Pigments
output followed by slowdown/failures. The retained corrected session has no
Collector rejection; its last native request is 23136. The host exited with
outer status 5 at editor stage 215, before vendor-view release completion.
There is no processing-thread-joined event. Positive containment is recorded.
This is neither a clean musical pass nor evidence of an installation/save-path
defect. The exact release/host-exit cause and slowdown remain unresolved.
See `evidence/ap18/pigments/event-output-corrected-live.json`. No further live
sequence or desktop input was used to obtain this evidence. After operator-confirmed closure, the sealed restore returned Pigments inactive.
Zero DSP leases, no pending transactions, service/keeper active and tracing off
were confirmed. CPUWeight was again found at 10000, writer unknown, and restored
to unset/effective 100; this interval is not performance-comparison evidence.

### Pigments retained-view lifecycle repair authorized

The operator accepted revision 6 MIDI output and selected a Pigments-only closed
`retain_editor_view_until_instance_retirement` policy. Ordinary close must cancel
focus, end gestures and hide the attached view/parent without SDK destruction;
reopen reuses them with a fresh logical epoch. Final quiescent retirement alone
performs setFrame(nullptr), removed, release, DestroyWindow, then handler restore.
LoFi/FRAGMENTS keep existing behavior. Revision 6 is retained byte-identically;
revision 7 will bind the new exact Windows artifact through the existing sealed
route. One functional audio/close/reopen/retirement check follows generated tests;
no pre-repair live observation or broad campaign. The operator identifies the
CPUWeight foreground override as SteamOS foreground-booster behavior; it does
not block this functional check.

### Revision 7 close/reopen passed; quiescent final release failed

Revision 7 is built and all four hosted lanes passed at 31e9753. The one
functional check produced nonzero local audio before close, while hidden, and
after one Bitwig reopen. The same processing session and vendor X11 target
survived; logical editor epoch advanced 1→2. No Collector rejection occurred.

Normal quit then completed setProcessing(false), worker join and setActive(false)
before final controller unbinding reached editor stage 215. Vendor view release
still did not complete; outer launcher exit was 5. Processing overlap therefore
is not a sufficient explanation for this remaining release failure. Cleanup was
positive, but SDK final retirement was not successful. No further live attempt,
sidechain, automation or save/recall campaign was run. Stop at this exact failure
under the current operator instruction.

Pigments is restored inactive; LoFi/FRAGMENTS revision 10 physical publications
and protected original projects are unchanged. Service/keeper active, zero DSP
leases, no pending/stale sessions, Bitwig/debugger absent, tracing off, effective
CPUWeight 100. 512 recommended and 256 unqualified. See
`evidence/ap18/pigments/retained-editor-live.json`. PR #95 remains draft/unmerged.

## AP18 process-scoped final retirement (review 5184501713)

Revision 7's ordinary hide/reopen and continued audio are accepted. Its final
view release failed after successful processing stop/join, deactivation, frame
detachment and removal. Do not repeat that release probe.

The next exact Pigments candidate uses a closed `process_scoped_vendor_retirement`
policy. The Windows owner detaches the retained view and destroys its parent
only after successful processing quiescence and no pending state. It then
acknowledges the existing endpoint and commits the session-bound LVRT v1 record.
It does not release vendor objects or unwind their destructors. The existing
Linux supervisor contains the exact cohort and reports process-scoped retirement
only after positive physical cleanup; this is not clean SDK destruction.

Revisions 1–7 remain immutable. Revision 8 binds the tested Windows host and
unchanged native proxy; generated lifecycle, manager and Linux supervision checks
pass. The one live load/audio/hide/reopen/retirement check passed with positive process-scoped cleanup (see `evidence/ap18/pigments/process-retirement-live.json`). Pigments is restored inactive;
LoFi and FRAGMENTS ordinary revision 10 are unchanged. PR #95 remains draft.

## Authorized revision-9 reporting continuation

The operator authorized revision 9 solely to drain the existing bounded Windows
input witness after processing joins and before process retirement commits. The
revision-8 destructor is intentionally bypassed and cannot own this report.
Revision 8 remains immutable; audio, hide/reopen and retirement are not retested
ceremonially. Continue the remaining sidechain, preset, automation, save/reopen
and sibling checks after the new Windows build and sealed artifact binding.
