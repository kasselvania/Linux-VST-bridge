# SHIELDXL0 — Raspberry Pi 5 hardware bring-up

## Purpose

SHIELDXL0 prepares the physical Raspberry Pi 5 + shieldXL appliance platform that the
separate RPI0 ARM/translation agent will consume.

This lane is not a VST host, Box64/Wine port, DAW, or commercial plug-in test. Its job
is to establish one reproducible current AArch64 Linux hardware contract:

```text
shieldXL CS4270 stereo codec
+ JACK/PipeWire-JACK audio ports
+ USB MIDI and optional shield TRS MIDI
+ three rotary encoders
+ three pushbuttons
+ shieldXL OLED
+ stable device identities and restart behavior
```

The resulting contract should be usable by the future standalone ARM instrument host
without importing norns itself or depending on an obsolete 32-bit image.

## Exact branch basis

- repository: `kasselvania/Linux-VST-bridge`
- branch: `experiment/shieldxl0-pi4-hardware-bringup`
- base: `a86f03e8a5d5302d9872f995a0a5ba376a0ab6d5`
- base tree: `11a9fefbbe8b184c52058f4b4610da3a5e3b06e4`

This branch is experimental. Do not merge it automatically into the ordinary x86-64
product line or into the separate RPI0 branch.

The branch name predates the fixture change and is retained for custody continuity. It
does not authorize a Pi 4 claim. The selected physical target is the Pi 5 fixture below.

## Physical fixture

Primary fixture:

- Raspberry Pi 5 with 8 GB RAM attached to a shieldXL through the standard 40-pin
  header;
- fresh removable storage for this experiment;
- existing working norns/shield card retained unchanged as rollback/reference media;
- correct 5 V / 5 A-class power;
- initially no active cooler, with the board open and unenclosed;
- HDMI display and USB keyboard/mouse available;
- network access only for pinned package/source acquisition;
- shieldXL line output connected to a safe monitor, mixer, or powered speaker;
- physical loopback cable available between shieldXL line output and line input;
- USB MIDI controller available.

Record the exact model, board revision, and RAM rather than assuming them.

A Raspberry Pi 3 Model B Plus may be used only for non-destructive preparation or a
bounded ShieldXL wiring/interface check when that materially reduces risk. It is not an
accepted audio-performance, sustained-performance, RPI0, or translated-plug-in fixture.
Never weaken the Pi 5/8 GB provisioning gate to admit it.

Never modify the existing working norns SD card. If it is inspected, mount it read-only.

## Pinned prior art

Treat these sources as prior art, not as scripts to execute blindly:

### shieldXL hardware repository

- repository: `okyeron/shieldXL`
- commit: `eace215e99e8e0c3cb06dcd029ff14964da433c2`

Relevant retained facts:

- CS4270 codec;
- stereo line input and output;
- NHD-2.7-12864WDW3-M OLED;
- three encoders and three buttons;
- Pi 4 installation precedent;
- UART/TRS MIDI precedent.

### monome kernel prior art

- repository: `monome/linux`
- commit: `458e2253667a09bd01f1d50cc2b4063c14c491ec`

Relevant source:

- `arch/arm/boot/dts/overlays/monome-snd-4270-overlay.dts`
- `arch/arm/boot/dts/overlays/norns-buttons-encoders-overlay.dts`
- the prior `ssd1322-spi` overlay/driver path;
- shield platform configuration.

The old audio overlay describes a `simple-audio-card`, I2S, two 32-bit slots, CS4270 at
I2C address `0x48`, 12.288 MHz system clock, and reset GPIO 17.

The shieldXL pin overrides are:

- encoder 1: GPIO 4 / 27;
- encoder 2: GPIO 12 / 25;
- encoder 3: GPIO 24 / 23;
- button 1: GPIO 22;
- button 2: GPIO 26;
- button 3: GPIO 13.

The historical ShieldXL boot file also enables SPI, I2S, I2C, the OLED overlay, the
CS4270 overlay, the button/encoder overlay, and UART MIDI.

Do not copy its old kernel image, hard-coded user paths, old ARMHF packages, permissive
file modes, or unpinned downloads into this experiment.

## Work separation from RPI0

The separate RPI0 agent owns:

- native ARM64 bridge/backend work;
- standalone VST host;
- Box64/FEX and Wine/Proton;
- x86-64 Windows VST3 fixture;
- translated editor and DSP;
- plug-in latency and translated cohort performance.

