# Linux Audio Compatibility Bridge

A managed bridge for using supported Windows audio plug-ins in native Linux DAWs without making musicians administer Wine prefixes, proxies or runtime versions.

This is an experimental implementation, not a consumer-ready release. Working repository name; not affiliated with Bitwig, Valve, Steinberg, Arturia, Xfer Records, or another plug-in vendor.

## Current baseline: AP11 accepted

AP8 (#67), AP9 (#69), AP10 (#71), and AP11 (#76) form the integrated development baseline. The implementation has now demonstrated actual Windows Serum 2 and Efx FRAGMENTS processing through our independent native Linux VST3 proxy in normally Applications-launched Bitwig.

The retained path includes note and audio input, dynamic inspected bus/class metadata within current bounds, opaque component/controller state, project recall, bounded returned VST3 events and parameter feedback, aligned continuation after transient gaps, and a detached vendor editor controlling the same Windows processing instance. With FRAGMENTS, real editor gestures reach Bitwig automation, hands-off replay reaches the vendor controller and processed sound, stopped edits save and recall, hidden/minimized focus works, and editor close/reopen leaves DSP alive.

AP11 also traced a real delivery stall to an unnecessary parameter refresh on every editor focus. That refresh caused Bitwig to capture unchanged plug-in state on the serialized control path. The false trigger was removed while genuine vendor refresh and save behavior remained intact. The final focused FRAGMENTS session completed 47,705 callbacks / 12,212,480 frames with zero missed or expired frames at the retained 512-frame setting.

See [AP11 results](docs/AP11.md), [AP11 focused follow-up](docs/AP11_REVIEW_FOLLOWUP.md), [AP10 findings](docs/AP10.md), [AP9 performance](docs/AP9.md), and [AP8 first Serum result](docs/AP8_RESULT.md). Historical attempts and failures remain evidence, not instructions to replay completed work.

## Operating recommendation and limits

Use **512 added bridge frames at 48 kHz (10.67 ms per proxy)** for the retained desktop fixture. Serial bridged devices add their presentation delays, and vendor-reported latency is additional. Observed mean service time is not presentation delay, physical round trip, or a worst-case guarantee.

Float32 is implemented. Supported buses, events, controls, and topology are negotiated within documented bounds; arbitrary multichannel/dynamic routing, every VST3 interface, all event types, and float64 are not universally implemented. Prepared artifacts, a compatible retained runner/environment, and the current preview-owner infrastructure are still required. No polished manager, automatic publication/startup, broad licensing guarantee, or customer-hardware qualification is claimed.

FRAGMENTS retains a resource-integrity warning and an explicit per-process Windows-accessibility workaround on this fixture. Serum's actual editor is reachable but its managed machine reported not authorized; the lawful vendor/operator qualification is tracked in [#77](https://github.com/kasselvania/Linux-VST-bridge/issues/77) and is not replaced by FRAGMENTS evidence.

Tracked engineering follow-through: [residual latency stalls #72](https://github.com/kasselvania/Linux-VST-bridge/issues/72), [historical native engine-close crash #73](https://github.com/kasselvania/Linux-VST-bridge/issues/73), and [prompt endpoint-failure notification #74](https://github.com/kasselvania/Linux-VST-bridge/issues/74). Later clean runs do not explain earlier failures.

## Current work

No implementation slice is active on this branch. The next selected product increment is persistent, human-loadable Bitwig publication for an Arturia instrument/effect pair: Pure LoFi followed by Efx FRAGMENTS. It should replace fixture-named endpoints and manually prepared owner startup with registered product identity, correct instrument/effect browser placement, independent sessions, and normal save/reopen use. That work starts from consolidated main after AP11 lands.

`giang17/wine` is retained as a serious candidate source for an audio-oriented runner beneath our project-owned proxy, Windows host, transport, state, automation, and editor integration. It is not a replacement for the bridge and does not reopen the no-yabridge decision. Runtime changes require a matching observed problem and a coherent, pinned, reversible comparison.

## Start here

Read [AGENTS.md](AGENTS.md) and [CURRENT_SLICE.md](CURRENT_SLICE.md), then the relevant code and [design dossier](docs/DESIGN_DOSSIER.md). [Development guidance](docs/DEVELOPMENT_PROCESS.md) and [governance](GOVERNANCE.md) retain the outcome-led workflow: implement, test proportionately, review one PR. No receipt-writing loop, compulsory duplicate campaign, or arbitrary retry quota.

Reuse [SSH/Moonlight desktop access](docs/DECK_REMOTE_DESKTOP.md); launch Bitwig through Applications. Remote access is development tooling, not a runtime dependency. Preserve dirty local work and user/vendor state when moving to a new branch.

## Code and records

- `native-vst3-proxy/`: Linux SDK-facing proxy and Rust backend.
- `native-audio-client/`: transport/client code.
- `windows-factory-probe/`: actual Windows SDK host.
- `tools/`: builds, private preview owners, inspection, and test helpers.
- `docs/` and `evidence/`: design and source-specific results.

Rust is primary; C++20 is used at SDK and platform-window edges. The optional `tools/proof-run.py` and historical ledgers do not govern new task permission. Build products, proprietary installers, plug-ins, presets, activation data, and credentials must remain outside version control.

## Product direction

A separate management plane will own installation, lawful authorization handoff, scanning, compatibility profiles, runner/environment selection, selective publication, diagnostics, update, repair, and rollback. Each published native proxy represents an exact Windows plug-in class using inspected product identity. The management UI should not be required for playback.

The user-facing goal is familiar devices in the DAW—such as Pure LoFi under instruments and Efx FRAGMENTS under audio effects—while the bridge machinery remains inspectable and recoverable underneath. No repository-wide software license or final product name has been selected; third-party licensing and distribution remain explicit future decisions.