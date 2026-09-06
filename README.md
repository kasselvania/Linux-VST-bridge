# Linux Audio Compatibility Bridge

A managed bridge for using supported Windows audio plug-ins in native Linux DAWs without making musicians administer Wine prefixes, proxies, runtime versions or recovery steps.

Working repository name; not affiliated with Bitwig, Valve, Steinberg or a plug-in vendor. This is an experimental implementation, not a consumer-ready release.

## What works

**AP7 is accepted through PR #65**, merge `cdfd05be9af2576768d8f2ccc55d3064e9e72a36`, reviewed at `5232870e6ce024efe52c2d6be0af7f62b7f021c3`. A temporarily late result now causes an accurately counted presentation gap and aligned continuation, not automatic destruction of the Windows instance. Mandatory validation remains; optional comparison and fingerprinting run independently. The bounded two-instance Bitwig check recorded zero gaps or terminal failures, 21,244,928 returned channel samples checked at zero error and no lost comparison coverage. See [AP7 repair and source-specific limits](docs/AP7_PLAYBACK_REPAIR.md).

This builds on [AP4's real state/normal Applications-launch recall](docs/AP4_RESULT.md), [AP5's independent instances](docs/AP5_RESULT.md) and [AP6's explicit recovery](docs/AP6_RESULT.md). Recovery restores the last confirmed complete snapshot, not later uncaptured edits. The private preview owner starts Windows endpoints, not the DAW. The [preview setup](docs/AP4_PREVIEW.md) remains useful, superseded where later results change its historical limits. Reuse the established [desktop access](docs/DECK_REMOTE_DESKTOP.md).

**Historical transport/timing failures remain unresolved unless separately explained.** Successful later runs and the repaired underrun policy do not identify their original causes or establish long-run reliability. Original [AP7 observations](docs/AP7_RESULT.md), AP4/D9/A2 and AP5/AP6 evidence retain their original labels. Review and merge records establish acceptance.

The accepted fixture remains AGain: float32 stereo at 48 kHz, 1–256-frame callbacks and **1024 samples of added latency (21.33 ms at 48 kHz)**. Native/owner capacity is four, with actual desktop evidence for two. Prepared artifacts/runtime and a private owner are required. Commercial compatibility, instruments, vendor editors, automatic installation, reboot persistence and low-latency suitability remain unproved. AP7 review N1 notes a nonblocking multi-record diagnostic emission limit; AP8 includes its small local repair.

## Current goal

**AP8 — First playable Serum 2 instrument**, tracked in #66 and [CURRENT_SLICE.md](CURRENT_SLICE.md). Play a real note clip through the Windows instrument in normal Bitwig, control a real parameter and save/reopen the sound. Replace the relevant AGain-only metadata, event, parameter and state assumptions by following the actual module's SDK interfaces. The recorded installed Serum candidate is a starting point, not proof of current authorization or compatibility.

One simple patch and one instrument instance are the target. Necessary module setup and ordinary vendor activation are included; a general installer, polished editor, preset manager and broad compatibility claims are not. An SDK fixture may isolate an interface problem but cannot substitute for the real Serum outcome.

## Start here

Read [AGENTS.md](AGENTS.md) and [CURRENT_SLICE.md](CURRENT_SLICE.md), then relevant code and design sections. [Development guidance](docs/DEVELOPMENT_PROCESS.md) covers iteration; [governance](GOVERNANCE.md) covers decisions and evidence. The [design dossier](docs/DESIGN_DOSSIER.md) and [architecture](docs/ARCHITECTURE.md) describe the broader product, not a checklist to implement in every slice.

An approved task includes implementation, focused tests and routine repair. No mandatory receipt-writing, two-run diagnostic rule, duplicated acceptance campaign or separate status-closure PR. Review the actual result and its limitations. Preserve safe process ownership, real-time behavior and existing user work.

## Code and records

- `native-vst3-proxy/`: official SDK-facing Linux plug-in shell and Rust backend.
- `native-audio-client/`: Linux transport/client code and tests.
- `windows-factory-probe/`: Windows SDK host, lifecycle and processing.
- `tools/`: builds, preview owner, supervised test helpers and legacy transaction tooling.
- `docs/` and `evidence/`: product design, guidance and retained observations with original labels/failures.

Rust is primary. C++20 is used at the official VST3 edges; C++ objects do not cross the C ABI. A Linux host loads a native proxy, not a Windows DLL directly. The supervised Windows endpoint runs the real plug-in.

`tools/proof-run.py` is an optional legacy exact-transaction interface, not the default development permission system. Its fail-closed defaults and old ledgers do not define the current task.

## Product direction and licensing

The intended experience is install, authorize, scan, publish to a DAW, use, save, reopen, update and recover predictably. Broader runtime management, commercial support and a user-facing installer still need implementation and verification.

No repository-wide software license has been selected. Third-party SDK/runtime licensing and a final product name remain open decisions. Do not commit proprietary installers, plug-ins, presets, license/activation data or credentials.
