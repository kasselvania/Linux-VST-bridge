# UIO1 — Cross-boundary editor observability

UIO1 is complete at its diagnostic boundary. A real Pigments interaction was
received by X11, then waited **at least 8.237 seconds before the Win32 mouse hook**.
The UI thread continued servicing heartbeats. The eventual button reached the
correct vendor child, was not consumed by the downstream hook chain, and was
followed by a VST BeginEdit. This identifies an input-admission delay before
Win32 retrieval. It does **not** identify an internal Wine function or fix the
responsiveness defect. No renderer correction is justified by this record.

AP18 was merged at `1fae92b31baa8dc8b1d444bdb5412124c6de6243`. Its accepted product,
profiles, audio protocol and retained qualification remain unchanged. PR #96
contains development diagnostics, generated fixtures, and the bounded result.

## Exact result

Fixture: ordinary Pigments 7.0.1.6772 revision 11 in Bitwig Studio 6.1 Flatpak,
SteamOS 3.8.16, the accepted pinned Arturia environment, and a 1280×724 XWayland
editor. The existing protected project was already playing its C3 material and
Macro 1 automation. UIO1 did not create a DSP instance or rewrite that project.

Three actions were selected: existing host-driven Macro 1 automation, a Macro 1
drag, and a page tab click. The first record was interpreted before any follow-up:

| Record | Result and reason for any follow-up |
| --- | --- |
| Initial three actions | Host parameter updates present. First drag landed at stale coordinates because XTEST motion had not taken effect; invalid control evidence. Page click reached the intended X11 coordinates, with no Win32 button retrieval or page-region change inside the bounded interval. |
| Settled XTEST refinement | Added exact pointer readback before Down and a thread-scoped WH_MOUSE observer. Correct drag and page coordinates; hit-testing reached the vendor child. No mouse-hook/button retrieval or gesture in the bounded interval; heartbeat remained below 28 ms. |
| Generated pinned-runner fixture | Real Win32 SendInput was visible to the observer. Accepted versus deliberately consumed downstream-hook input was distinguished. Its attempt to restore pointer position was not acknowledged; no further click was sent and no input remained held. |
| Two Moonlight actions | Compared genuine remote input with XTEST after interpreting the previous record. Correct X11 drag and page coordinates. Button-down eventually reached WH_MOUSE and GetMessage after an eight-second delay. The page appeared later, outside the bounded record. |

The manual comparison began on Play after the earlier page request eventually
took effect. Its fixed FX Macro capture region is therefore **not** a valid Macro
redraw-latency witness. The tab-region capture remains valid. This limitation is
explicit in the exported report; no repeated UI campaign was used to conceal it.

The important manual drag sequence, using Linux-relative time in the retained
waterfall, is:

| Observation | Time / meaning |
| --- | --- |
| X11 ButtonPress observed at the exact target, client (1014,237) | 25.995020139 s; actual server delivery occurred no later than this poll |
| X11 ButtonRelease, client (1014,207) | 36 ms after Down in the X server clock |
| WH_MOUSE enters for WM_LBUTTONDOWN, vendor child alias 2 | 34.232377478–34.234657163 s, including clock bracket and drift allowance |
| Downstream mouse-hook return | zero: this delivered Down was not consumed |
| WH_GETMESSAGE retrieves WM_LBUTTONDOWN | 34.233431784–34.235711257 s |
| BeginEdit(parameter 1) observed | by 34.247930992 s; GUI polling is not an emission timestamp |
| Maximum UI heartbeat during the drag action | 136.7359 ms; no continuous eight-second UI-thread stall |

Subtracting the latest possible observed X11 time from the earliest projected
Win32 entry gives the **8.237357339 s lower bound**. Approximately 8.24 s is the
observed-endpoint difference, not a claim of precise physical input latency.
The current event reached the correct child with HTCLIENT, focus on that child,
active editor parent, and no capture at Down. Later heartbeat records showed
capture on the child. The Up had already reached X11 but was not retrieved by
Win32 before collection ended. Begin/Value/End and a closed editor status were
subsequently retained for the same logical view/epoch after normal editor close.

