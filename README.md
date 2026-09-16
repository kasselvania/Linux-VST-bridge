# Linux Audio Compatibility Platform

**A managed compatibility system for Windows audio software on Linux.**

The project is currently proven through Windows VST3 plug-ins presented to native Linux DAWs:

```text
native Linux DAW
→ native Linux VST3 proxy
→ project-owned transport and supervision
→ pinned Proton/Wine environment
→ real Windows plug-in, editor, installer and vendor services
```

The longer-term product direction adds a second, separate mode for complete Windows audio applications:

```text
Linux desktop + audio/MIDI system
→ managed Windows application environment
→ real Windows DAW or creative runtime
→ its own projects, plug-ins, content, services and authorization
```

The objective is not merely to make a DLL or EXE open. The objective is to turn supported Windows audio software into controlled Linux products: installable through lawful vendor routes, observable when something fails, versioned, recoverable, and usable without making musicians administer Wine prefixes or reverse-engineer every application by hand.

> **Status:** experimental engineering preview, not a consumer-ready release or a broad compatibility claim. The project is not affiliated with Bitwig, Ableton, Cycling '74, Image-Line, Steinberg, Valve, Arturia, Xfer Records, Native Instruments, Kilohearts, Valhalla DSP, UVI, or another vendor.
>
> **Ownership:** this repository is publicly readable but proprietary. Copyright © 2026 Peter Kassel. All rights reserved. Public visibility does not grant an open-source, deployment, derivative-work, or redistribution license. See [COPYRIGHT.md](COPYRIGHT.md) and [CONTRIBUTING.md](CONTRIBUTING.md). Commercial use requires a separate written agreement.

## Why this project exists

Windows audio products already contain years of DSP, editors, presets, project formats, installers, authorization systems, content and support investment. A full native Linux port can be expensive. A generic Wine setup can run portions of that software, but usually leaves the musician or support engineer responsible for runtime selection, prefix management, process cleanup, plug-in discovery, crash diagnosis and rollback.

This project treats compatibility itself as a product layer.

For musicians, the intended experience is eventually:

```text
install through the product's normal route
→ open the plug-in or application
→ make music
```

For software vendors, the potential value is a controlled Linux channel that can preserve the real Windows product and its commercial systems while the compatibility platform supplies Linux integration, exact runtime policy, supervision, diagnostics, qualification and maintenance.

For the project owner and partners, each generic repair can become shared infrastructure rather than a one-product workaround. The commercial thesis is that later products should become less expensive to qualify as the compatibility suite grows.

Read [Project value and strategy](docs/PROJECT_VALUE.md), [Application compatibility roadmap](docs/APPLICATION_COMPATIBILITY_ROADMAP.md), and [Vendor and application-partner integration](docs/VENDOR_INTEGRATION.md).

## Two product modes

### 1. Native Linux DAW + Windows plug-in

This is the implemented and evidenced mode today. A stable native Linux VST3 identity is loaded by the Linux DAW, while the real Windows module runs in a supervised Windows-shaped environment.

The platform owns:

- native VST3 processor/controller identity;
- bounded real-time audio and event transport;
- Windows component, controller, state and editor lifecycle;
- exact runner, environment, module and profile identity;
- installer, vendor application and service custody;
- candidate preparation, experimental publication, ordinary publication and rollback;
- crash containment, incident attribution and positive cleanup;
- retained evidence that distinguishes generated fixtures from real DAW results.

### 2. Managed Windows audio application

This is a strategic future track, not an implemented support claim. The Windows DAW or creative application would run as the managed product rather than behind a native VST3 proxy.

That mode can reuse substantial platform infrastructure:

- lawful installer, updater, account and authorization workflow;
- immutable environments and evidence-backed runner selection;
- multi-process and Windows-service custody;
- UI, input, popup, window and focus diagnostics;
- crash capture, module attribution and cleanup;
- application profiles, candidate rollout and rollback;
- project/content storage policy and support evidence.

It also requires new shared capabilities that the plug-in bridge does not automatically provide:

