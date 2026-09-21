# Catalog reliability: Blackhole and Kontakt owned graphical sessions

Physical correction basis (2026-09-21): the first exact installed `f164048f`
Desktop Mode attempt did not load Blackhole. Bitwig reported `could not load
plugin`; no DSP owner or Windows plug-in host was created. The manager created
one keeper, but that keeper exited before readiness and cleaned its empty
cohort. The authenticated Flatpak peer supplied `DISPLAY=:0`,
`XAUTHORITY=/run/flatpak/Xauthority`, and a session bus at
`/run/flatpak/bus`. The supervisor passed those namespace-private paths
unchanged to a host-side Proton launch even though neither path existed in the
host namespace. The retained keeper result did not preserve its discarded
output, so the exact internal process error remains unavailable and is not
inferred. The sanitized physical result is retained in
`evidence/catalog-reliability/blackhole-first-flatpak-graphical-failure.sanitized.json`.

This corrective continuation binds graphical resources, not just environment
strings. The authenticated peer values remain the selection authority. A
private host-visible Xauthority file is admitted only when its complete bytes
match the peer-namespace file exactly. A DBus or Wayland endpoint is forwarded
only when the host-visible path identifies the same socket. A Flatpak-private
bus is never silently widened to the user's unrestricted host bus. Missing,
ambiguous, replaced, public or wrong-kind resources refuse before keeper
readiness and before any native binding is exposed.

The installed attempt also retained the old product-specific proxy binaries.
The 65-second typed, unowned `ServiceBusy` retry in this branch therefore did
not execute physically. The next offline package must include the exact current
preparation kit. Before either product is retried, Blackhole and Kontakt each
require a newly prepared immutable proxy candidate from that kit and an exact
inactive experimental replacement; their prior publications remain rollback
authority. Installing manager files alone must not be described as deploying
the changed native retry behavior.

Correction basis: stacked merge `18cc2c11b35c6c3c04d88c4703c8a6d88996249a`,
tree `9311091011ec69caa4799f2e12fa972e3ac74781`. That merge entered the
still-unmerged Blackhole branch before review of PR #134 was complete. It was
not merged to `main`, installed, or exercised physically. The corrective
candidate remains a separate draft review boundary.

Basis: commit `bbb43318e48ab5b3b40fab23a53f3766f437205f`, tree
`d3c32017e71bbeb089d48643af7667351a476adf`.

The operator selected the shared Linux-facing reliability work for both current
Blackhole and Kontakt publications. This source slice performs no Moonlight,
Bitwig, touchscreen, plug-in audio, installer, runner, prefix or vendor
operation. The operator will perform the original physical interactions after
the source candidate is installed separately.

Primary claim: a catalog instance is either owned from its authenticated DAW
graphical session through a live environment keeper and concrete supervisor, or
it is refused before any lease, transport or native binding is exposed. A
terminal editor, Windows-host or transport failure is visible in the manager
without depending on optional detailed crash capture.

In scope:

- derive the small graphical context allowlist from the authenticated LVB3 Unix
  peer and independently revalidate its process generation in the supervisor;
- bind keepers and DSP/editor processes to that exact context;
- warm a missing keeper behind an unowned bounded `ServiceBusy` response;
- retire and recreate a keeper after a graphical-session change only when no
  DSP owner remains;
- recheck the keeper immediately before admission and create the supervisor
  before publishing the accepted binding;
- admit exact managed-experimental publications to the existing read-only
  detailed capture path;
- project IF1's existing exact terminal class into catalog status and present it
  for any product, including Blackhole and Kontakt.

Acceptance is deterministic source behavior: no accepted binding without a
live exact keeper; no cross-display keeper reuse; no retirement while a DSP
lease exists; bounded retry without ownership; managed capture admission;
private graphical context with no ambient environment copying; and visible
editor/host/transport terminal states. The existing audio callback, VST3 bus
transport, runner policies and vendor-specific product behavior remain
unchanged.

The production crossings are explicit:

- IF1 is read only from the exact volatile tmpfs transport retained by the DSP
  owner; the durable Wine-prefix symlink is checked as a view and is never read
  as authority;
- spawning the Python supervisor is not ownership. The manager waits for an
  exact `LVO0 <session> ready` receipt after immutable and graphical preflight,
  before exposing admission, transport, or a native binding;
- readiness publication and every later prelaunch action are inside one guarded
  owner scope. A stop observed at the readiness boundary enters that finalizer;
  it cannot escape between `LVO0` and ownership;
- every later prelaunch failure completes the native protocol in order: publish
  failure, observe the native half-close, retire the exact directories, and
  return the exact retirement acknowledgment. Missing acknowledgment keeps the
  transport unconfirmed and the admission cleanup-blocking. An early keeper
  refusal publishes an exact empty-cohort cleanup result;
- a bounded sanitized terminal summary survives confirmed lease retirement so
  the manager can show the most recent editor, host, or transport failure even
  when optional detailed capture was disabled. Failure-summary persistence is
  not physical cleanup authority and cannot strand an already retired lease.

Nonclaims: this slice does not establish that physical touchscreen input works,
that Blackhole or Kontakt audio works in the next Bitwig session, that Gaming
Mode window association is complete, or that any additional audio layout is
qualified. Those remain original-action physical checks, not fixture results.

# Prior stopping point: Blackhole experimental renderer submission

Subsequent operator priorities are recorded in
[`docs/PLUGIN_RELIABILITY_FOLLOWUP.md`](docs/PLUGIN_RELIABILITY_FOLLOWUP.md):
touchscreen crashes and missing incident reporting, Gaming Mode/helper recovery
with panic/reset, and actual plug-in audio I/O. These are follow-up work, not
completed repairs or a new live-test run in this documentation update.

Latest update (2026-09-20): the operator reports editor disappearance on any
touch in installed non-Arturia plug-ins, no corresponding bridge failure, and
continued active status in Bitwig. Arturia is the working comparison. Audio
continuation after disappearance is unknown. Repository inspection identified
the existing UIO3 delay evidence and experimental-profile capture-admission gap;
no new touchscreen reproduction or input repair has run. See the follow-up
document for the next bounded comparison and its remaining questions.

The operator personally tested the installed Blackhole candidate and confirmed
working audio, then requested an end to testing, commit/push and PR submission.
The managed runner transition and fresh stereo inspection/preparation/
experimental publication completed. The real Bitwig editor rendered, responded
to Bypass and closed normally while the same processing instance stayed loaded.
Keep this working candidate installed and PR #131 frozen. Submit PR #133 for
review; do not merge or continue GUI/audio tests.

Audio is operator-confirmed. Standalone normal close/reopen passed; same-instance
Bitwig reopening, saved-project recall, performance qualification, additional
audio channels and full Bitwig instance retirement remain unverified/deferred.
At the original submission, two SSH attempts timed out, so neither the bounded recorder's final state
nor a completed automated audio result was read back. Its configured service
lifetime is 110 seconds; that bound is not a cleanup receipt.

A subsequent live recovery confirmed that the prior Blackhole instance and
bounded recorder had retired. The separate abandoned Serum launch was preserved
and quarantined with Bitwig/Wine closed; the restarted service reported no
cleanup block and zero owners. The automatic recovery defect remains pending.
See `evidence/blackhole-editor/abandoned-launch-recovery.sanitized.json`.