This is not permanent input loss. Both page requests eventually became visible
outside their bounded timing records. The record rules out a Moonlight-only
pixel delay for the measured wait, and the human comparison rules out an
XTEST-only failure. It does not separate every Wine hardware-input/focus/queue
condition from vendor interactions with those mechanisms. The production pump
uses unfiltered `PeekMessage(nullptr, 0, 0, PM_REMOVE)`; it was not changed.

See [result](../evidence/uio1/result.json),
[failed mechanisms](../evidence/uio1/failed-mechanisms.json), and the three
`*-waterfall.json` / `*-facts.json` pairs in [evidence/uio1](../evidence/uio1/).

## Ownership and measurement contract

`bridge-manager/src/ui_observation.rs` owns read-only ordinary-profile admission,
adapter selection, the closed public waterfall schema and cross-clock projection.
It verifies the compiled exact profile, physical publication, registration,
module/native/host/source manifest and pinned environment. `src/bin/uio1.rs` is a
development command, not installed software or a generic promotion mechanism.

`windows-factory-probe/source/uio1/` builds a separate observer, hook DLL and
optional accessibility probe. They are not linked into the product processing
host. The observer binds PID **plus process start**, exact HWND and UI-thread ID,
and restricts observation to that editor's root, children and owned popups.
There are no global hooks and no audio-thread instrumentation.

The Windows mapping is private, version 1, 1,839,104 bytes: a 4,096-byte header and
16,384 append-only 112-byte scalar records. A complete record precedes its commit.
Overflow increments a counter; it cannot stall a vendor callback. Action IDs are
bounded. HWNDs and process identities stay private; exported identities are aliases.
Text, WM_CHAR, arbitrary key codes, pointers, SysEx, vendor strings and state are
not exported. Keyboard observations retain only closed key categories.

Thread-scoped WH_GETMESSAGE records **retrieval** of posted messages.
WH_CALLWNDPROC/RET records **sent-message** entry/return; those are not general
posted WndProc timing probes. WH_MOUSE retains exact target/coordinates, hit-test
and downstream-chain boolean while returning CallNextHookEx unchanged. Read-only
facts include focus, active window, capture, mouse/key state, size/paint/timer/DPI
messages, message time, and action ID. The hook never redirects an event.

A private thread message supplies a 5 Hz heartbeat, with at most one outstanding
request. The request timestamp is retained before publishing its acknowledgement.
The generated fixture exposed and fixed an earlier early-acknowledgement race.
The observer runs at most 180 seconds and unregisters all four exact hooks on
exit. The target releases its mapping on stop. Unhooking does not forcibly unload
an in-flight DLL; an inert cached module may remain until normal process exit.

`tools/uio1/launch.py` uses the verified installed ownership helper in a separate
transient user unit. Only four build-pinned EXE/DLL identities can be staged.
Changed, symlinked, oversized or foreign helper bytes are refused. Output is
private, capped at 524,288 bytes with overflow and stderr byte accounting.
Deadlines, descendant ownership and positive cleanup apply to the helper cohort,
not the already-running vendor host. The Linux observer never consumes or writes
the production GUI message queues. Host-value traffic remains instance-scoped;
editor gestures retain their own native token and epoch.

## Desktop and visual boundary

The exercised adapter is exact-window XTEST on X11/XWayland. It derives desktop
coordinates from current window geometry and verifies pointer settlement before
Down. A single identical motion reissue is bounded and recorded; unresolved
movement refuses the click. Cancellation releases only keys/buttons it owns.
X RECORD observes delivered pointer events for that exact X client/window and
never records keyboard payload. Capacity is 4,096 records with overflow counts.
No hand-built XSendEvent is used for pointer or keyboard input. EWMH is used only
for an ordinary application activation request.

Native Wayland has an explicit RemoteDesktop-portal/EIS admission boundary. A
consent grant and EIS connection are required; there is no XTEST fallback for an
unresolved Wayland surface. **Native Wayland injection is not implemented or
qualified in UIO1.** The selected fixture is an XWayland window.

XCompositeNameWindowPixmap returned BadMatch on this fixture. The fallback is
XGetImage of the **exact foreground editor drawable**, never a whole-screen
capture, renderer redirection, or Moonlight frame. Frames are private and bounded:
10 Hz, at most 600 summary records, and a few explicitly retained compressed
frames. Public output retains dimensions, full/perceptual/region hashes, sampled
change bounds and percentage. Change bounds use an 8-pixel sampling grid. A stable
hash is not automatically a semantic page signature. No OCR or full-page UI map
is used. Page changes outside the capture interval have no inferred latency.

