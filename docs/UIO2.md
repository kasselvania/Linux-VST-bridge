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

`tools/uio2/windows.cpp` uses an actual SDK `CPluginView` and production
`VendorView`. A normal child has a transient owned popup and an unrelated
lookalike. Selecting its bounded menu region calls the real frame resize path;
the view receives `onSize`, resizes its child and repaints. The Windows self-test
passed in AP8 run 34773103847 at source 417b0a2. That is SDK fixture evidence,
not yet the pinned-Proton/XWayland or Pigments result.

The private KWin/XWayland runner uses the existing UIR1 isolation and helper
cleanup owners, a fresh unlicensed scratch prefix and exact pinned runner. It
records normal XTEST/X RECORD/Win32 receipts and exact before/after geometry.
Unit checks cover stale/foreign/ambiguous/disappeared windows, occlusion,
held input, interrupted graph publication and capsule refusal. A separate
Xvfb test exercises real X server capture, popup input and foreign occlusion.

The current implementation checkpoint has not run the UIO2 Pigments action.
Deck execution awaits a fresh Tailscale SSH authorization. Ordinary Pigments 11,
candidate 16 and all installed software/artifacts are untouched by this source
checkpoint. The prior menu attempt never selected Resize Window and remains
**not a resize failure**.