Implementation/build basis: `65347a2db8fe2b8b681cf16e31dd121cd159dea3`.
Current findings and exact receipt identities are in
[`docs/BLACKHOLE_EDITOR.md`](docs/BLACKHOLE_EDITOR.md) and
[`direct-composition-bitwig.sanitized.json`](evidence/blackhole-editor/direct-composition-bitwig.sanitized.json).
The earlier scoped instructions below remain the history of this work, not
authority to resume deferred testing after the operator's stopping point.

# Blackhole experimental DirectComposition runner transition

The operator authorized a bounded transition from the successful standalone
DirectComposition reference to one real Bitwig stereo/lifecycle test. The code
basis remains commit `13a79468dcfa8a6aa93684a0a737ba20a9322902`, tree
`b4ab4869e46eec16844108a79045b1128125427b`, on
`codex/blackhole-editor-diagnostic`; PR #131 remains frozen.

Primary claim: an explicitly selected, separately identified experimental
runner can become the exact managed environment authority without changing the
installed Blackhole module, prefix location, vendor licensing state, native
proxy bytes or retained history. A fresh stereo inspection, preparation and
ManagedExperimental publication must then bind that new runner, environment
revision and closed runtime policy before Bitwig can launch it. The prior
candidate/profile/publication must not be relabeled or reused.

The only accepted runtime policy in this slice is
`dcomp_wine_builtins_reference_v1`: built-in D2D1/D3D11/DXGI/DComp,
`PROTON_USE_WINED3D=1`, `PROTON_DISABLE_NVAPI=1`, and Proton's supported
`PROTON_DLL_COPY=*` provisioning. The policy is optional typed runner identity;
omission preserves all existing serialized runners and profiles byte-for-byte.
It must be copied into the generated profile requirements and applied by the
normal supervisor. No graphics trace channels, ambient DLL override, arbitrary
environment map or generalized runner switch is admitted.

The transition is inactive-only and compare-and-swap bound to the exact
environment marker digest, revision, prior runner digest, sealed candidate
manifest and exact removed experimental publication revision. It requires the
service stopped, all owners retired, the physical proxy absent, and exclusive
canonical/registry/environment-operation custody. It retires only that removed
current registry pointer, increments the environment revision, updates the
onboarding copy, and retains exact rollback bytes and a transition receipt.
Installation, prefix and historical result bytes are not rewritten. Stale,
published, shared-environment, foreign-manifest, changed-artifact and cleanup
states must refuse.

The three authority files are a coordinated retained rollback, not a claim of
crash-atomic multi-file storage. Normal validation failures restore their exact
bytes. A process or power interruption after the prepared receipt leaves
incompatible authority fail-closed; the custodian must apply the retained set
under the same service/canonical/registry/environment locks before resuming.

After the transition the parent owns live custody: rescan the exact onboarding
environment, request `stereo_main_pair`, prepare a new candidate, publish only
through the existing ExperimentalEnable/Replace path, then use the retained
12-second stereo source for one Bitwig input/output observation, editor normal
close/reopen, and exact retirement. Rendering and two normal closes already
passed in the standalone vendor-access mode with exact local pixels and
candidate graphics identities; those results do not establish Bitwig audio or
publication acceptance. Preserve the complete prefix archive and original
runner as rollback authority. Do not qualify the fork generally, add a new test
framework, change Windows/native host bytes, merge, or alter PR #131.

# Blackhole coherent DirectComposition runner continuation

The operator approved auditing the effective graphics backend, preparing a
separately identified coherent DirectComposition-capable runtime with rollback,
and testing the exact Blackhole editor, close/reopen, stereo processing and
retirement. Base commit `13a79468dcfa8a6aa93684a0a737ba20a9322902`, tree
`b4ab4869e46eec16844108a79045b1128125427b`. Preserve PR #131.

First obtain a working reference candidate before attempting patch reduction or
productization. Build a coherent Wine reference from `giang17/wine` commit
`c27f058814b402a5709e073adccd42baa66810b9` (`d2d1-dcomp-11.0`) in the pinned
rootless Proton/Valve SDK build lane, then compose it with the byte-verified
existing Proton distribution associated with source revision
`5b89db940e0ebe3a137a6009a3589232fe084c09`. This reference does not claim that
every packaged Proton component was rebuilt from that source revision. Record
the exact Wine source, SDK configuration, retained Proton artifact identity and
composed artifact digests. Keep the current Proton 11.0-2c runner and its Wine
`dc26e618…` bytes as rollback authority; do not change the installed runner,
prefix, registry, licensing, publication or product selection during the build.

The parent owns the backend audit, candidate transition/rollback and live test;
Sol owns build implementation. A later candidate run must be explicitly bound
to the exact Blackhole module/host/environment and retain phase-aware graphics
tracing, local pixels, normal close/reopen, the existing stereo input/output
check and exact retirement. Do not replay an unchanged failure, hide Wine
detection, read process memory, change security policy, create a new PR or merge.
The previous preference and graphics-call scopes below remain historical
evidence rather than authority to repeat them.

The private coherent reference build completed in the pinned SteamRT4 SDK. Both
architectures built and installed successfully. The composed candidate binds
12,278 entries and 2,750,371,829 regular bytes, includes the fork's exact Wine
Mono bootstrap dependency, retains only identified Proton bootstrap/runtime
auxiliaries, and carries the associated Proton Wine `steamuser` compatibility
policy as an exact recorded patch. The pinned Proton default-prefix packaging
helper converted Wine built-ins to candidate-relative links and removed host
device links before sealing.

A fresh unlicensed prefix then completed normal initialization and a Windows
hello process with `steamuser` `USERNAME`, `USERPROFILE` and `APPDATA`, exact
candidate bytes for D2D1, D3D11, DComp, DXGI and wined3d in both Windows
architectures, and confirmed cleanup. Failed intermediate bootstrap and harness
attempts remain retained rather than relabeled. The sealed reference and this
smoke are build/startup evidence only; actual loaded graphics identity,
Blackhole pixels, normal close/reopen, stereo processing and retirement remain
the parent-controlled live gates. Sanitized identities and limits are retained
in `evidence/blackhole-editor/direct-composition-reference-build.sanitized.json`.

The first existing-prefix initialization correctly stopped before plug-in
launch when 56 Wine distribution targets disappeared behind the candidate's
default-prefix links. Exact source comparison showed that the associated Proton
Wine carries an `is_wine_file` guard which refuses write-enabling files under
the Wine distribution; the DirectComposition fork does not. The candidate was
restored from its preserved final install stage to the original sealed tree.
Candidate-only provisioning now uses Proton's `PROTON_DLL_COPY=*` policy so an
existing prefix receives owned regular built-in copies before Wine updates it.
A focused unlicensed old-runner-to-candidate transition completed both
initializers and a Windows hello, retained `steamuser`, kept all ten graphics
bytes exact, retired cleanly, and left the sealed candidate tree unchanged.
The remaining candidate-relative prefix links were six explicit ICU resources
and eighteen fonts; no Wine built-in distribution link remained. The initial
mutation and an over-strict harness assertion which also counted those expected
resources remain recorded as failed boundaries.

# Blackhole software-renderer repair continuation

The operator approved a bounded software-renderer repair test from base
`d1b46f2ec1f42d093d56d9aa330df101e50080e8`, tree
`371e73a36ea17ca89271d3057afec5b6f2d04ca2`. Static inspection found a table
containing `Software Renderer` / `Direct2D` labels, but the protected callbacks
did not expose its persisted encoding. Run exactly one explicit reversible
hypothesis: change only `GRMd` from `0|0` to `1|0`. This is not a proven mapping
or product-support claim. Do not enumerate further values.

