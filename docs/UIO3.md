# UIO3 — Touch release attribution

This development observer follows one ordinary Pigments18 editor. The operator
supplies all touch and MIDI input. It does not repair or assign a cause to the
reported held-drag behavior. MF1 reviewed software is installed first.

## Observation design

The original exact-client X RECORD core path remains. UIO3 explicitly decodes
XInputExtension Generic Events by negotiated opcode, event type, endian, extent,
source/device and exact target. Unknown generic events are not called touch.
A separate libXi connection observes raw pointer/touch events only. It does not
select TouchBegin on the vendor window, grab input, accept/reject touch ownership,
read /dev/input or retain keyboard events. Raw events carry no destination: their
associated pointer chain/button mask is a *later query*, never delivery evidence.
XWayland may expose a virtual pointer without physical touch events; that is an
observable coverage limit, not evidence of a lost touch release. A device census
retains hashed device names, attachment, classes and reported touch capability.
XWayland can create a pointer source lazily. A bounded fresh census may extend
that set with verified pointer devices; replacement/removal of an existing
identity refuses observation. Every raw record still requires exact queried
window scope (or continuation of an already scoped touch contact). Device history
is retained privately, capped at 16 snapshots.

The existing thread-scoped Windows observer has an opt-in `observe-input` mode.
Normal UIO1 scope and ABI1 record size remain unchanged. Additional scalar records
carry pointer ID/type/flags or bounded touch-contact details. No touch handle,
source handle, extra-info, opaque payload or typed keyboard text is retained.
Touch handles are never closed by the observer. Failed API reads remain unavailable.
WH_MOUSE entry/next-hook result, PM_REMOVE retrieval, sent-call hooks and
actual window-procedure entry/return remain distinct. UIO3 installs transparent
Comctl32 subclasses on the exact UI thread and exact editor children, forwarding
once through DefSubclassProc without changing arguments/results. They are removed
on stop. The diagnostic DLL is pinned inert until that vendor process exits so
an abnormal observer exit cannot leave an unloaded callback address. A nonzero mouse-hook result differs
from a later message rewrite to WM_NULL. Capture, focus and active HWND facts
are private; no direct vendor WndProc call is introduced.

AP11 Begin/Value/End is observed without consuming either production queue.
Parameter times are poll intervals, not invented emission timestamps. QPC is
projected through UIO1 round-trip brackets with its 100ppm drift allowance.
X server time remains separate. Whole-editor frame hashes prove only local
change; they do not automatically identify a macro or its repaint latency.

## Operator session

A dedicated private development unit owns the helper and mappings. The exact
native view, activation, generation, epoch, process/start and HWND/XID are frozen
from admitted ordinary state. A private scalar mailbox labels `0` (waiting),
`1` (without held note), `2` (with held note), `3` (stop). It cannot authorize
input, coordinates, a repeat, reverse action or a new instance. The total bound
is 160 seconds; the first useful release boundary stops after a three-second
observation tail. The operator uses the same control for at most two drags.

Raw timeline and summary are private. The sanitized projection contains bounded
observation facts, hashes, parameter identity and cleanup facts; raw window,
process/device identifiers, paths, payloads and raw frames are excluded. Capture
still present alone is not a defect. Missing observation is not root cause.
On a terminal record the observer stops; IF1/IF2 keep ownership. CA1 is used only
if already explicitly armed before launch. No relaunch is authorized by this tool.

## Source basis

- [XInput2 protocol](https://www.x.org/releases/current/doc/inputproto/XI2proto.txt)
- [WM_POINTERUP](https://learn.microsoft.com/en-us/windows/win32/inputmsg/wm-pointerup)
- [GetPointerInfo](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getpointerinfo)
- [GetTouchInputInfo](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-gettouchinputinfo)

## Generated proof boundary

Real User32 mouse delivery proves the exact procedure observer, capture facts and
detachment. The same production pointer/touch scalar encoder is tested with
bounded API-result fixtures, including absent pointer information and oversized
contact counts. Fabricated WM_POINTER messages and synthetic touch injection
were refused in the hosted fixture; those failed attempts are retained in
`evidence/uio3/generated-development.json`. No generated physical-touch delivery
is claimed. The Xvfb/XWayland fixture separately exercises actual libXi raw
pointer events, exact-client X RECORD delivery and post-release button state.

## Physical result and status

The one human-only ordinary18 session produced a useful delay observation. The
operator reported that touch followed their input but was massively delayed. The
single labeled attempt contained three contacts, so it is not a controlled
single-drag baseline. The held-note action was not attempted.

All X11 releases were recorded by 17.740 seconds after the label. Windows release
procedure observations continued at least 16.514 seconds beyond that last release.
This stream-tail bound does not assume that Linux and Windows contact IDs share
an identity namespace. UI heartbeats continued, with a maximum observed latency of
42.955 ms; later mouse capture samples were clear. Translation, queueing, retrieval,
vendor servicing and observer perturbation remain unresolved possible owners.

X RECORD, raw XI2 and Windows rings reported zero drops. The GUI witness reported
one missing row, with bootstrap versus live phase unavailable. Three gesture begins
and ends were observed, but this is not complete gesture proof. The original
private summary's incomplete-observation classification is preserved alongside
the sanitized timing analysis in `evidence/uio3/result.json` and `result.md`.

The user quit Bitwig normally. All seven process-scoped retirement milestones,
positive cohort/transport cleanup, zero DSP/maintenance leases, no transaction or
stale transport, zero held core buttons/modifiers and healthy service/keeper were
confirmed. Capture remained off; observer hooks detached. No terminal record or
Collector rejection occurred. No product input was injected by the agent.

Generated validation includes Windows hosted and pinned-runner observer fixtures,
Linux Xvfb and isolated Deck XWayland, and 74 Python checks (13 UIO1, 4 UIR1,
32 UIO2, 25 UIO3). The XWayland fixture first exposed asynchronous placement and
lazy pointer-source creation; failed attempts and bounded repairs remain retained.
All three applicable workflows passed at the physical observation source
`0820ddb017d909d4b55e16124038f2426fcc62f8`.

No product host/native/profile revision or compatibility repair. Ordinary18,
rollback11, siblings10 and inactive history remain unchanged. 512 recommended;
256 unqualified. Return draft PR105 for independent review; do not merge or
repeat the product session.
