# Vendor and application-partner integration

## Audience and status

This document describes how a plug-in manufacturer, DAW/application vendor, distributor, Linux hardware maker, system integrator or commercial partner could work with Linux Audio Compatibility Platform.

The project is an experimental engineering preview, not a production support offer. No partnership, endorsement, license, price or service commitment is created by this document. The repository is proprietary and all commercial use requires a separate written agreement with Peter Kassel.

## The partner proposition

A vendor may be able to expand Linux availability without first rebuilding the entire Windows product stack as a native Linux product.

The platform is designed to preserve as much of the existing Windows product as practical:

- real Windows plug-in or application binaries;
- vendor DSP, editor, project and preset behavior;
- normal installer and updater;
- normal account and authorization systems;
- vendor-managed services, content and resources;
- stable product and project identity.

The compatibility platform supplies Linux-facing and operational layers:

- native Linux VST3 representation for plug-in mode;
- managed Windows application execution for application mode;
- exact Proton/Wine runtime selection;
- isolated environments and process/service ownership;
- typed audio, MIDI, event, state and editor boundaries where applicable;
- product-specific compatibility policy;
- installation, discovery, candidate preparation, publication and rollback;
- generated Windows, VST3, installer and application fixtures;
- real-product qualification;
- failure containment and private incident attribution.

This can be a lower-cost route for some products, but it is not a universal substitute for native Linux development. Feasibility depends on graphics, drivers, licensing, services, content, update model, audio/MIDI requirements, hardware assumptions and support expectations.

## Two engagement tracks

### Plug-in enablement

The vendor's real Windows instrument or effect is presented through a native Linux plug-in identity in a supported native Linux DAW.

Typical platform responsibilities include:

- module/class discovery;
- VST3 component/controller lifecycle;
- audio, event, parameter and state transport;
- editor and input behavior;
- stable project identity;
- exact publication and rollback.

### Application enablement

The complete Windows application is the managed product.

Typical platform responsibilities include:

- application installer, updater and services;
- low-latency audio-device integration;
- MIDI, controller and synchronization behavior;
- project/content/library storage;
- full UI, graphics, file dialog and multi-window behavior;
- nested plug-in scanning and hosting;
- application launch, recovery and rollback.

An application engagement can include a DAW, standalone creative runtime, vendor software center, editor, librarian or other audio production tool. It should not be presented as equivalent to plug-in mode merely because both run under Wine/Proton.

## What an engagement can look like

### Stage 1 — Compatibility assessment

The first stage establishes the actual product boundary.

Typical work includes:

- inventorying installers, applications, services, modules and content;
- identifying supported product and runtime versions;
- validating the lawful authorization path;
- checking graphics, input, popups, focus, file and process assumptions;
- identifying audio/MIDI and hardware dependencies;
- discovering exact plug-in classes or application executables;
- checking whether existing compatibility capabilities apply;
- producing an explicit feasibility result, risks and unqualified areas.

Deliverable: a scoped technical assessment and recommended proof plan.

### Stage 2 — Exact product enablement

The product is bound to a controlled profile and environment.

For a plug-in this may include:

- selecting and pinning a runner;
- creating or adopting a managed vendor environment;
- qualifying installer, authorization and content behavior;
- binding the exact module, class, host, native proxy and descriptor;
- producing an immutable candidate with rollback.

For an application this may include:

- binding the exact application, updater, services and prerequisites;
- defining private project/content/cache locations;
- selecting an exact audio/MIDI integration contract;
- retaining application launch and service ownership;
- establishing a bounded plug-in/content discovery policy;
- producing an immutable application candidate with rollback.

Deliverable: an exact candidate that can be exercised in the target Linux fixture.

### Stage 3 — Qualification

Generated tests and real product use remain separate.

A plug-in qualification may cover:

- audio, notes, events and automation;
- buses, sidechain and auxiliary input;
- presets, opaque state and save/reopen;
- editor, focus, popups, resizing and DPI;
- lifecycle reconfiguration;
- process failure, containment and cleanup;
- capacity, latency and retained performance limits.

An application qualification may additionally cover:

- audio-device enumeration, sample rates, buffers and channel topology;
- MIDI input/output, clocks, control surfaces and remote scripts;
- project creation, save, reopen, backup and migration;
- file dialogs, drag/drop, packs, libraries and content indexing;
- nested VST/CLAP scanning and plug-in operation;
- multi-window and full-screen behavior;
- updater, service and self-relaunch behavior;
- crash/session recovery;
- hardware and Linux distribution matrix.