- a qualified low-latency Windows audio-device path into PipeWire/ALSA;
- MIDI, controller and synchronization behavior;
- full-application GPU, DPI, drag/drop, file-dialog and multi-window operation;
- project, pack, sample-library and content lifecycle;
- nested plug-in scanning and hosting inside the Windows DAW;
- application-specific service, elevation and self-update behavior;
- full-session recovery and performance qualification.

Running a DAW is therefore a natural extension of the compatibility platform, but not merely “a larger plug-in.”

## Why complete DAWs materially expand the opportunity

Plug-in compatibility serves Linux users who already prefer a native Linux DAW. Full application compatibility serves an additional group: musicians whose workflow is tied to a Windows/macOS-only DAW, project format, device ecosystem or embedded creative runtime.

A successful application mode could create value across several relationships:

- musicians seeking a managed Linux production environment;
- plug-in vendors whose Windows products can run in either native-Linux-DAW or Windows-DAW mode;
- DAW and creative-tool vendors exploring a bounded Linux channel;
- Linux hardware, handheld and workstation makers seeking a credible audio software catalogue;
- commercial studios or integrators requiring retained versions and supportable recovery.

Ableton Live is especially high-leverage as a later target because Max for Live is integrated with Live. A successful Live environment could potentially unlock Live itself, its bundled Max for Live runtime, user Max for Live devices and Windows plug-ins inside one managed application profile. Each of those claims would still require separate exact qualification and lawful licensing.

## More than “something under Proton”

Proton/Wine is an important execution layer, but it is not the whole product. This repository owns the system around it:

- exact environment and runner identity;
- native plug-in integration where applicable;
- application and process ownership;
- typed audio, state, editor and failure boundaries;
- Windows fixtures and generic compatibility laws;
- vendor installer and service supervision;
- profile-driven capability selection;
- immutable candidates, publication and rollback;
- private diagnostics and sanitized incident export;
- a user-facing manager and retained support history.

Yabridge remains important prior art, but this is not a yabridge configuration project. The plug-in proxy, Windows host, transport, manager, profiles, tests, failure model and evidence custody are project-owned. A future application mode would reuse the same management and compatibility platform without pretending the native VST3 proxy is the correct boundary for a complete DAW.

## Compatibility should compound

The engineering loop is deliberately reusable:

