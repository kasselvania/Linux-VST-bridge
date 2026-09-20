# Plug-in reliability and audio I/O follow-up

Source candidate update (2026-09-20): the shared Blackhole/Kontakt reliability
slice now admits the exact current managed-experimental publication to optional
detailed capture, projects existing IF1 editor/host/transport terminal custody
into the catalog independently of that capture. Each request retains its
authenticated DAW process generation, while the keeper is shared only by the
exact allowlisted graphical context. Keeper warmup and a
graphical-session replacement occur behind an unowned retryable refusal; no DSP
lease, transport or native binding is exposed first. These are deterministic
source results. The original physical touch and product-audio checks have not
run and remain the operator's acceptance step.

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
