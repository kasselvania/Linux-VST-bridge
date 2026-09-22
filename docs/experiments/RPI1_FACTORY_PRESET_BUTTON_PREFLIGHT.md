# RPI1 factory-preset button investigation

## Bounded slice

- Base head: `ca07f08c27b3e06803e07edc3eedb504b425a4c8`.
- Base tree: `4e0cf427c48bdd320e4b84c226ee0fb753163aaf`.
- Basis: `RPI1.md` “Primary claim,” “Phase C — Pigments discovery and appliance qualification,” and “Hard boundaries”; repository `AGENTS.md` “Keep the engineering safeguards” and “Verification and review.”
- Primary claim: identify whether the exact Pigments VST3 controller exposes a host-selectable program list that can truthfully drive previous/next **factory** presets without an editor.
- Fixture: the accepted Pi 5/ShieldXL appliance, Pigments 7.0.1.6772, exact installed Proton/Box64 runtime, current private environment and saved 45% state. The Pi-side read-only probe runs under the existing bounded census supervisor.
- Source scope: add only VST3 `IUnitInfo`, program-list and program-change-parameter observation to the Windows census. The ordinary processing and editor paths remain unchanged. No vendor content is copied or modified.
- Negative result: if no usable program list is exposed, or its connection to the factory library cannot be established, buttons 2 and 3 remain unassigned. Parameter titles or files in private preset directories are insufficient authority.
- Evidence: retain the private raw census on the Pi; publish only sanitized interface/list counts, IDs, names if generic, and the resulting capability disposition. Do not publish preset names, preset bytes, account information, or opaque state.
- Cleanup: the census must retire its Windows host, owner and transport normally. Leave the existing Pigments executable, saved state, audio route and ShieldXL control service unchanged. A replacement census candidate is separately named so the prior executable remains available.

## Current status

The retained census reports sixteen `kIsProgramChange` parameters (IDs 2272–2287, each with 127 steps). Pigments' similarly named `Previous Preset` and `Next Preset` parameters are `kIsReadOnly`; they cannot be written by the host. The retained census omits `unitId` and `IUnitInfo`, so it does not establish what those program-change parameters select. The new observation closes that information gap before any physical button mapping or audio run.

The first hosted candidate built successfully and was run once on the Pi. It
reported `IUnitInfo` support, then encountered the probe's own strict
unit/list-count bound before printing either count. The host returned 90; the
existing supervisor reported confirmed cleanup and retired transport. Pigments
audio, editor and ShieldXL buttons were not launched. This is a failed probe,
not a Pigments preset result. The follow-up retains reported counts and samples
only a bounded prefix of units/lists; it never iterates an untrusted count.

## Physical read-only result

The follow-up candidate at `c92126ff7fa2d4841556ef7884e5d305d03c646c`
(tree `377cd6b28bc3f35597c5b89a9cd228dbfbe09c20`) passed the hosted
Windows lane. GitHub's synthetic PR merge had the same tree. Its exact Windows
host and source manifest were transferred under separate private names;
neither replaced the previously working binary. One Pi census then completed
with 347 records, `scanner_completed`, no supervisor error, confirmed cleanup,
and retired transport. Temperature after the census was 52.7°C with
`get_throttled=0x0`; later idle readback remained `0x0`.

The exact Pigments controller reports `IUnitInfo`, 302 units, and 16 program
lists. Units 1–16 bind channels 1–16 to lists 0–15. Every sampled list reports
128 programs. Each program-change parameter (IDs 2272–2287) belongs to its
corresponding channel unit and has 127 steps. The first four names in each
list are generic `ProgramChange 1` through `ProgramChange 4`, rather than
factory-preset names. The probe sampled the first 32 units and all 16 lists;
it did not walk 302 units or inspect vendor preset files.

This establishes a host-visible **MIDI program-slot** mechanism, not a
host-visible index of Pigments' factory library. Arturia's published guidance
for its individual instruments describes MIDI program changes as selecting
presets within Playlists/Songs/Presets; applying that guidance to this exact
Pigments build still requires a physical playlist-to-factory-preset proof.
The read-only `Previous Preset` and `Next Preset` parameters remain unavailable
as host-written commands. Assigning ShieldXL buttons to generic slot numbers
now would not satisfy the requested factory-preset navigation claim.

