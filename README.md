# Linux Audio Compatibility Bridge

**A managed Windows VST3 compatibility platform for native Linux DAWs.**

```text
Linux DAW
→ native Linux VST3 proxy
→ supervised per-instance Windows host
→ exact compatibility profile
→ pinned Proton/Wine environment
→ real Windows VST3, editor, installer and vendor services
```

The goal is not merely to make a DLL load. The goal is to make supported Windows plug-ins behave like dependable Linux products: installable through their normal vendor route, discoverable by the DAW, musically usable, observable when something fails, and recoverable through exact retained versions rather than hand-maintained Wine configuration.

> **Status:** experimental engineering preview, not a consumer-ready release or broad compatibility claim. The project is not affiliated with Bitwig, Valve, Steinberg, Arturia, Xfer Records, or another plug-in vendor.
>
> **Ownership:** this repository is publicly readable but proprietary. Copyright © 2026 Peter Kassel. All rights reserved. Public visibility does not grant an open-source, deployment, derivative-work, or redistribution license. See [COPYRIGHT.md](COPYRIGHT.md) and [CONTRIBUTING.md](CONTRIBUTING.md). Commercial licensing and integration require a separate written agreement.

## Why this project exists

Windows audio software already contains years of DSP, editor, preset, installer, authorization, content and support investment. A full native Linux port can be expensive, while a generic Wine bridge often leaves musicians or support teams responsible for choosing runtimes, administering prefixes, synchronizing proxies, diagnosing crashes and preserving working configurations.

This project treats compatibility itself as a managed product layer.

For musicians, the intended experience is:

```text
install through a familiar vendor path
→ open the plug-in from the Linux DAW
→ make music
```

For plug-in manufacturers, the potential value is a lower-friction route to Linux availability that can preserve the existing Windows VST3, normal account and authorization systems, vendor-managed content, and product identity. The bridge supplies the Linux-facing proxy, runtime selection, supervision, compatibility policy, qualification evidence, failure attribution and rollback machinery.

For the project owner and commercial partners, each solved Windows behavior can become a shared capability rather than a one-product workaround. That creates a path toward a growing supported-plug-in catalogue whose marginal qualification cost should decrease as the compatibility suite matures.

Read [Project value and strategy](docs/PROJECT_VALUE.md) and [Vendor integration and commercial pathways](docs/VENDOR_INTEGRATION.md).

## More than “a bridge under Proton”

Proton is an important runtime layer, but Proton alone is not the product. This repository owns the surrounding system:

- native Linux VST3 identity presented to the DAW;
- a supervised Windows host for each plug-in instance;
- versioned audio, event, state and editor transport;
- exact runner and environment selection;
- vendor installer and application custody;
- declarative per-product compatibility profiles;
- immutable candidate publication, ordinary activation and rollback;
- UI/input observation and deterministic Windows fixtures;
- terminal-failure containment and native-host survival;
- bounded per-instance exception, module, RVA and stack attribution;
- retained evidence that distinguishes generated tests from real DAW results.

Yabridge remains important prior art, but this is not a yabridge configuration project. The bridge, Windows host, transport, manager, product profiles, tests and failure model are project-owned. Proton/Wine runs beneath those boundaries and can eventually be selected by exact product or environment profile where evidence justifies a different runtime.

## Compatibility should compound

The engineering loop is deliberately reusable:

```text
real plug-in exposes a failure
→ retain the exact incident
→ identify the generic Windows, VST3 or host concept
→ build the smallest source-owned reproducer
→ define and test the shared compatibility law
→ confirm it once against the real plug-in
→ later products inherit the capability
```

Examples already produced by this process include:

- repeated VST3 activation and same-instance reconfiguration;
- fair Windows message pumping under sustained posted work;
- direct vendor-editor lifecycle and same-DSP reopen;
- process-scoped retirement for plug-ins that cannot safely destroy every SDK object in-process;
- durable terminal-failure custody;
- contained native silence and a surviving failure presentation after Windows-peer loss;
- late-crash attribution with actual loaded-module identity, relative addresses and bounded stacks;
- current work on conventional owned popups and ownerless multi-surface transient UI.

The intended result is a compatibility kernel, not a growing collection of hard-coded product names and screen coordinates.

## Current engineering proof

The accepted system currently demonstrates, within exact retained fixtures:

| Layer | Demonstrated capability |
| --- | --- |
| Linux DAW integration | Native Linux VST3 proxies load in Bitwig and retain stable processor/controller identity. |
| Windows execution | Real Windows VST3 modules run inside supervised, pinned Proton/Wine environments. |
| Audio and events | Bounded stereo audio, notes, automation, returned events, state and one qualified Pigments auxiliary-input topology. |
| Vendor acquisition | Arturia Software Center installation, user-owned authentication, Pigments download/install and exact discovery. |
| Editors | Direct vendor editors, focus, close/reopen on the same DSP instance, generic input-pump fairness and bounded UI observation. |
| Persistence | Opaque vendor state, save/reopen, parameter automation recall and retained project identity. |
| Management | Exact profiles, immutable publications, controlled activation, rollback, services, capacity refusal and reboot/service recovery. |
| Failure behavior | Windows-peer failure custody, native-host survival, controlled terminal silence, failure presentation and positive cleanup. |
| Attribution | Opt-in private crash reports with process outcome, exception, exact module identity when verifiable, RVA, available stack, first bridge failure and cleanup facts. |

