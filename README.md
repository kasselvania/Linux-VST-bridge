# Linux Audio Compatibility Bridge

A managed bridge for using supported Windows audio plug-ins in native Linux DAWs without making musicians administer Wine prefixes, proxies, or runtime versions.

This is an experimental implementation, not a consumer-ready release. Working repository name; not affiliated with Bitwig, Valve, Steinberg, Arturia, Xfer Records, or another plug-in vendor.

## Current baseline: AP11 accepted

AP8 (#67), AP9 (#69), AP10 (#71), and AP11 (#76) form the integrated development baseline. The implementation has demonstrated actual Windows Serum 2 and Efx FRAGMENTS processing through our independent native Linux VST3 proxy in normally Applications-launched Bitwig.

The retained path includes note and audio input, inspected bus/class metadata within current bounds, opaque component/controller state, project recall, bounded returned VST3 events and parameter feedback, aligned continuation after transient gaps, and a detached vendor editor controlling the same Windows processing instance. With FRAGMENTS, real editor gestures reach Bitwig automation, hands-off replay reaches the vendor controller and processed sound, stopped edits save and recall, hidden/minimized focus works, and editor close/reopen leaves DSP alive.

AP11 also traced a real delivery stall to an unnecessary parameter refresh on every editor focus. That caused Bitwig to capture unchanged plug-in state on the serialized control path. The false trigger was removed while genuine vendor refresh and save behavior remained intact. The final focused FRAGMENTS session completed 47,705 callbacks / 12,212,480 frames with zero missed or expired frames at the retained 512-frame setting.

See [AP11 results](docs/AP11.md), [AP11 focused follow-up](docs/AP11_REVIEW_FOLLOWUP.md), [AP10 findings](docs/AP10.md), [AP9 performance](docs/AP9.md), and [AP8 first Serum result](docs/AP8_RESULT.md). Historical attempts and failures remain evidence, not instructions to replay completed work.

## Operating recommendation and limits

Use **512 added bridge frames at 48 kHz (10.67 ms per proxy)** for the retained desktop fixture. Serial bridged devices add their presentation delays, and vendor-reported latency is additional. Observed mean service time is not presentation delay, physical round trip, or a worst-case guarantee.

Float32 is implemented. Supported buses, events, controls, and topology are negotiated within documented bounds; arbitrary multichannel/dynamic routing, every VST3 interface, all event types, and float64 are not universally implemented.

FRAGMENTS retains a resource-integrity warning and an explicit per-process Windows-accessibility workaround on this fixture. Serum’s actual editor is reachable but its managed machine reported not authorized; the lawful vendor/operator qualification is tracked in [#77](https://github.com/kasselvania/Linux-VST-bridge/issues/77) and is not replaced by FRAGMENTS evidence.

Tracked engineering follow-through: [residual latency stalls #72](https://github.com/kasselvania/Linux-VST-bridge/issues/72), [historical native engine-close crash #73](https://github.com/kasselvania/Linux-VST-bridge/issues/73), and [prompt endpoint-failure notification #74](https://github.com/kasselvania/Linux-VST-bridge/issues/74). Later clean runs do not explain earlier failures.

## Current work: AP12 — Everyday Arturia pair in Bitwig

[Issue #78](https://github.com/kasselvania/Linux-VST-bridge/issues/78) is active on `codex/ap12-arturia-everyday-use`, prepared from main `1d5d693e2e2d12547dfd6b02e2b8492b27418592`.

The normal Arturia installers complete in a persistent bridge-owned environment, and Pure LoFi initializes successfully. The original installed modules refused state capture. A later operator-replaced Pure LoFi candidate is now published and has passed a focused Bitwig editor/playback/save/close check at 512 added frames; the original failures and one unreproduced delivery timeout remain recorded. This does not qualify the original installer or vendor authorization. The registered startup service is installed, while the two-device workflow remains incomplete. See [AP12 results and remaining work](docs/AP12.md).

The target is a human-usable Pure LoFi → Efx FRAGMENTS chain. Use the normal user-owned installers to establish a persistent Arturia environment; publish Pure LoFi as an Arturia instrument and Efx FRAGMENTS as an Arturia audio effect; start the supervised bridge automatically when Bitwig loads either device; support both independent instances and editors simultaneously; and save/restart/reopen without an agent, SSH session, development checkout, or manually started preview owner.

A typed setup CLI is sufficient. This is not a full manager GUI or universal installer system. The intended publications remain installed at handoff.

Classification comes from actual VST3 factory/class metadata, not the presence of audio inputs. Stable class/parameter identity and saved projects must survive friendly naming and installation paths. The current pinned Proton runner remains the initial baseline. `giang17/wine` is retained as a candidate audio-oriented runner for a matching observed runtime defect, not as a bridge replacement, yabridge pivot, or mandatory build.

Read [CURRENT_SLICE.md](CURRENT_SLICE.md) for the complete implementation contract.

## Start here

Read [AGENTS.md](AGENTS.md) and [CURRENT_SLICE.md](CURRENT_SLICE.md), then the relevant code and [design dossier](docs/DESIGN_DOSSIER.md). [Development guidance](docs/DEVELOPMENT_PROCESS.md) and [governance](GOVERNANCE.md) retain the outcome-led workflow: implement, test proportionately, review one PR. No receipt-writing loop, compulsory duplicate campaign, or arbitrary retry quota.

Reuse [SSH/Moonlight desktop access](docs/DECK_REMOTE_DESKTOP.md); launch Bitwig through Applications. Remote access is development tooling, not a runtime dependency. Preserve dirty local work and user/vendor state when moving to the AP12 branch.

## Code and records

- `native-vst3-proxy/`: Linux SDK-facing proxy and Rust backend.
- `native-audio-client/`: transport/client code.
- `windows-factory-probe/`: actual Windows SDK host.
- `tools/`: builds, private preview owners, inspection, and test helpers.
- `docs/` and `evidence/`: design and source-specific results.

Rust is primary; C++20 is used at SDK and platform-window edges. The optional `tools/proof-run.py` and historical ledgers do not govern new task permission. Build products, proprietary installers, plug-ins, presets, activation data, and credentials must remain outside version control.

## Product direction

The management plane will own installation, lawful authorization handoff, scanning, compatibility profiles, runner/environment selection, selective publication, diagnostics, update, repair, and rollback. Each published native proxy represents an exact Windows plug-in class using inspected product identity. The management UI should not be required for playback.

The user-facing goal is familiar devices in the DAW—such as Pure LoFi under instruments and Efx FRAGMENTS under audio effects—while the independent bridge machinery remains inspectable and recoverable underneath. No repository-wide software license or final product name has been selected; third-party licensing and distribution remain explicit future decisions.
