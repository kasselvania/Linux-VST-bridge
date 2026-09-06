# Linux Audio Compatibility Bridge

A managed bridge for using supported Windows audio plug-ins in native Linux DAWs without making musicians administer Wine prefixes, proxies, runtime versions or recovery steps.

Working repository name; not affiliated with Bitwig, Valve, Steinberg or a plug-in vendor. This is an experimental implementation, not a consumer-ready release.

## What works

**AP4 is accepted through PR #59**, merge `a45a30b916f847d1cc683ef7e07121b57d52491c`. A normal Bitwig Applications launch can use the native Linux preview, send audio to actual Windows AGain, save its complete component state and recall it with fresh processes. The private preview owner starts only the Windows endpoint; no special DAW launch or injected session environment is required on the tested fixture.

The saved 0.1650 setting returned after reopening, with 5,250,048 fresh-recall samples checked at zero error before any control edit. Ordinary pauses and a Moonlight disconnection exceeding three minutes did not expire the instance. See [AP4 result](docs/AP4_RESULT.md), [preview setup and limits](docs/AP4_PREVIEW.md), and [desktop access](docs/DECK_REMOTE_DESKTOP.md). Historical reports retain their publication-time status and provenance; review 5124120833 and the merge record acceptance.

The callback uses preallocated queues and a separate transport worker. This remains a **single-instance**, float32 stereo, 48-kHz reference-effect preview with 1–256-frame callbacks and **1024 samples of added latency (21.33 ms at 48 kHz)**. Prepared artifacts/runtime and a running private owner are still required. It does not establish low-latency suitability, commercial compatibility, instruments, vendor editors, automatic installation or reboot persistence. Serum remains the intended commercial fixture, not a proven capability.

## Current goal

**AP5 — Two independent plug-in instances**, tracked in #60 and [CURRENT_SLICE.md](CURRENT_SLICE.md). Put two copies on separate Bitwig tracks, keep their audio/settings independent, save/reopen both, and remove one without disturbing the other. Extend the working bridge rather than building a new framework.

AP4's prior D9 diagnostic and failed A2 remain unchanged in [the attempt record](docs/AP4_ATTEMPT_STATUS.md). No repeat of that completed campaign is required.

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
