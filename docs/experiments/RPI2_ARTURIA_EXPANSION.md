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

## Current Pi and implementation findings

At the read-only inventory, the RPI2 filesystem had 77.11 GiB available, the CPU
was 44.1°C, no experiment units were active, and the ARM test prefix contained
only Pigments in its VST3 directory. No matching Fragments or Analog Lab installer
was found in the checked Pi staging/download locations or Mac Downloads.

The transferable bridge already carries plugin audio and events. The Pi test
application is still fixture-specific:

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

## Execution order

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
with enough quiet time for the effect or note tail. Installation may also load
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

## Status

Research and source inspection are complete. Installer staging is recorded in
`evidence/rpi2/arturia-expansion-groundwork.json`. Neither installer has been
executed, neither new product has been activated, and the Pi host has not yet
been changed for these products. No audio or performance result is claimed.