Deliverable: a reviewed compatibility claim with exact versions, evidence, limitations and rollback ancestry.

### Stage 4 — Distribution integration

A production relationship may package approved runtime and profiles into a vendor, partner or platform-managed experience.

Possible components include:

- branded or co-branded installer/manager flow;
- profile and runtime delivery;
- application and plug-in catalogue;
- update and rollback policy;
- supported hardware/DAW/application matrix;
- user-visible status and diagnostics;
- support handoff and sanitized incident export;
- telemetry only if separately designed, disclosed and authorized;
- release signing and distribution arrangements.

Deliverable: a distribution and support model agreed by the parties.

### Stage 5 — Ongoing maintenance

Windows products, Wine/Proton, DAWs, graphics stacks, Linux distributions and hardware change. A supportable product requires controlled maintenance.

Typical recurring work includes:

- qualifying vendor releases;
- updating exact profiles and runtime bindings;
- triaging private incidents;
- maintaining deterministic regressions;
- assessing runner upgrades;
- preserving project compatibility and rollback;
- updating the support matrix;
- escalating generic runtime defects upstream where appropriate.

Deliverable: a release and compatibility maintenance service.

## High-leverage application families

Some application families can create more value than a single executable.

Ableton Live is an example because Max for Live is integrated with Live. A successful exact application profile could potentially cover:

- Live itself;
- Windows plug-ins hosted by Live;
- the bundled Max for Live runtime where licensed;
- third-party Max for Live devices;
- controller and hardware workflows built through Live/Max.

Those are related but separate qualification claims. Live working does not automatically prove Max for Live editing, every device, standalone Max, every plug-in or every controller.

FL Studio and Cubase are other examples of complete Windows audio workstations with their own plug-in hosts, projects, graphics, audio/MIDI requirements, licensing and update systems. They belong after the generic application foundation, not as product-name branches in the current plug-in bridge.

No Ableton, Cycling '74, Image-Line or Steinberg partnership is claimed.

## Possible commercial structures

Commercial terms are deliberately not fixed in this repository.

### Paid proof or consulting engagement

A bounded assessment or product-enablement contract. The partner receives a technical result and can decide whether to continue.

### Per-product license

A license covering platform/runtime/profile integration for one exact plug-in or application.

### Portfolio or catalogue license

A broader agreement covering several products, shared vendor infrastructure and ongoing qualification.

### DAW/application enablement license

An agreement covering a complete application environment, audio/MIDI integration, nested plug-in support, updates and maintenance.

### OEM or white-label integration

The compatibility system is embedded into a vendor, Linux workstation, handheld or distribution partner offering under agreed branding and support terms.

### Managed compatibility service

The project owner operates and maintains the compatibility layer, qualification catalogue and incident workflow.

### Joint engineering and upstream work

The parties collaborate on source-owned reproducers, vendor-side changes or Wine/Proton improvements. Product-specific confidential evidence can remain private while generic fixes are shared under separately agreed terms.

These structures can be combined. A proof can lead to a product license; a product license can expand into a catalogue or managed-maintenance relationship.

## Division of responsibility

A commercial agreement should define responsibility explicitly.

### Vendor or application partner may retain responsibility for

- Windows binaries and intellectual property;
- account, authorization and licensing systems;
- product content, projects and presets;
- Windows product correctness;
- product support knowledge;
- approval of supported versions and distribution;
- vendor-side fixes where the failure belongs in the product;
- application-specific hardware or cloud services.

### Platform provider may retain responsibility for

- native Linux proxy where applicable;
- managed Windows application environment;
- audio/MIDI compatibility boundaries;
- exact runtime/environment management;
- compatibility profiles and qualification tooling;
- installation/discovery integration;
- process/service supervision, containment and cleanup;
- platform-side fixes;
- bounded incident capture and support evidence;
- publication, update and rollback machinery.

### Shared responsibility may include

- selecting the supported product/runtime/hardware matrix;
- interpreting mixed vendor/runtime failures;
- release qualification;
- end-user documentation;
- incident escalation;
- support-service levels;
- upstream reports and patches.

The platform must never bypass or duplicate vendor authorization. A partnership should make the lawful path clearer, not weaker.

## Compatibility profiles and runtime selection

A profile can bind closed, reviewed capabilities such as:

