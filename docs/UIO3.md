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

The existing thread-scoped Windows observer has an opt-in `observe-input` mode.
Normal UIO1 scope and ABI1 record size remain unchanged. Additional scalar records
carry pointer ID/type/flags or bounded touch-contact details. No touch handle,
source handle, extra-info, opaque payload or typed keyboard text is retained.
Touch handles are never closed by the observer. Failed API reads remain unavailable.
WH_MOUSE entry/next-hook result, PM_REMOVE retrieval, WH_CALLWNDPROC entry and
WH_CALLWNDPROCRET return remain distinct. A nonzero mouse-hook result differs
from a later message rewrite to WM_NULL. Capture, focus and active HWND facts
are private; no subclass or direct vendor WndProc call is introduced.

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

## Status

Implementation and generated validation in progress. No live UIO3 result yet.
No product host/native/profile revision or compatibility repair. Ordinary18,
rollback11, siblings10 and inactive history remain unchanged. 512 recommended;
256 unqualified. Independent review in draft PR105; do not merge.
