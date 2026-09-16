# Windows-host plug-in and application compatibility roadmap

## Decision and status

Linux Audio Compatibility Platform has three ordered tracks:

```text
Track A — current
Windows plug-ins in native Linux DAWs

Track B — next architectural expansion
Managed plug-ins in Windows DAWs running under Wine/Proton

Track C — later
Complete managed Windows audio applications
```

Track B comes before Track C because it reuses the current plug-in candidate, transport and qualification model more directly.

This document does not claim that FL Studio, Ableton Live, Cubase, standalone Max, Max for Live or another Windows DAW is supported today. It does not authorize a commercial DAW campaign before the source-owned host-adapter gates are complete.

## Architectural correction

The current Linux VST3 proxy cannot be loaded unchanged by a Windows DAW. A Windows DAW expects a Windows VST module.

That does not invalidate the existing architecture. It means the platform needs another host-facing frontend:

```text
one exact managed plug-in candidate
├── Linux VST3 proxy
│   └── native Linux DAW
└── Windows VST3 proxy
    └── Windows DAW under Wine/Proton
```

Both frontends can share:

- candidate identity;
- exact module/class/parameter contract;
- managed environment;
- isolated Windows backend;
- transport concepts;
- profile, evidence and rollback;
- failure custody.

Each frontend still requires its own tests and real-host proof.

## Direct and remote Windows hosting

A Windows DAW already speaks the Windows VST ABI. Therefore the normal direct route is valid:

```text
Windows DAW environment
→ original Windows plug-in
```

The platform adds a remote isolated route:

```text
Windows DAW environment
→ LVB Windows VST3 proxy
→ Linux-owned broker / transport
→ separate managed plug-in environment
→ original Windows plug-in
```

The direct route is the baseline. The remote route is selected when isolation has evidence-backed value:

- different runner requirement;
- vendor-service or authorization separation;
- independent update/rollback;
- crash containment;
- one installation shared across hosts;
- incompatible vendor suites;
- exact support custody outside the DAW prefix.

One host environment must expose only one implementation of an exact VST3 class.

## Why this route comes before complete DAW management

A complete DAW application plane introduces:

- audio-device integration;
- MIDI and synchronization;
- project/content lifecycle;
- full GPU/window/file behavior;
- application self-update;
- full-session recovery.

The Windows-host adapter can be proven with a source-owned VST3 host before those concerns are solved.

That gives the platform an earlier result:

> One exact managed plug-in can be used from both native Linux DAWs and Windows DAWs running on Linux.

## Gate 0 — finish the native Linux-hosted plug-in product

Before Track B:

- stabilize current Arturia and Xfer results;
- complete Native Instruments-style multi-stage installer/service attribution;
- qualify several additional vendor families;
- make install → discover → inspect → prepare → test ordinary manager operations;
- remove remaining agent-only product crossings;
- measure shared versus product-specific work.

Exit condition:

> A new Windows plug-in vendor can normally be installed, discovered, prepared, experimentally published, diagnosed and rolled back for a native Linux DAW through the manager.

## Gate 1 — WHA0 source-owned Windows host adapter

Build project-owned components only:

```text
source-owned Windows VST3 host under Wine
→ source-owned LVB Windows VST3 proxy
→ Linux-owned broker / transport
→ isolated source-owned Windows plug-in backend
```

### Required capabilities

- factory scan and stable class identity;
- component/controller creation;
- bus negotiation;
- MIDI/events;
- nonzero audio;
- parameters and automation;
- opaque state save/restore;
- latency reporting;
- editor open/close;
- one processing restart;
- backend terminal failure presentation;
- exact cleanup.

### Transport questions

- can independently managed Wine environments share Linux-owned memory safely?
- should control use Unix sockets while hot audio uses private tmpfs/shared memory?
- what wake-up primitive is bounded and portable through the selected runner?
- how are process and session identities bound across environments?
- how is a dead backend distinguished from a delayed result?

### Editor posture

First proof:

```text
detached vendor editor managed by LVB
```

Embedded cross-Wine editor forwarding is later work.

Exit condition:

> The source-owned Windows host treats the Windows LVB proxy as a functioning VST3 whose backend lives in another managed environment.

## Gate 2 — dual-host candidate publication

Extend the manager so one exact candidate can generate:

```text
Linux host artifact
Windows host artifact
```

The candidate retains one logical product identity while each artifact retains:

- target platform;
- build recipe;
- frontend version;
- transport protocol;
- host-specific evidence;
- limitations.

The manager must:

- publish each frontend explicitly;
- prevent duplicate direct/remote class exposure;
- show host-specific qualification;
- preserve shared backend candidate history;
- rollback one frontend without corrupting another.

Exit condition:

> One source-owned candidate can be loaded through both a native Linux VST3 host and a Windows VST3 host under Wine.

## Gate 3 — first commercial Windows DAW host proof

Choose a DAW by criteria:

- lawful operator-owned license;
- known Wine viability;
- conventional VST3 scan;
- useful project save/reopen;
- no kernel-driver requirement for first proof;
- practical process ownership;
- clear user value.

