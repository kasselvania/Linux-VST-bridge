# Linux Audio Compatibility Platform

**A managed compatibility system for Windows audio software on Linux.**

The project currently has one implemented product mode and two staged expansion modes.

```text
CURRENT
native Linux DAW
→ Linux VST3 proxy
→ project-owned transport and supervision
→ isolated managed Windows plug-in environment
→ real Windows plug-in, editor, installer and vendor services

NEXT ADJACENT MODE
Windows DAW under Wine/Proton
→ Windows VST3 proxy
→ project-owned transport and supervision
→ isolated managed Windows plug-in environment
→ the same exact qualified plug-in candidate

LATER APPLICATION MODE
Linux desktop + audio/MIDI system
→ managed Windows DAW or creative application environment
→ its projects, directly hosted plug-ins, content, services and authorization
```

The objective is not merely to make a DLL or EXE open. The objective is to turn supported Windows audio software into controlled Linux products: installed through lawful vendor routes, exposed through stable host identities, observable when something fails, versioned, recoverable, and usable without making musicians administer Wine prefixes or reconstruct working combinations by hand.

> **Status:** experimental engineering preview, not a consumer-ready release or a broad compatibility claim. The project is not affiliated with Bitwig, Ableton, Cycling '74, Image-Line, Steinberg, Valve, Arturia, Xfer Records, Native Instruments, Kilohearts, Valhalla DSP, UVI, or another vendor.
>
> **Ownership:** this repository is publicly readable but proprietary. Copyright © 2026 Peter Kassel. All rights reserved. Public visibility does not grant an open-source, deployment, derivative-work, or redistribution license. Commercial use requires a separate written agreement. See [COPYRIGHT.md](COPYRIGHT.md) and [CONTRIBUTING.md](CONTRIBUTING.md).

## Why this project exists

Windows audio products contain years of DSP, editors, presets, installers, authorization systems, content and support investment. A native Linux port can be expensive. A generic Wine setup can run portions of that software, but often leaves the musician or support engineer responsible for runtime choice, prefix management, plug-in discovery, proxy synchronization, crash diagnosis and rollback.

This project treats compatibility itself as a product layer.

For musicians, the intended experience is eventually:

```text
install through the product's normal route
→ open the plug-in or application
→ make music
```

For vendors, the potential value is a controlled Linux channel that preserves the real Windows product and commercial systems while the compatibility platform supplies Linux integration, exact runtime policy, supervision, diagnostics, qualification and maintenance.

For the project owner and partners, each generic repair can become shared infrastructure rather than a one-product workaround. The commercial thesis is that later products should become less expensive to qualify as the compatibility suite grows.

Read [Project value and strategy](docs/PROJECT_VALUE.md), [Windows-host and application roadmap](docs/APPLICATION_COMPATIBILITY_ROADMAP.md), and [Vendor and application-partner integration](docs/VENDOR_INTEGRATION.md).

## Three product tracks

### Track A — native Linux DAW + managed Windows plug-in

This is the implemented and evidenced product mode.

```text
Bitwig / REAPER / another native Linux host
→ native Linux VST3 proxy
→ LVB transport
→ supervised Windows host
→ exact Windows plug-in
```

The platform owns:

- native VST3 processor/controller identity;
- bounded real-time audio and event transport;
- Windows component, controller, state and editor lifecycle;
- exact runner, environment, module and profile identity;
- installer, vendor application and service custody;
- candidate preparation, experimental publication, ordinary publication and rollback;
- crash containment, incident attribution and positive cleanup;
- retained evidence that separates generated fixtures from real DAW results.

This remains the implementation priority and first commercial product.

### Track B — Windows DAW under Wine + managed plug-in adapter

This is the closest architectural expansion.

A Windows DAW can load original Windows plug-ins directly when they work in the same environment. That direct route is a valid baseline:

```text
Windows DAW environment
→ original Windows VST3
```

The platform can later add a Windows-facing VST3 proxy for plug-ins that benefit from independent isolation:

```text
Windows DAW under Wine/Proton
→ LVB Windows VST3 proxy
→ Linux-owned broker / transport
→ isolated managed plug-in environment
→ original Windows VST3
```

The Linux proxy binary cannot be loaded unchanged by a Windows DAW. The intended reuse is a shared candidate and bridge core with two host-facing frontends:

