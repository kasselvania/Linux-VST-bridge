# UIO2 — owned popup control

UIO2 makes a transient editor-owned popup an exact diagnostic target. It reuses
UIO1's thread-scoped hooks, XTEST/X RECORD input, heartbeat, private captures and
installed-profile admission. It does not change the audio path or replace the
product host. The accepted UIR1 pump is unchanged.

## Window custody

The development observer creates an additional private `UIO2` mapping beside
its existing record mapping. It contains two fixed slots of at most 128 windows.
A request asks the observer helper for a fresh census; an inactive slot is
published before its commit and response counters. Readers check both counters
around their copy and refuse incomplete, overflowing or unstable observations.
This mapping is diagnostic data, separate from the product/transport ABI.

Every window retains PID, UI thread, HWND, parent, owner, root, root-owner,
Wine's exact HWND-to-XID property, visibility, enabled/minimized state, DPI,
style, screen/client rectangles and monitor work area. No title is captured.
The Linux owner binds this graph to the admitted session, process creation
identity, native editor token, activation, generation and editor epoch.

Only one visible enabled top-level popup with a complete same-process/thread
owner chain to that exact editor is admitted. An unrelated lookalike cannot
become a target through its title, location or appearance. Multiple plausible
popups, a changed epoch, incomplete owner chain or changed selected popup stop
the action.

## Capture and input

The popup is captured through its own XComposite pixmap where available. The
fallback reads the exact popup drawable; it never captures an entire desktop.
Before and after capture it verifies the Windows graph, scale/geometry agreement,
viewability, editor application activation and the X11 stacking chain. A visible
higher X11 window overlapping the popup makes capture unavailable. Native
Wayland-only windows/portals and arbitrary compositor occlusion are not claimed.

Input still goes through UIO1's XTEST implementation. Pointer settlement and
held modifier/button checks remain mandatory. Popup input allows the exact
editor parent to retain application activation. A popup disappearing on mouse
Up is retained as an unavailable post-Up pointer observation; X RECORD and
Win32 hooks remain the delivery witnesses. No replacement window is guessed.

Win32 hooks additionally retain bounded `WM_WINDOWPOSCHANGING`,
`WM_WINDOWPOSCHANGED` and `WM_SIZE` dimensions. Those are User32 observations,
not fabricated VST3 return values. Candidate 16's existing view stages are
retained separately. It does not export the requested SDK ViewRect or numeric
`resizeView` return; full resize acceptance must not be asserted from geometry
alone if that distinction remains unobserved in the product record.

## Custodian and executor

The coding agent owns the exact candidate, protected project, CA1 admission,
identities, stop conditions, evidence, cleanup and verdict. A private loopback
surface presents a current exact-window frame and one frozen action capsule to
Luna at max reasoning. The executor cannot submit coordinates, shell commands,
retries, candidate/project changes or a technical verdict. It can consume only
the capsule's one permitted action. The custodian supplies any subsequent
capsule after revalidating the still-running session.

Capsules contain exact editor identity, target row, permitted action and point,
explicit forbidden operations and stop conditions, one-action capacity and a
maximum two-minute lifetime. The whole product observer has a 170-second bound.
A terminal record, lost identity or evidence overflow stops GUI actions. CA1
must already be collecting for this exact session/software before any action.

Action files, frame metadata and frame bytes have separate private directories.
The executor surface is bound to loopback and an unguessable private URL, checks
Host/Origin and accepts only a bounded closed JSON action. It has no arbitrary
file, process or command endpoint. Raw frames and identifiers never enter Git.

## Generated verification and current boundary

`tools/uio2/windows.cpp` uses an actual SDK `CPluginView` and unchanged production
`VendorView`. A normal child has a transient owned popup and an unrelated
lookalike. Selecting its bounded menu region calls the real frame resize path;
the view receives `onSize`, resizes its child and repaints.

