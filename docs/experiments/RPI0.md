# RPI0 — Standalone ARM appliance proof

## Purpose

RPI0 asks whether this project can become the engine of a standalone ARM hardware
instrument. It is not a DAW port.

The selected product shape is:

```text
USB MIDI controller
→ native ARM64 standalone host
→ existing bridge protocol and native proxy/backend
→ x86-64 Windows VST3 host and plug-in under a pinned translation/Wine lane
→ stereo audio output
→ visible, mouse-operable plug-in editor
```

The partner-facing thesis is that a hardware product built on an ARM SoC may be able
to reuse a vendor's legacy x86-64 Windows instrument software instead of requiring an
immediate ARM-native plug-in rewrite.

## Exact experimental basis

- repository: `kasselvania/Linux-VST-bridge`
- branch: `experiment/rpi0-standalone-arm64-appliance`
- base: `a86f03e8a5d5302d9872f995a0a5ba376a0ab6d5`
- base tree: `11a9fefbbe8b184c52058f4b4610da3a5e3b06e4`

This branch is experimental and must not be merged into the normal product line
without a separate decision.

## Current gate status

RPI0 has separate deterministic and physical gates:

- deterministic implementation: **PASSED**;
- Box64/Wine physical Pi preflight: **PASSED ON THE 4 KiB KERNEL**;
- physical MIDI/audio/editor acceptance: **PASSED**;
- overall RPI0: **FUNCTIONAL ARCHITECTURE PASSED; PERFORMANCE UNQUALIFIED**.

The initially qualified deterministic source remains preserved at
`53fb9f2a334e2f5fbc3d3c1cf4469148fef30b45`, tree
`fb4a662eba4ac1f8c733a41d63e5c98419832139`. Physical execution identified a
concrete JACK/state/editor defect, authorizing the bounded repaired candidate at
`791088bb31fcd75212b2df4ac2c7a6efb95182ba`, tree
`10f102fe04ce52a5a9436481331007146b85ebba`. Both identities remain retained.

The SHIELDXL0 hardware integration contract is now supplied. Physical preflight
identified a concrete kernel integration blocker before the MIDI/audio/editor
sequence: Box64/Wine requires the installed 4 KiB `rpt-rpi-v8` kernel, while the
original ShieldXL CS4270 module was built only for the 16 KiB `rpt-rpi-2712`
kernel. SHIELDXL0 has now explicitly admitted, built, loaded, and reboot-qualified
the same pinned codec source for the exact 4 KiB kernel. The historical blocker
remains retained evidence rather than being erased.

## Controlled fixture

The exact first physical RPI0 fixture is:

- Raspberry Pi 5 with 8 GB RAM;
- ShieldXL CS4270/JACK hardware governed by the SHIELDXL0 contract;
- AArch64 Linux;
- no active cooling initially;
- no overclock;
- 64-bit userspace;
- X11 or XWayland available for the Windows editor;
- one USB MIDI controller;
- ShieldXL stereo audio through JACK.

Raspberry Pi 4 Model B with 4 GB or 8 GB RAM and Compute Module 5 with at least
8 GB remain accepted RPI0 targets, but they are not the first physical fixture.
Use the exact ShieldXL device enumeration, JACK ports, sample format, clocking,
channel mapping, mixer state, startup, and cleanup behavior supplied by SHIELDXL0.

The first result does not require Bitwig, another DAW, Native Access, ASC, Pigments,
Serum, iLok, or any account/license flow.

## Translation lane

RPI0 uses one pinned x86-64-on-ARM translation lane. Box64/Wine passed the bounded
physical Pi preflight on the installed 4 KiB kernel and remains selected for the
accepted RPI0 run. Preserve the exact Box64 and Wine identities chosen by deterministic
implementation and the exact runtime artifacts actually tested.

A short deterministic preflight may reject Box64 in favor of FEX, but the slice must
not become a comparative emulator benchmark. Select one lane and continue. Any
fallback must be named, pinned, and kept separate from product claims.

Do not silently replace the project's Windows compatibility layer with an unrelated
native reimplementation.

## Physical preflight result — 2026-09-21

The first physical preflight used Raspberry Pi 5 Model B revision 1.1 with 8 GB RAM,
Raspberry Pi OS/Debian 13.7, firmware `ab8a9dde`, no active cooling, and no overclock.
The selected translation identities were:

- Box64 0.4.4 commit `2f130fab1d6e1a4ee8a71dc60cfdfcc839ad192a`;
- Wine 11.0 commit `db11d0fe6a169c457e23d007e20404643d067aa8`;
- source-owned Windows probe SHA-256
  `86161880adb171519bfa1b263ea136c6f40dd595097891fb5436d256cf727866`.

