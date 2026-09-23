# RPI2: Fragments and Analog Lab Pro groundwork

## Selected direction

The operator selected Efx FRAGMENTS first, then Analog Lab Pro, using the existing
ARM Wine/FEX runtime on Pi 5. REAPER is explicitly deferred for a later DAW demo.
The immediate task is installation groundwork followed by short functional audio
checks while waiting for active cooling. This is not a sustained-load campaign.

The operator reported a massive improvement in the responsiveness and stability
of the ARM-runtime Pigments session. Preserve that user observation alongside
the recorded gaps and short-window measurements in [RPI2](RPI2.md).

## What a successful result would establish

Fragments exercises an actual Windows effect: stereo input through the vendor
processor to stereo output. Analog Lab Pro exercises another commercial Windows
instrument and several sound-engine families through one plugin. Verify the
installed x86-64 module identity before and after testing; use the vendor binary
unchanged. No vendor source port or rewritten synth is proposed.

That would demonstrate a way to reuse existing desktop Windows instruments on
ARM Linux. It would not establish equivalence to a finished hardware instrument,
all Analog Lab engines/presets, a native ARM plugin, or Arturia's internal costs
and development history. The latter are not established by this research.

## Public packages checked on 2026-09-23

| Product | Selected Windows package | Public download size | Purpose |
| --- | --- | --- | --- |
| Efx FRAGMENTS | 1.3.1.6566 | 171.95 MB as listed by Arturia | Effect input/output and state |
| Analog Lab Pro | Analog Lab V 5.12.5.6878 | 3.46 GB as listed by Arturia | Single notes from distinct instrument families |