```text
one exact qualified candidate
├── Linux VST3 publication → native Linux DAW
└── Windows VST3 publication → Windows DAW under Wine
```

Remote isolation is valuable when a plug-in needs another runner, conflicting vendor services, independent updates, stronger crash containment or reuse across several DAW environments. Direct hosting remains preferable where it is simpler and equally reliable.

For one exact DAW environment and VST3 class, the manager must expose exactly one route—direct or remote—to avoid duplicate class identity.

### Track C — complete managed Windows audio applications

This is a later strategic track.

```text
Linux desktop + PipeWire/ALSA/MIDI
→ managed Windows application environment
→ Live, FL Studio, Cubase, standalone Max, or another exact application
```

This mode reuses installation, environment, service, UI, incident, evidence and rollback machinery. It also needs substantial new application-level capabilities:

- low-latency Windows audio-device integration;
- MIDI, controller and synchronization behavior;
- full GPU, DPI, drag/drop, file-dialog and multi-window operation;
- project, pack, sample-library and content lifecycle;
- nested plug-in scanning and hosting;
- application self-update, service and elevation behavior;
- full-session recovery and sustained-performance qualification.

A tightly integrated family such as Live + Max for Live may remain application-local while third-party VSTs inside Live use either direct or remote managed hosting.

## Direct and isolated hosting are complementary

The platform should not force every product into one topology.

```text
Direct in DAW environment
- simplest path
- normal Windows VST ABI
- useful baseline
- one environment owns DAW and plug-in

Remote isolated through LVB
- independent runner/environment
- vendor-service isolation
- per-plug-in rollback
- crash containment
- one qualified installation exposed to multiple hosts
```

Hosting mode is evidence-backed profile policy, not a random end-user compatibility switch.

## More than “something under Proton”

Proton/Wine is an important execution layer, but it is not the whole product. This repository owns the system around it:

- exact environment and runner identity;
- native or Windows host adapters;
- application and process ownership;
- typed audio, state, editor and failure boundaries;
- Windows fixtures and generic compatibility laws;
- vendor installer and service supervision;
- profile-driven capability selection;
- immutable candidates, publication and rollback;
- private diagnostics and sanitized incident export;
- a user-facing manager and retained support history.

Yabridge is important prior art and addresses Windows plug-ins in native Linux hosts. This project is not a yabridge configuration project: the proxy, Windows host, transport, manager, profiles, tests, failure model and evidence custody are project-owned. The Windows-host adapter track extends the same managed-candidate model to DAWs running under Wine rather than replacing the native Linux-host mode.

## Compatibility should compound

The engineering loop is reusable:

```text
real product exposes a failure
→ retain the exact incident
→ identify the generic Windows, VST3, host or application concept
→ build the smallest source-owned reproducer
→ define and test the shared compatibility law
→ confirm it once against the real product
→ later products inherit the capability
```

Examples already produced by this process include:

- VST3 lifecycle restart and same-instance reconfiguration;
- fair Windows message retrieval under sustained posted work;
- direct vendor-editor lifecycle and same-DSP reopen;
- state capture and save/reopen;
- process-scoped retirement for unsafe SDK teardown;
- native failure containment after Windows-peer loss;
- bounded exception, module and stack attribution;
- managed installer environments and exact product discovery;
- explicit discovered → inspected → prepared → experimental → qualified crossings;
- multi-generation candidate history and rollback.

Installer process graphs, service failures, UI surfaces and runtime behavior are relevant to all three tracks. That is why the platform can expand without discarding the work already done.

## Current engineering proof

Within exact retained fixtures, the project currently demonstrates:

| Layer | Demonstrated capability |
| --- | --- |
| Native Linux DAW integration | Linux VST3 proxies with stable class and parameter identity in Bitwig. |
| Windows execution | Real Windows VST3 modules under supervised, pinned Proton/Wine environments. |
| Audio and events | Bounded audio, notes, automation, state, returned events and an exact auxiliary-input topology. |
| Vendor acquisition | Managed vendor applications and generic Windows-installer onboarding with isolated environments. |
| Discovery and preparation | Exact module/class discovery, class-specific inspection, native candidate construction and reversible experimental publication. |
| Editors | Real detached vendor editors, input fairness, focus, close/reopen and bounded UI diagnostics. |
| Persistence | Opaque state, project save/reopen and parameter recall in exact fixtures. |
| Management | Profiles, capacity, services, immutable publication, candidate lineage, rollback and recovery. |
| Failure behavior | Terminal custody, native-host containment, cleanup, private crash attribution and honest unknowns. |

