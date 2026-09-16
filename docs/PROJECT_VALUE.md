# Project value and strategy

## Thesis

Linux Audio Compatibility Platform is not only a transport between a Linux DAW and a Windows VST3. Its durable asset is an accumulating body of **versioned compatibility knowledge** for Windows audio software:

- exact runner and environment bindings;
- lawful installer, updater and authorization workflows;
- reusable Windows, VST3, UI, service and process fixtures;
- deterministic regressions for generic failure classes;
- product-specific compatibility profiles;
- real-product qualification evidence and explicit limits;
- private incident attribution and positive cleanup records;
- immutable candidate, publication and rollback history.

The current product mode presents Windows plug-ins to native Linux DAWs. The future strategic extension is to manage complete Windows audio applications—DAWs, creative runtimes and their plug-in ecosystems—through the same control, evidence and recovery platform.

The product opportunity is to make existing Windows audio software available on Linux with less friction than asking every musician, support engineer or software vendor to rediscover the same Wine, lifecycle, UI, installer, audio and failure behavior independently.

## The problem is larger than loading a module or starting an EXE

A useful commercial audio product is a stack:

```text
installer, updater and prerequisites
account, authorization and licensing
services and background helpers
content, presets, packs and sample libraries
graphics, input, file dialogs and multi-window UI
DSP, audio devices, MIDI and synchronization
plug-in discovery and hosting
opaque state, projects and recall
process lifetime, failure and cleanup
release updates and support incidents
```

A generic bridge or compatibility runner can solve part of the launch problem while leaving the rest to manual prefix administration and ad hoc troubleshooting. This project treats those surrounding concerns as first-class product responsibilities.

The end-user goal remains deliberately simple:

```text
install
→ open the product
→ make music
```

The system absorbs the compatibility machinery beneath that experience.

## Two related markets

### Windows plug-ins inside native Linux DAWs

The native Linux DAW remains responsible for projects, audio devices and the top-level workflow. The compatibility platform presents each supported Windows instrument or effect through a native Linux plug-in identity and supervises the actual Windows module.

This mode is already evidenced and is the current engineering foundation.

### Complete Windows audio applications on Linux

The compatibility platform manages the application itself: its environment, installer, updater, processes, services, authorization, audio/MIDI endpoints, UI, projects, plug-in catalogue and recovery.

This mode can serve users whose workflow depends on a Windows/macOS-only DAW or creative runtime. It can also allow Windows plug-ins to run inside their original Windows host application, creating a second route through the same managed catalogue.

The two modes share a platform but do not share every implementation detail. The native VST3 proxy is central to plug-in mode and irrelevant to the top-level execution of a complete DAW. Conversely, a full DAW requires audio-device and MIDI integration that an individual plug-in does not own.

## The compounding asset

The project is designed so that a real failure becomes a reusable platform capability:

```text
real product incident
→ durable evidence
→ generic concept
→ source-owned reproducer
→ shared compatibility law
→ focused product confirmation
```

A product-name exception helps one build. A generic law can support plug-ins, installers and complete applications that exhibit the same Windows behavior.

The same pattern has already produced reusable work around:

- VST3 lifecycle restart and same-instance reconfiguration;
- fair Win32 input retrieval under sustained posted work;
- editor creation, focus, close and reopen;
- exact state capture and save/reopen;
- process-scoped retirement where SDK teardown is unsafe;
- terminal-failure custody and native containment;
- bounded crash attribution after long-running output;
- managed Windows installer environments;
- immutable candidate generations and exact publication crossings;
- process/service cleanup and versioned recovery.

The desired economic effect is:

> Each generic repair should reduce the engineering cost and uncertainty of qualifying the next product—whether that product is a plug-in or an application.

That hypothesis must be measured. Useful measures include:

- time from lawful installer to first usable UI;
- time to first audio and MIDI;
- number of new generic failure classes;
- product-specific policy count;
- fixture and test reuse;
- incident-to-diagnosis time;
- update and maintenance cost;
- percentage of qualification completed without computer-control intervention;
- percentage of applications sharing an existing runner and environment policy.

## Shared, plug-in-specific and application-specific layers

### Shared compatibility kernel

The shared kernel owns behavior that should not depend on a product name:

