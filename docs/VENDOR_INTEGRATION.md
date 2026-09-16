# Vendor and application-partner integration

## Audience and status

This document describes how plug-in manufacturers, DAW vendors, creative-tool vendors, Linux hardware companies, distributors and commercial integrators could work with Linux Audio Compatibility Platform.

The project is an experimental engineering preview, not a production support offer. No partnership, endorsement, license, price or service commitment is created by this document. The repository is proprietary and all commercial use requires a separate written agreement with Peter Kassel.

## Platform proposition

A partner may be able to expand Linux availability without first rebuilding the complete Windows product stack as a native Linux product.

The platform can preserve, where practical:

- the real Windows plug-in or application;
- vendor DSP, editor, project and preset behavior;
- normal installer and updater;
- normal account and authorization systems;
- vendor-managed content and resources;
- stable product identity.

The platform supplies:

- exact Wine/Proton runner and environment management;
- native Linux VST3 representation for native Linux DAWs;
- future Windows VST3 proxy representation for Wine-hosted Windows DAWs;
- supervised Windows execution;
- installer, updater, process and service custody;
- typed audio, event, state and editor transport where applicable;
- product-specific compatibility policy;
- discovery, candidate preparation, publication and rollback;
- generated Windows/VST3 tests;
- real-host qualification;
- failure containment and private incident attribution.

This can reduce the cost of some Linux enablement efforts. It is not a universal substitute for native engineering.

## Two plug-in delivery routes

### Native Linux DAW route

```text
native Linux DAW
→ Linux LVB proxy
→ isolated Windows plug-in backend
→ real Windows plug-in
```

This is the current proven route.

### Wine-hosted Windows DAW route

A partner can use either:

```text
Windows DAW under Wine
→ original Windows plug-in directly
```

or:

```text
Windows DAW under Wine
→ Windows LVB proxy
→ isolated managed plug-in backend
→ real Windows plug-in
```

The direct path is the comparison baseline. The isolated path is valuable for:

- runner incompatibility;
- vendor-service separation;
- independent plug-in updates;
- stronger crash containment;
- one managed installation exposed to several hosts;
- exact candidate and rollback custody outside the DAW environment.

One DAW environment must expose only one route for one exact VST3 class.

## Candidate and publication model

A qualification creates an exact candidate, not merely a claim that a product name works.

The candidate binds:

- module and class identity;
- product/version metadata;
- environment and runner;
- Windows host;
- buses, events, parameters, state and editor policy;
- evidence and limitations;
- candidate lineage and rollback.

That candidate can later produce separately tested publication artifacts:

```text
Linux host publication
Windows host publication
```

Host-specific proof remains mandatory. Linux-host success is not silently reused as Windows-host success.

## Plug-in manufacturer engagement

### Stage 1 — assessment

- inventory installer, services, modules and content;
- validate lawful authorization;
- discover exact classes, buses and identities;
- identify shared compatibility capability;
- select native Linux-host, Windows-host direct or Windows-host remote goals;
- retain risks and unsupported areas.

### Stage 2 — exact enablement

- create or adopt managed vendor environment;
- select runner;
- bind exact module/class;
- inspect VST3 behavior;
- prepare immutable candidate;
- generate required host frontend artifacts;
- preserve licensing and vendor commercial systems.

### Stage 3 — host qualification

For a native Linux DAW:

- scan;
- instantiate;
- notes/audio or effect audio;
- automation;
- state/project recall;
- editor;
- lifecycle;
- cleanup.

For a Wine-hosted Windows DAW:

- compare direct and remote modes;
- scan the Windows proxy;
- verify stable class/parameter identity;
- audio/events/state/editor;
- project save/reopen;
- backend failure presentation;
- cleanup and DAW survival.

### Stage 4 — distribution

- manager workflow;
- profile and runtime delivery;
- update and rollback;
- supported DAW/hardware matrix;
- user-visible status and diagnostics;
- signing and release policy;
- support handoff.

### Stage 5 — maintenance

- qualify vendor releases;
- update exact profiles and frontends;
- triage incidents;
- assess runner changes;
- preserve host project identity;
- update the support matrix.

## DAW and application partner engagement

A DAW/application partner track is broader.

### Windows-host adapter engagement

This is the smaller first step:

