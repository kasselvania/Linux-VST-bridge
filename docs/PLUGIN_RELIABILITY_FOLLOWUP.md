# Plug-in reliability and audio I/O follow-up

## Serum waveform touch release, bounded candidate C (2026-09-23)

The accepted Steam Deck input classification for Serum 2 2.1.5, class
`56534558667350736572756D20320000`, is narrower than the earlier broad
touch reports below. A mouse/trackpad action on the waveform dropdown passed;
physical touch on an ordinary Serum control passed; physical touch on that
dropdown left its editor and popup visible but stopped the editor thread's
message/menu progress. Bitwig, the Windows host and audio processing remained
alive. Normal Bitwig close retired the instance. The input method in the first,
uncontrolled occurrence remains unknown. Its audio delivery gap is not
attributed to touch. The retained private capture has no raw `WM_POINTERUP`
high-word, `GetPointerInfo` flag, or XInput Begin/End record; it therefore does
not independently prove the released pointer's flags.

Exact pinned Wine source `dc26e61847081a1b5cb0733dc30feba6ee575482`
does show the defect in `X11DRV_RawTouchEvent`: its common flag assignment
marks `XI_RawTouchEnd` as `INCONTACT` even after mapping it to `WM_POINTERUP`.
The source correction is limited to that touch message mapping. The
source-owned test proves Begin and Update still have contact while End does
not. Candidate C uses a distinct immutable runner and retains candidate B's
exact factory and selected-class results with explicit predecessor/transition
provenance. The old factory inventory remains stale under the new environment
revision; no fresh census is claimed. The currently published candidate B and
installed runner remain unchanged until bounded source review and one approved
physical transition. Mixed-six remains separately permitted with mouse input.

The pinned offline Steam Runtime 4 build and private Wine install completed.
The sealed successor runner has 8,262 entries, 1,447,967,926 regular-file
bytes and complete-tree SHA-256
`d095f1f052ecebb67c66d685dbd88e373b633b0c3000343a7607c4f1bc4d1920`.
Only the x86-64 Unix `winex11.so` and runner version file differ from the
verified predecessor. An isolated, unlicensed Windows command started and
retired under the successor without changing its tree. This is a runner
construction result, not a physical Serum touch-menu pass.

Physical correction update (2026-09-21): the next approved Blackhole attempt
failed quickly and cleanly before Wine started. Its retained private stderr
shows pressure-vessel asking Bubblewrap to project the explicit D-Bus denial
pathname, then Bubblewrap refusing because that intentionally nonexistent
mount source was absent. The repaired keeper classification produced two
bounded busy replies, one terminal `keeper_failed`, no replacement storm and
confirmed cleanup. Bitwig's `setupProcessing` failure was downstream of missing
bridge admission, not evidence about Blackhole audio or editor behavior.

The shared graphical projection now selects fixed product-owned dead AF_UNIX
sockets under the verified private tmpfs runtime root whenever no exact
authenticated Wayland or D-Bus alias exists. The socket paths exist so
pressure-vessel can materialize them, but no process listens, so clients cannot
connect and neither default discovery nor D-Bus autolaunch is available. The
service creates or verifies the objects; every graphical supervisor revalidates
their type, ownership, privacy, identity and dead state immediately before
launch. The same contract covers keeper and DSP/editor launch. Exact aliases
and Xauthority rules are unchanged. This remains a source correction pending
one reviewed, installed Blackhole retry; it is not a plug-in success claim.

Physical correction update (2026-09-21): the first approved Desktop Mode
Blackhole launch stopped before the planned touch comparison. Bitwig reported
that it could not load the plug-in. The exact keeper exited before readiness;
no DSP owner or Windows plug-in host was created. The requesting Flatpak's
authenticated Xauthority and DBus paths existed only in its mount namespace,
but the host-side Proton launch received those raw paths. This is now a source
repair boundary, not evidence about Blackhole rendering, touch, or audio.

The corrected supervisor resolves the peer-namespace Xauthority only to one
private host-visible file with identical complete bytes. DBus and Wayland are
forwarded only when the peer and host paths identify the same socket. The
intermediate correction used an explicit absolute, verified-absent endpoint so
omission could not fall through to host `wayland-0` or D-Bus autolaunch. The
later physical result above established that pressure-vessel could not project
that absent mount source, so the current correction replaces it with the dead
product-owned sockets. Neither version substitutes the ambient systemd
environment or grants the host session bus for a Flatpak-private proxy. Keeper
failures retain exact exit status and non-disclosing output byte counts/digests
rather than discarding all process evidence or retaining raw potentially
sensitive text.

