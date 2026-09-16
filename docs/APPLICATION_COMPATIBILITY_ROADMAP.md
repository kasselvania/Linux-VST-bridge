# Application compatibility roadmap

## Decision and status

Complete Windows audio applications are a **strategic future product mode** for Linux Audio Compatibility Platform.

This is not the active implementation slice and it is not a claim that any Windows DAW currently works. The present accepted architecture and evidence are centered on Windows VST3 plug-ins inside native Linux DAWs.

The roadmap decision is narrower:

> After the plug-in platform demonstrates broader vendor transfer and generic installer/application custody, extend the same management and compatibility system toward complete Windows audio applications.

The first application target should be selected only after the shared application foundation exists. Ableton Live is a high-leverage candidate because Max for Live is integrated into Live, but Live, Max, Max for Live, FL Studio, Cubase and every other application remain unqualified until exact evidence exists.

## Why the opportunity is credible

Several major creative applications currently publish Windows and macOS requirements rather than native Linux support:

- [Ableton Live system requirements](https://help.ableton.com/hc/en-us/articles/115001663530-Live-Minimum-System-Requirements)
- [Cycling '74 Max downloads and system requirements](https://cycling74.com/downloads)
- [FL Studio official download and system requirements](https://www.image-line.com/fl-studio/download)
- [Steinberg product system requirements](https://www.steinberg.net/system-requirements/)

Bitwig already provides a native Linux product and remains the project's current DAW fixture:

- [Bitwig Studio downloads and Linux requirements](https://www.bitwig.com/download/)

That leaves a real category of musicians who may want Linux hardware or operating environments while remaining dependent on a Windows/macOS-only DAW, project format, controller ecosystem or creative runtime.

The project has already built relevant shared machinery:

- exact Windows environments and runner identity;
- lawful installer and vendor-application supervision;
- process/cgroup ownership and cleanup;
- Windows services and helper investigation;
- UI/input/window diagnostics;
- crash and terminal-failure custody;
- immutable candidates, evidence and rollback;
- a native manager that presents controlled user crossings;
- real-time audio experience from the plug-in mode.

Those capabilities make full applications a natural adjacency rather than a completely unrelated product.

## Why application support is not automatic

The current native plug-in architecture is:

```text
Linux DAW owns the audio device and project
→ native Linux VST3 proxy
→ project-owned transport
→ supervised Windows plug-in host
```

A Windows DAW changes the top-level ownership:

```text
Windows DAW owns its audio engine, project and plug-in host
→ Windows audio/MIDI endpoint
→ Linux PipeWire/ALSA, MIDI and desktop services
```

The following capabilities are new or substantially broader:

- ASIO/WASAPI-facing audio-device behavior;
- PipeWire/ALSA routing and low-latency policy;
- sample-rate, buffer, channel and device restart behavior;
- MIDI input/output, clock, MPE, remote scripts and control surfaces;
- GPU/DirectX/Vulkan translation, DPI, full-screen and multi-window behavior;
- drag/drop, clipboard, file dialogs and file associations;
- project, backup, pack, sample-library and content indexing;
- Windows plug-in scanning and hosting inside the DAW;
- application self-update, service and elevation behavior;
- crash/session recovery and project-integrity guarantees;
- collaboration/export and external hardware boundaries.

The platform should therefore add an **application plane** rather than attempting to route a DAW through the native VST3 proxy.

## What should transfer

### Strongly reusable

- manager UI and closed operator actions;
- imported installer custody;
- isolated managed environments;
- runner selection and immutable profiles;
- multi-stage installer and service attribution;
- process, child, updater and relaunch ownership;
- UI/input/window observation;
- crash capture and module attribution;
- application candidate rollout and rollback;
- evidence, incidents, exact versions and nonclaims;
- lawful account and authorization handoff.

### Reusable with extension

- performance and capacity policy;
- state/project custody;
- content and filesystem policy;
- popup/focus/accessibility handling;
- hardware/controller qualification;
- update and rollback workflow.

### New shared foundation

- Windows audio-device bridge;
- MIDI and synchronization bridge;
- application project/content integration;
- nested plug-in-hosting qualification;
- full-session recovery.

The roadmap should measure this transfer rather than assigning a speculative percentage.

## Why Live and Max for Live are high leverage

Ableton documents Max for Live as an editor and creative environment used inside Live. It is bundled with Live installations and included with Live Suite, or available as an add-on for Live Standard:

- [Buying Max for Live](https://help.ableton.com/hc/en-us/articles/206407124-Buying-Max-for-Live)
- [Max for Live bundled in Live](https://help.ableton.com/hc/en-us/articles/360000036850-Max-for-Live-bundled-in-Live)

This creates a potentially valuable application family:

```text
Ableton Live
├── Live projects and devices
├── Windows VST plug-in hosting
├── bundled Max for Live runtime
├── third-party Max for Live devices
└── controller and hardware integrations
```

A successful Live environment could therefore unlock more than one executable.

However:

- Live authorization is still a separate lawful vendor boundary;
- Max for Live licensing depends on the Live edition/add-on;
- standalone Max is a separate product and license;
- Max for Live editing is not proved by Live launching;
- a particular Max device is not proved by the Max runtime launching;
- Windows plug-ins inside Live require their own installation and qualification;
- controllers, Link, external hardware and low-latency audio remain separate evidence.

Live is high-leverage precisely because the surface is large. It should not be the first test of an unfinished application foundation.

## Roadmap gates

### Gate 0 — Broader plug-in transfer

Before a full DAW target:

- retain a stable Arturia vertical;
- retain the Xfer cross-vendor result and lifecycle repair history;
- complete generic multi-stage installer/service attribution;
- qualify several additional vendor families;
- use Native Instruments and independent vendors such as Kilohearts, Valhalla DSP and UVI as possible probes, not promised support;
- measure reused versus new generic work;
- remove remaining agent-only ordinary workflows from the manager.

Exit condition:

> A new plug-in vendor can normally be installed, discovered, prepared, tested and diagnosed through the manager, with product-specific code remaining exceptional.

### Gate 1 — Source-owned Windows audio application fixture

Build a small project-owned Windows application that exercises the exact future application boundary without commercial software.

It should cover:

- installation and launch;
- a real Windows audio-device API;
- stereo output and input;
- MIDI input/output;
- sample-rate and buffer changes;
- device loss/restart;
- one project document with save/reopen;
- multiple windows, dialogs and drag/drop;
- one nested source-owned VST3;
- crash and positive cleanup;
- immutable candidate and rollback.

Exit condition:

> The platform can run and supervise a source-owned Windows audio workstation fixture through a qualified Linux audio/MIDI path.

### Gate 2 — Application manager workflow

The manager should expose:

```text
Add Windows application
→ install in managed environment
→ detect application/services/content
→ check application compatibility
→ prepare application candidate
→ enable experimental use
→ test audio/MIDI/projects/plug-ins
→ review exact configuration
→ publish as Ready or retain limitations
```

It must retain:

- application and updater versions;
- environment and runner;
- services and prerequisites;
- audio/MIDI policy;
- project/content locations;
- plug-in paths;
- evidence and rollback.

Exit condition:

> A human operator can manage an application without administering Wine or relying on an agent to build the environment by hand.

### Gate 3 — First commercial application proof

Select one product using criteria rather than excitement alone:

- lawful installer and license available to the operator;
- no kernel driver dependency required for the initial proof;
- documented audio/MIDI requirements;
- practical offline or bounded online authorization;
- high user value;
- useful plugin/project workflow;
- feasible support boundary;
- no need to bypass security or licensing.

A proof ladder should stop at the first material boundary:

1. install and authorize;
2. launch real UI;
3. produce machine-measured audio;
4. receive MIDI;
5. save/reopen a project;
6. scan one managed Windows plug-in;
7. use that plug-in and reopen the project;
8. normal quit and positive cleanup;
9. deliberate crash and recoverable project/session posture;
10. measured latency and bounded sustained use.

Exit condition:

> One exact application generation is usable for a bounded musical workflow with truthful limits and rollback.

### Gate 4 — Live and Max for Live family

If Live is selected, qualify in layers:

1. Live application installation and authorization;
2. audio/MIDI and project save/reopen;
3. Live's Windows plug-in scanning and one managed plug-in;
4. bundled Max for Live runtime launch;
5. one stock Max for Live device;
6. one third-party `.amxd` device;
7. Max for Live editor open/edit/save/reopen;
8. standalone Max only as a separate product track;
9. controller/Link/hardware only through separate exact evidence.

Exit condition:

> Live and the selected Max for Live scope operate as an exact managed application family, not merely a successful launcher.

### Gate 5 — Additional DAWs

FL Studio, Cubase or another DAW should be added only after the application foundation is reusable.

Each new DAW should answer:

- Which application capabilities transferred unchanged?
- Which new generic fixture was required?
- Which product-specific policy was unavoidable?
- Did the manager and diagnostic suite reduce time to diagnosis?
- Can the supported matrix and rollback be stated precisely?

## Application evidence model

A DAW/application qualification should keep separate areas:

```text
installation
first_launch
authorization
audio_output
audio_input
midi_input
midi_output
device_restart
project_save_reopen
content_and_packs
plugin_scan
plugin_processing
plugin_editor
multiwindow_and_dialogs
update
crash_recovery
normal_retirement
performance
```

Each area retains:

- passed;
- failed;
- not tested;
- unavailable;
- not applicable;
- machine observation, generated fixture or operator observation;
- exact application, runner and environment generation.

A clean launch cannot be promoted to full application support.

## Business validation criteria

The application strategy is working when:

- the same manager operates both plug-in and application modes;
- installer, UI, process and incident capabilities are reused;
- application-specific audio/MIDI work becomes shared infrastructure;
- later DAWs require less new code and fewer manual interventions;
- users can retain projects and exact rollback safely;
- vendor/hardware partners can understand the support boundary;
- the catalogue becomes more valuable without support claims becoming less precise.

It is not working if every DAW requires an unrelated custom launcher, arbitrary Wine flags, manual filesystem edits and unauditable support claims.

## Security, licensing and support guardrails

- No authorization or DRM bypass.
- No bundling or redistribution of commercial installers, applications, plug-ins, packs or projects without rights.
- No running the manager as root to satisfy a vendor installer.
- No exposing the operator's entire home directory by default.
- No arbitrary shell commands in profiles.
- No hidden host-wide service installation without explicit review.
- No random runner selector presented as support.
- No product-specific implementation dispatch when a generic application law can be built.
- No implication of vendor affiliation or official support.
- No consumer release before project integrity, cleanup and rollback are proved.

## Near-term decision

Do not start Ableton Live yet.

First complete the generic Native Instruments installer/service investigation and broaden the plug-in catalogue. In parallel, preserve application mode in architecture and product language so that current work is evaluated for reuse rather than accidentally hard-coded around VST3-only assumptions.

The next application work after those gates should be a source-owned Windows audio application fixture—not a commercial DAW campaign.