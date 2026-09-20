# Plug-in reliability and audio I/O follow-up

The operator confirmed working Blackhole Immersive audio with the installed
DirectComposition reference and identified the priorities below. This records
the next work; it does not claim these repairs are implemented or authorize a
new broad GUI test campaign. Preserve the working runtime and existing projects.

## 1. Touchscreen failures and missing incident reports

Operator report: using the Steam Deck touchscreen crashes Blackhole, Serum and
Kontakt, and the bridge does not show the corresponding plug-in crash report.
These are cross-product observations, not yet a demonstrated common cause in
the Windows message pump, renderer or input translation.

Start with one exact product/version, control and physical touch gesture in a
disposable project. Compare that gesture with a mouse action on the same control.
Capture the first observed failure and identify which process actually exits or
hangs: editor host, processing host, native proxy/DAW, or environment helper.
Physical Deck touch is a distinct input path from Moonlight mouse injection.
Reuse the existing observers and supervisor records; avoid speculative fixes in
every renderer or repeated reproductions that produce no new evidence.

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
service. The operator associates this with switching to Gaming Mode. Establish
the transition's causal role separately from the confirmed launch/recovery bug.
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
