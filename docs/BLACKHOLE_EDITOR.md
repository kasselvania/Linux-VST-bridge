# Blackhole editor rendering investigation

Tracking: [bug #132](https://github.com/kasselvania/Linux-VST-bridge/issues/132).
The stereo implementation and evidence in PR #131 stay frozen at
`1bda0e5c695fbc84e0ae5ab42728bca6d049e4ab` for review. This investigation changes
only development observation and reporting; it does not install a renderer fix.
The graphics-call continuation now identifies an actual missing runtime API:
the exact Wine DXGI implementation returns `E_NOTIMPL` when Blackhole requests
a composition swapchain. The provider renderer preference remains a repair lead.

## What is already observed

Blackhole Immersive 1.4.4 is the exact managed experimental stereo candidate
`d7933fdaedb30958669caacb801934399adac3819e22c03c7aada805ca8e7d02`.
The [previous receipt](../evidence/blackhole-immersive-deck/stereo-runtime.sanitized.json)
binds the module, native proxy, Windows host, runner, environment and publication.
It separates these facts:

| Boundary | Observation | What remains unproved |
| --- | --- | --- |
| Editor presentation | White surface in two earlier Luna/Moonlight sessions; both exact local frames are entirely white | Corrected pixels and usable controls |
| Graphics calls | Standalone exact editor reaches Direct2D/D3D11; composition swapchain creation returns E_NOTIMPL | A working alternate rendering path and Bitwig repair |
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

## Graphics-call continuation

The next observation used `tools/uio1/renderer_graphics.py` at `8bde6be` and the
existing installed supervisor's standalone `ap12-vendor-access` mode. Exact
managed admission revalidated the current candidate. The launcher coordinated
with the existing registry/operation locks and vendor-access lease, selected
Wine graphics debug channels only in its own process, and reused existing
process supervision and cleanup. Installed product/runner bytes and publication
were not replaced. There was no input, audio callback or licensing action.

The 30-second observation retained 4,104,682 private log bytes under a 16 MiB cap,
with zero discarded bytes. All enabled D2D/D3D11/DXGI records belong to one
Windows process. This separates real graphics calls from earlier mapped-module
hints:

| Operation | Observed records |
| --- | --- |
| D3D11 device creation | One `Created ID3D11Device` record |
| Direct2D device-context creation | 35 `Created device context` records |
| Direct2D `EndDraw` | 22 entry records; these are not returned HRESULTs |
| DXGI `CreateSwapChainForComposition` | Three `stub!` records |
| DXGI presentation functions | No records in the complete enabled-channel capture |
| DXGI `WaitForVBlank` | 33,691 `stub!` records |

Disassembling the exact pinned Wine `dxgi.dll`, SHA-256
`ef3fa2a3c199aa8f9cb97701778864ac70a76f2092cff821d645b8a21ba9a67a`,
confirms **every return path** of both stub functions returns `0x80004001`
(`E_NOTIMPL`, signed -2147467263). This conclusion is based on the actual tested
binary, not an assumed correspondence with the current Wine source branch.
Device/context construction is reached; the composition swapchain required by
this rendering route cannot be constructed. The VBlank loop is another measured
missing capability, not proof that adding a sleep would fix presentation.

The editor-open lifecycle record was present. Graceful editor-close completion
was absent; owned termination, child cleanup and transport retirement completed.
The lease was removed only after positive cleanup. Four protected files and
prefix device/inode remained unchanged, and final capacity had zero DSP, zero
maintenance and no unconfirmed cleanup.

This was a standalone editor diagnostic, not a new Bitwig/pixel acceptance run.
Luna could not read the dimmed Moonlight stream and sent no plug-in input.
Consequently, this receipt establishes the exact graphics-call failure; the
earlier local all-white images remain the visual evidence. The first launcher
attempt refused before opening any plug-in because it incorrectly required the
admitted accessibility capability to be false. The actual exact profile permits
it. That expectation was corrected, without invoking any accessibility query;
the preliminary failure is retained as a diagnostic error, not a plug-in failure.

## Renderer-preference hypothesis and remaining route

Blackhole's small vendor preferences file contains `GRMd="0|0"`. The exact
module also contains `Direct2D` and `Software Renderer` labels. This is a useful
lead, but neither the key encoding nor an exposed working selection has been
verified. The official [Blackhole Immersive guide](https://downloads.eventide.com/audio/manuals/plug-ins/Blackhole+Immersive+User+Guide.pdf)
checked here does not document that encoding. At that stage no guessed value had
been written. Within the user's approved repair continuation, the custodian later
selected one bounded reversible experiment that changed only `GRMd` from `0|0`
to `1|0`, without claiming the mapping was known. The candidate value persisted
during the exact stopped-Bitwig run, but both 874×552 local captures remained all
white and matched the baseline frame digest. The hypothesis therefore failed to
produce a usable renderer. It does not establish that software rendering was
selected, unsupported or defective.

The original 140-byte preference was restored byte-for-byte with its mode. No
close/reopen check followed because no rendered surface was observed. The owned
supervisor ended with raw exit -15; exact cleanup and transport retirement
passed, as did the four protected files, prefix identity and final capacity
readback. Luna removed the failed device, leaving the empty Blackhole input
chain in the stopped, unsaved project. Her single context click showed no menu.
The bounded accessibility helper returned exit 3 with no records;
because that result conflates `ElementFromHandle` and `get_RawViewWalker`
failures, it does not establish that the vendor control is absent. A proposed
process-memory read was rejected at approval because its process binding was
insufficient for that invasive operation; it was not retried and no ptrace or
security setting changed. See the sanitized
[software-renderer trial](../evidence/blackhole-editor/software-renderer-trial.sanitized.json).

This preference route now needs a verified vendor control or exact encoding
before another selection. A complete runtime correction for the missing
composition presentation path remains a separate, substantially larger route.

There is no demonstrated one-DLL replacement fix. JUCE's
[Wine fallback change](https://github.com/juce-framework/JUCE/commit/5179690)
and [follow-up explanation](https://github.com/juce-framework/JUCE/pull/1701)
address both the peer renderer and native image backing. That source change
cannot be assumed present in this proprietary binary. DXVK's
[dummy composition path](https://github.com/doitsujin/dxvk/blob/master/src/dxgi/dxgi_factory.cpp)
and [configuration caveat](https://github.com/doitsujin/dxvk/blob/master/dxvk.conf)
do not establish functional DirectComposition presentation. The Wine developer's
[DXGI/DComp implementation discussion](https://list.winehq.org/hyperkitty/list/wine-devel%40list.winehq.org/message/OMIIVWXQAWX7HEEEFIW4U4PCBGY3CXWT/)
describes the additional composition/driver work. A runner replacement would be
a separate, substantially larger correction route, not a justified toggle here.

The shared observation tools and this exact-product graphics launcher have
different applicability. The latter diagnoses the current Blackhole profile;
this does not establish a shared iLok defect or general plug-in compatibility.

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