SHIELDXL0 owns only:

- modern 64-bit Pi OS provisioning;
- ShieldXL kernel/device support;
- ALSA/JACK/PipeWire audio;
- hardware MIDI exposure;
- encoders/buttons;
- OLED;
- exact hardware identity, verification, and integration contract.

Do not edit the RPI0 branch. Do not cherry-pick unreviewed RPI0 work. The two agents
coordinate only through committed interface records and concise handoffs.

## Operating-system posture

Use a current stable 64-bit Raspberry Pi OS or Debian-family Pi image available at the
start of the run. Record and pin:

- image name and digest where available;
- release/version;
- kernel version and package identity;
- boot firmware identity;
- architecture and page size;
- enabled repositories;
- every installed package and exact version.

Do not use the historical norns image as the new product image.

Prefer the distribution kernel. Before considering a custom kernel, determine whether
the current kernel already provides:

- `snd-soc-cs4270`;
- `simple-audio-card`;
- RP1 DesignWare I2S support, including the external-clock-consumer path;
- RP1 GPIO, I2C, and SPI support for the 40-pin header;
- rotary-encoder input support;
- gpio-keys;
- the required SPI/OLED support.

Build or patch a kernel only when a concrete missing driver or incompatible interface is
shown. Preserve the exact failure that required it.

### Exact kernel profiles

SHIELDXL0 admits two and only two distribution-kernel/page-size pairs from the same
pinned Raspberry Pi OS image and package version:

- `shieldxl0-16k`: `6.18.50+rpt-rpi-2712`, 16,384-byte pages. This remains the isolated
  hardware-bring-up profile and refuses Box64, Wine, Proton, and FEX binaries.
- `rpi0-4k-integration`: `6.18.50+rpt-rpi-v8`, 4,096-byte pages. This is the exact RPI0
  integration profile after the retained 16 KiB Box64/Wine failure. It may coexist with
  the separately owned RPI0 runtime but SHIELDXL0 neither installs nor runs that runtime.

Both profiles use package version `1:6.18.50-1+rpt1`, the same Pi 5 base DTB digest,
and the same unmodified pinned upstream Linux v6.18 CS4270 source. The codec module must
be built against the exact matching headers and installed only below the corresponding
`/lib/modules/<release>` tree. Crossed kernel/page-size pairs refuse.

The 4 KiB integration profile admits exactly `kernel=kernel8.img` in the `[all]` boot
section. Any other explicit kernel selector refuses. This is a bounded integration
selector, not permission to replace the distribution kernel or weaken the ordinary
16 KiB hardware profile.

## Reproducible provisioning

Add an experiment-owned directory such as:

```text
tools/shieldxl0/
```

It should contain:

- a manifest of pinned source/package inputs;
- overlay source copied or adapted with license/provenance retained;
- deterministic overlay build commands;
- idempotent provisioning scripts;
- systemd units only where needed;
- udev rules for stable hardware aliases;
- verification scripts;
- an uninstall/rollback procedure for this fresh card;
- no credentials or local network configuration.

Provisioning must refuse an unsupported architecture, board, kernel, missing HAT, or
unexpected existing configuration rather than overwriting it silently.

Do not execute the historical `install-midi.sh` wholesale. In particular, do not retain
its hard-coded `/home/we`, ARMHF package, `chmod 777`, unpinned clones, or direct
replacement of the complete boot configuration.

## Phase 1 — passive inventory

Before changing the fresh image, record:

- `/proc/device-tree/model`;
- board revision;
- `uname -a`;
- `dpkg --print-architecture`;
- page size;
- RAM;
- boot configuration locations;
- loaded modules;
- current ALSA cards/devices;
- I2C buses and safe device-address census;
- input devices;
- SPI devices;
- serial aliases;
- GPIO consumer state;
- current PipeWire/JACK state;
- current temperature, clocks, governor, and throttling flags.

Do not probe arbitrary I2C registers. A bounded address presence check is sufficient.

## Thermal observation discipline

The absence of active cooling does not block functional bring-up. Begin with the board
open and unenclosed. Do not overclock.

Every physical run, including short functional tests, must retain:

