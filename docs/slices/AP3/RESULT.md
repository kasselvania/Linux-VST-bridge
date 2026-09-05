# AP3 — Fresh sustained audio and Bitwig verification

**Fresh acceptance verified; PR #56 awaits review and remains unmerged. AP2 remains accepted.**

The SDK-loaded queued VST3 proxy returned **6,366,144 independently checked float32 samples, maximum absolute error 0.0**. The first 5,764,096 samples cover two 30-second streams at 128 and 256 frames/48 kHz, including startup zeros and the drained tail at the declared **1024-sample / 21.333 ms** additional latency. One Windows AGain instance, mapping and control connection survived the primary stop/restart and gain persistence checks. Input seeds were chosen by the Linux host after activation. Actual Windows output was checked sample by sample and independently reconstructed by the result consumer.

| Measurement | 128-frame primary | 256-frame primary | 128-frame Moonlight-active |
|---|---:|---:|---:|
| Checked samples | 2,882,048 | 2,882,048 | 602,048 |
| Callback median | 5.050 µs | 7.810 µs | 5.930 µs |
| Callback p99 | 15.740 µs | 13.260 µs | 18.670 µs |
| Callback maximum | 221.634 µs | 47.131 µs | 162.613 µs |
| Callback period | 2,666.667 µs | 5,333.333 µs | 2,666.667 µs |
| Overruns / unexpected queue faults / forbidden callback effects | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 |

Primary request/result queue high-water marks were 3/8 of 2048 descriptors. The primary streams covered 3,433 zero-gain blocks with non-silent inputs, 1,224 one-channel-silent blocks, 2,412 in-place blocks and 888 zero-frame parameter flushes. The short stream-active measurement is separate: video/input streaming is machine load, never the numerical or timing oracle.

## Controlled Bitwig result

The same native and Windows artifacts ran through **Bitwig Studio 6.1**, system Flatpak commit `8a048e733e74dda8b897339436153a2d5df952f29d362dfcaca9d8e0d6f6c231`, Freedesktop Platform 25.08 commit `bd44a6230581917d04f89812a4c21090c304d390edb73995af1c2f9fd8abf4e8`. The existing PipeWire System Out route used 48 kHz and 256 frames. Existing filesystem/network/IPC permissions were recorded privately; only the current owned session directory was added per launch, with commercial plug-in directories hidden per invocation. No persistent Flatpak override was added.

Actual Mac Computer Use operated Bitwig through Moonlight: searched and loaded AGain Queued Preview on the disposable audio track, played and looped the modest stereo test clip, lowered gain, set gain to 0.0000, observed input activity with silent output, restored gain, stopped playback, removed the effect and quit without saving test edits. A fresh app/bridge reopen repeated these actions. Native termination records corroborate the GUI observations:

| Session | Frames / blocks processed | Zero-gain blocks | Gain range | Queue faults / callback rejections | App exit / native shutdown |
|---|---:|---:|---|---|---|
| First | 3,453,184 / 13,489 | 5,783 | 0–1 | 0 / 0 | 0 / clean |
| Reopen | 2,866,432 / 11,197 | 3,537 | 0–1 | 0 / 0 | 0 / clean |

Each session retained one live Windows instance/mapping/connection and one processing epoch, with request/result high-water 1/4. These are host integration, control and lifecycle observations; Bitwig meters and streaming are not sample-comparison evidence. This is a measured reference-effect preview at added latency, not arbitrary scheduling, production DAW state, vendor editor, commercial compatibility or universal realtime support.

## Implementation and focused verification

Rust owns fixed-capacity SPSC queues, epoch/position/sequence correlation and the sole transport worker. The C++ SDK edge snapshots inputs and parameter metadata, returns completed delayed Windows output, reports latency and exposes the small bridge-owned gain controller. Windows uses actual AGain in the selected realtime mode. Missing/stale/invalid work latches a fault, clears validated output spans and fails explicitly; it never replays or substitutes local gain processing. AP2's offline route remains separate.

