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
- Box64/Wine physical Pi preflight: **NOT RUN**;
- physical MIDI/audio/editor acceptance: **NOT RUN**;
- overall RPI0: **PENDING PHYSICAL VALIDATION**.

An unavailable Pi does not convert a passed deterministic implementation into an
implementation failure. Preserve executable source
`53fb9f2a334e2f5fbc3d3c1cf4469148fef30b45`, tree
`fb4a662eba4ac1f8c733a41d63e5c98419832139`. Do not reopen implementation
work unless physical execution identifies a concrete defect.

Resume this same branch at physical preflight and the acceptance sequence only
after the SHIELDXL0 hardware integration contract is supplied. Do not infer the
ShieldXL connection, control, or audio contract before that authority exists.

## Controlled fixture

The accepted primary RPI0 target is:

- Raspberry Pi 4 Model B with 4 GB or 8 GB RAM;
- AArch64 Linux;
- active cooling;
- 64-bit userspace;
- X11 or XWayland available for the Windows editor;
- one USB MIDI controller;
- one stereo audio sink through JACK or PipeWire's JACK compatibility layer.

Raspberry Pi 5 or Compute Module 5 with at least 8 GB RAM remains an accepted
alternate target. The user's Raspberry Pi 4 plus ShieldXL is the intended first
physical fixture. Its exact hardware integration remains governed by the pending
SHIELDXL0 contract.

The first result does not require Bitwig, another DAW, Native Access, ASC, Pigments,
Serum, iLok, or any account/license flow.

## Translation lane

RPI0 uses one pinned x86-64-on-ARM translation lane. Box64/Wine is provisionally
selected pending the physical Pi preflight. The implementation agent must preserve
the exact Box64 and Wine identities chosen by deterministic implementation and record
the exact runtime artifacts actually tested.

A short deterministic preflight may reject Box64 in favor of FEX, but the slice must
not become a comparative emulator benchmark. Select one lane and continue. Any
fallback must be named, pinned, and kept separate from product claims.

Do not silently replace the project's Windows compatibility layer with an unrelated
native reimplementation.

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

## First acceptance sequence

The first accepted physical run is:

1. Boot the exact Pi fixture and record OS, kernel, page size, CPU, cooling and
   translation-runner identities.
2. Start the standalone host with the exact source-owned Windows instrument.
3. Connect one physical MIDI controller and one stereo audio sink.
4. Play at least two notes with distinct velocities and hear nonzero stereo output.
5. Hold and release notes; all notes must retire correctly.
6. Open the real Windows plug-in editor.
7. Change the fixture parameter with the mouse and establish a corresponding audible
   or measured output change.
8. Close and reopen the editor while audio continues.
9. Save state, change the parameter, restore state, and establish the original value.
10. Stop the standalone host and confirm all owned processes/transports are gone.
11. Start a second fresh session and play again.

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

Do not present a mean as a worst-case guarantee. Do not include audio-interface or
converter latency in bridge-only measurements.

## Commercial follow-on

Pigments is the intended first partner-style demonstration, but it is not part of
RPI0 acceptance.

After RPI0 passes, a separate RPI1 slice may consume a user-owned, private exact
Pigments installation and authorization state, then attempt:

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
committed or redistributed.

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
