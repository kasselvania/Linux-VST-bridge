# Project value and strategy

## Thesis

Linux Audio Compatibility Platform is not only a transport between a native Linux DAW and a Windows VST3. Its durable asset is an accumulating body of **versioned compatibility knowledge** for Windows audio software:

- exact runner and environment bindings;
- lawful installer, updater and authorization workflows;
- reusable Windows, VST3, UI, service and process fixtures;
- deterministic regressions for generic failure classes;
- product-specific compatibility profiles;
- real-product qualification evidence and explicit limits;
- private incident attribution and positive cleanup records;
- immutable candidate, publication and rollback history.

The current product mode presents Windows plug-ins to native Linux DAWs. The nearest expansion is to publish the same exact managed plug-in candidates to Windows DAWs running under Wine through a Windows-facing proxy. Complete managed Windows DAWs and creative applications are a later product mode.

The opportunity is to make existing Windows audio software available on Linux with less friction than asking every musician, support engineer or vendor to rediscover the same Wine, lifecycle, UI, installer, audio and failure behavior independently.

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

The end-user goal remains simple:

```text
install
→ open the product
→ make music
```

The system absorbs the compatibility machinery beneath that experience.

## Three related markets

### 1. Windows plug-ins inside native Linux DAWs

The native Linux DAW owns projects, audio devices and the top-level workflow. LVB presents each supported Windows instrument or effect through a native Linux plug-in identity and supervises the real Windows module.

This mode is already evidenced and remains the first product.

### 2. Managed plug-ins inside Windows DAWs running on Linux

A Windows DAW under Wine can load a Windows plug-in directly when the DAW and plug-in are compatible in one environment.

LVB can add a second option: a Windows-facing VST3 proxy that exposes an independently managed plug-in environment to the Wine-hosted DAW.

```text
one exact candidate
├── Linux VST3 proxy → native Linux DAW
└── Windows VST3 proxy → Wine-hosted Windows DAW
```

This can extend one qualification investment across both host categories.

The remote route is particularly valuable when:

- DAW and plug-in require different runner generations;
- vendor services or authorization should remain isolated;
- plug-in updates must not mutate the DAW environment;
- a fragile plug-in should not share the DAW process;
- one qualified installation should serve several hosts;
- independent candidate history and rollback are important.

Direct hosting remains the simpler baseline where isolation adds no value.

### 3. Complete Windows audio applications on Linux

The platform manages the DAW or application itself: its environment, installer, updater, processes, services, authorization, audio/MIDI endpoints, UI, projects, plug-in catalogue and recovery.

This mode can serve users whose workflow depends on a Windows/macOS-only DAW or creative runtime. It is strategically valuable, but it requires a broader application plane than the current plug-in bridge.

## One candidate, multiple frontends

A core commercial insight is that candidate identity should outlive a particular host adapter.

An exact candidate binds:

- vendor module and class;
- environment and runner;
- Windows host and compatibility policy;
- buses, parameters, state and editor behavior;
- evidence, limitations and review history.

That candidate may eventually generate more than one publication artifact:

```text
candidate
├── Linux-native VST3 proxy artifact
└── Windows VST3 proxy artifact
```

Both frontends must preserve the same logical class, parameters, state contract and backend identity. Each frontend still requires its own generated and real-host evidence.

This creates a catalogue multiplier without pretending one host result proves another.

## Direct and remote hosting as profile policy

For Wine-hosted DAWs, the platform should qualify one of two deployment modes.

### Direct

```text
Windows DAW environment
→ original Windows VST
```

Advantages:

- simplest topology;
- ordinary Windows ABI;
- minimal extra transport;
- useful comparison baseline.

### Remote isolated

```text
Windows DAW environment
→ LVB Windows proxy
→ Linux-owned broker
→ separate managed plug-in environment
→ original Windows VST
```

Advantages:

- different runner per plug-in;
- stronger failure containment;
- vendor-suite isolation;
- independent updates and rollback;
- reuse across DAWs;
- no need to place every vendor service in the DAW prefix.

The manager should select this from evidence. It must not expose duplicate direct and remote modules with the same class identity inside one DAW environment.

## The compounding asset

The project turns real failures into reusable platform capability:

```text
real product incident
→ durable evidence
→ generic concept
→ source-owned reproducer
→ shared compatibility law
→ focused product confirmation
```

A product-name exception helps one build. A generic law can support native Linux hosts, Wine-hosted DAWs, installers and complete applications exhibiting the same Windows behavior.

Reusable work already includes:

- VST3 lifecycle restart and same-instance reconfiguration;
- fair Win32 input retrieval;
- editor creation, focus, close and reopen;
- state capture and save/reopen;
- process-scoped retirement;
- terminal-failure custody and native containment;
- bounded crash attribution;
- managed Windows installer environments;
- immutable candidate generations and exact publication crossings;
- process/service cleanup and versioned recovery.

The desired economic effect is:

> Each generic repair should reduce the engineering cost and uncertainty of qualifying the next product and the next host frontend.

That hypothesis must be measured.

Useful measures include:

- time from lawful installer to first usable UI;
- time to first audio and MIDI;
- number of new generic failure classes;
- product-specific policy count;
- fixture and test reuse;
- incident-to-diagnosis time;
- update and maintenance cost;
- percentage of qualification completed without computer-control intervention;
- percentage of one candidate's evidence reused between Linux-host and Windows-host frontends;
- direct-versus-remote performance and reliability cost.

## Shared and mode-specific layers

### Shared compatibility kernel

- installer and process supervision;
- exact runner/environment management;
- Windows services and helper ownership;
- graphics/input/window behavior;
- terminal failure and cleanup;
- crash attribution;
- candidates, evidence, publication and rollback;
- manager workflow.

### Native Linux host frontend

- Linux VST3 identity;
- Linux DAW scan and project identity;
- native/Windows real-time transport;
- Linux-host editor presentation.

### Windows host frontend

- Windows VST3 identity;
- Wine-hosted DAW scan and project identity;
- cross-environment transport/broker;
- Windows-host editor presentation;
- direct-versus-remote exposure authority.

### Complete application mode

- qualified low-latency audio-device integration;
- MIDI, controllers, clock and synchronization;
- full GPU/DPI/multi-window behavior;
- project, pack, sample and content storage;
- nested plug-in scanning and hosting;
- application updater, service and elevation behavior;
- full-session recovery and performance policy.

A roadmap that ignores these differences overstates transfer. A roadmap that ignores the shared kernel understates the value already built.

## Why Windows-host adapters are the natural next expansion

The current system already has both sides of much of the VST3 interaction:

- Linux proxy frontend;
- Windows plug-in host;
- exact class and parameter identities;
- audio/event/state protocol;
- detached editor handling;
- candidate and publication ownership.

A Windows-facing VST3 proxy is therefore a smaller and more focused expansion than a complete DAW application plane.

The source-owned proof should establish:

```text
source-owned Windows VST3 host under Wine
→ LVB Windows VST3 proxy
→ existing isolated backend
→ source-owned Windows VST3 fixture
```

Only after that should a commercial Windows DAW be used.

## Why complete DAWs still increase business value

A full DAW catalogue reaches users whose projects, collaboration, controllers or creative systems depend on a Windows/macOS-only application.

It can create value across:

- musicians;
- plug-in vendors;
- DAW and creative-tool vendors;
- Linux hardware and workstation manufacturers;
- studios and integrators needing retained versions and supportable recovery.

The support burden also rises sharply. Full application support is credible only if the platform preserves exact scope and avoids becoming a collection of unsupported Wine recipes.

## Live and Max for Live

Live is a high-leverage later application family because Max for Live is integrated into Live.

A successful exact Live environment could potentially unlock:

- Live projects and workflows;
- Live's Windows plug-in hosting;
- bundled Max for Live where lawfully licensed;
- third-party Max for Live devices;
- controller and hardware systems built through Live/Max.

This is not automatic. Live, standalone Max and Max for Live remain separate licensed and qualified products.

Third-party VSTs inside Live may eventually be:

```text
direct in the Live environment
or
remote through the LVB Windows adapter
```

Application-local Max for Live integration and remote VST isolation can therefore coexist.

## Commercial opportunity

The platform creates several non-exclusive commercial paths.

### Owner-operated product

A supported compatibility catalogue for native Linux DAWs, Wine-hosted Windows DAWs and later complete applications.

### Paid compatibility engineering

Exact proof, vendor qualification, host qualification, incident repair or portfolio assessment.

### Per-product or catalogue license

A candidate/profile may be licensed for one product, a family or a broader catalogue. Dual-host publication can increase the value of one qualification.

### DAW/application enablement

A DAW vendor or integrator can fund Windows-host adapter qualification, application-plane work or managed Linux distribution.

### OEM and hardware integration

A Linux workstation or handheld can ship with a supportable matrix rather than community installation recipes.

### Ongoing maintenance

Release qualification, incident analysis, profile/runtime updates, regression maintenance and rollback.

No rights are granted by repository visibility.

## Roadmap decision

The order is:

1. finish the native Linux-hosted plug-in platform;
2. complete Native Instruments-style installer/service attribution;
3. broaden vendor transfer and eliminate agent-only manager paths;
4. prove the Windows-host VST3 adapter with source-owned components;
5. generate Linux-host and Windows-host publications from one candidate;
6. prove one Wine-hosted commercial DAW;
7. compare direct and remote hosting;
8. build complete managed DAW/application mode;
9. pursue Live + Max for Live;
10. expand to additional DAWs only through exact evidence.

## Guardrails

- no authorization or licensing bypass;
- no proprietary vendor binaries, presets, credentials or opaque state committed;
- no universal support claim from one session;
- no generated fixture relabelled as real-product proof;
- no product-name dispatch where a generic law applies;
- no blocking, diagnostic I/O or allocation in real-time callbacks;
- no duplicate direct and remote class exposure in one host environment;
- no publication without exact identity, evidence and rollback;
- no automatic sharing of private incidents.

## Success condition

The project succeeds when supported Windows audio software can be delivered on Linux as controlled products rather than fragile local experiments—and when one exact qualification can safely serve more host environments without weakening evidence or containment.