The reviewed package was subsequently installed as immutable generation
`49758648eacc262dbd4c27856c7625c5c3705665324efca3c41670ac44225354`.
The service returned active and idle, and no product was launched. Candidate
preparation then stopped because Blackhole and Kontakt both retained inventory
from an older scanner source, while managed onboarding environments exposed no
lawful refresh action. The focused source repair retains `EnvironmentRescan` as
the sole managed-inventory operation. A stale or missing managed inventory
exposes that exact action once across the complete manager snapshot; the
onboarding card adds no second `InstallerScan`, and current inventory closes the
one-shot action. Scanner execution revalidates the current catalogue, complete
registry environment, optional retained onboarding support, installer
retirement, staleness and inactivity while holding the final `registry.lock`.
Authority drift therefore refuses before a scanner is spawned and service
restoration remains owned by the requesting operation. The repair does not
replay an installer, reopen initial installation, or alter a publication. After
that correction is reviewed and installed, Blackhole and Kontakt must each be
prepared from the exact current preparation kit and installed as new immutable
experimental publication revisions before their next physical checks. The old
publications remain rollback authority. The touch comparison below resumes only
after that boundary is reviewed and installed.

The approved managed-refresh build was then installed, but exact readback
stopped the procedure before either refresh. Blackhole and Kontakt were managed
by retained installation history plus current registry ownership, while the
installed catalogue contained only ordinary-profile environments. The manager
therefore displayed the instruction to use the managed refresh without
displaying either environment or its refresh action. The correction now derives
one exact managed roster from catalogue bindings plus registry-owned retained
managed-installation bindings and uses it for both projection and final scan
authorization. Conflicting identities refuse. The regression uses this physical
topology instead of manufacturing a catalogue entry. No scanner or product was
launched during discovery or repair.

Source candidate update (2026-09-20): the shared Blackhole/Kontakt reliability
slice now admits the exact current managed-experimental publication to optional
detailed capture and projects existing IF1 editor/host/transport terminal
custody into the catalog independently of that capture. IF1 is opened through
the exact retained volatile transport identity rather than its durable prefix
symlink. Each request retains its authenticated DAW process generation, while
the keeper is shared only by the exact allowlisted graphical context. Keeper
warmup and a graphical-session replacement occur behind an unowned retryable
refusal. A concrete supervisor must publish its exact preflight-ready receipt
before any DSP lease, transport, or native binding is exposed, and its outer
finalizer owns readiness publication and all later prelaunch failures. That
finalizer publishes failure, observes the native half-close, retires the exact
session directories, and returns the required native retirement acknowledgment;
an interrupted or missing acknowledgment remains cleanup-blocking. A small
sanitized terminal summary
survives successful owner cleanup so a fast failure does not disappear from the
manager merely because its lease retired. These are deterministic source
results. The original physical touch and product-audio checks have not run and
remain the operator's acceptance step.

The operator confirmed working Blackhole Immersive audio with the installed
DirectComposition reference and identified the priorities below. This records
the next work; it does not claim these repairs are implemented or authorize a
new broad GUI test campaign. Preserve the working runtime and existing projects.

## 1. Touchscreen failures and missing incident reports

Latest operator clarification (2026-09-20): any touchscreen contact with the UI
of the installed non-Arturia plug-ins makes the editor shut down/disappear.
Blackhole, Serum and Kontakt were specifically reported; Arturia editors do not
exhibit this behavior. No particular knob, menu or drag is required. The bridge
does not display the failure, and Bitwig continues to register the instance as
running. This is an operator-reported editor disappearance, not yet a measured
process crash or demonstrated common cause in the Windows pump or renderer.
Whether audio continues after the editor disappears is still unanswered.

Start with one exact product/version, control and physical touch gesture in a
disposable project. Compare that gesture with a mouse action on the same control.
Capture the first observed failure and identify which process actually exits or
hangs: editor host, processing host, native proxy/DAW, or environment helper.
Physical Deck touch is a distinct input path from Moonlight mouse injection.
Reuse the existing observers and supervisor records; avoid speculative fixes in
every renderer or repeated reproductions that produce no new evidence.