The Windows SDK self-test and the isolated pinned-Proton/KWin/XWayland run passed.
Normal XTEST input reached the selected popup at both X RECORD and Win32 hooks.
The production frame accepted one 800×500 request; parent and vendor child both
became 800×500, `onSize` advanced once, and paint count advanced from one to five.
The unrelated lookalike was not selected, and selection after popup closure was
refused. All helper and compositor cleanup was positive. The exact popup capture
used XComposite. Pixmap `XGetImage` returned zero color masks; the adapter now
resolves them through the exact source window's matching-depth TrueColor visual,
never a guessed default visual. An Xvfb regression also exercises that case.

The first read-only setup attempt could not admit ordinary Pigments 11 using the
current compiled ordinary UIO1 profile; no fixture launched. The generated run
instead used verified ordinary LoFi admission only to select the same runner.
The first actual generated run stopped at the pixmap-mask refusal before resize;
its positive cleanup is retained. These are setup/tool failures, not vendor
failures. See [generated evidence](../evidence/uio2/generated.json).

## One Pigments action and the exact boundary

Candidate 16 was published reversibly, CA1 was armed for that exact instance and
software, and Bitwig opened a new protected project copy through its normal
Applications desktop entry. Luna at max reasoning executed one frozen
`open_menu` capsule through a hidden browser surface. No Mac focus/input takeover
occurred. X RECORD and Win32 hooks retained Down and Up at the intended vendor
child and client coordinates. No terminal or collector record existed at the
action's end.

The new graph then showed five visible enabled top-level windows on the same
Windows process and UI thread. Every one reported `GW_OWNER = 0` and
`GA_ROOTOWNER = self`. None had the required owner chain to the editor. The menu
and its surrounding windows therefore could not be admitted as an exact
editor-owned target. Same process/thread, appearance and location do not repair
that authority gap. No Resize Window item was selected.

The executor's subsequent image showed the underlying editor, not the separate
menu. The first product implementation incorrectly fell back to root capture
when it found no owner-qualified popup. That image is retained as invalid for
popup observation. The source now refuses a visible unbound top-level window
instead of presenting the underlying drawable. A deterministic regression uses
the observed five-unowned-window shape and proves that no root capture occurs.
There was no live replay after this correction.

The whole observation used 676 retained Win32 records, zero ring drops and
positive hook/helper cleanup. The product owner did not persist the X RECORD
drop/unparsed counters, so this run makes no zero-drop claim for that recorder.
The exact required X11 receipts are retained. The generated recorder separately
reported zero drops and one unparsed/non-target packet.

See the [capsule, executor receipt and waterfall](../evidence/uio2/product-result.json).
The capsule is public only as a redacted projection plus its exact private
fingerprint. Raw screenshots, paths, process identities and session IDs stay
private.

## Subsequent quit incident

After the observer stopped, the custodian requested normal Bitwig close with the
menu still open. No Escape or resize action was sent. The native worker then
reported `InvalidData: state response deadline`. CA1 retained an access violation
on the Windows UI owner thread and a matching Windows self-exit status
`0xC0000005`; the outer Proton launcher independently exited 5. There was no
normal process-scoped retirement-ready record.

The fault instruction and next unwind frame use RVAs `0x7988` and `0x2d11`
relative to a base Wine had logged for builtin `UIAutomationCore.dll`. CA1 could
not verify that file's mapped identity/extent, so its authoritative module/hash/
symbol result remains unavailable. That logged base correspondence is a useful
lead, not proof of the underlying defect or permission for an accessibility/DLL
change. Forty stack frames were retained privately, including exact Pigments
module-relative offsets. Earlier exceptions had been followed by continued
processing; matching the final numeric code does not make those earlier events
fatal.

IF1 retained the first terminal context. IF2 acknowledged it in the controller,
kept the native host alive at CA1 finalization and later reported 7,585 contained
callbacks / 3,883,520 silent frames with no automatic reload. Bitwig truthfully
reported a state-save warning. The custodian proceeded with quit without saving
the protected project; native terminal cleanup completed. No failure view was
reopened in this stopped test, and no successful normal SDK retirement is claimed.

These remain separate observations: the missing popup owner chain stopped the
resize test; the later state timeout and Windows access violation occurred during
the quit attempt. Resize was never selected. No causal claim about resize, menu
dismissal or quit is made. See [sanitized incident](../evidence/uio2/incident.json).

## Final disposition

