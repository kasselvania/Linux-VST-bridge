# Linux Audio Compatibility Bridge

A managed compatibility layer for using supported Windows VST3 plug-ins in native Linux DAWs without making musicians administer Wine prefixes, proxy synchronization, runner versions, or recovery machinery by hand.

```text
Bitwig on Linux
→ native Linux VST3 proxy
→ project-owned transport and supervision
→ pinned Proton/Wine environment
→ real Windows VST3 and vendor editor
```

**Experimental engineering preview, not a consumer-ready release.** The project is not affiliated with Bitwig, Valve, Steinberg, Arturia, Xfer Records, or another plug-in vendor.

> **Ownership:** This repository is publicly readable but proprietary. Copyright © 2026 Peter Kassel. All rights reserved. Public visibility does not grant an open-source or redistribution license. See [COPYRIGHT.md](COPYRIGHT.md) and [CONTRIBUTING.md](CONTRIBUTING.md).

## Accepted system

AP8–AP11 established the native Linux VST3 proxy, supervised Windows host under a pinned Proton environment, bounded audio/events, opaque state, project recall, automation, independent instances, and real detached vendor editors.

AP12 installed the exact Pure LoFi → Efx FRAGMENTS vertical with automatic service startup. AP13 removed a demonstrated state-capture delivery barrier and added the inactive-only 256/512 delay selector. AP14 added exact profiles, manager-derived registration, immutable publication, recovery, and rollback. AP15 made the real vendor editor open directly from Bitwig and close/reopen on the same DSP instance. AP16 moved hot session mappings from journaled storage to private tmpfs, removing one measured 27–29 ms preparation-stall class.

AP17 is integrated at:

```text
2329706a6e797137e68d719edbbbe5cc1e0cdbf1
```

It establishes the exact Steam Deck / Bitwig / pinned Arturia operating envelope:

- 16 bounded service workers;
- 6 simultaneous DSP instances globally;
- at most 3 Pure LoFi instances;
- at most 4 Efx FRAGMENTS instances;
- native hard capacity of 4 per loaded class image;
- 3 qualified parallel tracks;
- serial bridged depth 3;
- 2 simultaneous direct vendor editors exercised;
- 7 instances remain unqualified;
- 8 were excluded for the tested workload.

The result also covers typed over-capacity refusal before partial ownership, exact one-unit removal/replacement, same-class save/reopen, one-host failure containment, service restart, bounded unexpected service loss, and normal reboot recovery. See [AP17 closure](docs/AP17_CLOSURE.md).

One historical Bitwig quit left the frontend/audio-engine alive after all bridge owners had retired. It remains retained and unexplained; it did not recur in the focused matrix or two complete repetitions of the triggering sequence. No causal repair is claimed. SteamOS `foreground_booster` was identified as the writer of quit-time CPUWeight overrides; that is a controlled-performance confound, not a product requirement to disable normal SteamOS behavior.

Revision 10 is the accepted ordinary AP17 source generation. Revision 9 and 8 remain candidate history; revision 7 is its immediate rollback generation and revision 3 remains earlier retained ancestry. The Deck was deliberately left on revision 7 after the final investigation. AP18 may activate revision 10 once through the existing bounded transition without replaying AP17.

## Current exact fixture

The accepted claims remain deliberately narrow:

- Steam Deck / SteamOS 3.8.16;
- Bitwig Studio 6.1 Flatpak;
- exact pinned Proton-SLR runner and Arturia environment;
- Pure LoFi 1.0.0.6121;
- Efx FRAGMENTS 1.0.0.2925;
- 48 kHz, float32, bounded main stereo/event behavior;
- exact reviewed Windows host and Linux native artifacts.

This is not a claim of broad Arturia, Linux, DAW, VST3, hardware, or customer-installation support.

## Performance posture

**512 added frames per proxy is selected, supported, and recommended.** The opt-in 256 setting remains available but unqualified.