Sources: [Fragments downloads](https://www.arturia.com/support/downloads-manuals/product/efx-fragments),
[Analog Lab downloads](https://www.arturia.com/support/downloads-manuals/product/analoglab-v).
The operator confirmed owning Analog Lab Pro. Arturia explains that the plugin
and application retain the name **Analog Lab V** even though the edition is Pro.
Its supplied sounds span V Collection instruments; installing every separate
instrument is not the starting requirement. Full instrument editing and added
content have their own ownership requirements. Use the normal ASC activation
flow for the edition actually licensed by the operator.
[Arturia product FAQ](https://support.arturia.com/hc/en-us/articles/11077881030556-Analog-Lab-Pro-General-Questions),
[product description](https://www.arturia.com/store/software-instruments/analoglab-v).

The public packages are staged privately on the Pi. Their download sizes and
SHA-256 hashes are recorded separately; a download hash pins received bytes and
is not a vendor signature validation or an installation result. Proprietary
installers, state, presets and licensing material stay outside Git.

## Initial Pi and implementation findings

At the read-only inventory, the RPI2 filesystem had 77.11 GiB available, the CPU
was 44.1°C, no experiment units were active, and the ARM test prefix contained
only Pigments in its VST3 directory. No matching Fragments or Analog Lab installer
was found in the checked Pi staging/download locations or Mac Downloads.

The transferable bridge already carried plugin audio and events. At the initial
inspection, the Pi test application was still fixture-specific:

- `rpi1/standalone/src/config.rs` requires the Pigments module hash/class.
- `rpi1/standalone/src/contract.rs` supplies Pigments' exact bus census and class
  to the Windows host. Its auxiliary stereo input receives silence.
- `rpi1/standalone/src/master.rs` assumes parameter 0 is Master Volume in dB.
- `rpi0/standalone/src/jack.rs`, reused by the Pigments app, registers MIDI input
  and stereo output, but no external audio input ports.
- `rpi1/qualification_jack.rs` already has short tone and note generation plus
  stereo capture. Its current tone mode connects the generator directly to the
  meters and playback, so an effect test must instead route generator -> bridge
  input -> bridge output -> meters. Reuse the stimulus/capture mechanics; do not
  mistake the present direct-tone check for a plugin-processing result.

Therefore changing only the plugin filename would be wrong. The next host change
should select the exact plugin identity and discovered bus layout as data,
preserve Pigments' known layout, and provide stereo JACK input for an effect.
Do not carry Pigments-specific parameter IDs or event-bus exceptions into the
other products without their own reported metadata. The Windows host and shared
audio transport should be reused unless an observed interface gap requires a
change. A new host application per vendor product is not the intended design.

## Selected execution order

1. **Install and discover Fragments.** Reuse the pinned ARM runtime and existing
   Arturia environment/machine identity. Preserve the working Pigments setup and
   private state before changes. Run the official installer under the existing
   process and resource supervision; inspect its installed module version,
   class, audio/event buses and parameters. Compare with retained Deck/Ubuntu
   identities rather than assuming that every historical Fragments result used
   this build. ASC activation remains an operator/vendor interaction.
2. **Connect effect input through the existing bridge.** Use the discovered
   Fragments bus layout and expose stereo JACK input. First use the existing
   short, low-level signal source, retaining input and output. Observe a dry or
   bypass setting and a clearly wet setting, allowing for the effect's tail.
   Nonzero output alone cannot establish processing: show input delivery and a
   meaningful output change. Capture gap counters separately from JACK xruns.
3. **Check one state change.** Save the selected Fragments setting, close the
   process, restore, and compare the actual setting and short processed output.
   Manual physical input/listening can follow once digital routing is confirmed.
4. **Install and discover Analog Lab Pro.** Reuse the same runtime, normal ASC
   flow and plugin discovery. Select one single-instrument preset from each of
   three available families, for example analog, FM and sample-based. Record the
   exact instrument/preset names. Play isolated notes and note-offs, verify
   preset switching, then save/restart/restore one selection. Defer layered
   Multis and polyphony ladders.

Run only one test instance at a time. Begin each audio check cool, retain the
existing 75°C/current-throttle stop conditions, and use seconds-long captures
with enough quiet time for the effect or note tail. The operator subsequently
requested a longer listening pass; its two-minute result is recorded below.
Installation may also load
the CPU: a thermal interruption is an incomplete installation to inspect before
retrying. Do not blindly replay or recreate the prefix. These short runs establish
functionality only; sustained performance remains for the active cooler.

The same supervised standalone host can potentially load a plugin that has no
vendor standalone application, including Serum 2. A complete DAW is not inherently
required. Each plugin's interface requirements still need verification. REAPER
later adds the distinct Linux ARM proxy/DAW/project-recall boundary; it is not a
dependency of this installation and short-playback work.

## Cooling

A small passive aluminium heatsink can help: it increases heat transfer area and
can delay a temperature rise. Its benefit depends on contact, size and airflow;
there is no measurement for the proposed topper here. Raspberry Pi's own test of
the larger official cooler with its fan disconnected still reached throttling
after roughly 200 seconds of heavy load. This supports short tests now and active
cooling for sustained work; it does not make a passive heatsink useless or prove
our earlier 74°C gap was thermal throttling.
[Raspberry Pi cooling measurements](https://www.raspberrypi.com/news/heating-and-cooling-raspberry-pi-5/).

## Fragments implementation and observed result

Efx FRAGMENTS 1.3.1.6566 is installed in the existing private ARM Arturia
environment. The operator activated it through ASC. The official installation
completed in 69.67 seconds and left the Pigments module hash unchanged. Discovery
found a stereo main input, stereo auxiliary sidechain, stereo output, a 16-channel
MIDI input and 2,415 parameters. The sidechain is explicitly inactive in this
test; external sidechain processing is not claimed.

The native host now selects the exact module, class, bus layout and selected
control metadata from a pinned binding. It exposes stereo JACK input and sends
those samples through the existing shared-memory transport and Windows host.
The Windows host and vendor module were reused unchanged. This is the actual
Windows VST3 hosted by our standalone bridge, not Arturia's standalone executable
or a Linux rewrite. The incremental native build took 26.06 seconds including
supervision. The previously installed Pigments executable was preserved.

`rpi2/fragments-binding.json` contains the non-sensitive module identity and
configuration. A native configuration selects it with `binding_path` and
`binding_sha256`, replacing the four `pigments_*` fields; it uses
`event_output_policy=strict` and a distinct JACK client name. The binding's bus
rows are `[media, direction, index, channels, kind, active, arrangement]` using
the existing VST3 setup protocol. Unsupported layouts and mismatched identities
are refused before audio activation. Pigments' default binding remains available.

The `lvb-arm-plugin-standalone` target uses the same application code, and
`pigments_session.py` accepts optional absolute `--config` and `--binary` paths.
`parameter id [normalized]` uses the existing bounded audio parameter queue and
controller channel, checking readback against selected parameter metadata. The
Fragments binding selects Grain Mix (1, percent) and Freeze (8). The legacy
Pigments `master` command is not applied to Fragments.

The capture helper's new `effect` mode routes its low-level stereo stimulus
through the plugin; `input` routes the physical ShieldXL capture ports through
the plugin. Both retain five seconds of stereo input and output privately.
The original `tone` mode still measures direct tone routing.

Observed results with the passive aluminium heatsink and externally powered
PiSugar 3 Plus:

- Two digital captures delivered bit-identical input with Grain Mix at 0% and
  100%, verified by parameter readback. Their outputs differed, including a much
  stronger final-second tail at 100%. Grain Mix 0% is not a full plugin bypass;
  the other effects can still process audio.
- A 162,176-byte private state was saved, the process closed, and a fresh process
  restored Grain Mix to 100% before any parameter write. This is one setting's
  process-restart recall, not broad preset or DAW project persistence.
- Both physical input channels contained the user's source and produced finite
  stereo output. The operator confirmed hearing both short passes and the effect
  change. No editor was opened for these processing or recall tests.
- All four five-second capture intervals added zero bridge gaps and zero JACK
  xruns. Each session already had one 1,024-frame startup gap; that limitation
  remains visible. Full-session temperature peaked at 50.7°C for the physical
  test, with no throttle flags. These measurements do not identify the cause of
  any earlier Pigments failure.
- At the operator's request, a fresh session then ran physical stereo input
  continuously for 120 seconds at Grain Mix 100%, with no changes to routing or
  controls during that interval. Temperature was 45.2–48.5°C, sampled supply
  voltage 5.01562–5.05716 V, and throttle flags remained zero. It added zero
  missing frames, bridge gaps, JACK xruns, process failures or callback deadline
  misses. The operator reported that it sounded great. This longer pass used
  live counters and listening, not a continuous audio recording.

All three audio sessions ended with an intentional quit, clean host shutdown,
no remaining session directories and the original JACK graph restored. The
silence at the ends of the short passes and continuous pass was deliberate.

Sample rate remains 48 kHz, JACK period 512 frames, Windows process blocks 256
frames and bridge reserve 2,048 frames. Fragments reported 192 vendor frames,
giving 2,240 total frames (46.67 ms). This is reported transport/plugin latency,
not a measured physical round trip or an optimized latency result.

Focused checks covered binding identity, stereo buses, inactive sidechain and
parameter metadata refusals, plus the existing standalone and capture tests.
The ARM build and actual digital and physical sessions exercised the Linux JACK
path. Sanitized observations are in
`evidence/rpi2/fragments-arm-stereo.json`; installers, audio, state and licensing
material remain private.

## Battery-only repeat

The operator unplugged the PiSugar 3 Plus USB-C charger while the audio host was
idle and confirmed that the Pi stayed on. A fresh Fragments process restored the
same private state, confirmed Grain Mix 100% and Freeze off, and processed the
same physical stereo route continuously for another 120 seconds. No binary,
buffer, routing or control changes were made during this repeat. The operator
reported that it sounded just as good.

| Audio interval | External power connected | Battery only |
| --- | --- | --- |
| Duration | 120 seconds | 120 seconds |
| CPU temperature | 45.2–48.5°C | 43.55–46.85°C |
| Sampled supply at the Pi | 5.01562–5.05716 V | 5.01562–5.05984 V |
| Added bridge gaps / JACK xruns | 0 / 0 | 0 / 0 |
| Throttle flags | 0 | 0 |

The battery repeat also added zero missing frames, process failures, callback
deadline misses or paused frames. Its pre-interval 1,024-frame startup gap remains
recorded. Full-session temperature, including launch and teardown, peaked at
47.4°C. Shutdown was intentional and clean; no experiment units or session
directories remained, and the JACK graph was restored. The Pi was left on.

This establishes one battery-powered Fragments workload with user-confirmed
audio, not battery endurance or a measured power budget. The supply readings are
at the Pi, not battery current or energy measurements. Unplugging happened before
the audio session, so switchover during processing was not tested. Different
initial conditions do not establish a thermal advantage of battery power.
Sanitized results are in `evidence/rpi2/fragments-battery.json`.

## Remaining work

Analog Lab Pro is staged but has not been installed, activated or tested.
REAPER remains deferred. Active-cooling capacity tests, editor interaction,
lower latency, external sidechain use and measured battery power/endurance are
separate follow-ups. The earlier download-only result in
`evidence/rpi2/arturia-expansion-groundwork.json` remains an unchanged historical
observation.