`UIO2_GENERATED_OWNED_POPUP_PROVED_PRODUCT_OWNER_CHAIN_ABSENT_SUBSEQUENT_TERMINAL_INCIDENT`

The bounded task is complete at an actionable boundary; Pigments resize remains
unqualified. The next work must establish a real owner relationship for these
unowned menu windows and independently interpret the captured exit path. This PR
does not select an alternate identity rule or a speculative vendor repair.

Ordinary Pigments 11 is physically restored. Candidate 16 is inactive and all
product bytes/profiles are unchanged; no revision 17 was created. LoFi/FRAGMENTS
10, the reporter software, original/protected project hashes and authorization
are preserved. Bitwig and owned cohorts are absent; service/keeper are active;
leases, pending transactions, stale transports and held input are zero. Capture
and tracing are off; no GUI executor, tunnel or UIO transient unit remains.
512 is selected/recommended; 256 remains unqualified. PR #101 stays draft and
unmerged for independent review. See [final readback](../evidence/uio2/installed-final.json)
and [exact artifacts](../evidence/uio2/artifacts.json).

## TSG1 generated result and delegated-executor boundary

Review 5191850998 accepts the first menu click and containment evidence, but does
not accept completion of UIO2. The active cut adds action-bound ownerless surface
groups and exact candidate17's process-local accessibility comparison. The prior
five-surface observation and later quit incident remain unmodified.

The development observer's `observe-surfaces` mode adds thread-scoped CBT and
WinEvent lifecycle observation. Ordinary `observe` keeps UIO1's existing exact
owner-chain message scope. Observation of a peer never authorizes its input.
Graph schema2 adds extended style, class atom/hash, focus/capture, stacking peers
and WindowFromPoint at the current pointer. Raw creation parameters stay distinct
from stabilized census. X11 metadata retains exact parent/frame chains, viewable
state, EWMH facts, bounding/input shapes and root stacking position.

TSG1 requires a complete pre-graph, exact Down/Up receipts on both sides and a
bounded two-second post interval. New/shown windows need create/show evidence;
recycled handles and pre-existing moved peers are refused. Components use a
maximum two-desktop-pixel edge gap, contiguous X stacking and one nonempty input
shape member. Before Down, two complete snapshots, actual X pointer target and
Win32 WindowFromPoint must agree. Missing or ambiguous input shape is a tool
boundary, not permission to infer content from largest area. A nested menu
invalidates the consumed group and requires a new capsule.

Candidate17 reuses every candidate16 artifact. Only revision/evidence and the
process-scoped accessibility policy/limitation change. This is an authorized
comparison against the retained UIAutomationCore signature, not a root-cause or
stability claim. The isolated generated canonical, ownerless, composite, ambiguity,
nested, disappearance and Escape tests pass. See
[generated facts](../evidence/uio2/tsg1/generated.json).

One candidate17 instance was loaded from an Applications-launched protected
project with CA1 armed. Its real editor opened. No menu action occurred: Luna's
tool inventory exposed no browser, and an explicit attempt to access the
custodian's hidden browser tab also returned browser unavailable. The custodian
could access its own browser but did not substitute itself for the required
executor. No popup was created, no group was bound and Resize Window was never
selected. UIO2's real action remains incomplete.

Before that executor boundary, an initial private executable permission error
prevented observer launch; it was corrected without a product relaunch. A capture
attempt then exposed Xlib's process-global error-handler ownership: the newer
graph connection received the older capture connection's XComposite error. A
Display-keyed dispatcher now retains errors on the exact connection, restores the
prior handler after the last private connection closes, and handles arbitrary
connection close order. The regression and two simultaneous real read-only
captures passed. The repaired observer used the same still-running vendor
instance. Private frame metadata now retains the group authority needed for the
custodian to issue a second capsule; the executor still supplies no coordinates.

The action capsule was never consumed. Both observation attempts and their
cleanup are retained. The second observer was cancelled through its unit:
its hook-detached marker was present, but the helper received SIGTERM before its
closed marker. This is bounded cancelled cleanup, not a normal helper exit.
The subsequent ordinary Bitwig quit completed all seven process-scoped vendor
retirement milestones, transport/lease cleanup and native/Windows process exit.
Temporary edits to the protected copy were declined; recorded project bytes are
unchanged. CA1 finalized without a first terminal record. Its observed exception
records remain classified as fatality unavailable and are not called crashes.