Current exact products include Arturia Pure LoFi 1.0.0.6121, Efx FRAGMENTS 1.0.0.2925 and Pigments 7.0.1.6772 under the retained Steam Deck / SteamOS 3.8.16 / Bitwig Studio 6.1 Flatpak fixture. These are engineering fixtures, not a general Arturia or Linux support claim.

The accepted multi-instance envelope for the exact LoFi/FRAGMENTS fixture is six simultaneous DSP instances globally, with narrower per-class and topology limits retained in [AP17 closure](docs/AP17_CLOSURE.md).

**512 added frames per proxy remains selected, supported and recommended.** The opt-in 256 setting remains available but unqualified. Residual delivery classes remain tracked in [#90](https://github.com/kasselvania/Linux-VST-bridge/issues/90).

## Opportunity for plug-in manufacturers

A manufacturer engagement can focus on the existing Windows product instead of beginning with a full native rewrite. Depending on the product, the work can include:

1. normal installer, account, authorization and content validation;
2. exact VST3 module and class discovery;
3. a product-specific Proton/Wine and environment profile;
4. generated tests for lifecycle, buses, state, events, editor and failure behavior;
5. real DAW qualification with explicit limits;
6. private incident reports suitable for joint vendor/bridge diagnosis;
7. controlled releases, rollback and ongoing compatibility maintenance.

Possible commercial structures include paid compatibility engineering, per-product or portfolio enablement, OEM/white-label integration, licensed compatibility profiles/runtime components, and ongoing release qualification and support. The repository does not publish commercial terms and does not grant any such rights by being visible.

The current Arturia vertical demonstrates the shape of such an engagement, but no Arturia partnership, endorsement or support relationship is claimed.

## Product direction

The near-term direction is:

1. finish the generalized transient-surface model and remaining Pigments editor/resize boundary;
2. continue normal-use crash capture rather than forcing reproduction campaigns;
3. qualify a second vendor, currently planned around Serum 2 and its lawful authorization path;
4. expand the compatibility catalogue while measuring whether later products become cheaper to qualify;
5. build a user-facing manager around installation, status, supported profiles, diagnostics, repair and rollback;
6. support evidence-backed per-product runtime selection rather than one mutable global Wine setup.

A plug-in may eventually select a different exact Proton/Wine generation or compatibility layer than another product. The architecture supports that direction through immutable runner and environment identities, but broad simultaneous multi-runner operation is not yet a consumer-qualified feature.

## Architecture

The system separates four planes:

- **Management:** environments, runners, installers, vendor applications, discovery, profiles, publication, rollback, diagnostics and future user-facing management.
- **Native host:** the Linux VST3 proxy loaded by the DAW.
- **Windows plug-in:** a project-owned Windows host running the real proprietary module under a selected Proton/Wine environment.
- **Transport:** versioned control, lifecycle, state and editor channels plus preallocated real-time audio/event memory.

Rust owns product state, environment/application supervision, transport, profiles, publication and diagnostics. C++20 is contained at the VST3 and Win32 SDK boundaries. Blocking I/O, allocation, process work and logging stay out of real-time audio callbacks.

The compatibility model is intentionally layered:

```text
shared compatibility kernel
→ runtime/environment profile
→ vendor-family policy
→ exact product/version profile
→ reviewed evidence and retained limits
```

## Repository map

- `native-vst3-proxy/` — native Linux VST3 SDK proxy and Rust callback/transport backend;
- `native-audio-client/` — mapped transport and startup/refusal primitives;
- `windows-factory-probe/` — supervised Windows VST3 host, processing, state, editor and source-owned Windows fixtures;
- `bridge-manager/` — environments, installer/application ownership, profiles, publication/rollback, service admission, supervision and diagnostics;
- `compatibility/` — immutable exact-fixture profile history;
- `tools/` — bounded development and qualification tooling;
- `docs/` — architecture, decisions, product strategy and reviewed slice results;
- `evidence/` — bounded retained results; never proprietary plug-in, installer, preset, credential or license payloads.

## Read next

- [Project value and strategy](docs/PROJECT_VALUE.md)
- [Vendor integration and commercial pathways](docs/VENDOR_INTEGRATION.md)
- [Product design dossier](docs/DESIGN_DOSSIER.md)
- [Architecture](docs/ARCHITECTURE.md)
- [Current implementation slice](CURRENT_SLICE.md)
- [Working instructions](AGENTS.md)

The long-term objective is an invisible, managed compatibility layer: musicians use supported plug-ins, manufacturers gain a practical Linux channel, and every well-understood failure improves the shared platform rather than remaining an isolated workaround.