Keep the exact module, Windows host, runner, licensing state, prefix identity,
candidate publication and protected project bytes unchanged. Luna alone may
perform any bounded GUI action and reopen. Apply the file change only with no
live Blackhole owner, under the existing registry and environment-operation
locks, after retaining the exact original bytes, hash and metadata. Verify the
stopped Bitwig editor with the exact local observer; if it renders, close and
reopen it once to check persistence. Restore the exact original preference
immediately if ineffective. Do not rerun audio or performance work, add a
workflow or framework, override Wine/DXVK DLLs, patch the vendor binary, or
alter PR #131. Retain the hypothesis, local-frame result, close/reopen result,
restoration and cleanup facts without publishing private paths or proprietary
bytes. The rejected process-memory route remains a diagnostic boundary; do not
retry it or change ptrace/security settings.

Observed: the exact `1|0` value persisted during one stopped-Bitwig run, but
both 874×552 local captures contained 482,448 white pixels and matched the
baseline raw-frame digest. The hypothesis therefore failed to produce a usable
renderer. It did not prove that software rendering was selected, unsupported or
itself defective. No close/reopen check followed because no rendered surface was
observed. The original 140-byte preference was restored with its exact digest
and mode. The owned supervisor ended with raw exit -15; cleanup and transport
retirement, four protected files, prefix identity and final zero-DSP/
zero-maintenance capacity passed. Luna removed the failed device, leaving the
empty Blackhole input chain in the stopped, unsaved project. A context click
showed no menu; the bounded accessibility helper returned exit 3 with no records,
which does not distinguish its two internal lookup failures. The process-memory
route was approval-rejected and was not retried. Retain the sanitized
[trial receipt](evidence/blackhole-editor/software-renderer-trial.sanitized.json).
A verified vendor control or encoding is still required for the preference
route; a complete runtime presentation correction remains a distinct larger
route.

# Blackhole graphics-call continuation

The operator approved continuing from the qualified local white surface to the
actual graphics initialization/presentation gap. Base commit
`237034ca0fcaafd7e58c8d47f12ca2a523d45099`, tree
`9a583b40cdab176beb0a59cebc794478d828a12e`, on the existing isolated diagnostic
branch/PR #133. PR #131 remains unchanged for independent review.

Use the exact existing activated Blackhole candidate, pinned runner and Windows
host. First reuse the graphics diagnostic channels already compiled into that
Wine build, with bounded private capture and selected sanitized findings.
Avoid a new interception framework or a renderer/backend change merely to
obtain logs. No authentication, installer, prefix recreation, vendor binary,
publication, DLL override or real-time callback change belongs to this step.

Primary claim: identify actual renderer initialization or submission calls and
their reported results, or retain the precise remaining observation gap.
A standalone vendor-editor run may isolate startup without an active audio
callback, provided it uses exact admission and existing supervision/cleanup;
its changed mode must remain explicit and cannot establish Bitwig acceptance.
Do not describe a trace entry as a returned success or loaded modules as a
presenting API. Preserve truncation and absence-of-coverage limits.

Sol owns code; the parent owns the experiment and read-only SSH interpretation;
Luna alone performs any necessary bounded GUI input. Keep raw logs private,
limit the run and output, stop on drift or unexpected vendor flow, retire only
the owned test, and verify protected files/prefix identity and capacity afterward.
Retain the previous white-surface and close-hang findings separately. Commit
and push the continuation to PR #133; do not merge or replay unaffected tests.

Observed: diagnostic source `8bde6be` ran the exact standalone editor with the
selected Wine channels. One D3D11 device-created record, 35 Direct2D context-created
records and 22 EndDraw entry records preceded three composition-swapchain stub
calls; no DXGI Present record occurred. Disassembly of the exact pinned DXGI
module confirms every return path of CreateSwapChainForComposition and
WaitForVBlank returns E_NOTIMPL (0x80004001). The latter emitted 33,691 stub
records. The complete 4,104,682-byte private log had no discarded bytes.
Editor-open was observed; graceful editor-close was not. Exact cleanup,
transport retirement, protected files and prefix identity passed; no DSP or
maintenance owner remains. The first launcher invocation refused before launch
because its accessibility-capability expectation was wrong; the correction and
failed attempt are retained. No workaround is installed. Eventide's GRMd=0|0
preference and renderer label strings are a lead, not a verified setting map.

# Blackhole editor rendering investigation

The operator requests investigation of the blank Blackhole editor using the
existing UI/rendering diagnostics and Pigments Wayland/X11 findings, while
PR #131 is independently reviewed. Base commit
`1bda0e5c695fbc84e0ae5ab42728bca6d049e4ab`, tree
`01fc3a39bf55c600481970c35929641983f3a3ba`; retain that PR head unchanged.
Work on `codex/blackhole-editor-diagnostic` in its isolated worktree.

Primary claim: qualify the observed Blackhole visual failure at the actual
local editor surface and window/message boundary, using existing bounded UI
observers rather than a new testing framework. Record an actionable visual bug
with supported findings and remaining diagnostic gaps. A renderer fix is not
claimed from a white Moonlight image, running heartbeat, or loaded graphics DLL.

Fixture: installed Blackhole Immersive 1.4.4, candidate `d7933fdaedb3`, Windows
host `5fa0907b045df5ccb993a135fccffe932b365182bec7fa16c1a48a126b1869c2`,
existing activated environment `8b064373d533b72230cb4009952eaab4`, pinned Proton
11.0-2c / SLR4, Bitwig 6.1 and the current KDE Wayland/XWayland Deck desktop.
Keep module, runner, prefix identity, licensing, publication and installed product bytes
unchanged. Use only the disposable LVB-BH audio project for any reproduction;
no repeat audio, performance, activation or broad plug-in campaign.

Basis: AGENTS.md / Work to an outcome, engineering safeguards and GUI test
custody; docs/ARCHITECTURE.md / 6.6 Editor; UIO1/UIO2 exact-window/capture and
read-only identity contracts; UIR1's distinction between a responsive heartbeat
and actual message-class delivery; retained Blackhole stereo runtime receipt.
Sol 5.6 Extra High owns any code; Luna 5.6 Max alone performs bounded GUI input.
The parent owns fixture identity, observation setup, interpretation and cleanup.

Scope: reuse existing profile/candidate, publication, session and exact-window
identity facts; local capture, child/parent geometry, DPI/style, graphics-module
presence, and paint/message progress only as supported by the existing tools.
Do not fabricate legacy observer admission or silently broaden its authority.
Any small diagnostic adaptation must retain changed/foreign/stale identity and
capture-refusal checks, bounded records/deadlines, and separate raw private
artifacts from sanitized reports. No input injection, renderer setting, DPI,
window-style, Wine DLL override, SDK-call or real-time-path change is implied.

Verification: interpret the existing Pigments tests before one relevant live
observation. Check any changed diagnostic adapter with focused failure tests;
retain the actual local pixels privately and publish only bounded metrics and
non-sensitive structural facts. A frame hash alone is not semantic UI success;
a parent HWND alone is not a rendered child surface. Preserve the independent
close hang in `IPlugView::removed()` and real stereo audio result.

Stop the observer on identity drift, lost capture scope, terminal failure or
capacity overflow. Only the exact test helper/host may be retired, using current
owned cleanup. Leave protected project bytes and other plug-ins unchanged.
Commit and push this investigation separately; do not merge either branch.