Use a working Arturia editor as a comparison for the same first-touch action.
Distinguish a window being hidden/closed/destroyed, an editor thread hanging or
failing while processing survives, and the entire Windows host exiting while
Bitwig retains stale status. An active audio instance after an ordinary editor
close is not itself incorrect. The bridge must surface an actual editor or host
failure at the right boundary.

No new physical touch reproduction or input repair has been performed. Two
source prerequisites have now been closed while preserving that boundary:

- [UIO3's retained Pigments observation](../evidence/uio3/result.md) showed
  Windows release handling continuing at least 16.514 seconds after the last
  X11 release, without a terminal failure. That earlier delay is not proof of
  the cause of the newly reported disappearing editors. Reuse its observer
  only after verifying admission for the selected current instance.
- `crash_capture::arm` now selects the separate exact managed-experimental
  observation admission when the current retained revision requires it. The
  catalog also reads the existing exact IF1 terminal class without requiring
  capture to have been armed. Neither change establishes why touch makes an
  editor disappear.

Next bounded step: verify diagnostics can record the selected instance before
launch, then have the operator perform one mouse/touch comparison. Retain the
first useful input/window/process/terminal boundary and stop. Repair that
demonstrated boundary and repeat the original action before extending the check
to the other products. Do not replace physical touch with automated clicking.

Investigate reporting as its own defect: determine whether the failed process
was supervised, whether the manager received its exit/failure, and whether
profile eligibility, capture settings or report persistence hid the incident.
A missing detailed dump must not make a known failed instance appear healthy.
Keep minimal failure status separate from optional detailed diagnostic capture
and its privacy controls. Identify the affected instance and recovery state.

Completion means the original physical touch interaction works, mouse behavior
still works, and a deliberately exercised failure is visibly attributed to the
correct instance with an honest cleanup/recovery result. Then check the same
interaction class on the other reported products. Include touch and reporting
in future editor validation where that input is supported; a common observation
does not justify one universal renderer fix.

## 2. Gaming Mode transitions and a panic/reset control

Observed: four environment helpers exited; a subsequent Serum launch checked
its dead helper after exposing the native binding and left an abandoned launch
record that blocked new instances. Manual recovery restored an empty, available
service. The source candidate now starts/rechecks the keeper before creating an
instance and retains a concrete supervisor before returning acceptance. It also
refuses cross-display keeper reuse and retires an obsolete keeper only after all
DSP owners are gone. The operator associates the original loss with switching
to Gaming Mode; establish that physical transition separately.
See [the retained recovery](BLACKHOLE_EDITOR.md#post-test-service-recovery-and-requested-follow-up).

Correct the failed-launch cleanup and helper-loss handling. Desktop to Gaming
Mode and back should leave an accurate stopped/recoverable state and permit a
fresh launch after recovery without manual lease editing.

The requested panic control should explain that it interrupts bridged audio,
stop only bridge-owned instances/helpers, preserve failure evidence, verify
retirement, and restart the bridge service and manager. If cleanup remains
uncertain, name what remains rather than clearing the warning. Closing the
manager window must not be presented as a service reset. DAW project recovery
and automatic recreation of plug-in instances require a separate explicit
contract; the bridge must not discard unsaved host work.

## 3. Expose the actual audio inputs and outputs

The operator now considers full usable audio I/O essential. Enumerate each exact
plug-in's declared buses, supported channel arrangements, activation state and
host-visible routing. Preserve bus/channel identity through native proxy,
shared transport and Windows host; distinguish multiple stereo buses from a
single multichannel bus. Unsupported arrangements must be explicit.

First inventory existing bus support, including the previous Kontakt work,
before adding another transport or increasing a fixed limit. Use Blackhole's
real supported layouts to select the next bounded implementation. Preallocate
the required capacity outside the audio callback. Validate distinct signals on
each enabled input/output, routing isolation and saved-project routing recall;
names in Bitwig and nonzero stereo output alone do not establish full I/O.

## Delivery order and preserved limits

Address touch failures together with visible failure reporting first, then
helper/session recovery and reset, then full audio I/O. Each change should have
one concrete reproduction and a real product result, with focused code checks.
Do not start a new broad benchmark or installer campaign. Rendering, physical
input, audio processing, state recall and lifecycle recovery remain separate
claims. The working Blackhole reference is a demonstrated exact-fixture result;
upstream runtime support or general compatibility is not inferred from it.
