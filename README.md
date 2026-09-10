# Linux Audio Compatibility Bridge

A managed bridge for using supported Windows audio plug-ins in native Linux DAWs without making musicians administer Wine prefixes, proxy synchronization commands, or changing runtime versions by hand.

**Experimental engineering preview, not a consumer-ready release.** The project is not affiliated with Bitwig, Valve, Steinberg, Arturia, Xfer Records, or another plug-in vendor.

> **Ownership:** This repository is publicly readable but proprietary. Copyright © 2026 Peter Kassel. All rights reserved. Public visibility does not grant an open-source or redistribution license. See [COPYRIGHT.md](COPYRIGHT.md) and [CONTRIBUTING.md](CONTRIBUTING.md).

## Accepted baseline: AP15 direct managed Arturia workflow

AP8–AP11 established the independent native Linux VST3 proxy, supervised Windows VST3 processing under a pinned Proton/Wine environment, bounded audio/events, opaque state and project recall, automation, independent instances, and detached vendor editors.

AP12 installed the exact Pure LoFi → Efx FRAGMENTS chain with automatic service startup. AP13 removed a demonstrated state-capture delivery barrier, reduced supervisor overhead, and added the installed 256/512 delay selector. AP14 added closed compatibility profiles, manager-derived registration, immutable publication revisions, recovery, and exact rollback.

AP15, integrated as `d4076e9cc02028bf268089ca0bb86aeac36ef947`, makes the direct detached vendor editor the ordinary behavior for the exact accepted fixture. Opening the plug-in editor in Bitwig directly presents or focuses the vendor editor. Vendor close retires only that editor generation; one normal action reopens a greater editor epoch for the same DSP instance. Normal managed publication now selects immutable revision-7 `verified_exact_fixture` profiles, while exact revision-3 publications remain retained rollback targets.

The accepted matrix is deliberately narrow:

- Steam Deck / SteamOS 3.8.16;
- Bitwig Studio 6.1 Flatpak;
- exact pinned Proton-SLR runner and environment;
- Pure LoFi 1.0.0.6121;
- Efx FRAGMENTS 1.0.0.2925;
- float32, bounded main stereo/event behavior;
- exact reviewed Windows host and Linux native artifacts.

This is not a claim of broad Arturia, Linux, DAW, VST3, hardware, or customer-installation support.

## Current operating limits

At 48 kHz, Pure LoFi reports 512 bridge + 48 vendor frames and Efx FRAGMENTS reports 512 + 192. The serial chain therefore reports 1,264 frames / 26.333 ms, excluding DAW, device, and acoustic latency.

**512 added frames per proxy is selected, supported, and recommended.** The opt-in 256 setting remains available but unqualified because startup and later FRAGMENTS deadline misses remain.

Residual short gaps and historical 24–31 ms timing outliers are not fully classified. Float64, arbitrary multichannel routing, sidechains, broad MIDI/MPE, every VST3 interface, native Wayland views, and general customer-hardware reliability are not yet supported claims. The accepted Arturia processes disable Windows UI Automation through an exact process-scoped compatibility choice; VST parameter automation remains active.

## Active slice: AP16 causal 512-frame delivery reliability

AP16 activates [issue #72](https://github.com/kasselvania/Linux-VST-bridge/issues/72) from integrated main at `510d88bd89adbf8b101140907f27dc9885af9f25`.

The goal is not an open-ended performance campaign. Under the ordinary revision-7 Pure LoFi → FRAGMENTS workload at 512 frames, AP16 must reproduce one residual missed-delivery class, assign elapsed time to a real owner, repair its demonstrated project-owned cause or provide a truthful bounded external mitigation, and show matched before/after evidence without hiding gaps or changing musical timing.

Read [CURRENT_SLICE.md](CURRENT_SLICE.md) for active authority and [docs/AP16.md](docs/AP16.md) for the source-grounded investigation boundary.

## Follow-through

- [#80](https://github.com/kasselvania/Linux-VST-bridge/issues/80) retains FRAGMENTS Advanced-panel expansion and redraw smoothness.
- [#77](https://github.com/kasselvania/Linux-VST-bridge/issues/77) retains lawful Serum 2 authorization and exact editor/product qualification.
- Multi-instance capacity and recovery qualification remains the recommended slice after AP16.

These are not silently absorbed into the active delivery slice.

## Repository map

- `native-vst3-proxy/`: native Linux VST3 SDK proxy and Rust callback/transport backend;
- `native-audio-client/`: mapped transport and callback-facing primitives;
- `windows-factory-probe/`: supervised Windows VST3 SDK host, processing, state, and editor owners;
- `bridge-manager/`: environments, installation, profiles, publication/rollback, service admission, and supervision;
- `compatibility/`: immutable exact-fixture profile history;
- `docs/`: architecture, decisions, slice contracts, and reviewed results;
- `evidence/`: bounded retained identities and results, never proprietary plug-in binaries or license payloads.

Read [AGENTS.md](AGENTS.md), [CURRENT_SLICE.md](CURRENT_SLICE.md), and [docs/DESIGN_DOSSIER.md](docs/DESIGN_DOSSIER.md) before implementation. Rust is primary, with C++ at SDK and platform edges. The bridge remains independent: suitable Wine/Proton work is a runner beneath project-owned proxy, host, transport, state, and management boundaries—not a yabridge pivot.