Windows QPC is projected through measured Linux-before/Linux-after brackets with
a 100 ppm drift allowance and maximum ten-second distance. X server timestamps
remain a separate domain. GUI witnesses have conservative action-to-poll intervals.
No unsynchronized clock subtraction is presented as exact event latency.

## Optional probes and overhead

The profile permits Pigments' normal Windows accessibility posture. The bounded
UIA probe failed at its initial ElementFromHandle/RawViewWalker boundary (exit 3),
returned zero nodes and cleaned up. This does not establish a canvas-only tree.
The probe never reads Name/Value/text patterns or account material; profiles that
disable Windows accessibility cannot invoke it. Semantic automation is optional.

Wine's remote Toolhelp module census returned access denied (5). The successful
window census remains retained, and a bounded allowlisted Linux mapping read
observed OpenGL/GDI/UIA modules. No D3D/DXVK modules were observed. A loaded module
is not proof of the presenting API or thread. DXVK logging/HUD, RenderDoc and GPU
tracing were not appropriate to this observed path and were not enabled.

In the manual record, 6,231 hook observations cost 16.9122 ms total, with a
0.3023 ms maximum. This counts observer work, not downstream hook/vendor execution.
597 local frame summaries averaged 14.534 ms CPU each (maximum 18.793 ms); the
Python observer used 19.100 s CPU across the interval. The process-wide CPU sample
was about 0.419 cores during the short trace-off context and 0.445 cores during
diagnostic startup/idle. These are all-thread CPU observations with 10 ms ticks,
not UI-thread CPU or a matched causal overhead estimate. No dropped Windows, X11,
GUI or frame records occurred. No audio or product-performance claim follows.

## Verification and state left behind

The production-path Windows fixture covers exact child scope, foreign-window
exclusion, posted versus sent messages, a delayed heartbeat, accepted and consumed
SendInput, scalar privacy, capacity overflow and safe unhooking. It passed on
Windows CI and the pinned Proton runner. Linux tests cover coordinates, stale
pointer refusal, owned cancellation, frame hashes, X RECORD scope, interrupted
record publication, helper identity/refusal, profile accessibility gating and
public projection. A real Xvfb/XTEST/X RECORD/capture fixture runs in the manager
workflow. The closed Rust schema tests unknown-field refusal and clock uncertainty.

See [validation](../evidence/uio1/validation.json) and PR #96 for exact final-head
checks. The live helper is pinned to source `1e4b20eced6b164d7e16f5fe7960338dcb8537cf`
and the workflow's synthetic merge commit is retained separately. It is not
claimed to have been rebuilt from a later report/evidence commit.

Final [physical readback](../evidence/uio1/installed-final.json) verifies unchanged
ordinary Pigments revision 11, LoFi/FRAGMENTS revision 10, exact artifacts and
publication parents. Service and keeper remain active. There is **one pre-existing
DSP lease and one keeper lease**, no publication transaction, no UIO1 helper or
transient unit, no held input, and no debugger. The editor is logically closed;
the existing unsaved Bitwig project/session remains open. Its protected on-disk
project hash is unchanged. No original project was saved over.

SteamOS still owns effective CPUWeight 10000; UIO1 did not change it or claim a
matched timing result. Heavy tracing is off. 512 remains selected, supported and
recommended; 256 remains unqualified. Residual audio counters remain truthful
whole-session observations under issue #90, not a new UIO1 audio diagnosis.

## Source references

- [Microsoft: hooks and their different observation points](https://learn.microsoft.com/en-us/windows/win32/winmsg/about-hooks).
- [Microsoft: unhooking and in-flight callbacks](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-unhookwindowshookex).
- [Microsoft: WM_NCHITTEST and HTTRANSPARENT/HTCLIENT](https://learn.microsoft.com/en-us/windows/win32/inputdev/wm-nchittest).
- [X.Org: XTEST](https://xorg.freedesktop.org/archive/X11R7.5/doc/man/man3/XTestFakeKeyEvent.3.html).
- [X.Org: RECORD library](https://xorg.freedesktop.org/archive/X11R7.7/doc/libXtst/recordlib.pdf).
- [XDG: application RemoteDesktop portal](https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.RemoteDesktop.html).
