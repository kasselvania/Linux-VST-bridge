# UIR2 — Touch release admission and retrieval differential

UIO3 merged unchanged at `a204c2d`. This slice preserves its three-contact result,
16.513815470-second unpaired release tail, live heartbeats, missing GUI row and
normal cleanup. No product/runner/profile compatibility change is made.

## Phase A: retained records

The private UIO3 timeline remains available. `tools/uir2/analysis.py` derives core
and XI release records and Windows pointer/mouse-up retrieval, contact-up detail,
downstream hook and actual procedure records. Private targets, coordinates,
source/contact identities, capture/focus and clock values remain private.

Delayed Windows pointer releases carry old MSG.time values. They cannot be safely
paired to the three X11 releases from counts or numerical contact IDs. The old
source-owned fixtures did not retain an independent X-server/MSG.time calibration
alongside these records; UIO3 did not sample queue classes. Consequently Phase A
cannot select admission versus retrieval. Old message time alone does not prove
the corresponding Windows message was already available to this UI thread.

## Source-owned fixture

`tools/uir2/windows.cpp` includes the production `vendor_view.h` unchanged. Local
compile-time wrappers forward the pump's exact PeekMessageW and DispatchMessageW
calls once, without changing arguments, results, order or filters. No production
source or product artifact is changed. The fixture owns its parent/child and their
WndProc; no hook or injected subclass is used. Default pointer/touch registration
posture is retained rather than forcing a synthetic route.

A fixed mapping retains at most 262144 scalar records (128 bytes each). UI-thread
records cover each pump turn, queue-state hint, removal before/after, MSG.time,
actual dispatch/procedure entry/return, capture, pointer flags and heartbeat.
No arbitrary keyboard content, opaque touch handles or application payload is
recorded. Touch handles remain owned by DefWindowProc. No input is injected.

The high word of GetQueueStatus is current classes; the low word is changed
classes. Calling it clears change bits, which is an observer side effect. Neither
word is a per-message admission proof; User32 may internally process a class
without returning that message. The production pump does not use these sampled
change bits. The fixture's diagnostics can perturb elapsed time, so it does not
claim a performance deadline or exclude all observer effects.

## Clock and classification custody

The Linux companion reuses exact XI2/X RECORD scope. Raw events' pointer target
is a later query, not delivery evidence. No touch ownership/grab, /dev/input,
keyboard payload or XTEST action is added. A source-owned PropertyNotify brackets
X server milliseconds in Linux monotonic time. The fixture's UI-thread handshake
brackets QPC and GetTickCount. Real posted calibration messages verify MSG.time
against the fixture's own tick interval, retaining a 64ms rounding allowance.
Unsigned 32-bit wrap is resolved only within a known half-wrap interval; ambiguous
half-wrap is refused. X and Windows clocks are never assumed equal.

One physical single-finger contact is required, with matching raw source/detail
begin/end and exact core release. No Linux/Windows contact-ID equality is used.
A prompt source-owned removal/procedure path selects embedded-product-route work,
not a Pigments cause. Old timestamp plus queue hints alone cannot select host
retrieval. Absent sampled classes plus a late timestamp alone cannot select Wine
admission: absence between samples and actual creation remain unproved. Those
outcomes require stronger exact availability/creation evidence or remain
insufficient coverage. The generated classifier tests cover both evidential rules.

The diagnostic prompt threshold is 100ms removal and 20ms procedure gap, including
clock bounds. It only distinguishes this generated path from UIO3's multi-second
tail; it is not a product latency guarantee.

## Execution and cleanup

Each run owns a fresh scratch prefix on the pinned runner's filesystem, with a
private HOME and runtime scratch. Only the physical desktop display/authorization
is shared; no vendor prefix, account, project, product session or installed manager
is touched. The existing Helper/process-containment owner drains bounded output
and retires the exact child cohort inside a timed transient cgroup.

The companion exposes one scalar begin label, no input or coordinate mailbox.
The operator performs one short drag, lifts and remains hands off. Multiple
contacts stop as insufficient without retry. The fixture expires even if the
companion disappears. Normal stop destroys both windows, removes the timer and
closes the mapping. Final report must retain positive cohort cleanup, empty held
input and unchanged canonical product/service state.

## Sources

- [MSG.time](https://learn.microsoft.com/en-us/windows/win32/api/winuser/ns-winuser-msg)
- [GetQueueStatus words and hints](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getqueuestatus)
- [PeekMessage ordering](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-peekmessagew)

## Result

Disposition: **UIR2_INSUFFICIENT_PHYSICAL_COVERAGE**. The generated Windows and
pinned-runner tests, Xvfb/XWayland clock fixture and 56 relevant Python tests pass.
One physical-desktop observation window ran; no operator completion arrived and
no contact/release was recorded before its bound expired. There was no second
window or forced retry. It does not select admission, retrieval or vendor cause.
See `evidence/uir2/result.json` and `result.md`.

Both fixture windows and cohorts retired positively. Ordinary products and
installed software were not changed. Service/keeper healthy, capture off, no
leases, transactions, stale transports or held core input. PR106 remains draft
and unmerged; no product retest or compatibility repair is proposed.
