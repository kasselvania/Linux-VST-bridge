# Linux Audio Compatibility Bridge

A managed compatibility layer for using supported Windows VST3 plug-ins in native Linux DAWs without making musicians administer Wine prefixes, proxy synchronization, runner versions, or recovery machinery by hand.

```text
Bitwig on Linux
→ native Linux VST3 proxy
→ project-owned transport and supervision
→ pinned Proton/Wine environment
→ real Windows VST3 and vendor editor
```

**Experimental engineering preview, not a consumer-ready release.** The project is not affiliated with Bitwig, Valve, Steinberg, Arturia, Xfer Records, Native Instruments, or another plug-in vendor.

> **Ownership:** This repository is publicly readable but proprietary. Copyright © 2026 Peter Kassel. All rights reserved. Public visibility does not grant an open-source or redistribution license. See [COPYRIGHT.md](COPYRIGHT.md) and [CONTRIBUTING.md](CONTRIBUTING.md).

## Current project truth

Use these documents instead of reconstructing status from chronological campaign files:

- **Active branch work:** [CURRENT_SLICE.md](CURRENT_SLICE.md)
- **Supported products and platforms:** [docs/SUPPORT_MATRIX.md](docs/SUPPORT_MATRIX.md)
- **Shared failures, accepted fixes, and remaining gaps:** [docs/FAILURE_CLASSES.md](docs/FAILURE_CLASSES.md)
- **System architecture:** [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)
- **Engineering rules:** [AGENTS.md](AGENTS.md)

Historical AP/UIO/UIR/IF documents and `evidence/` remain authoritative for what a particular experiment observed. They are not, by themselves, current deployment or support authority.

## Accepted system

The bridge has established:

- a native Linux VST3 proxy loaded by Bitwig;
- a supervised Windows VST3 host under exact Proton/Wine runners;
- callback-safe bounded audio, events, parameter control, opaque state, project recall, automation, and independent instances;
- detached real vendor editors with explicit lifecycle and failure ownership;
- immutable profiles, revisioned publication, rollback, environment custody, and manager readback;
- Steam Deck product operation and Ubuntu portability/cold-boot operation for exact reviewed fixtures.

The accepted Steam Deck capacity envelope remains six simultaneous bridged DSP instances globally for the exact AP17 fixture, with lower product/class limits where recorded. Seven remains unqualified and eight was excluded for the tested workload. See [AP17 closure](docs/AP17_CLOSURE.md).

The supported performance posture remains **512 added frames per proxy**. The opt-in 256 setting remains available but unqualified. Residual startup, queue/reply, editor/removal, and shutdown-window misses remain tracked as [FC-AUDIO-001](docs/FAILURE_CLASSES.md#fc-audio-001--residual-audio-deadline-misses).

## Product posture

The currently exercised fleet includes Pure LoFi, Efx FRAGMENTS, Pigments, Serum 2, Blackhole Immersive, and Kontakt. Their current platform-specific claims, runner policies, limitations, and last physical evidence are maintained in [docs/SUPPORT_MATRIX.md](docs/SUPPORT_MATRIX.md).

A source patch or built runner is not a physical product pass. A pass on one exact plug-in does not establish the same result for another plug-in. Shared mechanisms and product coverage are tracked separately in [docs/FAILURE_CLASSES.md](docs/FAILURE_CLASSES.md).

## Architecture

The system separates four planes:

- **Management:** environments, runners, installers, vendor applications, profiles, publication, rollback, diagnostics, and operator UI.
- **Native host:** Linux VST3 proxy loaded by Bitwig.
- **Windows plug-in:** project-owned Windows host running the exact proprietary module under Proton.
- **Transport:** versioned control/state/editor channels and preallocated real-time audio/event memory.

Rust owns product state, supervision, transport, profiles, publication, and diagnostics. C++20 is contained at VST3 and Win32 SDK boundaries. Blocking I/O, allocation, process work, and logging remain outside audio callbacks.

## Repository map

- `native-vst3-proxy/` — native Linux VST3 SDK proxy and Rust callback/transport backend;
- `native-audio-client/` — mapped transport and startup/refusal primitives;
- `windows-factory-probe/` — supervised Windows VST3 host, processing, state, and editor owners;
- `bridge-manager/` — environments, installation/application ownership, profiles, publication/rollback, service admission, supervision, and manager readback;
- `compatibility/` — immutable exact-fixture profile history;
- `docs/` — architecture, current support/failure ledgers, decisions, and historical slice records;
- `evidence/` — bounded retained results; never proprietary plug-in, installer, preset, credential, or license payloads.

Read [AGENTS.md](AGENTS.md), [CURRENT_SLICE.md](CURRENT_SLICE.md), [docs/SUPPORT_MATRIX.md](docs/SUPPORT_MATRIX.md), and [docs/FAILURE_CLASSES.md](docs/FAILURE_CLASSES.md) before implementation. Proton/Wine is a runner beneath project-owned proxy, host, transport, state, installation, and management boundaries—not a replacement architecture.
