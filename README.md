# Linux Audio Compatibility Bridge

A managed bridge for using supported Windows audio plug-ins in native Linux DAWs without making musicians administer Wine prefixes, proxies or runtime versions.

Experimental engineering preview, not a consumer-ready release. Working repository name; not affiliated with Bitwig, Valve, Steinberg, Arturia, Xfer Records or another vendor.

## Accepted baseline: AP13 installed Arturia workflow and delivery repair

AP8–AP11 established the independent Linux VST3 proxy, actual Windows processing under a selected runner, opaque state/recall, audio and note input, bounded returned events/parameter feedback, transient-gap continuation and same-instance detached editors. AP12 [#79](https://github.com/kasselvania/Linux-VST-bridge/pull/79) added persistent managed publication and automatic startup for the installed pair:

```text
Bitwig notes → Arturia Pure LoFi → Arturia Efx FRAGMENTS → output
```

AP13 [#82](https://github.com/kasselvania/Linux-VST-bridge/pull/82), reviewed at `5620c56660651c0624deb3935c91c91eb0653aa6` and integrated by `ca0e2f7d5c85515c8ec22b67d434c6288ca611f7`, repairs a demonstrated active-save delivery barrier. Protocol minor 12 lets ordered mailbox audio continue while the Windows UI owner performs a bounded read-only state capture; restore and lifecycle operations remain exclusive. Deterministic Windows/native regressions prove later real processing can progress while state capture is unfinished, including refusal and prior-snapshot behavior.

Routine supervisor ownership tracking now follows validated PID/start-time identities without repeatedly scanning every system process. In matched 45-second host512/bridge512 runs, the combined owner-process CPU fell from 63.136% to 12.014% of one core—about 81%—while both baseline and repaired intervals already had zero gaps. This is a supervisor-cost result, not an 81% DSP or dropout improvement.

The installed inactive-only delay selector is real and preserves `delay >= negotiated host maximum`. **512 added frames per proxy remains installed, supported and recommended.** At an actual 256-frame host setup, the opt-in 256 setting reported the correct 15.7 ms serial-chain latency and passed bounded musical/live-note intervals, but startup and later FRAGMENTS gaps prevent qualification or recommendation.

The AP12 workflow evidence still covers both real editors, recorded/replayed automation, edited-state saving, actual Deck reboot and recall without setup tooling, and independent effect removal. See [AP13 results](docs/AP13.md), [AP13 evidence](evidence/AP13/README.md), [AP12 results](docs/AP12.md) and the [current design dossier](docs/DESIGN_DOSSIER.md). Earlier evidence remains unchanged.

## Current operating limits

At the supported 48 kHz setting, Pure LoFi reports 512 bridge + 48 vendor frames and Efx FRAGMENTS reports 512 + 192. The serial chain is **1,264 frames / 26.333 ms**, including 1,024 bridge frames / 21.333 ms. Device/DAW latency is additional; sink-monitor captures are not physical loopback.

Short gaps and 24–31 ms native preparation/publication outliers remain. Their elapsed time is not yet classified as CPU execution, allocator blocking or scheduler delay. The old AP12 untraced timeout is not retrospectively explained. The selected Arturia hosts disable Windows UI Automation/screen-reader integration through an exact process-scoped compatibility choice; musical VST automation remains active. This is not an upstream Wine repair.

Float32 and bounded stereo/main-bus/event paths are implemented. Arbitrary multichannel/dynamic routing, sidechains, every SDK interface, float64, broad MIDI/MPE, native Wayland editor hosting and general customer-hardware reliability are not claimed. Operator-supplied modules, original installer outputs, entitlement and authorization posture remain separate.

## AP14 managed compatibility publication

[AP14 / #83](https://github.com/kasselvania/Linux-VST-bridge/issues/83) is implemented for the exact installed Arturia fixture on `codex/ap14-profile-publication`, pending independent review in PR #85. It provides the first closed declarative compatibility profiles and one coherent manager workflow that derives exact registrations and immutable publications from supervised local observations. The normal operation does not require class IDs, module hashes, compatibility flags, native build paths or Wine commands.

AP14 also adds an explicit revisioned publication update/reconcile/rollback transaction. A failed or incompatible candidate leaves the prior known-good publication active; rollback selects an exact retained revision without reinstalling or modifying vendor software. Read [CURRENT_SLICE.md](CURRENT_SLICE.md) on the prepared branch for the active contract.

This remains a narrow Arturia productization cut. It does not add another vendor, promote 256 frames, redesign the editor, migrate runtimes or build a consumer GUI.

## Follow-through

[#72](https://github.com/kasselvania/Linux-VST-bridge/issues/72) retains remaining delivery/performance classification. [#80](https://github.com/kasselvania/Linux-VST-bridge/issues/80) retains FRAGMENTS Advanced-panel expansion and redraw. [#84](https://github.com/kasselvania/Linux-VST-bridge/issues/84) records the later direct editor lifecycle: opening the DAW editor should show/focus the vendor window without the visible native open/close panel, and closing the vendor window should retire only the editor session. Serum vendor-access qualification is #77 and the historical native engine-close issue is #73.

## Start here

Read [AGENTS.md](AGENTS.md), [CURRENT_SLICE.md](CURRENT_SLICE.md), the relevant production code and the [design dossier](docs/DESIGN_DOSSIER.md). Continue on the prepared AP14 branch and implementation PR. Use ordinary focused engineering and one reviewed PR; preserve the operator's project, installed environment, vendor files, credentials and useful failed evidence.

`native-vst3-proxy/` is the Linux SDK proxy and Rust backend; `native-audio-client/` contains transport; `windows-factory-probe/` is the Windows SDK host; `bridge-manager/` owns environments, registration/publication and installed supervision. Rust is primary, with C++ at SDK/platform edges. Proprietary binaries, presets, license data and credentials stay out of version control.

The independent bridge remains selected. Maintained Wine/Proton work may provide a suitable runtime underneath it; it is not a yabridge pivot. Full manager UI, general vendor updates, broad profile distribution/trust, final product naming and repository-wide licensing/distribution remain later decisions.
The [AP14 implementation and managed commands](docs/AP14.md) have completed focused tests and the exact Deck workflow in PR #85, pending independent review. The two exact compatibility profiles remain review candidates.
