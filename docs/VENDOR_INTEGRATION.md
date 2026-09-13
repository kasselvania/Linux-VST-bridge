# Vendor integration and commercial pathways

## Audience and status

This document describes how a plug-in manufacturer, distributor, integrator or commercial partner could work with Linux Audio Compatibility Bridge.

The project is currently an experimental engineering preview, not a production support offer. No partnership, endorsement, license, price or service commitment is created by this document. The repository is proprietary and all commercial use requires a separate written agreement with Peter Kassel.

## The vendor proposition

A manufacturer may be able to expand Linux availability without first rebuilding the complete product stack as a native Linux application.

The bridge is designed to preserve as much of the existing Windows product as practical:

- the real Windows VST3 module;
- vendor DSP, editor and preset behavior;
- normal installer and updater;
- normal account and authorization systems;
- vendor-managed content and resources;
- stable plug-in identity in user projects.

The compatibility platform supplies the Linux-facing and operational layers:

- native Linux VST3 representation;
- supervised Windows execution;
- exact Proton/Wine runtime selection;
- managed environment and process ownership;
- typed audio, event, state and editor transport;
- product-specific compatibility policy;
- installation, discovery, publication and rollback;
- generated Windows/VST3 tests;
- real DAW qualification;
- failure containment and private crash attribution.

This can be a lower-cost route for some products, but it is not a universal substitute for native Linux development. Feasibility depends on the plug-in’s graphics, licensing, drivers, services, content, update model and support expectations.

## What a manufacturer engagement can look like

### Stage 1 — Compatibility assessment

The first stage establishes the actual product boundary rather than assuming that loading the module is sufficient.

Typical work includes:

- inventorying installer, applications, services, VST3 modules and content;
- identifying supported product and runtime versions;
- validating the lawful vendor authorization path;
- discovering exact VST3 classes, roles, buses and stable identities;
- checking graphics, popup, focus, state and process-lifetime assumptions;
- identifying whether an existing compatibility capability applies;
- producing an explicit feasibility result, risks and unqualified areas.

Deliverable: a scoped technical assessment and recommended proof plan.

### Stage 2 — Exact product enablement

The product is bound to a controlled profile and environment.

Typical work includes:

- selecting and pinning a Proton/Wine runner;
- creating or adopting a managed vendor environment;
- qualifying installer, authorization and content behavior;
- binding the exact module, class, host, native proxy and descriptor;
- implementing only demonstrated missing generic capabilities or narrow product policy;
- preserving the vendor’s licensing and commercial systems;
- producing an immutable engineering candidate with rollback.

Deliverable: an exact candidate that can be exercised in the target Linux DAW fixture.

### Stage 3 — Qualification

Generated tests and real product use are kept separate.

A qualification may cover:

- audio, notes, events and automation;
- buses, sidechain or auxiliary input as applicable;
- preset and parameter behavior;
- opaque state, save and reopen;
- direct editor, focus, popups, resizing and DPI;
- lifecycle reconfiguration;
- installation/update behavior;
- process failure, native containment and cleanup;
- capacity and multi-instance behavior;
- measured latency and retained performance limits;
- crash-report readiness.

Deliverable: a reviewed compatibility claim with exact versions, evidence, limitations and rollback ancestry.

### Stage 4 — Distribution integration

A production relationship may package the approved runtime and profiles into a vendor, partner or bridge-managed experience.

Possible components include:

- branded or co-branded installer/manager flow;
- profile and runtime delivery;
- update and rollback policy;
- supported hardware/DAW matrix;
- user-visible status and diagnostics;
- support handoff and sanitized incident export;
- telemetry only if separately designed, disclosed and authorized;
- release signing and distribution arrangements.

Deliverable: a distribution model and support boundary agreed by the parties.

### Stage 5 — Ongoing maintenance

Windows plug-ins, Proton/Wine, DAWs, graphics stacks and Linux distributions change. A supportable product requires controlled maintenance rather than a one-time successful launch.

Typical recurring work includes:

- qualifying vendor releases;
- updating exact profiles and runtime bindings;
- triaging private incidents;
- maintaining deterministic regressions;
- assessing runner upgrades;
- preserving project compatibility and rollback;
- updating the published support matrix;
- escalating generic runtime defects upstream where appropriate.

Deliverable: a release and compatibility maintenance service.

## Possible commercial structures

Commercial terms are deliberately not fixed in this repository. Depending on the relationship, structures could include:

### Paid proof or consulting engagement

A bounded assessment or product-enablement contract. The manufacturer receives a technical result and can decide whether to continue.

### Per-product license

A license covering the bridge/runtime/profile integration for one exact plug-in or product family, potentially paired with implementation and maintenance services.

### Portfolio or catalogue license

A broader agreement covering several products, shared vendor infrastructure and ongoing profile qualification.

### OEM or white-label integration

The compatibility system is embedded into a manufacturer or distribution partner’s Linux offering under agreed branding, packaging, update and support terms.

### Managed compatibility service

The project owner operates and maintains the compatibility layer, qualification catalogue and incident workflow as an ongoing service.

### Joint engineering and upstream work

The parties collaborate on source-owned reproducers, vendor-side changes or Wine/Proton improvements. Product-specific confidential evidence can remain private while generic fixes are contributed or shared under separately agreed terms.

