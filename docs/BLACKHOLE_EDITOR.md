# Blackhole editor rendering investigation

Tracking: [bug #132](https://github.com/kasselvania/Linux-VST-bridge/issues/132).
The stereo implementation and evidence in PR #131 stay frozen at
`1bda0e5c695fbc84e0ae5ab42728bca6d049e4ab` for review. This investigation changes
only development observation and reporting; it does not select a renderer fix.

## What is already observed

Blackhole Immersive 1.4.4 is the exact managed experimental stereo candidate
`d7933fdaedb30958669caacb801934399adac3819e22c03c7aada805ca8e7d02`.
The [previous receipt](../evidence/blackhole-immersive-deck/stereo-runtime.sanitized.json)
binds the module, native proxy, Windows host, runner, environment and publication.
It separates these facts:

| Boundary | Observation | What remains unproved |
| --- | --- | --- |
| Editor presentation | White surface in two earlier Luna/Moonlight sessions; both new exact local frames are entirely white | Presenting API and failed rendering operation |
| Child geometry | One visible, enabled 874×552 child at 96 DPI; same extent as captured client area | Successful rendering within that child |
| Owner progress before close | Owner publication and GUI heartbeat advanced over two seconds | Paint delivery and renderer progress |
| Effect processing | Real stereo source produced signal and a late reverb tail in Bitwig | Audio quality, timing stress and project recall |
| Normal editor close | Owner blocked in `IPlugView::removed()` for at least the retained 257.823-second progress separation | Cause, and whether it shares the white-rendering defect |
| Explicit test retirement | Child cleanup and transport retirement confirmed | Successful ordinary editor retirement |

The live preflight again identifies KDE Wayland with KWin Wayland and XWayland.
A Wayland desktop does not identify a particular plug-in's display driver.
The actual admitted editor HWND/XID and its mapped Wine driver must identify
that boundary. The new live census identifies `winex11.drv`; this editor uses
the X11 route on the Wayland desktop. No global desktop switch, runtime change or DLL override follows
from the desktop session type.

## Prior tests that apply

- [UIO1](UIO1.md): exact profile/session/process/window binding, bounded Win32
  hooks, message and heartbeat observations, exact local drawable capture and
  graphics-module hints. Its Pigments result was a measured X11-to-Win32 input
  retrieval delay. No renderer fix was established by that result.
- [UIR1](UIR1.md): generated competing-message workload and bounded pump fairness
  repair, followed by real Pigments interaction and local page capture. Paint,
  timer and input categories remain distinct from a responsive heartbeat.
- [UIO2](UIO2.md): parent/child/popup graph with DPI, styles and rectangles;
  exact XWayland capture with occlusion and identity refusal. A generated child
  actually resized and repainted. Pigments' popup ownership and subsequent
  shutdown incident remained separate observations.
- [AP11 follow-up](AP11_REVIEW_FOLLOWUP.md): physical Wayland/XWayland focus and
  real editor control/audio evidence. Remote screenshots alone did not stand in
  for local rendering or complete control delivery.

These tools are reusable diagnostic infrastructure. The Blackhole defect is
product-specific until evidence establishes a shared owner. Existing iLok
activation and installer results do not establish editor compatibility.

## Selected diagnostic

The legacy observer admission only supports compiled verified profiles and
specific historical candidates. Blackhole's current managed experimental
publication needs read-only admission through the existing retained candidate,
physical completed publication, registration and artifact identities. A title,
class name, arbitrary path or hand-written legacy admission is insufficient.

Reuse the existing Windows observer binaries, the ordinary observation mode and
window graph, and the existing exact-X11 capture adapter. The bounded run has no
mouse/key input, activation, resizing, parameter changes or accessibility query.
Luna alone opens the exact device in the disposable stopped LVB-BH project.

Observe the local root surface, child geometry/visibility, DPI/styles,
paint/message progress, owner heartbeat and normalized graphics/display-module
presence. Revalidate the current native view, activation, generation and editor
epoch around capture. Preserve unavailable/occluded/foreign-window results;
never substitute a whole-screen image or background drawable. Raw frames and
process/window identifiers remain private; public evidence contains bounded
metrics, aliases and exact artifact identities.

No loaded DLL alone proves the presenting API. A frame hash proves byte identity,
not usable controls. A successful `attached()` call does not prove a drawable
child exists. The live result below determines the next diagnostic step.

## Live local result

Diagnostic source `6bb1480fd96072a46704d73001c63443ffbd66e5` ran once against
the unchanged experimental publication. The 30-second collector completed in
7.943 seconds, with two frames about two seconds apart. Luna alone inserted the
device and left its editor foreground, with transport stopped. There was no
timeline playback or additional audio test.

- Both locally captured client images are 874×552, with identical raw SHA-256
  `c50ed2d07666922857da04488dfc3054782a97d05f8297fff0dba6b60511795f`.
  Full decompression/readback counted all 482,448 pixels in each image as RGB
  `(255,255,255)`. This independently reproduces the white surface seen remotely.
- The root and its single child are visible, enabled and not minimized. Both
  report 96 DPI; the child client extent matches the captured area. Fresh
  identity, geometry, viewability and occlusion checks passed before and after
  each direct-drawable capture. A missing or zero-sized child is not the observed
  failure.
- Eleven heartbeat records had a maximum measured delay of 4.8035 ms. No
  WM_PAINT record appeared during this settled observation. Initial paint
  predates it; a retained surface or no invalid region can also produce zero
  paint records. Neither heartbeat continuity nor zero paint establishes the
  rendering call path.
- Linux mapped-module readback identifies `winex11.drv`, Direct2D, D3D11,
  DXGI, OpenGL and WineD3D. Exact mapped D3D11/DXGI and four other module bytes
  match the pinned runner's Wine built-ins. These are not identified as DXVK
  modules. The Windows module census was unavailable and remains explicitly
  incomplete. Module presence alone does not prove that Blackhole used an API
  successfully or presented a frame.
- The existing retained stderr contains no recognizable D2D/D3D/DXGI graphics
  failure. Graphics creation/presentation calls were not instrumented; absence
  of a logged failure is not success.

The initial idle engine counters already contained four gaps and 2,048
missing/expired frames; those counts did not increase during observation. This
is a preserved residual, not a clean timing result. The earlier real stereo
effect result remains separate.

The helper exited zero with no overflow, observer drops, scope/unhook errors or
owned descendants. The exact digest-verified test supervisor was then retired
through its existing cleanup path; child cleanup and transport retirement were
confirmed. Luna removed only the failed unsaved device, leaving the source track
and stopped project. Four protected files and prefix device/inode were unchanged.
Final capacity reported zero DSP, zero maintenance and no unconfirmed cleanup.
No normal-close success is claimed by explicit retirement.

See the [evidence index](../evidence/blackhole-editor/README.md) for source/build
custody, local metrics, mapped module identities and cleanup. Raw pixels,
window/process/session IDs and vendor log text remain private on the fixture.

## Next discriminant

Observe renderer initialization and frame submission for this exact candidate:
the actual API family, device/factory/render-target creation results, and a
bounded set of frame submission results correlated with local drawable changes.
For a DXGI path this includes swap-chain creation and `Present`; a different
path requires its actual equivalent rather than assuming DXGI from a loaded DLL.
Record numeric results and bounded call counts, without raw pointers, shaders,
vendor strings or account data.

| Observation | What it would localize |
| --- | --- |
| No relevant creation call | The loaded API may be incidental, or the renderer never starts |
| Creation returns failure | Graphics initialization and its exact failure result |
| Creation succeeds, no frame submission | Render-loop progress before presentation |
| Submission succeeds, local drawable remains white | Submitted content or the Wine/X11 presentation path; success alone cannot distinguish them |
| Local drawable changes but remote view remains white | Remote presentation, unlike the current all-white local result |

This is the next bounded investigation, not an implemented trace or a selected
fix. Do not repeat the same screenshot campaign or change backends without a
new discriminating observation.

## Repair leads, not established causes

The current parent omits `WS_CLIPCHILDREN` and `WS_CLIPSIBLINGS`, present in the
[pinned Steinberg editor-host example](https://github.com/steinbergmedia/vst3_public_sdk/blob/586dc5e6c8012c3e4b01c79389375cbe96bdb1da/samples/vst-hosting/editorhost/source/platform/win32/window.cpp).
Microsoft's [window-style reference](https://learn.microsoft.com/en-us/windows/win32/winmsg/window-styles)
describes their clipping behavior. Actual child geometry and local paint evidence
must determine whether this matters here. Content scale also currently derives
from work-area fitting rather than an independently measured process/window DPI
contract. Neither difference is a demonstrated explanation for Blackhole's white
surface, and no flag or DPI change is selected merely from this comparison.

Normal close must remain a separate repair/verification target. Hiding the editor
or deferring `removed()` could avoid this particular invocation without fixing
rendering or proving safe final retirement. A rendering-only improvement does not
close the lifecycle defect.