- temperature before, sampled during, and after;
- `vcgencmd get_throttled` before, during, and after;
- observed CPU frequency before, during, and after;
- monotonic workload duration;
- whether the attached ShieldXL obstructs passive airflow;
- any external-airflow fixture change.

Do not shorten or reduce a declared workload merely to avoid an observed thermal limit.
If a sustained run throttles, retain the result and classify sustained performance as
thermally unqualified. That classification does not erase otherwise valid bounded
functional hardware results.

## Phase 2 — CS4270 audio

Adapt the pinned `monome-snd-4270` overlay against the selected current Pi 5/RP1 kernel.
Do not assume the old Pi 4 targets, clock direction, pin controller, or bus aliases work
unchanged. The CS4270 owns bit and frame clocks, so the Pi 5 CPU DAI must use the RP1 I2S
clock-consumer controller. Retain exact source and compiled DTBO digests and prove static
application against the exact packaged Pi 5 base DTB before physical admission.

The accepted card must expose:

- one stable ALSA card identity;
- stereo playback;
- stereo capture;
- 48 kHz full-duplex operation;
- a documented supported sample format;
- deterministic mixer state;
- no unexplained kernel errors or repeated XRUNs during the bounded test.

Verify in this order:

1. overlay loads and the codec is present at the expected I2C address;
2. ALSA card and PCM devices appear;
3. silent playback starts/stops cleanly;
4. bounded low-level tone playback reaches the physical line output;
5. bounded capture reads the physical line input;
6. physical output-to-input loopback records the expected tone in both channels;
7. repeated open/close does not lose the card;
8. reboot preserves the exact card identity.

Start at 48 kHz with conservative periods/buffers. Do not pursue lowest latency here.

Keep HDMI and the Pi analogue output available as diagnostic alternatives; do not make
them the accepted ShieldXL audio path.

## Phase 3 — JACK / PipeWire-JACK contract

Select one primary low-latency server already suitable for the chosen OS:

- PipeWire with JACK compatibility; or
- JACK2.

Do not maintain two conflicting production configurations.

Expose stable ports equivalent to:

```text
shieldxl:capture_1
shieldxl:capture_2
shieldxl:playback_1
shieldxl:playback_2
```

Exact naming may differ, but write the discovered names and aliases into the integration
record.

Verify:

- 48 kHz;
- 128, 256, 512, and 1024-frame periods when supported;
- full-duplex loopback;
- start/stop/restart;
- bounded XRUN count;
- no device disappearance after server restart;
- the server can be used by an ordinary user without running the future synth as root.

The first integration recommendation should be conservative: 256 or 512 frames unless
actual results require a larger value.

## Phase 4 — encoders and buttons

Adapt or reuse the pinned `norns-buttons-encoders` overlay against the current kernel.

Expose all six controls through stable udev-backed evdev aliases rather than `/dev/input/eventN`
numbers.

For each encoder, record:

- stable device identity;
- direction;
- detent behavior;
- events per detent;
- bounce/reversal observations;
- maximum bounded manual-turn event rate;
- behavior across reboot.

For each button, record:

- stable device identity;
- press and release codes;
- active level;
- debounce/repeat behavior;
- behavior across reboot.

Do not map controls to plug-in parameters in this lane. Produce a small non-real-time
reader or library that emits bounded typed events for the RPI0 host to consume later.

Suggested logical contract:

```text
encoder(index=0..2, delta=signed integer, monotonic_ns)
button(index=0..2, pressed=bool, monotonic_ns)
```

No control reader may run inside the future audio callback.

## Phase 5 — OLED

Bring up the ShieldXL OLED using the pinned prior art or a current maintained path.

The result must expose a stable user-space drawing interface suitable for a small status
surface. A framebuffer, DRM device, or narrowly owned SPI user-space service is
acceptable when its ownership and performance are explicit.

Verify:

- exact display geometry;
- orientation;
- complete clear/fill/text test;
- update rate adequate for status and parameter values;
- no SPI conflict with other ShieldXL functions;
- clean stop/restart;
- boot-time failure does not break audio.

Do not build the final synth UI in this lane. Supply a minimal status renderer and an
interface contract for later display of preset, macro values, CPU, gaps, and thermal
state.

## Phase 6 — MIDI

USB MIDI is the required first path.

Verify:

- one physical USB MIDI controller appears through ALSA sequencer and JACK MIDI;
- note-on, note-off, velocity, sustain, and all-notes-off can be observed;
- hot-unplug is reported rather than blocking the server;
- reconnect restores a stable selectable identity.

