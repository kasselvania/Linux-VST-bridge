# Linux Audio Compatibility Bridge

AP16 is accepted and integrated at `bee44977a9db6b512f5a67076ceedabec5c656b4`. Hot per-session transport mappings now use private tmpfs rather than journaled environment storage, removing one demonstrated 27–29 ms preparation-stall class while preserving the existing Windows host, native proxies, profiles, timing semantics, and rollback. Residual startup, queue/reply, and lifecycle misses remain tracked in [#90](https://github.com/kasselvania/Linux-VST-bridge/issues/90).

A managed bridge for using supported Windows audio plug-ins in native Linux DAWs without making musicians administer Wine prefixes, proxy synchronization commands, or changing runtime versions by hand.

**Experimental engineering preview, not a consumer-ready release.** The project is not affiliated with Bitwig, Valve, Steinberg, Arturia, Xfer Records, or another plug-in vendor.

> **Ownership:** This repository is publicly readable but proprietary. Copyright © 2026 Peter Kassel. All rights reserved. Public visibility does not grant an open-source or redistribution license. See [COPYRIGHT.md](COPYRIGHT.md) and [CONTRIBUTING.md](CONTRIBUTING.md).

## Accepted baseline: AP16 direct managed Arturia workflow

AP8–AP11 established the independent native Linux VST3 proxy, supervised Windows VST3 processing under a pinned Proton/Wine environment, bounded audio/events, opaque state and project recall, automation, independent instances, and detached vendor editors.

AP12 installed the exact Pure LoFi → Efx FRAGMENTS chain with automatic service startup. AP13 removed a demonstrated state-capture delivery barrier, reduced supervisor overhead, and added the installed 256/512 delay selector. AP14 added closed compatibility profiles, manager-derived registration, immutable publication revisions, recovery, and exact rollback.

AP15, integrated as `d4076e9cc02028bf268089ca0bb86aeac36ef947`, makes the direct detached vendor editor the ordinary behavior for the exact accepted fixture. Opening the plug-in editor in Bitwig directly presents or focuses the vendor editor. Vendor close retires only that editor generation; one normal action reopens a greater editor epoch for the same DSP instance. Normal managed publication selects immutable revision-7 `verified_exact_fixture` profiles, while exact revision-3 publications remain retained rollback targets.

AP16 moves only ephemeral high-frequency transport mappings into an identity-checked private runtime tmpfs. Durable environment, ownership, readiness, report, profile, publication, state, and project data remain persistent. A measured FRAGMENTS musical interval with 1,536 missed frames behind a 28.502 ms native preparation stall became zero missed frames in the matched corrected interval; two independent trace-off confirmations also had zero new gaps. This is a causal repair for one class, not a gap-free or hard real-time claim.

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

**512 added frames per proxy is selected, supported, and recommended.** The opt-in 256 setting remains available but unqualified.

Residual startup, queue/reply, editor/removal, and shutdown-window misses remain explicit in [#90](https://github.com/kasselvania/Linux-VST-bridge/issues/90). Float64, arbitrary multichannel routing, sidechains, broad MIDI/MPE, every VST3 interface, native Wayland views, and general customer-hardware reliability are not yet supported claims. The accepted Arturia processes disable Windows UI Automation through an exact process-scoped compatibility choice; VST parameter automation remains active.

FRAGMENTS’ Advanced panel is now accessible and rendering response has improved materially. Further “buttery smooth” graphical optimization is polish rather than a current functional blocker.

## Active slice: AP17 real-project capacity and recovery

AP17 activates [issue #91](https://github.com/kasselvania/Linux-VST-bridge/issues/91) from integrated AP16 main at `bee44977a9db6b512f5a67076ceedabec5c656b4`.

AP17 now distinguishes sixteen service workers, six global DSPs, three LoFi, four FRAGMENTS and the separate four-slot native image ceiling. The exact six-device fixture passed the retained parallel/serial playback, distinct-instance recall, failure containment and service/reboot recovery checks. R1 distinguishes temporary native insertion contention from hard capacity and handle exhaustion.

Both engineering native candidates are restored to exact ordinary revision 7 with revision 3 retained. The immutable AP17 manager remains installed. Short delivery gaps, opaque-state comparison limits and excluded scheduling-confounded intervals remain explicit; 512 is recommended and 256 unqualified. The result is awaiting independent review in PR #92, not a general capacity guarantee.

Read [CURRENT_SLICE.md](CURRENT_SLICE.md) for active authority and [docs/AP17.md](docs/AP17.md) for the source-grounded investigation boundary.

## Planned vendor pathway

After AP17, the intended next vertical is the official Arturia Software Center → Pigments pathway:

```text
official vendor application
→ user-owned sign-in and activation
→ product download and installation
→ exact module/resources discovery
→ managed candidate profile and publication
→ Pigments qualification in Bitwig
```

The ASC installer may be downloaded privately in preparation, but it is not executed, committed, or mixed into AP17.

Serum 2 remains the planned second-vendor generalization after the Arturia acquisition/install vertical.

## Follow-through

- [#90](https://github.com/kasselvania/Linux-VST-bridge/issues/90) retains residual delivery classes after AP16.
- [#77](https://github.com/kasselvania/Linux-VST-bridge/issues/77) retains lawful Serum 2 authorization and exact editor/product qualification.
- Further editor rendering/frame-pacing optimization remains later polish rather than the retired FRAGMENTS access defect.

These are not silently absorbed into AP17.

## Repository map

- `native-vst3-proxy/`: native Linux VST3 SDK proxy and Rust callback/transport backend;
- `native-audio-client/`: mapped transport and callback-facing primitives;
- `windows-factory-probe/`: supervised Windows VST3 SDK host, processing, state, and editor owners;
- `bridge-manager/`: environments, installation, profiles, publication/rollback, service admission, and supervision;
- `compatibility/`: immutable exact-fixture profile history;
- `docs/`: architecture, decisions, slice contracts, and reviewed results;
- `evidence/`: bounded retained identities and results, never proprietary plug-in binaries or license payloads.

Read [AGENTS.md](AGENTS.md), [CURRENT_SLICE.md](CURRENT_SLICE.md), and [docs/DESIGN_DOSSIER.md](docs/DESIGN_DOSSIER.md) before implementation. Rust is primary, with C++ at SDK and platform edges. The bridge remains independent: suitable Wine/Proton work is a runner beneath project-owned proxy, host, transport, state, and management boundaries—not a yabridge pivot.