The retained fixtures include Arturia Pure LoFi, Efx FRAGMENTS and Pigments, plus a managed Xfer Serum 2 installation and exact instrument candidate. Serum's operator work is positive cross-vendor evidence, not a universal support claim.

**512 added frames per proxy remains selected, supported and recommended.** The opt-in 256 setting remains available but unqualified.

## Roadmap logic

The evidence-backed sequence is:

1. finish the native-Linux-DAW plug-in platform;
2. complete generic Native Instruments-style multi-stage installer and service attribution;
3. broaden plug-in evidence across additional vendor families;
4. remove remaining agent-only ordinary manager workflows;
5. build a source-owned Windows VST3 host adapter and prove remote isolated hosting;
6. publish one candidate to both Linux-host and Windows-host frontends;
7. prove one Wine-hosted commercial DAW, with direct hosting as the comparison baseline;
8. measure direct versus isolated hosting;
9. build the broader managed Windows-application plane;
10. pursue Live + Max for Live as a high-leverage, separately licensed application family.

[The detailed gates are retained here](docs/APPLICATION_COMPATIBILITY_ROADMAP.md).

## Commercial pathways

The platform supports several non-exclusive business models:

- owner-operated end-user compatibility software;
- paid compatibility assessment and engineering;
- per-product, product-family or catalogue licensing;
- dual-host plug-in enablement for native Linux and Wine-hosted DAWs;
- DAW/application enablement and maintenance;
- OEM or white-label integration for vendors and Linux hardware partners;
- managed release qualification, incident analysis and support;
- joint upstream work where a generic defect belongs in Wine, Proton or another dependency.

No commercial terms, deployment rights or third-party software rights are granted by this repository.

## Architecture

The platform separates shared and mode-specific planes:

- **Management plane:** installers, applications, environments, runners, services, discovery, profiles, candidates, qualification, diagnostics, updates and rollback.
- **Windows execution plane:** pinned Wine/Proton runtime, process/service ownership, graphics/input behavior, content and authorization boundaries.
- **Linux-host plug-in plane:** native Linux VST3 proxy, Windows VST3 host and real-time transport.
- **Windows-host plug-in plane:** future Windows VST3 proxy, Linux-owned broker and isolated plug-in backend.
- **Future application plane:** Windows application audio/MIDI endpoints, project/content integration and nested plug-in hosting.
- **Evidence plane:** source-owned fixtures, real-product observations, incidents, exact versions, limitations and installed-state readback.

Rust owns product state, environment/application supervision, transport, profiles, publication and diagnostics. C++20 is contained at VST3 and Win32 SDK boundaries. Blocking I/O, allocation, process work and logging stay out of real-time audio callbacks.

## Repository map

- `native-vst3-proxy/` — current Linux VST3 frontend and Rust callback/transport backend;
- `native-audio-client/` — mapped transport and startup/refusal primitives;
- `windows-factory-probe/` — supervised Windows VST3 host, processing, state, editor and source-owned Windows fixtures;
- `bridge-manager/` — environments, installer/application ownership, profiles, publication/rollback, service admission, supervision and diagnostics;
- `compatibility/` — immutable exact-fixture profile history;
- `tools/` — bounded development and qualification tooling;
- `docs/` — architecture, decisions, product strategy and reviewed slice results;
- `evidence/` — bounded retained results; never proprietary plug-in, installer, preset, credential or license payloads.

## Read next

- [Project value and strategy](docs/PROJECT_VALUE.md)
- [Windows-host and application roadmap](docs/APPLICATION_COMPATIBILITY_ROADMAP.md)
- [Vendor and application-partner integration](docs/VENDOR_INTEGRATION.md)
- [Product design dossier](docs/DESIGN_DOSSIER.md)
- [Architecture](docs/ARCHITECTURE.md)
- [Current implementation slice](CURRENT_SLICE.md)
- [Working instructions](AGENTS.md)

The long-term objective is an invisible, managed compatibility layer: musicians use supported Windows audio software, vendors gain practical Linux channels, and every well-understood failure improves the shared platform rather than remaining an isolated workaround.