- installer and process supervision;
- exact runner/environment management;
- Windows services and helper ownership;
- graphics/input/window behavior;
- terminal failure and cleanup;
- crash attribution;
- candidate, evidence, publication and rollback;
- application and product manager workflow.

Fixes here can benefit every admitted product.

### Plug-in mode

Plug-in mode additionally owns:

- native Linux VST3 identity;
- native/Windows real-time transport;
- VST3 lifecycle, buses, events, parameters and state;
- per-instance editor and DSP ownership;
- stable DAW project identity.

### Application mode

Application mode additionally requires:

- qualified low-latency audio-device integration;
- MIDI, controllers, clock and synchronization;
- complete GPU/DPI/multi-window behavior;
- project, pack, sample and content storage;
- nested plug-in scanning and hosting;
- app-specific updater, service and elevation behavior;
- full-session recovery and performance policy.

A roadmap that ignores these differences would overstate transfer. A roadmap that ignores the shared kernel would understate the value already built.

## Declarative compatibility profiles

Profiles bind exact product facts and closed policy choices:

- application or module identity;
- native proxy and Windows host where applicable;
- runner and environment revision;
- qualified audio/MIDI topology;
- editor, accessibility and retirement policy;
- installer/service requirements;
- content/project storage policy;
- known limitations and evidence.

Profiles do not contain arbitrary code, secrets, proprietary payloads or licensing bypasses. They select reviewed capabilities implemented by the shared system.

A future application profile should not become a shell script disguised as configuration. It should remain a closed, inspectable contract with exact identities and rollback.

## Evidence and incidents

Evidence separates what is implemented from what has actually been demonstrated:

- generated source-owned fixtures;
- exact real-product checks;
- private incidents;
- sanitized shareable reports;
- installed-state readback;
- limitations and nonclaims.

This allows the project to say:

- “the generic mechanism works”;
- “this exact plug-in passed”;
- “this application launched but audio is unqualified”;
- “this failure remains unattributed”;

without collapsing those statements into one marketing claim.

## Why complete DAWs increase the business value

A plug-in catalogue primarily serves Linux users already committed to a native Linux DAW. A supported Windows DAW catalogue reaches users whose projects, habits, collaborators, devices or creative systems depend on a Windows/macOS-only application.

That increases the possible value of the platform in several ways:

1. **Broader end-user demand.** The product can address both “I need this plug-in” and “I need this entire workflow.”
2. **Higher switching-cost problems.** DAW projects, Max for Live devices, controller scripts and collaboration formats can be more central than any one plug-in.
3. **Larger vendor relationships.** DAW vendors, plug-in manufacturers, Linux hardware makers and workstation integrators can all be potential partners.
4. **Catalogue reuse.** Windows plug-ins qualified in one managed environment may later be useful inside a managed Windows DAW, subject to separate exact evidence.
5. **More valuable management software.** One application can own installers, versions, environments, support evidence and rollback for both DAWs and plug-ins.
6. **Stronger OEM potential.** A Linux workstation or handheld can be sold around a supportable audio-software matrix rather than a collection of community recipes.

The support burden also rises sharply. A full DAW owns more surfaces, more hardware behavior and more project data. The business opportunity is real only if the platform preserves exact scope and does not turn every user machine into an unsupported Wine experiment.

## Why Live and Max for Live are strategically attractive

Ableton Live is a particularly high-leverage later target because Max for Live is integrated into Live. A successful exact Live environment could potentially unlock:

- Live projects and workflows;
- Live's Windows plug-in hosting;
- the bundled Max for Live runtime where licensed;
- third-party Max for Live instruments and effects;
- controller and hardware integrations built through Max for Live.

This does not make Max for Live automatic. Live, standalone Max and Max for Live have separate licensing and product boundaries. Each requires exact authorization, runtime, project, UI, audio/MIDI and failure qualification.

The correct commercial statement is:

> Live support could create a high-leverage application family, not that current plug-in success proves Live or Max for Live compatibility.

## Why Proton/Wine remains useful

Proton/Wine provides a mature Windows-shaped execution layer beneath the project-owned system. The value comes from making that layer exact and managed:

```text
product profile
→ exact runner generation
→ exact environment family
→ exact application or Windows host
→ exact plug-in/application artifacts
```

Different products may need different validated runtime generations. A legacy plug-in, a modern DAW, a vendor service and a content manager do not necessarily share the same runtime posture.