The bounded claim is now observed. Diagnostic source `6bb1480` reused the pinned
UIO1 observer and UIO2 graph/capture: two exact local 874×552 frames contain only
white RGB pixels, while a visible, enabled child has the same extent at 96 DPI.
The heartbeat remained responsive; zero settled-window WM_PAINT records do not
establish paint failure. Mapped `winex11.drv`, D3D11 and DXGI bytes match the
pinned runner's Wine built-ins. The presenting API and failing operation remain
unobserved. Exact owned retirement and protected-file/prefix preservation passed;
Luna removed the unsaved failed device. Bug #132, docs/BLACKHOLE_EDITOR.md and
evidence/blackhole-editor retain the result and next discriminant. No renderer
repair, normal-close repair, or additional audio claim follows from this slice.

# Blackhole stereo-layout continuation

The operator authorized the recommended next step: test genuine stereo
negotiation for the installed Blackhole Immersive 1.4.4. Base commit
`150042b6a078b0b87adf1e4739a8a01b2c617460`, tree
`d4ab06a3e8640183e15c1bd02df9fd2561b43f82`. Primary claim: determine whether
the exact component accepts one stereo input and one stereo output through
`IAudioProcessor::setBusArrangements`, with matching bus-info and arrangement
readback. Eventide lists stereo among supported layouts, but that is not live
evidence for this runner/host combination.

Reuse the existing supervised candidate-host inspection in the unchanged
activated environment. Any diagnostic request must be explicit; preserve the
normal default inspection. Record original layout, requested layout, actual
SDK result, subsequent bus counts/info/arrangements and restoration of the
original layout before cleanup. A false result may still change the layout;
never infer readback or discard channels. Reuse the existing Windows fixture
for focused coverage. Sol 5.6 Extra High owns code; the parent owns supervision,
fixture identity and evidence; Luna 5.6 Max alone performs any GUI actions.

No full immersive implementation, license/prefix/runner changes, publication,
or DAW/audio acceptance is implied by negotiation. Preserve installed software,
inventory, existing native proxies and prefix identity. Retain sanitized build,
negotiation and cleanup evidence. Successful negotiation establishes the next
bounded integration path; refusal remains an explicit observed result. Commit
and push the changes on the existing draft PR; do not merge.

Basis: AGENTS.md / Work to an outcome, Keep the engineering safeguards, GUI
test custody; architecture SDK boundary and exact identity; official VST3
IAudioProcessor arrangement contract. Fixture remains the existing Deck,
Blackhole 1.4.4, PACE 5.10.2.4663 and pinned Proton 11.0-2c / SLR 4.

The live diagnostic from source `b984c77` passed: Blackhole accepted stereo
with result 0, changed both buses to two channels/arrangement 3, then restored
its exact original 16-channel layout with result 0. Cleanup, transport retirement,
55 protected files and prefix identity were preserved. See the stereo receipt.

The operator also explicitly authorizes Luna 5.6 Max to use Bitwig through
Moonlight. Continue to a useful experimental stereo candidate and real DAW
effect check. Add one closed `stereo_main_pair` arrangement choice to normal
inspection/preparation and its exact candidate/profile identity. The Windows
inspection must emit canonical stereo bus facts only after successful request
and readback; runtime must apply the same policy before validating its bus
contract. Preserve omitted-policy identity/default behavior for existing
products. Limit this choice to one main audio input/output; no general immersive
implementation or new work in the real-time callback.

Use existing supported candidate preparation and experimental publication,
then Luna's new disposable Bitwig project with an actual 12-second stereo
audio clip. Capture only Bitwig's outputs and distinguish dry baseline, effect
output, editor access and saved-project recall as observed. The existing
instrument-shaped headless commercial test is not effect evidence and need not
be expanded for this DAW check. Preserve all existing projects/publications;
retain rollback for delivered software and the new experimental publication.


Source `443277f` implements the closed policy; focused checks, Linux manager
and frontend builds, and exact Windows CI `35466904843` passed. It is installed
with the prior default scanner and inventory identity preserved. The ordinary
stereo inspection passed and candidate `d7933fdaedb3` was built and published
experimentally as revision `0332c95f49d207729d84820080190b6b`. Bitwig's native
index contains its exact processor ID and name. The dry audio fixture's captured
output matches its source sample for sample. Luna then inserted the exact
Blackhole device in Bitwig; its runtime independently verified the stereo
policy and admitted a DSP session. The editor opened blank and became
unresponsive before playback started. The attempted wet recording is silent
because playback was never initiated; it establishes no audio outcome.
The custodian stopped only the verified test supervisor, confirmed child cleanup
and transport retirement, and Luna removed the failed unsaved device. The audio
track, clip and saved dry baseline remain intact. Normal manager observations
record DAW load passed, editor failed and audio not tested. Wet processing and
saved-project recall remained unproved at that point.

A single before-close repetition showed the owner loop and editor heartbeat
advancing while the editor stayed blank. The prior close hang is localized to
the synchronous `IPlugView::removed()` call; the white surface remains unresolved.
With that editor left open, Luna played the real source once in Bitwig. Its
recorded stereo output contains a late reverb tail where the sample-exact dry
baseline is silent. This establishes bounded real stereo effect processing;
the manager audio observation is updated accordingly. Exact test-host cleanup
and transport retirement passed again. Editor usability, audio quality, timing
stress and saved-project recall remain unproved. See the stereo runtime receipt
for exact identities, both attempts and their separate outcomes.

# Blackhole controller-interface continuation

The operator authorized continuation after successful vendor activation and the
post-activation scan's `controller query tuple` rejection. Base commit
`4e53c42a55d6063bd37f75c909e5397099c7ebe3`, tree
`4d675fff30e50f558921b610ddfe06508ca0d3ff`. Primary outcome: determine the actual
controller-interface response and correct the host's acquisition path where
official SDK semantics support it, then verify the real Blackhole inspection.
Basis: AGENTS.md, Work to an outcome / Keep the engineering safeguards / GUI test
custody and delegated control; existing architecture's SDK boundary and exact
identity preservation. Fixture remains Blackhole Immersive 1.4.4, PACE
5.10.2.4663 and the existing pinned Proton 11.0-2c environment on the Deck.

Scope: Windows SDK controller acquisition and structured diagnostic records,
focused regression coverage, verified build artifacts and one relevant live
inspection per materially changed candidate. Sol 5.6 Extra High handles code;
the parent owns supervision/deployment/evidence; Luna 5.6 Max alone operates the
GUI. First measure the missing return code and pointer-presence without logging
addresses or secrets. Preserve successful component initialization and distinguish
valid separate-controller fallback from malformed responses. No licensing,
runner, prefix, vendor-binary, existing publication or unrelated plug-in change.

Use a separately identified candidate host with existing supervision for the
initial diagnostic, preserving installed software/inventory authority. Retain
the actual query tuple, selected association, subsequent failure or completed
inspection, source/build identity and cleanup. Install a proven repair through
normal setup with rollback if needed for the manager's follow-up path. Do not
claim audio or Bitwig usability from inspection alone; no repeated unchanged
scan, UI test campaign or broad compatibility claim is in scope.

