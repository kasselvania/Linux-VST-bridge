# Blackhole experimental DirectComposition renderer

Next reliability and routing work:
[touchscreen failures, crash reporting, session recovery/reset and full audio I/O](PLUGIN_RELIABILITY_FOLLOWUP.md).

Tracking: [bug #132](https://github.com/kasselvania/Linux-VST-bridge/issues/132).
The stereo implementation and evidence in PR #131 stay frozen at
`1bda0e5c695fbc84e0ae5ab42728bca6d049e4ab` for review. A separately identified
DirectComposition runtime reference is now installed and experimentally
published for Blackhole in Bitwig. The actual editor renders, responds to Bypass,
and closes normally while its processing instance remains loaded. The operator
personally tested audio and reported it working, then requested that testing
stop and this work be committed and submitted for review.
The baseline graphics trace identified an actual missing runtime API:
the original Wine DXGI implementation returns `E_NOTIMPL` when Blackhole requests
a composition swapchain. The working reference supplies that implementation;
the unknown provider preference encoding is not being guessed again.

## Delivered Bitwig result and stopping point

The [managed Bitwig receipt](../evidence/blackhole-editor/direct-composition-bitwig.sanitized.json)
records implementation `65347a2db8fe2b8b681cf16e31dd121cd159dea3`, the delivered
manager, the new runner/profile/publication, and observations retained before
the final SSH disconnect. The exact installed module, Windows host and native
proxy bytes remain unchanged. The existing prefix root and Windows profile
convention were preserved; no activation material was exported or new licensing
flow performed. The original runtime and private rollback material remain
available. The working experimental candidate is intentionally left installed.

The manager now admits one closed runtime policy,
`dcomp_wine_builtins_reference_v1`, through an inactive-only, exact-artifact
runner transition. The normal supervisor applies matching Wine graphics
built-ins and Proton's copy provisioning. Rescan, stereo inspection, preparation
and experimental publication produced new identities instead of relabeling the
old candidate. Existing runners omit the optional policy and retain their
serialized identity. Authority-file updates have coordinated retained rollback;
they are not a crash-atomic multi-file transaction.

Two exact local 874×552 Bitwig editor captures have SHA-256
`7873aeb8a4e5ff77f26d804144d91b05f7fe43f92341acb7341a295adc39934f`,
matching the successful standalone frame. Luna observed readable controls and
toggled Bypass on and back off. The mapped graphics bytes match the candidate.
Normal window closure returned through view stage 217, after `removed()`, view
release and window destruction, with editor-open and failure both zero. The
same supervised processing instance remained loaded without a terminal result.
The old stage-212 close block did not recur in this observation.

Audio success is **operator-confirmed**, not a completed automated recording
assertion. A bounded Bitwig-output recorder had been started, but its result was
not read before the operator ended testing. Two final SSH attempts timed out;
the recorder had a configured 110-second service lifetime, but final retirement
was not independently read back. No further GUI action was authorized after
the stopping point, and the working DAW instance was left for the operator.

Same-instance editor reopening remains unverified: the correct Bitwig reopen
action was not established, so the navigation attempts do not demonstrate a
runtime reopen failure. Additional VST audio channels, saved-project recall,
performance/latency qualification, broader regression and clean retirement of
this Bitwig instance are deferred. Earlier counters included gaps and missing
frames; no zero-dropout claim follows. This is an experimental exact-fixture
result, not general Blackhole, iLok, Wine-fork or Proton qualification.

Focused implementation validation passed four transition/rollback checks, two
profile checks, four session checks, 21 UIO1 checks, all-target Clippy and Linux
release builds. Final documentation/evidence checks reuse those results; no
additional live test is required by this stopping point.

## Post-test service recovery and requested follow-up

After the operator reported switching to Gaming Mode and returning to Desktop,
the manager continued to block new instances. Live records showed four exited
environment keepers, each with confirmed process cleanup. A later Serum 2 launch
exposed its native binding and lease before checking the dead keeper. That check
failed before the Windows supervisor was spawned, leaving a reservation and
transport without a retirement receipt and latching the global admission block.
Why the keepers exited remains unknown; Gaming Mode is an operator-reported
possible trigger, not an established cause.

The previous tested Blackhole session independently had confirmed process and
transport cleanup. The bounded audio recorder was also inactive with no unit
cgroup at this follow-up; its audio result was not analyzed. These later facts
do not rewrite the original finalization-time SSH limitation.

With Bitwig and Wine absent and the bridge cgroup containing only its daemon,
the custodian stopped the service, held its service/canonical/registry locks,
verified the exact failed-launch owner and transport identity, and quarantined
that one lease and its six temporary transport files while preserving them
privately. No successful plug-in lifecycle receipt was fabricated. Restart used
normal reconciliation for the four already-confirmed keeper records. The first
capacity read preceded readiness; the subsequent read verified an active
service, no cleanup block, zero owners/leases, and six available global slots.
No plug-in, installation, runtime, registry or project was changed or tested.
See the [recovery receipt](../evidence/blackhole-editor/abandoned-launch-recovery.sanitized.json).

The launch-order/recovery defect still needs a code fix. The operator also
requested a future panic control: clearly warn that audio will be interrupted,
stop the bridge's owned instances, verify cleanup, retain failure details, and
restart the service and manager. Clearing a warning alone is insufficient;
reopening DAW projects or plug-ins is separate from restarting the bridge. That
control is deferred, not implemented by this manual recovery.

## Working standalone reference

The [candidate editor receipt](../evidence/blackhole-editor/direct-composition-editor.sanitized.json)
establishes the real Blackhole Immersive 1.4.4 editor drawing under the coherent
DirectComposition candidate. Luna observed readable Eventide controls and the
equalizer. Two exact local 874×552 captures bind the same owned process/window
to the candidate's mapped D2D, D3D11, DXGI, DirectComposition, WineD3D and X11
files. Toggling Bypass visibly changed its highlight and changed 1,830 local RGB
pixels. Returning Bypass to off and closing the window produced normal editor
closure, module exit/unload and completed host records before the deadline.
A subsequent clean reopening reproduced the original rendered frame exactly
and closed normally again.

The retained trace observes a composition backing window, target creation,
visual content/root connection, a commit and `Present1` entries. The 16 MiB
trace limit was reached; counts are from the retained portion, not complete
lifecycle totals. Actual displayed controls and their response establish the
visible result; intermediate API entries alone do not.

The first rendered attempt reached its 180-second deadline while the foreground
window was being located. It receives no ordinary-close credit. The two later
operator-closed runs completed the SDK lifecycle normally. All three retain an
outer launcher exit of `-15`: the supervisor cleans remaining owned processes
after the SDK host's completion. This is separate from editor-close success.

This standalone receipt has no DAW audio and cannot establish same-instance
editor reopening, candidate Bitwig processing or saved-project recall. The
managed Bitwig result above is separate evidence; the previous measured stereo
proof remains evidence for its original runtime.

### Runtime construction lesson

The [build receipt](../evidence/blackhole-editor/direct-composition-reference-build.sanitized.json)
records the coherent Wine fork, pinned SDK, inherited Proton distribution and
bootstrap dependencies. Preserve Proton's `steamuser` API behavior and supply
the fork's expected Mono version; a compiled Wine tree alone is not a complete
compatible Proton distribution.

The first live initialization also exposed a concrete integration difference.
The selected Proton Wine protects distribution DLLs against write-permission
changes through prefix symlinks in `dlls/ntdll/unix/file.c`; this fork lacks that
protection. Wine's update removed 56 candidate distribution targets through
those links. The editor launcher refused the damaged artifact before launching
Blackhole. The original runner remained available and the environment was
restored from its verified private archive without changing the prefix root.
This failed attempt is retained in the
[transition receipt](../evidence/blackhole-editor/direct-composition-transition.sanitized.json).

Use Proton's existing `PROTON_DLL_COPY=*` for this reference. An old-runner to
candidate scratch transition and the corrected live transition both preserved
the complete sealed candidate tree. Validate exact DLL bytes whether provisioning
creates a copy or a confined symlink; do not require a particular file form
without checking Proton's actual behavior. The remaining explicit ICU/font
resource links are distinct from Wine builtin DLL links.

These are candidate-specific construction requirements, not reasons to modify
every working environment or change all `runinprefix` calls. A managed runner
transition must update the declared runtime identity and obtain fresh profile/
publication identities before its Bitwig results can be attributed to that runner.

## Effective backend audit and runtime continuation

The [read-only backend audit](../evidence/blackhole-editor/graphics-selection-audit.sanitized.json)
explains why the retained run used Wine's graphics even though the prefix contains
DXVK. Both Windows system directories contain `d3d11.dll`, `dxgi.dll` and
`d3d10core.dll` byte-identical to the selected runner's DXVK distribution. There
are no relevant global or application graphics overrides in `user.reg`,
`userdef.reg` or `system.reg`. The installed supervisor's recomputed launch
environment supplies no graphics override, and the runner has no `user_settings.py`.
This was a baseline configuration audit before the candidate transition, not a
new observation of a running editor's environment; the earlier mapped-file
receipt establishes what actually loaded in that baseline.

The installed Proton script matches revision
`5b89db940e0ebe3a137a6009a3589232fe084c09`. Its `runinprefix` path initializes the
session but skips `setup_prefix()`, including the native DXVK overrides normally
added there. At the exact associated Wine revision
`dc26e61847081a1b5cb0733dc30feba6ee575482`, an otherwise unspecified DLL uses
`LO_DEFAULT`: look for a built-in, allowing native preference only when the
built-in carries Wine's `0x0010` preference flag. The actual D3D11/DXGI built-ins
have characteristics `0x0160` (64-bit) and `0x0140` (32-bit), without that flag.
This selection chain explains the observed built-ins without assuming an explicit
`PROTON_USE_WINED3D=1`. Changing every `runinprefix` call is not warranted.

Sources: pinned [Proton launcher](https://github.com/ValveSoftware/Proton/blob/5b89db940e0ebe3a137a6009a3589232fe084c09/proton),
[Wine load-order selection](https://github.com/ValveSoftware/wine/blob/dc26e61847081a1b5cb0733dc30feba6ee575482/dlls/ntdll/unix/loadorder.c),
and [Wine loader](https://github.com/ValveSoftware/wine/blob/dc26e61847081a1b5cb0733dc30feba6ee575482/dlls/ntdll/unix/loader.c).

The separately built reference uses `giang17/wine` revision
`c27f058814b402a5709e073adccd42baa66810b9`, with its coherent Wine graphics stack
and declared removal of Wine-hiding defaults. Its purpose is to test the real
Blackhole editor before selecting a maintained Proton graphics change. Build
success, DLL loading, a changed frame hash, and successful swapchain creation are
not editor acceptance. Acceptance requires recognizable controls and text,
responsive interaction, normal close/reopen, continued stereo processing and
clean instance retirement. The results above establish a narrower stopping
point: rendering, interaction and normal close, plus operator-confirmed audio.
Standalone close/reopen passed; same-instance Bitwig reopen, recall and full
Bitwig instance retirement remain unverified. Keep these scopes separate.

## Original-runtime observations

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
composition presentation path is the selected runtime continuation above.

There is no demonstrated one-DLL replacement fix. JUCE's
[Wine fallback change](https://github.com/juce-framework/JUCE/commit/5179690)
and [follow-up explanation](https://github.com/juce-framework/JUCE/pull/1701)
address both the peer renderer and native image backing. That source change
cannot be assumed present in this proprietary binary. DXVK's
[dummy composition path](https://github.com/doitsujin/dxvk/blob/master/src/dxgi/dxgi_factory.cpp)
and [configuration caveat](https://github.com/doitsujin/dxvk/blob/master/dxvk.conf)
do not establish functional DirectComposition presentation. The Wine developer's
[DXGI/DComp implementation discussion](https://list.winehq.org/hyperkitty/list/wine-devel%40list.winehq.org/message/OMIIVWXQAWX7HEEEFIW4U4PCBGY3CXWT/)
describes the additional composition/driver work. The selected coherent candidate
implements that surrounding machinery; replacing one DLL or toggling an unknown
preference would not establish it.

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
