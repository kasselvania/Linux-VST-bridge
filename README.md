# Linux Audio Compatibility Bridge

A managed bridge for using supported Windows audio plug-ins in native Linux DAWs without making musicians administer Wine prefixes, proxies, runtime versions or recovery steps.

Working repository name; not affiliated with Bitwig, Valve, Steinberg or a plug-in vendor. This is an experimental implementation, not a consumer-ready release.

## What works

**AP3 is accepted:** a native Linux VST3 preview sends audio to actual Windows AGain under the tested Proton/Steam Linux Runtime combination and returns processed audio to Bitwig. The callback uses preallocated queues and a separate transport worker. The measured preview adds **1024 samples of latency (21.33 ms at 48 kHz)**.

The retained verification checked 6,366,144 samples with maximum numerical error 0.0 and demonstrated controlled Bitwig playback, gain/mute, removal and fresh reopening. It also established Mac-to-Deck Moonlight/Sunshine control for development. See [AP3 results](docs/slices/AP3/RESULT.md) and [desktop access](docs/DECK_REMOTE_DESKTOP.md).

This accepted baseline is a single-instance, float32 stereo, 48-kHz reference-effect preview with 1–256-frame callbacks. It does not establish low-latency suitability, commercial plug-in compatibility, instruments, vendor editors, general installation management or release readiness. Serum remains an intended commercial fixture, not a proven capability.

## Current goal

[AP4 is working and awaiting review in PR #59](docs/AP4_RESULT.md): ordinary Applications launches saved and reopened the real Windows AGain setting at **0.1650**, with **5,250,048 returned samples checked at zero error before any control edit**. The save session survived stopped transport and more than three minutes disconnected from Moonlight. A private preview owner reuses the existing launcher and cleanup; no special DAW launch is required. Test settings/publication were restored and the saved project retained.

This extends the observed development result, pending review; AP3 remains the accepted baseline. See [current work](CURRENT_SLICE.md), [preview setup and limits](docs/AP4_PREVIEW.md), and the unchanged [historical D9/A2 record](docs/AP4_ATTEMPT_STATUS.md). No new acceptance campaign is required or scheduled.

## Start here

Read [AGENTS.md](AGENTS.md) and [CURRENT_SLICE.md](CURRENT_SLICE.md), then the code and design sections relevant to the task. [Development guidance](docs/DEVELOPMENT_PROCESS.md) explains normal iteration; [governance](GOVERNANCE.md) explains decisions and evidence. The lead's [design dossier](docs/DESIGN_DOSSIER.md) and [architecture](docs/ARCHITECTURE.md) describe the broader product, not a checklist to implement in every slice.

An approved task includes implementation, focused tests and routine repair. No mandatory receipt-writing, two-run diagnostic rule, duplicated acceptance campaign or separate status-closure PR. Review the actual result and its limitations. Preserve safe process ownership, real-time behavior and existing user work.

## Code and records

- `native-vst3-proxy/`: official SDK-facing Linux plug-in shell and Rust backend.
- `native-audio-client/`: Linux transport/client code and tests.
- `windows-factory-probe/`: Windows SDK host, lifecycle and processing.
- `tools/`: existing builds, supervised test helpers, validation and legacy transaction tooling.
- `docs/`: product architecture, current guidance and historical task records.
- `evidence/`: retained observations; original labels and failures are preserved.

Rust is primary. C++20 is used at the official VST3 edges; C++ objects do not cross the C ABI. A Linux host loads a native proxy, not a Windows DLL directly. The supervised Windows endpoint runs the real plug-in.

`tools/proof-run.py` is an optional legacy exact-transaction interface. It remains available for recorded results and deliberately selected legacy runs, but is not the default development permission system. Its fail-closed defaults and old ledgers do not define the current task.

## Product direction and licensing

The intended experience is install, authorize, scan, publish to a DAW, use, save, reopen, update and recover predictably. Broader runtime management, commercial support and a user-facing installer still need implementation and verification.

No repository-wide software license has been selected. Third-party SDK/runtime licensing and a final product name remain open decisions. Do not commit proprietary installers, plug-ins, presets, license/activation data or credentials.
