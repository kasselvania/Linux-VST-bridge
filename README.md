# Linux Audio Compatibility Bridge

A managed bridge for using supported Windows audio plug-ins in native Linux DAWs without making musicians administer Wine prefixes, proxies, runtime versions or recovery steps.

Working repository name; not affiliated with Bitwig, Valve, Steinberg or a plug-in vendor. This is an experimental implementation, not a consumer-ready release.

## What works

**AP5 is accepted through PR #61**, merge `6694f6e410cb1ee7395879b570ae817b7ceed38f`, following passing rereview 5125675629 at `24cae167c4b3e61fc59f07fa51ec6cce9e638996`. Two bridged Windows AGain effects run inside one native Bitwig host with independent audio, controls, saved state and lifetime. Both restored their different settings before edits; 61,495,808 samples compared at zero error. Removing one left its sibling running. The owner reporting/cleanup repair passed focused tests and CI without repeating the desktop workload.

**An earlier isolated transport loss remains unexplained.** Acceptance of the bounded two-instance capability is not a reliability guarantee or a claim that the later successful run fixed that cause. See [AP5 results and limits](docs/AP5_RESULT.md). Original reports retain publication-time labels; the review and merge record acceptance.

This builds on [AP4's normal Applications-launch state recall](docs/AP4_RESULT.md). The private preview owner starts Windows endpoints, not the DAW. The [preview setup](docs/AP4_PREVIEW.md) remains useful; AP5 supersedes its historical one-instance limit with capacity four, with real desktop evidence for two. Use the established [desktop access](docs/DECK_REMOTE_DESKTOP.md).

The callback uses preallocated queues and separate transport workers. The retained scope is float32 stereo at 48 kHz, 1–256-frame callbacks and **1024 samples of added latency (21.33 ms at 48 kHz)**. Prepared artifacts/runtime and a running private owner remain required. Commercial compatibility, instruments, vendor editors, automatic installation, reboot persistence and low-latency suitability remain unproved. Serum remains the intended commercial fixture; the current reference host is not yet a general vendor implementation.

## Current goal

**AP7 — Survive transient playback underruns**, tracked in #64 and PR #65. The repair counts missing presentation spans and resumes aligned audio on the existing instance; optional comparison no longer delays transport. Focused SDK tests pass, and the normally launched two-instance Bitwig check recorded zero gaps, terminal failures or sample errors with complete comparison coverage. Review is pending; historical transport/timing losses remain unexplained. See [the repair result and source-specific limits](docs/AP7_PLAYBACK_REPAIR.md) and [CURRENT_SLICE.md](CURRENT_SLICE.md).

AP6 recovery is accepted at `c861993b886dc89e1358a3005be899f46492bfb7`: explicit recovery restores the last confirmed complete state while the healthy sibling continues. [Its results and snapshot-age limits](docs/AP6_RESULT.md) remain unchanged.

AP4's D9 diagnostic and failed A2 remain unchanged in [the attempt record](docs/AP4_ATTEMPT_STATUS.md). No repeat of completed AP4/AP5 campaigns is required.

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