ShieldXL TRS/UART MIDI is a secondary result. Use the pinned ShieldXL/ttymidi work only
as prior art. If implemented:

- establish the exact UART and baud/clock configuration physically required by this
  hardware;
- pin the exact ttymidi source or replace it with a smaller reviewed equivalent;
- do not install old ARMHF binaries on the AArch64 system;
- use a least-privilege systemd unit;
- record Type A/B switch posture and actual successful direction;
- prove ordinary MIDI events through the physical jack.

Failure of TRS MIDI does not block the first RPI0 integration when USB MIDI passes, but
it must remain visible as incomplete.

## Integration record

Publish a machine-readable record, for example:

```text
evidence/shieldxl0/hardware-contract.json
```

It must include:

- Pi model, RAM, OS, kernel, boot firmware, architecture, page size;
- ShieldXL prior-art commits;
- overlay source and DTBO digests;
- ALSA card and PCM identities;
- JACK/PipeWire server and exact port names;
- accepted rate/period/buffer combinations;
- loopback result and XRUN counts;
- USB MIDI identity and port names;
- UART MIDI status;
- encoder/button stable aliases and event semantics;
- OLED device/interface and geometry;
- temperature, throttling, CPU-frequency, duration, airflow-obstruction, and cooling-
  fixture observations for every physical run;
- provisioning commit and package manifest;
- known limitations.

Also provide a concise human-readable `README.md` beside it.

The record must contain no Wi-Fi credentials, SSH keys, hostnames that identify the
operator, private network addresses, or unrelated device inventory.

## Physical interaction protocol

The agent may prepare scripts and commands without the Pi.

When physical work is needed, give the operator one concise, reversible instruction at
a time, such as:

- write the named image to the fresh SD card;
- insert the card and boot;
- connect the loopback cable;
- turn encoder 1 clockwise;
- press button 2;
- connect the MIDI keyboard;
- confirm whether the test tone is audible.

Do not assume an action occurred. Wait for the operator's response before interpreting
its result.

Never ask the operator to alter the working norns SD card.

## Tests and acceptance

SHIELDXL0 passes only when the fresh AArch64 image demonstrates:

1. repeatable boot;
2. ShieldXL CS4270 stereo playback and capture at 48 kHz;
3. physical line-output-to-line-input loopback;
4. stable JACK/PipeWire-JACK stereo ports;
5. USB MIDI note/control input;
6. all three encoders;
7. all three buttons;
8. OLED test output;
9. clean audio-server restart;
10. reboot with all accepted identities preserved;
11. complete thermal observations for every run, with sustained performance explicitly
    classified as qualified or thermally unqualified;
12. a complete integration record suitable for the RPI0 agent.

TRS MIDI is desirable but may remain a recorded follow-up if USB MIDI passes.

No Windows plug-in, Box64, Wine, Proton, Native Access, Pigments, Serum, source synth,
VST host, or DAW result is part of SHIELDXL0 acceptance. The thermal permission for a
future short integration smoke test does not authorize installing or running those
components in this hardware task.

## Hard boundaries

Do not:

- modify the active x86-64 product installation;
- modify or erase the working norns SD card;
- edit the RPI0 branch;
- run a DAW;
- install Box64/FEX, Wine, Proton, or commercial software in this lane;
- run the future VST host;
- claim plug-in performance from native audio tests;
- run the audio process as root merely to avoid correct permissions;
- replace exact device ownership with broad chmod or `chmod 777`;
- copy old kernel binaries into the new image;
- commit an SD-card image, credentials, paid content, or private system data;
- merge the experimental branch automatically.

## Delivery

Open one draft PR against `main` titled:

```text
Experiment: SHIELDXL0 Pi 5 hardware bring-up — do not merge
```

Return:

- draft PR URL;
- exact branch/head/tree;
- exact OS/kernel and Pi identity;
- pinned prior-art inputs;
- overlay and package identities;
- audio/JACK results;
- loopback and XRUN results;
- MIDI results;
- encoder/button results;
- OLED result;
- integration-record path;
- provisioning and rollback instructions;
- physical failures and retained limitations;
- concise handoff for the separate RPI0 agent.

Stop when the hardware integration contract is ready in the draft experimental PR. Do
not merge.
