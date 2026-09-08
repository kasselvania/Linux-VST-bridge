# Linux Audio Compatibility Bridge

A managed bridge for using supported Windows audio plug-ins in native Linux DAWs without making musicians administer Wine prefixes, proxies or runtime versions.

This is an experimental implementation, not a consumer-ready release. Working repository name; not affiliated with Bitwig, Valve, Steinberg or a plug-in vendor.

## Current baseline: AP10 accepted

AP8 (#67), AP9 (#69) and AP10 (#71) form the integrated development baseline. AP10's code was reviewed at `fb0faed7cfc097ecb78653421d6e704bb9f25a00`, including the native returned-payload lifetime repair. The integration resolves documentation/status only; original code/build observations keep their provenance.

**Actual Windows Serum 2 and Efx FRAGMENTS work through our native Linux proxy in normally Applications-launched Bitwig.** Retained checks cover notes or audio input, real controls, opaque component/controller state and project recall. The shared delivery path includes bounded returned VST3 events and parameter feedback, aligned continuation after transient audio gaps, and independent optional observation. Reference-effect multi-instance and recovery evidence remains separately scoped; this does not certify simultaneous commercial instances.

See [AP10 findings and limits](docs/AP10.md), [R1 regression evidence](evidence/AP10/r1-payload-lifetime.json), [AP9 performance](docs/AP9.md), [owner CPU improvement](docs/AP9_OWNER_COST.md), and [AP8 first Serum result](docs/AP8_RESULT.md). Historical attempts and handoffs are evidence, not instructions to resume completed tasks.

## Operating recommendation and limits

Use **512 added bridge frames at 48 kHz (10.67 ms)** for the retained desktop fixture. Short 256-frame mailbox checks passed, but do not establish a new general default. Vendor-reported latency is additional and reported separately; FRAGMENTS' retained setup reported 192 vendor frames, for 704 total. Observed mean non-plug-in service of roughly 241 microseconds is not presentation delay, physical round trip, or a worst-case guarantee.

Float32 is implemented. Host setup and supported buses are negotiated within documented bounds; auxiliary routing, arbitrary dynamic topology, all input-event types and float64 are not universally implemented. Prepared artifacts, a compatible retained runtime/vendor environment and a private preview owner are still required. No polished vendor editor, installer/manager, reboot persistence, general licensing guarantee or broad customer-hardware qualification is claimed.

Tracked follow-through: [residual latency stalls and lower settings #72](https://github.com/kasselvania/Linux-VST-bridge/issues/72), [historical native engine close crash #73](https://github.com/kasselvania/Linux-VST-bridge/issues/73), and [prompt endpoint-failure notification #74](https://github.com/kasselvania/Linux-VST-bridge/issues/74). Later clean runs do not explain earlier failures.

## Current work: AP11 — Actual vendor editor

[Issue #75](https://github.com/kasselvania/Linux-VST-bridge/issues/75) selects a native launch/status panel and a detached vendor window controlling the same processing instance. The outcome is actual Serum editing, host automation and sound recall, editor open/close without restarting audio, and a focused FRAGMENTS check through the same mechanism. Product-derived names are included. Full embedding and the manager UI are not prerequisites. Editor implementation and its performance are not yet proved.

Work on `codex/ap11-vendor-editor`, prepared from integrated main `b8c332c2e536e4d9d6103d6d3d0696ffd6369f29`. [CURRENT_SLICE.md](CURRENT_SLICE.md) settles the window, control and verification direction. Keep 512 as the baseline and check audio under actual editor interaction rather than replaying the historical benchmark matrix.

## Start here

Read [AGENTS.md](AGENTS.md) and [CURRENT_SLICE.md](CURRENT_SLICE.md), then the relevant code and [design dossier](docs/DESIGN_DOSSIER.md). [Development guidance](docs/DEVELOPMENT_PROCESS.md) and [governance](GOVERNANCE.md) retain the outcome-led workflow: implement, test proportionately, review one PR. No receipt-writing loop, compulsory duplicate campaign or arbitrary retry quota.

AP8/AP9/AP10 branches are completed work. Preserve dirty local work when fetching the AP11 branch; repository preparation does not authorize discarding it. Ordinary implementation, builds and in-scope repairs are included in AP11's authorization.

Reuse [SSH/Moonlight desktop access](docs/DECK_REMOTE_DESKTOP.md); launch Bitwig through Applications. Remote access is development tooling, not a runtime dependency. Older [preview setup](docs/AP4_PREVIEW.md) is historical where later results supersede it.

## Code and records

- `native-vst3-proxy/`: Linux SDK-facing proxy and Rust backend.
- `native-audio-client/`: transport/client code; `windows-factory-probe/`: actual Windows SDK host.
- `tools/`: builds, private preview owner and test helpers; `docs/` and `evidence/`: design and source-specific results.

Rust is primary; C++20 is used at the SDK edges. The optional `tools/proof-run.py` and historical ledgers do not govern new task permission. Build products, proprietary installers, plug-ins, presets, activation data and credentials must remain outside version control.

## Product direction

A separate manager will organize installation, vendor authorization, scanning and selective publication. Each published native proxy represents its actual Windows plug-in class; the manager window should not be needed for playback. Generic prototype names are temporary. Vendor editors, correct identity/automation, low-latency instruments and audio effects are product priorities.

No repository-wide software license or final product name has been selected. Third-party licensing and distribution remain explicit future decisions.