The x86-64 Linux probe passed through Box64 on the original 16 KiB
`6.18.50+rpt-rpi-2712` kernel. Wine reported its exact version, but starting the
Windows probe failed with exit 139 in `ntdll.so/signal_init_process` while accessing
`0x7ffe1000`. The exact translated process cohort was absent after failure. This is a
preserved 16 KiB-page translation failure, not a deterministic implementation failure.

The same card already contained the distribution's 4 KiB
`6.18.50+rpt-rpi-v8` kernel. After one reversible boot selection, the frozen Windows
probe passed through the same Box64/Wine artifacts. The repository-owned full preflight
then passed twice, including the source-owned Windows host start, exact cgroup/process
identity, natural close, unit retirement, and session-directory cleanup. The two exact
sessions were `94a328565d43c78bb7aea52a46ffae8d` and
`9933b83197accc4cb027ee3ade072db2`.

Two concrete defects remain preserved:

1. `rpi0/package.py` copies only the Wine launcher ELF. The generated appliance config
   therefore failed because the copied launcher could not find its adjacent Wine runtime
   tree. The physical preflight used a distinct retained config pointing at the same
   hashed Wine ELF in its exact installed runtime tree; no executable was rebuilt.
2. The first 4 KiB boot had no matching SHIELDXL0 CS4270 module, so ALSA reported only
   HDMI devices and the sound node remained deferred with `asoc-simple-card: parse
   error`. SHIELDXL0 subsequently extended its fail-closed contract for the exact
   `rpt-rpi-v8` kernel. Module build/load, codec binding, ALSA/JACK enumeration, and
   reboot identity now pass; physical MIDI/audio/editor acceptance remains separate.

During provisioning the highest observed temperature was 80.7 degrees C. No current or
historical throttling bit was observed, and the CPU was observed at 2.4 GHz under load.
These are provisioning observations only, not performance qualification. The retained
receipts are under `evidence/rpi0-standalone-arm64-appliance/`.

## Physical acceptance result — 2026-09-21