The measured tuple was `kResultFalse`/null. Source `39c8da7` admits that additional
tuple through the exact separate-controller path and keeps malformed responses
out of owned cleanup. Windows CI and the real candidate inspection passed;
the scanner was installed and the ordinary inventory scan also passed. The
normal compatibility action exposed two remaining delivery details: it uses its
own preparation-kit host, and its post-suspension registry acquisition could
refuse immediately while a GUI snapshot held the lock. Update only the kit's
host/source entries (all 73 other entries retained), and use the existing bounded
operation-attributed acquisition at the inspector admission seam. This is an
in-scope repair of the ordinary verification path, with a focused contention
test and one final GUI action after deployment. The GUI-closed compatibility
action already completed successfully on the updated kit. The newly observed
16-channel input/output layout is a separate audio target; the operator has
been asked to choose stereo negotiation first or full immersive support.

The repaired manager is installed. Luna clicked **Refresh preliminary
inspection** once with the manager open; operation
`4b449f7bfa0790ecb4d581984be44510` completed. Its inner registry acquisition
waited 30.522 ms over four attempts, then the exact separate-controller
inspection completed and the bridge returned active and idle. The manager
visibly showed the completed result and offered **Prepare a test candidate for
this inspection and recipe**. All 59 protected records/reports/proxy files
remain unchanged. No preparation, publication or audio test was attempted;
the observed 16-channel layout remains outside the current stereo-bus support.
See `evidence/blackhole-immersive-deck/controller-gui-readback.sanitized.json`.

# Blackhole Immersive load failure and actionable scan recovery

2026-09-19: the operator requested investigation and repair after Blackhole
Immersive installed but its first scan timed out at `load_library`. Base:
`c9758f58ed51254cf828d5622f1a86c0f57687e0`. Primary outcome: identify the observed
blocking stage using retained installation and scanner facts, repair misleading
scan reporting and provide deliberate exact-module retry after a relevant change.
Investigate the installed PACE/iLok dependency in the existing isolated environment.
Use bounded supervised diagnostics where retained evidence cannot answer the
question; preserve the original failed scan and all installed identities.

Scope: existing manager inventory, scan actions and presentation, dependency
diagnosis, narrow verified launch preparation if needed, focused regressions and
sanitized findings. No Codex GUI input, license/account inspection, arbitrary
activation changes, prefix recreation, unrelated plug-in tests or performance
campaign. The operator handles any vendor sign-in or activation UI. A timeout is
not a proven PACE defect; installed files are not service-readiness evidence.
Commit and push reviewable changes; do not merge. Record any remaining live gate.

Base tree: `e584752e955f102266ff68c5fde006eb4beabfe1`. Basis: AGENTS.md,
Core product invariants / Evidence requirements; GOVERNANCE.md, What evidence
means; docs/ARCHITECTURE.md, 4.3 Readback / 5.1 Manager core. Fixture: the existing
Deck installation of Blackhole Immersive 1.4.4, bundled PACE 5.10.2.4663 and its
pinned Proton 11.0-2c / SLR 4 environment. No dependency version changes.

Acceptance: show the retained scanner error separately from parser quarantine;
offer a deliberate retry bound to the current environment, scan, module and
report; scan only that module through existing supervision; retain other results
and the original report. Reject changed identities and busy runtime state.
Ordinary rescans retain their existing quarantine behavior. Verify the onboarding
environment route without requiring publication, and permit identical verified
scanner bytes to relocate during a manager update. Use focused regressions,
Linux release builds and read-only installed projection checks. Retain sanitized
findings under `evidence/blackhole-immersive-deck/`. Deployment changes only the
manager/frontend through setup, retaining prior immutable software and private
launcher/manifest backups for rollback. Audio, activation, successful loading,
and universal reporting classification remain unproved or deferred.

The bounded reporting/retry repair is implemented and its focused checks and
Linux build passed (`fea71b4`). After the operator closed the manager, setup
installed the exact binaries and read-only projection verified the retained
timeout and enabled exact-module retry. Service readiness, runtime digests,
43 protected records and nine native proxy files were preserved; see
`evidence/blackhole-immersive-deck/deployment.sanitized.json`. Blackhole's load
timeout remains unresolved despite PACE reporting RUNNING; the next live
investigation step is iLok startup in its existing environment with the operator
reading the window. No retry or GUI input occurred.

The operator subsequently authorized Luna 5.6 at Max to use Moonlight computer
control for the manager steps. The bounded GUI check opens the installed
manager, clears filters, finds Blackhole and reads its failure/retry control.
The parent retains candidate/evidence custody and does not operate the GUI.
This check does not authorize another unchanged module scan or establish iLok
startup, activation or successful plug-in loading.

Luna completed that check: the card visibly showed the retained timeout, generic
factory reason and active-styled retry control. The manager remains open on the
card. Independent process readback confirmed the installed frontend, unchanged
original scan/report and idle healthy bridge. No retry was pressed; iLok startup
and the underlying load failure remain the next investigation step.

On the operator's request to continue, run one bounded iLok startup observation
in the same Blackhole environment, pinned runner and HOME. Use the installed
verified launch adapter and companion cgroup ownership; restore the idle bridge
after cleanup. Luna 5.6 Max observes Moonlight and reports non-sensitive startup
text. No sign-in, activation, dependency update, prefix recreation or automatic
module retry is part of this observation.

iLok startup succeeded: Luna observed the Sign In screen without a startup/error
dialog, and process evidence bound the exact installed image. The bounded
diagnostic retired cleanly; a normal supervised user session is now open in the
same environment/HOME with no runtime cutoff and bridge restart configured on
application close. The operator has been asked to sign in, activate Blackhole
if available and close iLok. Authorization and the follow-up module retry are
pending that user action; the original loading timeout remains unresolved.

The operator now authorizes a private SSH credential handoff so Luna can enter
their iLok User ID/password into the existing sign-in form. Add a bounded
operator helper with hidden terminal prompts, private temporary runtime storage,
15-minute expiration and one-time password consumption. Bind input to the exact
owned iLok window/process; send text through stdin, never argv, environment,
clipboard or diagnostic output. Luna owns field selection and sign-in submission;
the parent never reads credential values. Keep Remember credentials unchecked.
This is a developer/operator handoff for this live session, not a general product
credential store or expanded compatibility claim. Verify refusal/consumption
behavior and one harmless input path before providing the command to the user.

The helper is installed and verified. Seven focused tests and one fictional-data
SSH/GUI check passed, including hidden prompts, masked password input and
one-time deletion. Luna restored both fields empty with Remember unchecked;
status is `none`. The next user action is running `lvb-ilok-login` over an
interactive SSH session and reporting that the handoff is ready. Luna may then
select the exact fields, invoke the fixed fill commands without reading the
credential file, and submit Sign In. Actual sign-in and activation are unproved.

The operator supplied the handoff and requested Luna to continue. One sign-in
succeeded; the handoff file was verified absent. Luna then activated the exact
Blackhole license on the offered current local computer through the vendor UI,
observed Successful Activation and closed iLok normally. Owned cleanup completed
with zero remaining processes, the unit retired and the bridge returned healthy
and idle. Luna submitted one post-activation exact-module retry. The original
load timeout did not recur: factory discovery returned three classes, component
creation/initialization and the audio-processor query succeeded. Inspection then
stopped at `controller query tuple` (SDK host code 90). The existing record omits
the query result/pointer tuple, so the precise incompatible response remains
unknown. The manager now shows Eventide Blackhole Immersive 1.4.4 as Installed -
not published, with the error and Check compatibility action. Cleanup and
transport retirement were confirmed, the original report was retained and the
bridge is active with no DSP, pending work or stale transports. No publication,
audio test, additional scan or runtime change occurred. See the sanitized
activation and post-activation scan receipts; the next technical issue is the
controller-interface rejection, not another unmodified installer/iLok replay.

