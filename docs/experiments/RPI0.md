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

The exact first physical RPI0 fixture is:

- Raspberry Pi 5 with 8 GB RAM;
- ShieldXL CS4270/JACK hardware governed by the pending SHIELDXL0 contract;
- AArch64 Linux;
- no active cooling initially;
- no overclock;
- 64-bit userspace;
- X11 or XWayland available for the Windows editor;
- one USB MIDI controller;
- ShieldXL stereo audio through JACK.

Raspberry Pi 4 Model B with 4 GB or 8 GB RAM and Compute Module 5 with at least
8 GB remain accepted RPI0 targets, but they are not the first physical fixture.
Do not infer ShieldXL device enumeration, JACK ports, sample format, clocking,
channel mapping, mixer state, startup, or cleanup behavior before SHIELDXL0
supplies that contract.

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

## Physical resumption and acceptance sequence

After SHIELDXL0 supplies the hardware contract, resume this same branch in this
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
