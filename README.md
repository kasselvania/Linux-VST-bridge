# Linux Audio Compatibility Bridge

A managed bridge for using supported Windows audio plug-ins in native Linux DAWs without making musicians administer Wine prefixes, proxies or runtime versions.

Experimental engineering preview, not a consumer-ready release. Working repository name; not affiliated with Bitwig, Valve, Steinberg, Arturia, Xfer Records or another vendor.

## Accepted baseline: AP14 managed Arturia workflow

AP8–AP11 established the independent native Linux VST3 proxy, supervised Windows VST3 processing under the selected Proton/Wine environment, bounded audio/events, opaque state and project recall, automation, independent instances and detached vendor editors. AP12 installed the exact Pure LoFi → Efx FRAGMENTS chain with automatic service startup. AP13 repaired a demonstrated state-capture delivery barrier, reduced supervisor overhead and added the installed delay selector.

AP14 [#85](https://github.com/kasselvania/Linux-VST-bridge/pull/85), integrated as `84abd6a9755ab10b015c3744549b1e46ab1b85cb`, adds the first profile-driven managed publication and exact rollback workflow. The manager discovers and supervises the exact module/class census, selects a closed verified profile, derives registration, obtains the product-owned native artifact, and atomically publishes the Linux VST3 bundle. The ordinary operator supplies no class IDs, module hashes, compatibility flags, registration JSON, native build paths or Proton commands.

Pure LoFi and Efx FRAGMENTS use immutable revision-3 `verified_exact_fixture` profiles for the exact Steam Deck / SteamOS 3.8.16 / Bitwig 6.1 / pinned Proton-SLR fixture. Revision-2 candidate and older publication history remain exact rollback targets. These are narrow exact-artifact claims, not universal Arturia or Linux support.

## Current operating limits

At 48 kHz, Pure LoFi reports 512 bridge + 48 vendor frames and Efx FRAGMENTS reports 512 + 192. The serial chain reports 1,264 frames / 26.333 ms, excluding DAW/device latency. **512 added frames per proxy remains installed, supported and recommended.** The opt-in 256 setting remains available but unqualified because startup and later FRAGMENTS gaps persist.

Short gaps and 24–31 ms preparation/publication outliers remain unclassified. Float32 and bounded stereo/main-bus/event paths are implemented. Arbitrary multichannel routing, sidechains, float64, broad MIDI/MPE, native Wayland views, every VST3 interface and general customer-hardware reliability are not claimed. The selected Arturia processes disable Windows UI Automation through an exact process-scoped compatibility choice; VST automation remains active.

## Active slice: AP15 direct detached editor lifecycle

AP15 activates [#84](https://github.com/kasselvania/Linux-VST-bridge/issues/84). Opening a plug-in editor in Bitwig should directly present or focus the exact vendor editor. Closing the vendor window should retire only that editor generation while DSP, automation, state and project identity remain alive. Reopening should produce exactly one fresh editor for the same instance. The visible native **Open / focus vendor editor** / **Close vendor editor** intermediary must disappear without abandoning the host-correct VST3 `IPlugView` lifecycle.

AP15 starts from integrated AP14 on `codex/ap15-direct-editor-lifecycle`. Read [CURRENT_SLICE.md](CURRENT_SLICE.md) for the active contract and [docs/AP15.md](docs/AP15.md) for the prepared implementation boundary.

## Follow-through

[#80](https://github.com/kasselvania/Linux-VST-bridge/issues/80) retains FRAGMENTS Advanced-panel expansion/redraw. [#72](https://github.com/kasselvania/Linux-VST-bridge/issues/72) retains residual delivery/performance classification. [#77](https://github.com/kasselvania/Linux-VST-bridge/issues/77) retains lawful Serum 2 authorization/editor qualification. These are not silently absorbed into AP15.

## Repository map

Read [AGENTS.md](AGENTS.md), [CURRENT_SLICE.md](CURRENT_SLICE.md), the relevant result documents and [docs/DESIGN_DOSSIER.md](docs/DESIGN_DOSSIER.md). `native-vst3-proxy/` is the Linux SDK proxy and Rust backend; `native-audio-client/` owns transport; `windows-factory-probe/` is the Windows SDK host; `bridge-manager/` owns environments, profiles, publication/rollback and installed supervision. Rust is primary, with C++ at SDK/platform edges. Proprietary binaries, presets, license data and credentials stay out of version control.

The independent bridge remains selected. Maintained Wine/Proton work may provide a runner beneath it; it is not a yabridge pivot. Preserve user projects, installed environments, historical evidence, credentials and unrelated dirty work.
AP15 revision-6 engineering qualification completed on the exact SteamOS 3.8.16 / Bitwig 6.1 Arturia fixture. Both real editors opened directly, closed and reopened in the same DSP sessions; edits, existing automation, explicit save/relaunch recall and independent FRAGMENTS removal were exercised. The 1×1/absent-child candidate and the later timestamp-refusal candidate remain recorded failures. See [actual results and limitations](docs/AP15.md#revision-6-exact-fixture-qualification).

Both exact verified revision-3 publications are restored. Revision 6 remains `review_candidate`, available only through the finite engineering route, and is not ordinary support or independent acceptance. 512 remains selected/supported/recommended; 256 is unqualified. Short delivery gaps remain, with exact counts in the evidence. PR #86 is for independent review and remains unmerged.