These structures can be combined. A proof engagement can lead to a product license; a product license can later expand into a portfolio or managed-maintenance relationship.

## Division of responsibility

A commercial agreement should define responsibility explicitly.

### Manufacturer may retain responsibility for

- Windows plug-in binaries and intellectual property;
- account, authorization and licensing systems;
- vendor content and presets;
- Windows product correctness;
- product support knowledge;
- approval of supported versions and distribution;
- vendor-side fixes where the failure belongs in the plug-in.

### Bridge provider may retain responsibility for

- native Linux proxy and Windows host;
- bridge transport and real-time boundaries;
- exact runtime/environment management;
- compatibility profiles and qualification tooling;
- installation/discovery integration;
- process supervision, containment and cleanup;
- bridge-side fixes;
- bounded incident capture and support evidence;
- publication, update and rollback machinery.

### Shared responsibility may include

- selecting the supported product/runtime matrix;
- interpreting mixed vendor/runtime failures;
- release qualification;
- end-user documentation;
- incident escalation;
- support-service levels;
- upstream reports and patches.

The bridge should never bypass or duplicate vendor authorization. A vendor partnership should make the lawful path clearer, not weaker.

## Compatibility profiles and runtime selection

The system is designed to bind an exact product to an exact compatibility profile.

A profile can select closed, reviewed capabilities such as:

- runner and environment revision;
- host/native artifact identity;
- event-output interpretation;
- editor lifetime and retirement behavior;
- Windows accessibility posture;
- qualified bus topology;
- precision and latency posture;
- retained limitations.

A manufacturer’s products may share one environment and runner, or different products may eventually use different validated runtime generations. Runtime selection is evidence-backed policy, not an arbitrary end-user experiment.

A profile does not contain arbitrary code, secrets, licensing state or destructive commands. It is a controlled description of what the platform has qualified for that exact product generation.

## Test and incident value

The project’s growing source-owned suite is part of the commercial value.

A vendor should not need to reproduce every failure manually. The bridge can convert a real incident into a reusable test for concepts such as:

- lifecycle start/stop/reconfiguration;
- state capture and restoration;
- message-pump fairness;
- focus and pointer capture;
- owned and ownerless transient window groups;
- process exit and containment;
- accessibility-provider lifetime;
- installer process trees;
- module identity and exception attribution.

Private crash reports can retain available exception codes, exact module identities, relative addresses, bounded stack frames, first bridge failure and cleanup state. A sanitized export can support collaboration without publishing credentials, paths, process identities, vendor state or proprietary payloads.

A faulting module or nearby UI action remains evidence, not automatic proof of root cause. The platform is designed to retain that uncertainty honestly.

## Current Arturia vertical as a case study

The current exact Arturia work demonstrates a full vertical:

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

Pure LoFi and Efx FRAGMENTS also demonstrate installed exact-fixture instruments/effects, same-instance editors, automation/state recall, independent removal, capacity and rollback behavior.

This is a technical case study only. Arturia has not endorsed the project and no partnership or commercial relationship is claimed.

## What a vendor receives from a mature engagement

Depending on scope, a completed engagement could provide:

- an exact supported-product profile;
- pinned and retained runtime/environment artifacts;
- deterministic compatibility fixtures;
- a real-DAW qualification record;
- installer and authorization integration;
- a support and limitation matrix;
- candidate/ordinary publication and rollback history;
- private incident-report tooling;
- a release-qualification process;
- a roadmap for remaining unsupported behavior;
- integration or distribution rights defined by contract.

The intended value is not a one-time demo. It is a maintained route from the manufacturer’s existing Windows product to a supportable Linux offering.

## Engagement principles

A credible vendor relationship should preserve these rules:

- use the real vendor installer and product where practical;
- preserve vendor licensing and authorization;
- do not publish proprietary binaries, presets, state or credentials;
- distinguish generated tests from real product proof;
- state exact supported versions and limits;
- fix generic problems at the shared layer;
- keep product-specific policy declarative and narrow;
- retain rollback before activating updates;
- stop repetitive testing when it produces no new evidence;
- treat private incidents as sensitive by default;
- do not claim a partnership until one exists.

## Intellectual property and licensing

The original bridge source, documentation, compatibility profiles, test fixtures and designs in this repository are proprietary. Public visibility is for inspection and development convenience and does not grant use, modification, redistribution, commercial deployment or derivative-work rights.

Third-party products and dependencies remain owned and licensed by their respective parties. A commercial engagement would define the rights for bridge use, integration, distribution, support, branding, profiles, fixtures and modifications separately in writing.

See [COPYRIGHT.md](../COPYRIGHT.md) and [CONTRIBUTING.md](../CONTRIBUTING.md).

## Next commercial proof points

The most valuable near-term proof is not a larger marketing claim. It is evidence that the cost curve improves:

1. complete the remaining Pigments transient-surface and stability boundary;
2. qualify a second vendor and record which capabilities are reused unchanged;
3. measure the new generic work required for that vendor;
4. demonstrate controlled per-product runtime/profile selection;
5. turn internal operations into a simple user-facing manager;
6. establish a repeatable release-qualification and incident-support workflow.

If later plug-ins require less new code, fewer live interventions and shorter diagnosis cycles, the platform and licensing thesis are working.