- qualify the DAW under Wine as a VST3 host;
- make the LVB Windows proxy scan and instantiate;
- connect it to an isolated candidate backend;
- retain project identity;
- compare direct versus remote hosting;
- expose supported plug-ins without manually modifying the DAW prefix.

### Complete managed application engagement

This additionally covers:

- DAW installer and authorization;
- audio-device and MIDI integration;
- projects and recovery;
- packs, sample libraries and content;
- native devices and bundled runtimes;
- plug-in scanning and hosting;
- updater, services and helper processes;
- full-session performance and crash recovery.

The Windows-host adapter path can deliver value before complete application management is finished.

## Live and Max for Live

Live is a later high-leverage application family.

A Live engagement should separate:

1. Live installation and authorization;
2. audio/MIDI and project save/reopen;
3. Windows plug-in hosting;
4. Windows LVB proxy hosting where isolation is beneficial;
5. bundled Max for Live runtime;
6. stock Max for Live devices;
7. third-party `.amxd` devices;
8. Max for Live editor behavior;
9. standalone Max as a separate product.

Live and Max for Live remain lawfully licensed vendor products. No compatibility layer should bypass those boundaries.

## Hardware and OEM opportunity

A Linux workstation, handheld or instrument vendor can integrate:

- qualified native Linux DAWs with managed Windows plug-ins;
- qualified Windows DAWs under Wine;
- exact supported catalogue and versions;
- manager UI;
- updater and rollback;
- private incident capture;
- controlled hardware/audio/MIDI profiles.

The value is a supportable audio-software matrix rather than a collection of community setup recipes.

## Possible commercial structures

- paid proof or consulting engagement;
- per-product license;
- product-family or catalogue license;
- dual-host plug-in enablement license;
- DAW/application enablement;
- OEM or white-label integration;
- managed compatibility service;
- recurring release qualification and support;
- joint upstream work.

Commercial terms are not defined in this repository.

## Responsibility boundaries

### Vendor may retain

- Windows binaries and intellectual property;
- account and licensing systems;
- content and presets;
- Windows product correctness;
- supported versions;
- vendor-side fixes;
- product support knowledge.

### Platform provider may retain

- Linux and Windows host adapters;
- transport and real-time boundaries;
- environment/runner management;
- profiles and qualification tooling;
- installer/discovery integration;
- supervision and cleanup;
- platform-side fixes;
- publication and rollback;
- incident evidence.

### Shared

- supported product/host matrix;
- release qualification;
- mixed vendor/runtime diagnosis;
- end-user documentation;
- incident escalation;
- service levels;
- upstream reports.

## Test and incident value

The source-owned suite can convert incidents into reusable tests for:

- lifecycle and reconfiguration;
- state capture/restoration;
- message-pump fairness;
- focus/pointer capture;
- transient windows;
- process exit and containment;
- accessibility-provider lifetime;
- installer process graphs;
- Windows service behavior;
- cross-environment proxy transport;
- module identity and exception attribution.

Private reports remain sensitive by default.

## Distribution modes

A mature manager may show an exact product like:

```text
Serum 2 — exact candidate
Host publications:
✓ Linux VST3 proxy
○ Windows VST3 proxy

Windows DAW exposure:
○ Direct
● Remote isolated
```

This is a controlled qualification result, not a casual runtime toggle.

## Engagement principles

- use real lawful vendor installers and products;
- preserve authorization;
- do not publish proprietary payloads or credentials;
- distinguish generated fixtures from real-product proof;
- state exact supported versions and hosts;
- fix generic problems at the shared layer;
- keep product-specific policy declarative;
- retain rollback;
- stop repetitive testing that adds no information;
- avoid duplicate class exposure;
- do not claim a partnership until one exists.

## Near-term proof points

1. complete generic multi-stage installer/service attribution;
2. broaden native Linux-host plug-in vendor evidence;
3. remove agent-only onboarding and preparation paths;
4. build the source-owned Windows-host adapter fixture;
5. publish one exact candidate to Linux and Windows frontends;
6. prove one Wine-hosted commercial DAW;
7. compare direct and remote hosting;
8. only then expand into complete DAW application management and Live/Max for Live.

If later products and hosts require less new code and fewer live interventions, the platform and licensing thesis are working.