# User guide for crash capture

2026-09-19: the operator requested step-by-step crash-capture instructions as a
PDF on the Steam Deck desktop. Document the installed manager's actual controls,
next-instance scope, current eligibility limits, report export and fix comparison.
No runtime change or crash reproduction is part of this documentation task.

The three-page [guide](output/pdf/Plug-in-Crash-Capture-Guide.pdf) was rendered and
visually checked, then copied to the Deck desktop as `Plug-in Crash Capture Guide.pdf`.
The remote SHA-256 matches the repository PDF. The read-only manager projection
confirmed current action availability and capture off. Editable text lives in
`docs/user/plug-in-crash-capture.md`; delivery details are in
`evidence/manager-library/README.md`.

# Authorized Deck installation of the approved manager UI

2026-09-19: the operator approved installing the new manager after closing the
manager and completing the Blackhole Immersive installer. Deploy the manager and
frontend built from `ef9f36a29d6af70538bf3a2db6be6016faf530a3` through the existing
setup procedure, with a private rollback record. Preserve installed host/native
artifacts, runner/prefix identities, registry, installer results and user projects.
Verify executable hashes, desktop/command targets, service readiness and read-only
operator inventory. GUI interaction and new audio/plug-in qualification remain
outside this installation. This approval supersedes the earlier no-deployment
limit below; the implementation and earlier local result remain unchanged.

Installation completed through `setup` with no UI interaction. New executable
hashes, desktop target, running service identity and schema-7 inventory were
verified. The eight native proxies, runtime artifact hashes, 34 protected
registry/environment/installer records and two Kontakt project files were
preserved. See `evidence/manager-library/deployment.sanitized.json`.

# Manager plug-in library presentation

Operator-approved implementation, 2026-09-19.

- Base: `813a179355c9e22159e3adaaa6e93004e3ef2923`; tree:
  `585c5497eb245edb5bce6485bda74cc7bd4c2d6b`.
- Basis: AGENTS.md; GOVERNANCE.md, What evidence means;
  docs/ARCHITECTURE.md, 2.5 UI toolkit / 4.3 Readback / 5.1 Manager core.
- Primary claim: the existing native manager presents all discovered plug-ins in
  a searchable vendor-grouped library, with role/version/publication status,
  relevant existing actions, and expandable technical details.
- Scope: manager-ui and exact environment metadata in the existing operator
  projection. Preserve schema 7, request validation, disabled reasons, and
  manager ownership. No runtime or dependency version updates.
- Verification: local frontend build, existing frontend checks, focused checks
  for filtering/status visibility and exact action association, plus a visual
  preview from the actual widget using clearly labeled synthetic records.
  Unknown states must remain visible; disabled actions must remain disabled;
  matching vendor names must not associate different environments.
- Non-goals: Deck deployment, Bitwig interaction, computer-use automation,
  Kontakt routing, capacity/timing changes, compatibility or audio acceptance.
  Kontakt and Jev GUI experiments are deferred by the operator.
- Deliver one reviewable PR, commit and push. Preserve the installed Deck,
  projects and vendor environments. Rollback is the prior frontend revision;
  this change performs no data migration. Retain prior observations below as
  history only, not active work.

Local implementation is complete: frontend build, 35 frontend tests, Clippy and
manager compile passed. Wide/narrow/search images use the actual widget with
synthetic records. See `evidence/manager-library/README.md`. Deck deployment and
interaction are not part of this result.

# Prior work (deferred / historical)

# Operator steering: Jev-only GUI validation

2026-09-19: the operator withdrew the Kontakt multi-output task and prohibited
Codex GUI control. Jev remains authorized for bounded GUI actions. Do not resume
the four-output exercise or replace it with a new headless testing campaign.
Use the existing Jev worker, short explicit action/time bounds, and existing
process/audio evidence where it answers the actual usability question.

The first continuation ran seven Jev decisions with zero Codex GUI actions. It found
and repaired a streamed-window target-check limitation in the existing Jev
harness. The repaired worker selected Kontakt's device and opened Bitwig's device
menu; the application, engine and plugin-host process identities survived.
Opening the actual Kontakt editor remained incomplete. This is not an audio,
state-recall or general compatibility pass. See
`evidence/kontakt-native-access-deck/jev-usability.sanitized.json`.

The four-part project setup below is retained history, not active authority.
Its unused MIDI/checker drafts were retained privately rather than delivered as
new product or test machinery. The working KontaktStereo installation and its
previous playback/recall evidence remain the baseline.

# Kontakt four-part routing and independently checked Jev workflow

Operator-approved continuation, 2026-09-19.

- Bridge base: `fd23d7567ffefe707870a0e98127183ca9ce335d`;
  tree: `1c3dcbd9706530ecdc2429ed8da236e7ebaeef01`.
- Existing Jev harness base: `f658435f55b295236a7041f509a7e1b4367af495`.
- Basis: AGENTS.md, Work to an outcome / Keep the engineering safeguards /
  GUI test custody and delegated control; GOVERNANCE.md, What evidence means;
  docs/ARCHITECTURE.md, 5.7 Native Linux VST3 proxy / 5.8 Windows VST3 host /
  7.3 Audio shared memory / 7.5 Events and automation / 15.2 Commercial fixture
  harness; docs/FIXTURE_CARDS.md, F3 Runtime/bridge questions and Required
  acceptance classes.

Primary claim: on the existing Deck installation, a four-part Kontakt project
produces correctly separated audio and preserves its routing across a complete
Bitwig restart, with one Jev-operated workflow independently verified.

Fixture: existing Kontakt 8 Player 8.13.1 / Factory Selection 1.4.2, Native
Access 3.26.0 environment, pinned corrected Proton runner, Bitwig 6.1 Flatpak /
freedesktop runtime 25.08. Retain exact installed candidate identities from
`evidence/kontakt-native-access-deck/shared-bus-playback.md`. Run at 48 kHz and
512 host frames. Preserve the working KontaktStereo project and downloaded
libraries; create a separate owned KontaktFourParts project.

Scope: this contract, owned project setup, reusable development-only audio and
state checker, the smallest integration with the existing Jev worker, necessary
routing/editor/harness repairs, and sanitized evidence. Jev handles foreground
interaction; controlled tools own lifecycle and independent verification. No
replacement computer-use framework or vendor installation machinery.

Acceptance: four separate MIDI parts reach separately mixable Bitwig outputs,
including the highest declared output bus (index 31); effect processing on one
output leaves the others independent. Capture the outputs, verify separation,
save the project, fully close/restart Bitwig and verify instrument/routing recall.
Run a real bounded Jev workflow whose success comes from the independent checker.
Deliberately mute or disconnect one owned test output and require the checker to
detect the missing result; restore the playable project afterward. Retain actual
failures, versions, source identities, captures/hashes and project identity.

Non-goals: all 32 simultaneous outputs, capacity or memory increases, 256/128
frames, general timing qualification, Serum 2/FX, GUI redesign, iLok, ARM/Pi,
other-library qualification and bulk campaigns. No merge or broad compatibility
claim. Preserve existing successful observations without treating them as proof
of this new routing claim.

Cleanup/rollback: change only the owned project and test processes; retain its
private save/capture artifacts without publishing proprietary state. Restore
test mutes/effects and changed settings, retire only owned captures/sessions,
record bridge activity, commit/push the focused result and update the draft PR.

Earlier completed scope and observations follow as history.

# Kontakt through Native Access on the Deck