Runtime selection should not become a user-facing “try random Proton versions” control. It belongs to reviewed profiles with evidence and rollback.

## Value to musicians

The intended value is less compatibility administration:

- no hand-selected loader on every launch;
- no manual proxy synchronization;
- no hidden prefix and service cleanup work;
- lawful vendor installation and authorization where supported;
- normal product/project identity;
- visible failure instead of a silent zombie;
- exact rollback when an update fails;
- diagnostics that can be handed to support;
- a published matrix of what is and is not qualified.

The system should eventually feel like infrastructure, not a hobbyist assembly exercise.

## Value to plug-in manufacturers

A managed compatibility layer may offer a commercially attractive intermediate or long-term Linux path compared with a complete native port.

Potential value includes:

- reuse of the shipping Windows VST3 and editor;
- preservation of normal account, licensing and content systems;
- a bounded Linux support matrix;
- deterministic Windows/VST3 regressions;
- private exception/module/stack reports;
- controlled candidate rollout and rollback;
- release qualification and maintenance.

## Value to DAW and application vendors

A full application engagement can preserve the Windows application while the platform supplies:

- a managed Linux installation and launch experience;
- audio/MIDI integration with an exact supported matrix;
- process, service, updater and crash custody;
- project/content storage policy;
- nested plug-in and library validation;
- hardware/controller qualification;
- versioned release channels and rollback;
- private support evidence.

This is not a promise that every application can avoid native Linux work. Deep drivers, copy protection, kernel services, graphics assumptions or support requirements may require vendor changes or make a compatibility route unsuitable.

## Commercial paths

### Owner-operated product

The project owner can build and maintain a supported catalogue of plug-ins and applications, charge for access, updates, support or managed environments, and expand the matrix over time.

### Paid compatibility engineering

A vendor or integrator can fund an exact proof, product qualification, release repair or portfolio assessment.

### Per-product or catalogue license

A partner can license specific compatibility profiles and platform components for one product, a family, or a broader catalogue.

### DAW/application enablement

A vendor can fund a managed Linux channel for a complete Windows audio application, including installation, audio/MIDI, plug-ins, content and ongoing release qualification.

### OEM or white-label integration

The compatibility system can be embedded into a vendor, Linux workstation, handheld or distribution partner offering under agreed branding and support terms.

### Managed compatibility service

The project owner can operate qualification, incident analysis, profile/runtime updates and support as an ongoing service.

### Joint upstream work

Where a demonstrated problem belongs in Wine, Proton or another dependency, exact source-owned reproducers can support high-quality upstream fixes.

No commercial terms or rights are granted by this document or repository visibility.

## Roadmap logic

The roadmap should expand through gates rather than aspiration:

1. strengthen generic multi-stage installer, service and application custody;
2. broaden plug-in evidence beyond Arturia and Xfer;
3. use Native Instruments and selected independent vendors such as Kilohearts, Valhalla DSP and UVI as possible probes;
4. measure marginal work and eliminate product-name implementation branches;
5. build a source-owned Windows audio-application fixture;
6. qualify the application audio/MIDI, UI, project and nested-hosting foundation;
7. select one commercial DAW for a bounded proof;
8. consider Ableton Live and Max for Live after the foundation, not before it;
9. expand toward FL Studio, Cubase and other applications only through exact evidence;
10. publish a supportable catalogue rather than a universal-compatibility claim.

See [Application compatibility roadmap](APPLICATION_COMPATIBILITY_ROADMAP.md).

## Guardrails

The broader commercial thesis does not weaken the engineering standard.

- No authorization or licensing bypass.
- No bundled third-party installers or binaries without explicit rights.
- No proprietary projects, presets, credentials or opaque state committed as evidence.
- No universal support claim from one clean launch.
- No relabeling generated fixtures as real-product proof.
- No product-name dispatch where a generic law can be expressed.
- No diagnostic I/O, blocking or allocation in real-time callbacks.
- No publication or promotion without exact identity and rollback.
- No automatic sharing of private incidents.
- No implication of vendor affiliation, endorsement or official support.
- No treating a Windows DAW as “working” before audio, MIDI, projects, plug-ins and cleanup have exact evidence.

## Success condition

The project succeeds when supported Windows audio products can be delivered on Linux as controlled, maintained products rather than fragile local experiments—and when each qualification improves the platform enough that the next plug-in or application becomes faster, cheaper and easier to support.