The loaded native proxy tests covered variable lengths, in-place/silence, parameter persistence/flush, stop/restart/reopen and the actual failure boundary: delayed/dead/stalled peers, underflow, overflow, stale position/sequence, invalid flags, false silence and nonfinite output. Test-only callback instrumentation checked allocation/deallocation, waiting, filesystem/network/logging and process effects, with positive controls. The existing AP2 loader cases also passed against the retained artifact. Rust ownership/codec tests, the closed Windows call audit and the retained MSVC A/B producer passed. Focused worker/adapter/runtime/policy/backend regressions and PR CI passed before publication.

Routine harness repairs retain the original error and final combined process containment before retirement, preserve bounded nested segment facts, give the GUI app a separate 60-second exit window, and bound the combined result envelope at 512 KiB while preserving each observation/admission's 256 KiB limit. A local regression reproduces an OBSERVED transaction interrupted by the old envelope limit, then closes and re-admits that same reservation with one invocation total. No new execution framework was introduced.

## Exact evidence and cleanup

- Executed source: `5823ec68a719194e281b9f6f3b6d0853e2baf3b0`.
- Candidate: `074da5c26cefdfa8118a7e22ccf38a7a75e490ebaabe8b9b13149291235fd7ec`.
- Reservation: `0ea53ba4623b93379eb08417f383f433c5efbf046194e6b5b317e176ea5fc148`.
- Result: `ce9c8f735801c7c427260d8b5fbd6aaab83c9fbbb00d49ca83833356a7c9dbfd`.
- Windows artifact: `9974517171`; native manifest: `609e5e6ce3ec9334fd8506ac19594f4546de828167cc092f973958035a02319d`.
- Exact artifact, retained AGain, Proton 11.0-2c/Runtime 4 and delivered-source bindings: [acceptance packet](../../../evidence/ap3-sustained-native-audio/TRANSACTION.json), [authority](../../campaigns/AP3_A1.md).

All four owned Windows sessions stopped and joined processing before deactivation/release/unload; mappings closed, caller and Windows groups emptied, and the disposable stage became absent. Protected state stayed unchanged at `307e53f193155ee471e5a7d8ae5ab106ddc9ea253e0a42d203f5429b66eef305`.

After acceptance, the exact original Bitwig preferences were restored (SHA-256 `b5d4f90df6acc62c4c879c94b385b4ab1a77cb647f8ff3f4696b9b23e7ae12d2`) and only the verified temporary bridge publication was removed. Private before/after preference copies and the disposable project/audio remain available. No existing plug-in installation or music project was overwritten. A final process/protected-state readback passed. Sunshine remains a separate development service and SSH remains available; its setup and reversal are documented in [Deck desktop access](../../DECK_REMOTE_DESKTOP.md).

## Cumulative allowance and preserved history

Consumed / remaining: **Windows producers 1/6 (5 left); diagnostic reservations 5/10 (5 left); acceptance candidates 1/2 (1 left only after a concrete tested repair)**. A1 consumed its one authorized batch of four Windows instances. No blind retry, fixture seed, unchanged artifact redownload or count reset occurred.

D1 retained the first primary stream but failed during the second; its missing original fault detail stays unknown. D2 completed both primary streams. D3 completed core/stream-active checks but timed out before GUI insertion; its final containment flag was lost by a batch wrapper, and a separate verified cleanup retired its owned stage without rewriting the historical failure. D4 completed the audio and both Bitwig processing sessions, but rejected the second app's exit after the old 30-second wait. D5 passed all segments and cleanup; its Mac envelope-limit failure was repaired through the existing locked backend with launch disabled, preserving source, authority and the same consumed reservation. D5 result `139301bbcd51a1383cae71eb03485e648e59fbec0b671bd0fee847d894b549a2` remains acceptance-ineligible. Only fresh A1 supports this review candidate.

## Separate desktop outcome

Video, injected keyboard/mouse, and **actual Codex-through-Moonlight control: all verified**, including unique scratch-editor text, clicks, disconnect/reconnect and the above Bitwig operation. Official user Flatpak Sunshine and existing Mac Moonlight were used; the operator handled normal certificate, credential, pairing and portal approvals. No streamed terminal was operated, no immutable-base or broad firewall change was made, and no audio route was replaced. Issue #55's operational outcome is demonstrated independently of #54's audio result. PR #56 may close both on review/merge; it is left unmerged.