The preserved source reached the physical MIDI/audio/editor/state sequence and exposed
one concrete defect: state restore deactivated JACK, dropped the external MIDI/stereo
routes, produced missing/expired audio and gaps, and did not redraw the already-open
editor. The bounded repair at source `791088b` / tree `10f102f` keeps JACK active during
an explicitly counted callback pause, waits for exact backend epoch acknowledgement,
and invalidates the live Win32 editor after state change. Hosted deterministic run
[`35639347278`](https://github.com/kasselvania/Linux-VST-bridge/actions/runs/35639347278)
and Pi-native build/tests passed before the repaired candidate was used.

The final accepted run used the required 48 kHz / 256-frame JACK graph and 2,048 bridge
frames. Physical Monolit MIDI produced pitch 48 at velocity 58 and pitch 50 at velocity
93, plus note-offs; sustain hold/release and all-notes-off passed. ShieldXL produced
audible and measured nonzero stereo output. The real Win32 editor was visible and
mouse-operable, gain changed audio, editor close left DSP running, and a fresh editor
generation reopened on the same DSP instance. A 680-byte state save/change/restore
visibly updated the already-open editor, preserved all JACK routes, and returned audio
at the restored value.

The session ran 86,859 callbacks and delivered 22,230,528 frames with zero process
failures, deadline misses, missing frames, expired frames, or gaps. Callback maximum was
34.740 microseconds. Exact unit/cgroup/session/mapping/editor/process cleanup passed. A
second fresh 256/2,048 session again produced audible physical-MIDI audio, ran 10,609
callbacks with all failure/gap counters zero, and cleaned up exactly.

The 2,048/1,024/512 bridge-delay descent also completed without missing/expired frames,
gaps, deadline misses, or process failures on the ShieldXL contract's normal 512-frame
JACK graph. Operator-perceived delay improved, but this is not an objective latency
guarantee.

The firmware register began at `0x0`. Waking the USB-bus-powered Monolit display was
operator-observed coincident with a power jump, after which the sticky register read
`0x50000` (historical undervoltage and throttling) while current bits remained clear.
Memory high-water telemetry was unavailable. RPI0 therefore establishes the complete
functional architecture, but not accepted performance, latency, polyphony, or sustained
stability. Exact results and nonclaims are in `RPI0_RESULT.md` and
`evidence/rpi0-standalone-arm64-appliance/physical-acceptance.json`.

## Smallest complete test

Build one original source-owned x86-64 Windows VST3 instrument fixture with:

- note-on/note-off and velocity input;
- deterministic stereo synthesis;
- one bounded gain/timbre parameter;
- one simple real Windows editor control that can be changed by mouse;
- deterministic state save/restore;
- no network, service, installer, account, or external content dependency.

The fixture exists only to prove the complete cross-architecture musical path. It must
not become the long-term product UI.

Build one native AArch64 standalone host which:

- loads the AArch64 native bridge/proxy or directly consumes the same reviewed backend
  boundary without duplicating the Windows DSP implementation;
- uses a JACK process callback for stereo output and JACK MIDI input;
- converts only a closed MIDI subset for RPI0: note on, note off, velocity, sustain,
  and all-notes-off;
- preallocates callback storage and performs no allocation, lock, filesystem access,
  process launch, logging, or UI work in the audio callback;
- uses the existing asynchronous presentation-delay model rather than waiting for
  Windows from the callback;
- creates and pumps the existing editor lifecycle outside the audio thread;
- permits ordinary mouse interaction with the real Windows plug-in editor;
- closes and reopens the editor without stopping the synth instance;
- owns and positively retires every process and shared transport it starts.

Manual JACK/PipeWire routing is acceptable for the first proof. Do not add a DAW,
plugin scanner, publication catalogue, vendor manager, or general desktop shell.

## Physical acceptance sequence

The complete sequence was executed on the repaired candidate and exact fixture in this
order:

1. Start the native AArch64 standalone host.
2. Run the Box64 x86-64 Linux probe.
3. Run the Wine Windows probe.
4. Establish exact translated-host cleanup.
5. Start the source-owned synth at 2048 bridge frames.
6. Connect physical MIDI and ShieldXL audio; prove nonzero stereo output with
   distinct pitches and velocities, note retirement, sustain, and all-notes-off.
7. Exercise the real mouse-operable editor, close/reopen, and state
   save/change/restore.
8. Stop cleanly, establish complete owned-resource retirement, then start and
   play a second fresh session and stop cleanly again.

Success requires the complete sequence. A headless render, editor screenshot, MIDI
receipt without audio, or one unrestartable session is not sufficient.

## Performance posture

RPI0 is a feasibility proof, not a low-latency product claim.

Begin with:

- 48 kHz;
- 128 or 256 host frames according to the actual JACK graph;
- 2048 bridge presentation frames for first bring-up;
- then 1024;
- test 512 only after the larger settings complete without missing audio.

Report separately:

- fixed bridge presentation frames and milliseconds;
- Windows plug-in process time;
- total admission-to-publication service time;
- correlated non-plug-in residual;
- callback deadline misses;
- missing/expired frames and contiguous gaps;
- native ARM host CPU;
- translated x86/Wine cohort CPU;
- memory high-water marks;
- temperature, throttling and clock state.

Record temperature, throttling status, and observed clock state throughout every
physical phase. A functionally successful run that thermally throttles may
establish basic compatibility, but it cannot establish accepted performance,
latency, polyphony, or sustained stability.

Do not present a mean as a worst-case guarantee. Do not include audio-interface or
converter latency in bridge-only measurements.

## Commercial follow-on

Pigments is the intended first partner-style demonstration, but it is not part of
RPI0 acceptance.

The functional RPI0 gate passed. The recommended separate RPI1 slice may consume a
user-owned, private exact Pigments installation and authorization state, then attempt:

```text
physical MIDI controller
→ ARM standalone host
→ bridged x86-64 Pigments
→ audible synthesis
→ mouse-operable Pigments editor
→ preset/state restore
→ clean close and restart
```

No paid plug-in bytes, credentials, license payloads, presets, or content may be
committed or redistributed. RPI1 must not inherit a performance, low-latency,
polyphony, sustained-stability, or general-ARM claim from RPI0.

## Hard boundaries

RPI0 must not:

- modify the active x86-64 product branch or installed production system;
- require or launch a DAW;
- install or automate commercial vendor software;
- access user credentials or licensing payloads;
- claim Pigments, Native Access, ASC, iLok, or universal plug-in support;
- claim production latency, thermal stability, or hardware readiness from one run;
- weaken callback safety to obtain audio;
- replace exact ownership with process-name killing;
- hide missing frames, stalls, translation faults, or cleanup failures;
- merge this experiment automatically.

## Required handoff

Return one draft experimental PR with:

- exact base/head/tree;
- Pi hardware and OS identity;
- selected translation lane and pinned identity;
- AArch64 native build identities;
- x86-64 Windows host and fixture identities;
- standalone host architecture;
- callback-safety review;
- complete acceptance-sequence result;
- latency/service distributions and gap counts;
- CPU, memory and thermal observations;
- editor and state result;
- cleanup/restart result;
- failures and nonclaims;
- a concrete RPI1 recommendation for Pigments only if RPI0 passes.

Stop at the draft PR. Do not merge.
