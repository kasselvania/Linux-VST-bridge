# Blackhole editor observation

Claim: the exact managed experimental Blackhole Immersive 1.4.4 editor has an
all-white local client surface while its visible child is correctly nonzero in
size and its owner/heartbeat continues. This is an editor-rendering failure,
separate from the previously observed normal-close block and stereo processing.
The graphics continuation below identifies composition-swapchain creation as
an actual missing runtime operation. A working correction remains unverified.

Product/review base is `1bda0e5c695fbc84e0ae5ab42728bca6d049e4ab`; diagnostic
code is `6bb1480fd96072a46704d73001c63443ffbd66e5`. No installed product, runner,
prefix, compatibility setting or publication was replaced. Exact product,
candidate, host and native identities are inherited from the
[stereo runtime receipt](../blackhole-immersive-deck/stereo-runtime.sanitized.json)
and reverified by managed-observation admission. The admission fingerprint in
`result.sanitized.json` is the current exact profile, not a new support claim.

| Receipt | Evidence |
| --- | --- |
| `preflight.sanitized.json` | Installed executable/proxy/project digests and initial capacity |
| `diagnostic-build.sanitized.json` | Exact diagnostic commit, ELF CLI digest, deployed script closure and focused check results |
| `observer-custody.sanitized.json` | Existing pinned Windows observer source/build, artifact archive and helper digests; no new Windows build |
| `result.sanitized.json` | Two local frame metrics, parent/child geometry, message counts, owner progress, helper cleanup and retained counter residuals |
| `graphics-module-identities.sanitized.json` | Six exact live mapped modules match the pinned runner's Wine built-ins; no presenting-API inference |
| `stderr-categories.sanitized.json` | Existing retained stderr inspected for recognizable graphics-channel failures, without publishing vendor text |
| `cleanup.sanitized.json` | Full-pixel white validation for both images and exact owned test-host cleanup |
| `preservation.sanitized.json` | Protected files/prefix identity, Luna's device removal, zero remaining DSP/maintenance and no unconfirmed cleanup |
| `graphics-preliminary.sanitized.json` | Launcher preflight error before any plug-in launch; corrected exact capability expectation |
| `graphics-trace.sanitized.json` | Actual D2D/D3D11/DXGI calls in the exact standalone editor, bounded private trace hash and no truncation |
| `graphics-stub-returns.sanitized.json` | Exact Wine DLL disassembly establishes E_NOTIMPL on every return path of the composition and VBlank stubs |
| `graphics-preservation.sanitized.json` | Separate standalone graphics run cleanup and protected-state readback |
| `graphics-preference-lead.sanitized.json` | Unmodified vendor graphics-preference lead; encoding and working software fallback unverified |

## Method and limits

Luna 5.6 Max alone opened the exact Eventide x64 VST3 from Bitwig's File Kind →
Plug-ins browser in the existing disposable stopped project. A separately staged
Linux diagnostic CLI admitted the current retained managed experimental
publication. `tools/uio1/renderer_observe.py` then reused the pinned UIO1 helper,
UIO2 window graph and existing local X11 capture adapter for one bounded run.
The collector completed in 7.943 seconds under a 30-second observation limit and
50-second external service limit. The two captures were about two seconds apart.
They used the existing direct-drawable fallback, with before/after exact identity,
geometry, foreground-stack and occlusion checks; no desktop fallback was used.

Invocation shape, once the exact editor is already open and foreground:

```sh
python3 -B tools/uio1/renderer_observe.py DIAGNOSTIC_CLI CLASS_ID PINNED_HELPER_DIR OBSERVATION_ID
```

This development collector sends no input, activation, resize, parameter or
audio request. It refuses stale/foreign scope and incomplete capture. It does
not install a manager UI action, automatically detect all visual failures, or
grant a candidate activation/compatibility authority. A blank but live editor
need not generate a process-crash report.

The private raw report and compressed frame hashes are retained in the sanitized
result. Raw frames, window/process/session identities, paths and vendor stderr
remain private on the Deck. Sampled color metrics were independently checked by
decompressing each whole frame and counting all 482,448 RGB pixels; both were
entirely white. Initial paint occurred before observation. No WM_PAINT record
during the settled interval cannot establish failed paint delivery.

The Windows module census was unavailable. Linux maps and exact mapped-file
digest comparison provide the module findings. Device creation, drawing and
presentation calls remain unobserved. Four gaps and 2,048 missing/expired frames
predated the observation and remained unchanged; no timing acceptance follows.

The collector's helper detached and exited normally. The custodian retired only
the exact digest-verified test supervisor through its existing child cleanup,
then Luna removed the failed unsaved device without saving. This deliberately
does not repeat or qualify the independently known `IPlugView::removed()` hang.

Focused validation: the managed-publication admission regression checks changed
retained inspection/module/native/host/source bytes, missing physical publication,
foreign class and ordinary-admission rejection. Sixteen focused Python checks,
targeted Rust formatting, focused Clippy with warnings denied, and the Linux
release diagnostic build passed. These validate the diagnostic adaptation;
the live capture provides the product failure evidence.

Analysis and next discriminant: [BLACKHOLE_EDITOR.md](../../docs/BLACKHOLE_EDITOR.md).
Tracking: [bug #132](https://github.com/kasselvania/Linux-VST-bridge/issues/132).

## Graphics-call continuation

Source `8bde6be37e67bb90f9f77481f7d2d6f9d4f6b46e`, script SHA-256
`c741d67ef3bee0b56e9926371cf94fbf15e33b1abc09e6801e4bb1eaa08530d7`,
adds an exact-Blackhole development launcher without rebuilding or installing
product software. It uses the same admitted class/module/host/environment in
standalone vendor-access mode, holding the manager admission lock and a normal
vendor-access lease. Its process-local Wine diagnostics tee the exact root
stderr pipe already drained by the installed supervisor. No ambient debug
setting, DLL override or prefix registry change is used.

```sh
python3 -B tools/uio1/renderer_graphics.py DIAGNOSTIC_CLI OBSERVATION_ID
```

The observation lasts 30 seconds, followed by the existing stop/cleanup path;
the external service limit was 80 seconds plus a 15-second stop bound. Capture
is private and capped at 16 MiB. The actual run retained 4,104,682 bytes with no
discard, reached editor-open, and confirmed owned cleanup and transport
retirement. Graceful editor-close did not complete. Private trace/disassembly
and process/session identities remain on the Deck. The script retains a blocking
lease if cleanup is uncertain.

The selected channels expose device/context creation and EndDraw activity, three
composition-swapchain stub calls, no DXGI Present record, and 33,691 VBlank stub
calls. Exact DLL disassembly, rather than a latest-source inference, supplies
the constant failing HRESULT. EndDraw entries are not treated as successful
returns. No new visual or Bitwig acceptance follows from this standalone run;
Luna's Moonlight view was too dim to interpret.

The initial script `fcef9ab` refused before launch because it hardcoded the wrong
accessibility capability. `8bde6be` matches the current exact profile's true
value and still performs no accessibility operation. Eighteen focused Python
checks passed for the launcher; the two affected graphics checks passed after
the correction. No additional broad test matrix or Windows build was run.