FL Studio is a plausible candidate, not a commitment.

### Bounded proof

```text
install/launch DAW through an already controlled environment
→ scan exact Windows LVB proxy
→ instantiate one existing qualified candidate
→ MIDI and nonzero audio
→ editor
→ automation
→ project save/reopen
→ normal removal
→ DAW quit and positive cleanup
```

The first commercial proof does not require full manager ownership of every DAW feature.

Exit condition:

> One exact Wine-hosted DAW generation can use the remote LVB proxy for a bounded musical workflow.

## Gate 4 — direct versus isolated comparison

For the same DAW and plug-in, compare:

```text
Direct:
DAW → original Windows VST

Remote:
DAW → Windows LVB proxy → isolated original Windows VST
```

Measure:

- scan and launch reliability;
- audio/MIDI latency;
- CPU and wake-up cost;
- editor behavior;
- automation/state recall;
- crash containment;
- update and rollback;
- vendor-service interaction;
- project portability.

Outcome:

- choose direct where it is simpler and sufficient;
- choose remote where isolation has demonstrated value;
- retain unsupported combinations honestly.

## Gate 5 — managed Windows DAW application mode

Only after the host-adapter proof, add the full application plane:

```text
Add Windows audio application
→ install in managed environment
→ detect services/content
→ qualify audio/MIDI/project behavior
→ prepare application candidate
→ enable experimental use
→ review exact configuration
→ publish as Ready or retain limitations
```

### New shared foundation

- Windows audio-device bridge into PipeWire/ALSA;
- MIDI/controller/synchronization bridge;
- project/content storage policy;
- full-session recovery;
- application updater/service custody;
- nested plug-in-hosting policy;
- sustained-performance qualification.

### Source-owned application fixture

Before a commercial full-DAW campaign, build a project-owned Windows audio application covering:

- stereo output/input;
- MIDI input/output;
- sample-rate/buffer changes;
- device loss/restart;
- one project save/reopen;
- windows/dialogs/drag-drop;
- one nested source-owned VST3;
- crash and cleanup;
- candidate and rollback.

Exit condition:

> The platform can supervise a source-owned Windows audio workstation fixture through a qualified Linux audio/MIDI path.

## Gate 6 — Live and Max for Live

If Live is selected, qualify in layers:

1. Live installation and authorization;
2. audio/MIDI and project save/reopen;
3. Live's direct Windows plug-in hosting;
4. Windows LVB proxy hosting for one isolated plug-in;
5. bundled Max for Live runtime;
6. one stock Max for Live device;
7. one third-party `.amxd` device;
8. Max for Live editor open/edit/save/reopen;
9. standalone Max as a separate track;
10. controller/Link/hardware as separate exact evidence.

Live, Max for Live and standalone Max remain separate lawful product boundaries.

## Later DAWs

FL Studio, Cubase and other DAWs should be admitted through the same host/application evidence model.

No DAW name dispatch belongs in shared transport or process ownership. Product-specific policy stays narrow and declarative.

## Manager model

The manager should eventually show:

```text
Product: exact plug-in candidate

Backend:
✓ installed
✓ inspected
✓ prepared

Host publications:
✓ Linux VST3 proxy
○ Windows VST3 proxy

Windows DAW exposure:
○ Direct in DAW environment
● Remote isolated

Qualification:
Linux/Bitwig: experimental result retained
Windows/source-owned host: generated result retained
Windows/commercial DAW: not tested
```

For applications:

```text
Application candidate
→ environment/runner
→ audio/MIDI profile
→ projects/content
→ direct and remote plug-in catalogue
→ evidence
→ rollback
```

## Evidence areas

### Windows-host plug-in adapter

- scan/class identity;
- buses and events;
- audio;
- automation;
- state;
- latency;
- editor;
- lifecycle;
- backend loss;
- cleanup.

### Complete application

- install/update;
- authorization;
- audio device;
- MIDI/controller;
- project save/reopen;
- content;
- direct plug-in host;
- remote plug-in host;
- UI/file behavior;
- crash/session recovery;
- performance.

Generated fixtures and real commercial results remain distinct.

## Security and licensing guardrails

- no licensing bypass;
- no cross-prefix credential copying;
- no unrestricted real HOME exposure;
- no root manager;
- no arbitrary shell/profile commands;
- no duplicate class exposure;
- no hidden runner fallback;
- no product support claim from launch alone;
- no vendor partnership claim without agreement;
- no proprietary payload in public evidence.

## Near-term order

```text
Native Instruments installer/service attribution
→ additional plug-in vendors
→ manager workflow completion
→ WHA0 Windows host adapter
→ dual-host publication
→ one Wine-hosted DAW
→ direct/remote comparison
→ source-owned application fixture
→ managed full DAW
→ Live + Max for Live
```

## Success criteria

The roadmap is working when:

- new plug-in vendors require fewer generic repairs;
- one candidate can serve both Linux and Windows hosts through separately qualified frontends;
- remote isolation provides measurable value where selected;
- direct hosting remains available where preferable;
- a full DAW can later be managed without discarding the plug-in platform;
- support claims remain exact, versioned and reversible.