- runner and environment revision;
- application/module/host/native artifact identity;
- audio and MIDI topology;
- event, state and editor policy;
- process, service and retirement behavior;
- Windows accessibility posture;
- project/content storage;
- qualified latency and performance posture;
- retained limitations.

Different products may share one environment, while others may require isolated runtime generations. Runtime selection is evidence-backed policy, not an arbitrary user experiment.

A profile does not contain arbitrary code, secrets, licensing state or destructive commands.

## Test and incident value

The growing source-owned suite is part of the commercial value.

A partner should not need to reproduce every failure manually. The platform can convert an incident into a reusable test for concepts such as:

- lifecycle start/stop/reconfiguration;
- state and project capture;
- message-pump fairness;
- focus and pointer capture;
- transient window groups;
- process exit and containment;
- installer bootstrapper/prerequisite/service graphs;
- self-update and relaunch;
- audio-device restart;
- MIDI and controller behavior;
- nested plug-in scanning;
- module identity and exception attribution.

Private reports can retain exact process outcome, module identity, relative addresses, bounded stacks, first platform failure and cleanup state. A sanitized export can support collaboration without publishing credentials, private paths, vendor state or proprietary payloads.

A faulting module or nearby UI action remains evidence, not automatic proof of root cause.

## Current case studies

### Arturia vertical

```text
Arturia Software Center
→ user-owned sign-in and authorization
→ Pigments installation
→ exact VST3 discovery
→ managed publication
→ notes, presets, automation, state and auxiliary input
→ editor/input compatibility
→ terminal failure containment
→ private crash attribution
```

Pure LoFi and Efx FRAGMENTS also demonstrate exact-fixture instruments/effects, state recall, independent removal, capacity and rollback behavior.

### Xfer vertical

The manager imported the Xfer installer, created a managed environment, discovered Serum 2 instrument and FX classes, prepared an exact instrument candidate and retained a cross-vendor operator session. The highly responsive editor and sustained first processing interval are encouraging; a later strict lifecycle restart boundary remains retained rather than hidden.

These are technical case studies only. No vendor has endorsed the project and no partnership or commercial relationship is claimed.

## What a mature engagement can deliver

Depending on scope:

- an exact supported-product profile;
- pinned runtime/environment artifacts;
- deterministic compatibility fixtures;
- a real-product qualification record;
- installer, authorization and service integration;
- a support and limitation matrix;
- candidate/publication and rollback history;
- private incident-report tooling;
- a release-qualification process;
- a roadmap for remaining unsupported behavior;
- integration or distribution rights defined by contract.

The intended value is not a one-time demo. It is a maintained route from the partner's existing Windows product to a supportable Linux offering.

## Engagement principles

- Use the real vendor installer and product where practical.
- Preserve vendor licensing and authorization.
- Do not publish proprietary binaries, projects, presets, state or credentials.
- Distinguish generated tests from real-product proof.
- State exact supported versions and limits.
- Fix generic problems at the shared layer.
- Keep product-specific policy declarative and narrow.
- Retain rollback before activating updates.
- Stop repetitive testing when it produces no new evidence.
- Treat private incidents as sensitive by default.
- Do not claim a partnership until one exists.
- Do not market a DAW as supported before audio, MIDI, projects, plug-ins and cleanup have exact evidence.

## Intellectual property and licensing

The original platform source, documentation, profiles, fixtures and designs in this repository are proprietary. Public visibility does not grant use, modification, redistribution, commercial deployment or derivative-work rights.

Third-party products and dependencies remain owned and licensed by their respective parties. A commercial engagement would define rights for platform use, integration, distribution, support, branding, profiles, fixtures and modifications separately in writing.

See [COPYRIGHT.md](../COPYRIGHT.md) and [CONTRIBUTING.md](../CONTRIBUTING.md).

## Next commercial proof points

The valuable near-term proof is that the cost curve improves:

1. complete generic multi-stage installer and service attribution;
2. qualify additional independent plug-in vendors and record capability reuse;
3. measure new generic work per vendor;
4. make the manager sufficient for ordinary installation, preparation, testing and repair;
5. prove a source-owned Windows audio application with real audio/MIDI and project lifecycle;
6. select one full commercial DAW only after those gates;
7. establish repeatable release qualification and incident support for both product modes.

If later plug-ins and applications require less new code, fewer live interventions and shorter diagnosis cycles, the platform and licensing thesis are working.