Observed continuation result, 2026-09-19: source `9d47606` is installed as an
experimental candidate on the Deck. Actual Kontakt 8 Player 8.13.1 loaded Factory
Selection Pad - Noir in Bitwig 6.1, played the owned MIDI clip, and produced
finite nonzero stereo audio. Full Bitwig close/restart restored the instrument
and produced audio again. Two 45-second captures and all seven actual load
attempts are retained in `evidence/kontakt-native-access-deck/shared-bus-playback.md`
and its sanitized JSON. Observed underruns, taskbar foreground workaround,
124 MiB maximum extra output storage, and unproved graceful Windows retirement
remain explicit limits. This is the requested first sound/recall test, not
ordinary profile qualification, broad multi-output routing or timing acceptance.

Operator continuation: implement shared bus handling and get Kontakt audio into
Bitwig for a first real test. Start from `544c02e2781e77d08c8e7ce7aa9e7cec49c69e45`.
Preserve every declared output identity and negotiate the complete layout. The first-stereo-only milestone was tried and failed: Bitwig activates
output 31 before running Kontakt. The continuation therefore carries every
active stereo output through a versioned shared mapping and bounded, owner-
allocated native storage. No output is silently discarded. This mechanism has
no Kontakt-specific executable branch. Keep the installed NI environment and existing published
profiles intact; build a separate preparation kit and experimental candidate.
Verify bus metadata/activation/buffer safety, actual Kontakt audio and a
disposable Bitwig save/reopen. Retain the actual result and any remaining failure.

Transport continuation: protocol 1.13 selects mapping layout 2 (two input planes,
64 output planes, 68,176 bytes); 1.12/layout 1 remains readable by the Windows
host. SDK bus indices map to consecutive stereo planes. The existing 2,048
completion descriptors retain their capacity. Extra planes use one pre-touched
slot pool, allocated during inactive setup: 4 MiB per additional stereo output,
124 MiB at 32 outputs, and no extra pool for a single output. Slots remain owned
until callback delivery/discard; no callback allocation or deallocation. This is
a bounded first implementation, not a many-instance memory-efficiency claim.


Operator continuation, 2026-09-18: install Kontakt using Native Access on the
Deck, then discover/publish the real VST3 for Bitwig and test it. Each observed
stage is reported separately; installation alone does not establish audio.

Base commit: `0f7d122bb08e5b491461847a8c818a19fe3d2c86`.
Base tree: `8142b9ca0aec5883a1d8631ea6d6f4c72fddda88`.
Primary outcome: the existing NI environment uses the corrected pinned runner
and Native Access performs the real Kontakt installation.

Scope: the MSI correction, runner/environment transition and existing Native
Access session admission; subsequent exact Kontakt scan/profile/proxy work
needed for the requested Bitwig test. Rust owns product changes. Existing
supervision is reused; there is no replacement vendor installer. Preserve
historical installation evidence and the existing prefix/machine identity.

Fixture: Steam Deck Galileo, existing Native Access 3.26.0 environment and lawful
Kontakt 8 Player 8.13.1 package, pinned Proton 11.0-2c candidate documented below.
Acceptance proceeds through actual Native Access install and reopen recognition,
SDK class discovery, native proxy publication, then a disposable Bitwig project
with editor/audio/state checks. Report any concrete failure at its actual stage.
Before live mutation retain a private prefix/metadata rollback copy, close owned
sessions, and verify the exact runner modules. Do not alter other environments,
Steam's installed runner, account state, or vendor product registration manually.

Observed result: Native Access installation and recognition after reopening
completed; scan and SDK inspection completed. Proxy preparation refused Kontakt's
32 stereo outputs at the current single-output implementation. No proxy was
published and no Bitwig audio/editor/state test was run. Preserve these completed
sub-results. The remaining work is actual bus support across the proxy/Windows
host, not another installer implementation or a removed bounds check. See
`evidence/kontakt-native-access-deck/README.md`.

The earlier investigation follows as retained evidence and reproduction detail.

# Kontakt MSI runner correction investigation

Selected by the operator's 2026-09-18 reset: identify the actual Kontakt
installation failure, correct the compatibility boundary, and retain the lesson.

- Base: `745974b2325e9ace4f7aef21c82f62f29e7e2ff3`
- Base tree: `b6213087918811f1e90f27d827285dc30785bb3f`
- Basis: operator-provided AGENTS.md, Mission / Repository roles /
  Compatibility-profile rules; checkout AGENTS.md, Work to an outcome /
  Keep the engineering safeguards; GOVERNANCE.md, What evidence means;
  docs/ARCHITECTURE.md, 2.1 Rust owns the product / 5.2 Runner builder and
  inventory / 5.3 Environment manager / 5.4 Installer supervisor.
- Fixture: retained lawful Kontakt 8 Player 8.13.1 package; Steam Deck Galileo;
  pinned Proton 11.0-2c, Wine `dc26e61847081a1b5cb0733dc30feba6ee575482`.

Primary claim: determine whether ambiguous MSI string-reference widths remain
misparsed after the two earlier upstream backports, and whether correcting that
runner defect lets the vendor installer finish in a fresh disposable prefix.

Scope: this file, `docs/KONTAKT_MSI_FINDINGS.md`,
`evidence/kontakt-msi-root-cause/**`, and
`kontakt-msi-ambiguous-strrefs.patch`. Private Wine build/source and disposable
prefixes are permitted outside the repository. No new product deployment engine.
The K8I1 candidate remains preserved on its existing draft branch.

Acceptance: explain the exact observed row-width error from the retained package
and Wine source, build the narrow MSI correction, and run one bounded ordinary
vendor installation in a fresh disposable environment. The first correction run
deployed the final payload and exited in 232.22 seconds with Linux status 100.
One additional fresh run with InstallAware's documented native logging switch
finished in 232.65 seconds with identical final binary hashes. Its native MSI
INSTALLEND record establishes a successful MSI execute sequence. Outer Linux
status 100 and vendor variable COPYERROR=TRUE remain unexplained; full wrapper
success is not claimed. The runner correction remained identical.
Record actual deployment or the next actual failure. A successful table read alone
is not installation.

Negative acceptance: preserve the original failing cases; do not claim success
from wrapper exit, staged files, tests, or partial registration. Stop at a new
failure rather than silently broadening this correction.

Evidence: sanitized package identities, table sizes/counts/reference validity,
source/build identities, exact installer outcome, final payload observations,
remaining gaps, and a reproducible route. No proprietary payload or MSI tables.

Cleanup: retire only disposable processes from this operation, retain their
private logs and prefix for inspection, and preserve the working managed NI
environment and earlier candidates. No managed-prefix migration, licensing,
Native Access interaction, DAW/audio acceptance, merge, or product release.

Earlier NAO1 authority follows as retained context, not the active task.

# NAO1 — Bind the owned session to Proton's initialized command service

Focused repair basis:

- merged qualified-recovery repair: `c38b5c9deb8bd32f5a509258c30f31b3e795fe59`
- merged tree: `6d661a80a707157f30d3f4815e04a72b1cb38586`
- installed immutable generation: `91ff69f049863e074f907a051d5c4c5913cb04c9633d71db3b2838246ed03b2e`
- retained failed physical operation: `ca95619b7609089e3a9b3a439780785d`
- retained error: `dependency_qualified_registration_absent`