At 48 kHz, the accepted LoFi → FRAGMENTS chain reports 1,264 frames / 26.333 ms across the two bridged devices, excluding DAW, audio-device, and acoustic latency. Three serial bridged devices in AP17 reported approximately 41 ms per tested chain.

Residual startup, queue/reply, editor/removal, and shutdown-window delivery classes remain tracked in [#90](https://github.com/kasselvania/Linux-VST-bridge/issues/90). A narrow fail-closed capacity-scan/lease-retirement race is tracked in [#93](https://github.com/kasselvania/Linux-VST-bridge/issues/93); it can cause a temporary unnecessary refusal but does not permit over-admission or invalidate AP17.

FRAGMENTS' Advanced panel is accessible and rendering response has improved materially. Further frame-pacing polish is not a current functional blocker.

## Active slice: AP18 — Arturia Software Center to Pigments

AP18 is tracked in [#94](https://github.com/kasselvania/Linux-VST-bridge/issues/94) from integrated AP17 main.

ASC is installed and the operator installed and authorized Pigments through its
real UI. Exact Pigments 7.0.1.6772 candidate revision 10 now passes the bounded
LC1 same-session routing check, note audio, preset/control automation recall,
sibling independence and process-scoped retirement. It remains a ReviewCandidate,
restored inactive pending independent review. LoFi and FRAGMENTS ordinary
revision 10 remain active and unchanged. See the [LC1 result](evidence/ap18/lc1/live-completion.json).

```text
official ASC installer
→ managed ASC application
→ user-owned Arturia authentication
→ Pigments download/install
→ exact module/class/resource discovery
→ immutable candidate publication
→ editor, preset, notes, automation, state, save/reopen and cleanup
```

The manager owns ASC launch/status and exact discovery; Arturia's real UI owns
account, licensing, catalogue and download operations. Pigments uses a retained
editor view and explicit process-scoped final retirement, not clean SDK object
destruction. Residual delivery gaps remain #90. 512 added frames remain
recommended; 256 and multi-instance Pigments capacity remain unqualified.

Read [CURRENT_SLICE.md](CURRENT_SLICE.md) and [docs/AP18.md](docs/AP18.md) for active authority and implementation boundaries.

Serum 2 remains the planned second-vendor generalization after the Arturia acquisition/install vertical.

## Architecture

The system separates four planes:

- **Management:** environments, runners, installers, vendor applications, profiles, publication, rollback, diagnostics, and future UI.
- **Native host:** Linux VST3 proxy loaded by Bitwig.
- **Windows plug-in:** project-owned Windows host running the exact proprietary module under Proton.
- **Transport:** versioned control/state/editor channels and preallocated real-time audio/event memory.

Rust owns product state, environment/application/installer supervision, transport, profiles, publication, and diagnostics. C++20 is contained at the VST3 and Win32 SDK boundaries. Blocking I/O, allocation, process work, and logging stay out of audio callbacks.

## Repository map

- `native-vst3-proxy/` — native Linux VST3 SDK proxy and Rust callback/transport backend;
- `native-audio-client/` — mapped transport and startup/refusal primitives;
- `windows-factory-probe/` — supervised Windows VST3 host, processing, state, and editor owners;
- `bridge-manager/` — environments, installer/application ownership, profiles, publication/rollback, service admission, and supervision;
- `compatibility/` — immutable exact-fixture profile history;
- `docs/` — architecture, decisions, slice contracts, and reviewed results;
- `evidence/` — bounded retained results; never proprietary plug-in, installer, preset, credential, or license payloads.

Read [AGENTS.md](AGENTS.md), [CURRENT_SLICE.md](CURRENT_SLICE.md), and [docs/DESIGN_DOSSIER.md](docs/DESIGN_DOSSIER.md) before implementation. The bridge remains independent: Proton/Wine is a runner beneath project-owned proxy, host, transport, state, installation, and management boundaries—not a yabridge configuration project.