Ordinary Pigments11 is restored, candidates12–17 inactive, sibling publications
unchanged, service/keeper active, and all leases/transactions/transports/input,
capture, helper units, tunnel and executor tabs are clear. CPUWeight is unset and
effective100. No product byte was rebuilt or promoted. See the
[exact bounded outcome](../evidence/uio2/tsg1/product-boundary.json).

The next product action requires a functioning delegated browser route before
launch, then the still-unperformed closed menu/resize test. The immutable host
also does not export numeric live `resizeView` request/result fields; current
live tooling retains Win32 geometry and view stages separately and must not
substitute those for a raw SDK result. Generated SDK resize facts remain valid.
No real ownerless-group, resize, accessibility-stability or performance success
is claimed here.

### Native Moonlight executor continuation

The operator authorized Luna to use native Moonlight keyboard/mouse controls.
`tools/uio2/direct.py` observes those external actions using the same private
capsules, X RECORD, Win32 hooks and transient-group owner. It does not inject
input or depend on a browser. A bounded rolling graph selects a complete snapshot
before actual input receipt; delivered point, button state, target and epoch
remain checked. The observer cannot prevent an external Down: this limitation is
recorded explicitly, and an invalid observation grants no further action.
Resize geometry observations do not implicitly authorize a new popup.

Luna reached the streamed Deck desktop. The first connection failed before
Bitwig; the running Sunshine instance predated the current desktop and could not
open its Wayland display. Restarting that existing application restored display
capture and the connection, without changing pairing or certificate validation.
This is connection setup evidence, not a Pigments resize result.

### Operator continuation and final cleanup

Luna used native Moonlight mouse control to open the menu and select Resize
Window. Both exact targets retained X11 and Win32 Down/Up. TSG1 bound the first
ownerless group and captured its content window. A temporary chooser window was
destroyed during X11 graph enrichment; BadWindow stopped the observer, whose
hooks and helper retired cleanly. No automated size-choice claim follows.

The operator took control in that same candidate17 session and confirmed choosing
a new size, smooth new-wavetable selection, tab navigation and knob interaction.
The operator then quit normally. CA1 retained no terminal failure or Collector
rejection; all seven process-scoped retirement milestones committed, with positive
process/transport cleanup. The four diagnostic exception entries are not relabeled
as terminal crashes. One 512-frame delivery gap remains recorded.

Ordinary11 is restored; candidate17 is inactive. LoFi/FRAGMENTS10, installed
software, protected originals and test project bytes remain unchanged. Capture is
disarmed; Bitwig and cohorts are absent; DSP leases, transactions, tmpfs sessions,
held input and UIO units are zero. Service and keeper are active. The successful
operator result and incomplete automated resize waterfall remain separate; no
product promotion, universal stability or historical root-cause claim is made.
See `evidence/uio2/tsg1/operator-resize-and-cleanup.json`.

### Reviewed transient-census repair

Review 5192912686 accepted the focused operator/product result and identified
normal transient destruction during graph enrichment as the remaining tooling
repair. `SurfaceGraphX11.snapshot` discards an entire attempt on typed X11
BadWindow and re-reads Win32 and X11, at most three attempts. No incomplete graph
is returned or retained. A fresh absent/hidden surface grants no old authority;
a still-visible unverifiable XID fails at the bound. Other server/errors remain
terminal diagnostic failures. Snapshot interval storage is copied, so failure
cannot mutate a retained pre-action graph. Product, generated and Moonlight
observation paths all use the same owner.

The real Xvfb regression destroys a source-owned transient after snapshot but
before XGetWindowAttributes, exercising production error capture and retry. It
proves fresh absence, refusal of a stale popup target, no injected input, and
bounded refusal when Win32 keeps claiming the deleted visible surface. Unit
regressions additionally cover hidden state, unrelated/mixed errors and terminal
changes. No Pigments or pinned-Proton product replay is part of this repair.