The second physical NAO1 attempt correctly preserved the complete qualified-recovery
authority but again stopped before readiness or application launch after two exact SCM
queries returned error 1060. Read-only comparison then established that this was not a
missing installation: the intended physical prefix was unchanged, its selected control
set contained the exact fixed `NTKDaemonService` registration, and the daemon image was
unchanged. Direct queries through the exact pinned Proton runtime saw that service as
`STOPPED`.

The defect is the operation-private runtime topology. It starts a bare
`srt-launcher-service` outside Proton and then submits a nested `proton runinprefix` as a
child of that service. That constructed runtime therefore queried a different Wine/SCM
universe from the initialized prefix and truthfully returned 1060 for the wrong
universe.

The selected repair keeps one exact source-owned Windows anchor alive as the single
Proton `runinprefix` root wrapped by the verified Pressure Vessel launcher interface.
It binds the private command-service name emitted by that exact root and submits every
dependency helper and the Native Access launch adapter through the verified launch
client and exact Proton `runinprefix` in that same session. The retained anchor keeps
the Wine/SCM universe alive; Proton reconstructs its own closed Wine environment for
each inserted command. Proton `run` is excluded: the pinned runner routes it through
built-in `steam.exe`, and the first disposable candidate produced a source-owned
qualification assertion rather than an admissible product runtime. Bare Wine is also
excluded because it loses Proton's prefix/session environment. A bare service without
the interface remains excluded. The Windows root validates and retains one exact
operation/nonce-bound hold-file generation; forwarded stdin is not lifetime authority.
Only removal of that exact hold after dependency retirement requests root closure. The
anchor, helpers, daemon, and application stay inside the existing renderer cgroup and
cleanup boundary. No bus name, command, path, service, or environment value becomes
caller authority.

Generated qualification must prove that:

- the exact entry point, Proton script, launcher interface, launch client, Wine image,
  Windows adapter, operation, and nonce are verified and bound;
- one verified launcher-interface + Proton `runinprefix` root is retained for the
  operation and all closed commands reuse its one private command service;
- the old bare-service plus `runinprefix` topology is absent from the dependency and
  application path;
- startup refuses missing, duplicate, malformed, changed, or mismatched identities and
  readiness frames, or missing/aliased/replaced hold authority;
- a stopped fixture service established and queried through a separate exact Proton
  reference entry remains visible to a cold product runtime without reinstalling it;
- continuous draining, callback privacy, exact SCM/process/listener ownership, one-stop
  authority, and exact-owned cleanup remain unchanged;
- a generated artifact-absent qualified-recovery session reaches readiness, launches
  its source-owned application fixture, closes truthfully, and can reopen once.

This repair does not reinstall or repair NTKDaemon registration. The physical
installation is already registered in the intended initialized Proton session. During
implementation and qualification, do not install the candidate, launch Native Access,
start or stop the real daemon, replay the installer, or mutate the real prefix. Preserve
both failed physical operations and the private recovery checkpoint. Return one draft,
uninstalled PR for independent source review; direct user interaction resumes only
after an exact reviewed generation is installed.

The earlier NAO1 authorities follow and remain in force where they do not conflict with
this later, physically established repair.

# NAO1 — Qualified-recovery readiness handoff repair

Focused repair basis:

- merged NAO1 source: `a544645792aa1428d0388191e4fee45e7801b921`
- merged tree: `d438c67d68f2f64ff94cd7110aaff390c05e9e04`
- installed immutable generation: `c4380746a60f85d711c9bd39fc9e5e48bafc501a18eddff7a49d4d68b0cdc3c0`
- retained failed physical operation: `d3762548dc0597464d52ef2b63a77b5a`
- retained disposition: `REAL_NATIVE_ACCESS_SESSION_ADMISSION_FAILED_DEPENDENCY_PREPARE_REQUIRED`

The first physical NAO1 attempt admitted the exact
`qualified_recovered_installation` origin and reverified the installer and daemon
images. Its only SCM query then reported exact error 1060 (`service does not exist`).
No start/readiness loop or application launch followed. The retained record cannot
distinguish a transient private-runtime SCM initialization observation from persistent
missing registration.

This repair carries the complete validated session authority into `Nad1Owner` instead
of reducing it to the daemon image. An initial exact 1060 observation for either closed
session origin receives exactly one delayed query in the same operation-owned runtime.
Exact registration is not process-generation ownership or retirement authority. After
that delayed query reports exact registration, the owner revalidates the physical
prefix and runs one fresh bounded process census. `RUNNING` or `START_PENDING` may
continue only with one exact same-prefix generation already owned by this renderer
cohort. `STOPPED` may continue only with no candidate, through this operation's one
owned start. Unavailable, ambiguous, foreign, deleted-prefix, or same-prefix unowned
candidates refuse before application launch and leave stop authority absent. Another
SCM absence or an unavailable observation also refuses. The repair performs no
registration mutation, installer replay, artifact/preparation synthesis, or second
physical operation. A persistently absent real registration remains a separately
reviewed product/design decision.

The original NAO1 authority follows and remains in force.

# NAO1 — Native Access owned session

Active basis:

- merged NAD2: `a86f03e8a5d5302d9872f995a0a5ba376a0ab6d5`
- merged tree: `11a9fefbbe8b184c52058f4b4610da3a5e3b06e4`
- qualified NAD2 executable source: `81bbf198e8bbe7df337312f2529c84b318c1ed64`
- installed Native Access identity: exact 3.26.0 application retained by NAUI2
- installed dependency: exact NTKDaemon 1.32.0 payload and `NTKDaemonService`

NAD1 and NAD2 established exact dependency identity, fresh same-prefix process and
listener readiness, one-stop ownership, truthful stop-response characterization,
bounded process cleanup, renderer/browser-return ownership, and a prior rendered
signed-in Native Access session.

The remaining product blocker is the historical preparation gate. Native Access launch
currently requires a prior dependency operation that both proved readiness and achieved
graceful SCM retirement. The real daemon proved ready but remained `RUNNING` after the
single submitted stop request, so no preparation receipt was created even though exact
owned cleanup restored the bridge and keepers.

Selected next slice: **NAO1 — Native Access owned session**.

Authoritative slice document: `docs/NAO1.md`.

NAO1 makes the Native Access application operation own its exact NTKDaemon dependency
for the same session. Launch must use one of two mutually exclusive exact installation
origins: the retained installation-artifact record, or the fixed qualified-recovery
record while that artifact pointer remains absent. Both origins require the current
application, software, installer, daemon, environment, and prefix identities plus fresh
per-session readiness; launch must not depend on a historical graceful-retirement
receipt.

At session close, the manager still attempts one graceful SCM stop. If graceful
retirement is not confirmed, exact-owned process cleanup may complete the Native Access
session only when all owned processes and listeners are absent and all identity,
privacy, preservation, and bridge-recovery checks pass. The result must continue to
report `service_retirement_confirmed=false` and `forced_cleanup_used=true`; it must not
create a dependency-preparation receipt or claim graceful shutdown.

This is an exact Native Access 3.26.0 / NTKDaemon 1.32.0 compatibility policy, not a
general Windows-service framework.

During the implementation PR:

- do not install the candidate;
- do not launch Native Access or transition the real daemon;
- do not change the selected `--disable-gpu` renderer policy;
- do not weaken exact process, listener, prefix, application, or software identity;
- do not change Arturia, VST, audio, editor, product-publication, or capacity behavior;
- do not rewrite prior evidence.

Return one draft PR, uninstalled and unmerged, for independent tech-lead review. After
that source is accepted, the next action is one installation and one real Native Access
session—not another daemon-diagnostics slice.
