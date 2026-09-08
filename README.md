# Linux Audio Compatibility Bridge

A managed bridge for using supported Windows audio plug-ins in native Linux DAWs without making musicians administer Wine prefixes, proxies, or runtime versions.

This is an experimental implementation, not a consumer-ready release. Working repository name; not affiliated with Bitwig, Valve, Steinberg, Arturia, Xfer Records, or another plug-in vendor.

## Accepted baseline: AP11

AP8 (#67), AP9 (#69), AP10 (#71), and AP11 (#76) form the integrated development baseline on main `1d5d693e2e2d12547dfd6b02e2b8492b27418592`. Actual Windows Serum 2 and Efx FRAGMENTS processing has been demonstrated through our independent native Linux VST3 proxy in normally Applications-launched Bitwig.

The retained path includes note and audio input, inspected bus/class metadata within current bounds, opaque component/controller state, project recall, bounded returned VST3 events and parameter feedback, aligned continuation after transient gaps, and a detached vendor editor controlling the same Windows processing instance. FRAGMENTS evidence includes real automation replay, stopped-edit recall, hidden/minimized focus, and editor close/reopen without restarting DSP.

AP11 traced a delivery stall to an unnecessary parameter refresh on editor focus that caused Bitwig to capture unchanged state. Removing that trigger preserved genuine vendor refresh/save behavior. Its final focused FRAGMENTS session completed 47,705 callbacks / 12,212,480 frames with no missed or expired frames; this is not a universal reliability claim.

See [AP11 results](docs/AP11.md), [focused follow-up](docs/AP11_REVIEW_FOLLOWUP.md), [AP10](docs/AP10.md), [AP9](docs/AP9.md), and [AP8](docs/AP8_RESULT.md). Historical failures and successes retain their original artifacts and scope.

## Current work: finish AP12, not a new successor

[PR #79](https://github.com/kasselvania/Linux-VST-bridge/pull/79) and [issue #78](https://github.com/kasselvania/Linux-VST-bridge/issues/78) remain open on `codex/ap12-arturia-everyday-use`. **Do not merge yet:** LoFi's terminal delivery timeout and the full installed pair/restart qualification remain unresolved. [CURRENT_SLICE.md](CURRENT_SLICE.md) replaces completed repair instructions with the next execution task.

The candidate now has persistent Arturia environments, SDK-derived names/vendor/categories, registered native publications, automatic startup and independent supervised Windows sessions. Both devices remain installed. State repairs preserve genuine opaque captures when supplemental parameter readback is unavailable, keep ordinary save refusals distinct from terminal failures, allow fresh vendor-editor access before saving is available, and separate diagnostic reporting from physical retirement.

LoFi's candidate-specific edited-state recall and automation have passed. After the operator replaced FRAGMENTS' module, its fingerprint mismatch was correctly refused; reinspection and an explicit inactive proxy-binding update restored load/editor/save/close. That check processed silence. Original installer outputs, replacement artifacts and authorization claims remain separate; a missing login prompt is not proof of authorization. See [AP12 results](docs/AP12.md) and [latest load/removal evidence](evidence/AP12/operator-replacement-load-removal.json).

The immediate task is to retain the in-flight request before a timeout/containment, repair the demonstrated cause, and finish real **Pure LoFi → Efx FRAGMENTS** sound, editor automation, pair recall, automatic startup after a Deck/full user-session restart, and removal with a healthy audible sibling. The trace must not depend on a hung request completing. Later clean retries do not explain the earlier timeout; do not substitute an information-only campaign for the musical workflow.

[Editor follow-through #80](https://github.com/kasselvania/Linux-VST-bridge/issues/80) tracks FRAGMENTS Advanced expansion and unsmooth redraw. Defer general graphics work unless it blocks required controls, loses musical updates or shares the demonstrated timeout cause.

## Operating recommendation and limits

Retain **512 added bridge frames at 48 kHz (10.67 ms per proxy)**. Serial bridged devices accumulate that delay; vendor and hardware latency are additional. Report service time, presentation delay and physical round trip separately. AP12 still has short gaps and an intermittent terminal reply loss at this setting; it is not certified gap-free.

Float32 and the documented bounded bus/event paths are implemented. Arbitrary multichannel/dynamic routing, every VST3 interface/event type, float64 and broad customer-hardware reliability are not universally supported.

Historical FRAGMENTS evidence retains its resource-integrity warning. The current profile retains an explicit per-process Windows-accessibility workaround. Neither is erased by publication or a changed artifact. Serum's lawful authorization/editor qualification remains [#77](https://github.com/kasselvania/Linux-VST-bridge/issues/77).

Other follow-through: [delivery stalls and lower latency #72](https://github.com/kasselvania/Linux-VST-bridge/issues/72), [historical native-close crash #73](https://github.com/kasselvania/Linux-VST-bridge/issues/73), and [prompt endpoint-failure notification #74](https://github.com/kasselvania/Linux-VST-bridge/issues/74). Relevant AP12 failures are in scope; unrelated historical issues are not mandatory replay campaigns.

## Start here

Read [AGENTS.md](AGENTS.md) and [CURRENT_SLICE.md](CURRENT_SLICE.md), then relevant production code and the [design dossier](docs/DESIGN_DOSSIER.md). [Development guidance](docs/DEVELOPMENT_PROCESS.md) retains ordinary implementation, proportionate tests and one reviewed PR, not receipt-writing loops or arbitrary retry quotas.

Reuse [SSH/Moonlight desktop access](docs/DECK_REMOTE_DESKTOP.md) and launch Bitwig through Applications. Preserve dirty work, user projects and vendor environments. Remote access is development tooling, not a playback dependency. Leave intended installations/publications available at handoff; clean temporary experiments rather than uninstalling the product.

## Code and product direction

`native-vst3-proxy/` contains the Linux SDK proxy and Rust backend; `native-audio-client/` contains transport/client code; `windows-factory-probe/` contains the Windows SDK host; `bridge-manager/` contains registration, publication and installed supervision. `tools/`, `docs/`, and `evidence/` retain build helpers, design and source-specific results. Rust is primary, with C++20 at SDK/platform-window edges. Proprietary binaries, presets, licensing state and credentials remain outside version control.

The management plane owns installation, vendor-access handoff, scanning, selective publication, compatible environment/runner selection, diagnostics and eventually updates/repair/rollback. Each published device represents an exact Windows class; the manager UI should not be required for playback. Friendly names and browser categories do not change class/parameter identity.

The independent bridge remains selected. `giang17/wine` is a potential runtime source beneath it, not a replacement or a yabridge pivot. Retain the pinned working runtime until a matching observed defect justifies a coherent reversible comparison. No repository-wide software license or final product name has been selected; third-party licensing and distribution remain explicit future decisions.