```text
real product exposes a failure
→ retain the exact incident
→ identify the generic Windows, audio, host or application concept
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

Installer process graphs, service failures, UI surfaces and runtime behavior are relevant to both plug-ins and complete applications. That is why the platform can grow beyond its initial VST3 proof without discarding the work already done.

## Current engineering proof

Within exact retained fixtures, the project currently demonstrates:

| Layer | Demonstrated capability |
| --- | --- |
| Linux DAW integration | Native Linux VST3 proxies with stable class and parameter identity in Bitwig. |
| Windows execution | Real Windows VST3 modules under supervised, pinned Proton/Wine environments. |
| Audio and events | Bounded audio, notes, automation, state, returned events and an exact auxiliary-input topology. |
| Vendor acquisition | Managed vendor applications and generic Windows-installer onboarding with isolated environments. |
| Discovery and preparation | Exact module/class discovery, class-specific inspection, native candidate construction and reversible experimental publication. |
| Editors | Real detached vendor editors, input fairness, focus, close/reopen and bounded UI diagnostics. |
| Persistence | Opaque state, project save/reopen and parameter recall in exact fixtures. |
| Management | Profiles, capacity, services, immutable publication, candidate lineage, rollback and recovery. |
| Failure behavior | Terminal custody, native-host containment, cleanup, private crash attribution and honest unknowns. |

The retained fixtures include Arturia Pure LoFi, Efx FRAGMENTS and Pigments, plus a managed Xfer Serum 2 installation and exact instrument candidate. Serum's first operator session showed a highly responsive real editor and sustained processing before exposing a strict processing-restart lifecycle boundary. That is positive cross-vendor evidence, not a broad Serum support claim.

**512 added frames per proxy remains selected, supported and recommended.** The opt-in 256 setting remains available but unqualified.

## Roadmap logic

The project should not jump directly from one working plug-in to a marketing claim about every DAW.

The evidence-backed sequence is:

1. finish the current generic installer/process-attribution and candidate-management work;
2. broaden plug-in evidence across additional independent vendor families, with Native Instruments, Kilohearts, Valhalla DSP and UVI as possible probes rather than promised support;
3. measure which capabilities transfer unchanged and which remain vendor-specific;
4. build a source-owned Windows audio-application fixture for audio device, MIDI, windowing, project storage and nested plug-in hosting;
5. expose application installation, launch, versioning, diagnostics and rollback through the manager;
6. select one full commercial DAW for a bounded proof;
7. treat Live, standalone Max and Max for Live as related but separately licensed and qualified products;
8. expand toward FL Studio, Cubase or other applications only through retained exact evidence.

[A detailed gate sequence is retained here](docs/APPLICATION_COMPATIBILITY_ROADMAP.md).

## Commercial pathways

The platform supports several non-exclusive business models:

- owner-operated end-user compatibility software;
- paid compatibility assessment and engineering;
- per-product, product-family or catalogue licensing;
- DAW/application enablement and maintenance;
- OEM or white-label integration for vendors and Linux hardware partners;
- managed release qualification, incident analysis and support;
- joint upstream work where a generic defect belongs in Wine, Proton or another dependency.

A complete audio-application mode would materially increase the value of the catalogue because the same manager could cover native Linux DAWs using bridged plug-ins and Windows DAWs using managed application environments.

No commercial terms, deployment rights or third-party software rights are granted by this repository.

## Architecture

The platform separates shared and mode-specific planes:

- **Management plane:** installers, applications, environments, runners, services, discovery, profiles, candidates, qualification, diagnostics, updates and rollback.
- **Windows execution plane:** pinned Wine/Proton runtime, process/service ownership, graphics/input behavior, content and authorization boundaries.
- **Plug-in plane:** native Linux VST3 proxy, Windows VST3 host and real-time transport.
- **Future application plane:** Windows application audio/MIDI endpoints, project/content integration and nested plug-in hosting.
- **Evidence plane:** source-owned fixtures, real-product observations, incidents, exact versions, limitations and installed-state readback.

Rust owns product state, environment/application supervision, transport, profiles, publication and diagnostics. C++20 is contained at VST3 and Win32 SDK boundaries. Blocking I/O, allocation, process work and logging stay out of real-time callbacks.

## Repository map

- `native-vst3-proxy/` — native Linux VST3 SDK proxy and Rust callback/transport backend;
- `native-audio-client/` — mapped transport and startup/refusal primitives;
- `windows-factory-probe/` — supervised Windows VST3 host, processing, state, editor and source-owned Windows fixtures;
- `bridge-manager/` — environments, installers, applications, profiles, candidates, publication/rollback, service admission, supervision and diagnostics;
- `compatibility/` — immutable exact-fixture profile history;
- `tools/` — bounded development and qualification tooling;
- `docs/` — architecture, decisions, commercial strategy and reviewed results;
- `evidence/` — bounded retained results; never proprietary installer, plug-in, project, preset, credential or license payloads.

## Read next

- [Project value and strategy](docs/PROJECT_VALUE.md)
- [Application compatibility roadmap](docs/APPLICATION_COMPATIBILITY_ROADMAP.md)
- [Vendor and application-partner integration](docs/VENDOR_INTEGRATION.md)
- [Product design dossier](docs/DESIGN_DOSSIER.md)
- [Architecture](docs/ARCHITECTURE.md)
- [Current implementation slice](CURRENT_SLICE.md)
- [Working instructions](AGENTS.md)

The long-term objective is an invisible, managed compatibility layer: musicians use supported Windows audio software on Linux, vendors gain a practical additional channel, and every well-understood failure improves the shared platform instead of remaining a fragile local workaround.