At that preflight point the headless physical button/audio/processing experiment
had not been launched. The raw census reports remain private; the allow-listed
scalar result is in
[`factory-preset-button-preflight.json`](../../evidence/rpi1/factory-preset-button-preflight.json).

## Exact MIDI-mapping result and headless physical attempt

At `988ed79870f5e371b61a4c4e5ed557df5302f9b5` (tree
`1dd53239b7f4d227a40c9a933da64ada62aea6e2`), the hosted Windows probe
added a read-only `IMidiMapping` observation. The exact Pigments controller
reported channel-1 CC28 -> parameter 2394 and CC29 -> parameter 2395, both
`MIDI CC helper` parameters. This is a host translation map, not proof that
either CC is assigned to Pigments' factory-preset actions. The installed
Arturia `Default` MIDI configuration has no CC28/29 preset assignment, while
the installed MiniLab mkII template names `Previous Preset` and `Next Preset`
for those CCs. These private configuration files were inspected, not copied
into this repository.

The experimental native source at `d5f472f35340aaa3c8557d6c5a38acbe10c68a03` (tree
`bb9b48e80fa76f7da53d53591312e789902f697e`) read exact ShieldXL evdev
buttons 2 and 3 off the JACK callback and translated press/release to CC28/29
helper parameter updates. Its Pi binary SHA-256 was
`e212d59463138016f0a4a1a2edb15d215a814f37eb6d0f8637a775f2910fbb39`.
Fifteen offline Pi unit tests and the `jack-runtime` release build passed. The
candidate stayed in a separate private path and did not replace the previously
working binary. The editor and Screen Sharing were closed throughout this
headless attempt.

The first physical run restored the proven 45% Pigments state, connected the
ShieldXL UART MIDI input and stereo playback ports, and took an opaque state
snapshot. The operator pressed button 3 three times. All three press/release
pairs reached the native command path, but the after-snapshot was byte-for-byte
identical to the before-snapshot (381,326 bytes). The operator also heard no
preset change. A source-owned deterministic note confirmed Pigments stereo
output when the physical controller initially delivered no notes; physical MIDI
later arrived. Final counters showed 67 accepted MIDI events, 483,408 nonzero
samples per channel, peak 0.26587746, zero xruns, deadline misses, process
failures, nonfinite output, or terminal transport fault. One 3,584-frame startup
gap occurred, with no later gap. The session recorded all 127 retirement
milestones and `RPI1_CLEAN_SHUTDOWN`.

A second, shorter headless check temporarily selected Arturia's installed
MiniLab mkII controller name in the **user preference** (the original bytes
were backed up). One next-preset press/release sent through the same native
command path still left the 381,326-byte Pigments state unchanged. It also
retired with all 127 milestones and no terminal transport fault. The original
Generic MIDI Controller preference was restored byte-for-byte. This negative
result applies to this tested host-parameter route; it does not prove that
Arturia's own MIDI-learn or controller-template UI cannot navigate presets.

The ineffective native button binding was reverted at
`abd5e821539472da2541082a7ec3d680d4ebfce2`. Button 2/3
must not be presented as working factory-preset navigation. The accepted 45%
state, published appliance binary, plug-in installation, UART route, and
ordinary audio configuration remain unchanged. JACK itself was restarted once
before the attempt because its connection socket was absent despite a live
server; the exact ports returned. The Pi ended idle, with JACK and UART MIDI
services active, `get_throttled=0x0`, and no Pigments ports or owner.

The next implementation boundary is the **vendor-owned preset action**. It
requires a verified Arturia-supported mapping that invokes Previous/Next
Preset while the editor is closed, followed by a state/audio change after one
button press. Arturia's [Pigments 7.0.1 manual](https://dl.arturia.net/products/pigments/manual/pigments_Manual_7_0_1_EN.pdf)
documents MIDI Learn for the preset arrows; this run did
not create or verify such a custom MIDI configuration. If the action requires
the editor to remain open, that is a product boundary to report, not a reason
to leave hidden graphics running in a claimed headless appliance. VST3 program
slots, helper parameter writes, and arbitrary preset-file indexing are not
evidence of the requested factory traversal and must not be substituted for it.
