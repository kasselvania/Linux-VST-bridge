# FATES0 — Raspberry Pi 5 hardware bring-up

## Purpose

FATES0 prepares a reproducible Raspberry Pi 5 + Fates v1.8.1 hardware platform for
the separate RPI0 standalone-host work. It is an independent parallel experiment,
not a ShieldXL revision and not a plug-in-host task.

The bounded hardware contract is:

```text
Fates v1.8.1 WM8731 stereo input/output and headphone output
+ JACK2 stereo capture/playback ports
+ USB MIDI
+ three required encoders and three buttons
+ optional fourth encoder
+ 128x64 SSD1322 OLED
+ stable aliases, rollback, thermal records, and a machine-readable handoff
```

## Exact branch basis

- repository: `kasselvania/Linux-VST-bridge`
- branch: `codex/fates0-pi5-hardware-bringup`
- base: `c38b5c9deb8bd32f5a509258c30f31b3e795fe59`
- base tree: `6d661a80a707157f30d3f4815e04a72b1cb38586`

This branch is experimental, parallel to SHIELDXL0, and must not be merged
automatically. It does not modify the SHIELDXL0 or RPI0 branches.

## Selected fixture

- Raspberry Pi 5 Model B, 8 GB RAM;
- operator-identified Fates PCB revision v1.8.1 on the 40-pin header;
- official 5 V / 5 A-class power connected to the Raspberry Pi power input;
- Fates board USB-C power disconnected;
- fresh removable storage using the pinned AArch64 image;
- board open and initially without active cooling;
- HDMI, keyboard/mouse, Wi-Fi, physical stereo loopback cable, and USB MIDI;
- existing norns media retained unchanged.

The printed PCB revision is operator evidence until photographed or otherwise
independently retained. Never power the Pi and Fates USB-C inputs simultaneously.

## Prior art and licensing boundary

Fates prior art is pinned to:

- repository: `https://github.com/okyeron/fates`
- commit: `f7f4cf69a8886f77be943c564b69f925b95d2e88`
- selected hardware revision: v1.8.1

The inspected repository supplies board facts, assembly guidance, historical scripts,
and old overlays, but no repository-wide license declaration was found. No Fates source
file is copied or adapted here. The FATES0 overlay is an original implementation from
the documented electrical pin assignments and current Raspberry Pi bindings.

The OLED userspace service separately adapts the SSD1322 initialization and wire-format
path from `monome/linux` commit
`458e2253667a09bd01f1d50cc2b4063c14c491ec`, file
`drivers/staging/fbtft/fb_ssd1322.c`, under GPL-2.0-only. Attribution, source hash and
the license text are retained with the tooling.

Relevant v1.8.1 facts:

- WM8731 codec at I2C address `0x1a` with a 12.288 MHz crystal;
- two 3.5 mm line inputs, two 3.5 mm line outputs, and stereo headphone output;
- encoders GPIO `(5,6)`, `(13,12)`, `(27,22)`, optional `(26,16)`;
- buttons GPIO `24`, `25`, `23`;
- OLED reset GPIO4, data/command GPIO17, SSD1322 on SPI0;
- optional board USB-C power input that must not be used simultaneously with Pi power;
- raw UART header, but no accepted onboard physical TRS MIDI interface.

Historical Fates images and installers are prior art only. Do not copy their complete
boot configuration, enable root login, use `/home/we`, force the performance governor,
write codec registers directly with `i2cset`, or install their old kernel/image.

## Pi 5 adaptation

Use the pinned current distribution kernel. It already ships:

- `snd-soc-wm8731` and `snd-soc-wm8731-i2c`;
- `snd-soc-rpi-proto`;
- the official `proto-codec.dtbo` overlay (the current name for historical
  `rpi-proto`);
- RP1 I2S, I2C, SPI and GPIO support;
- rotary-encoder, gpio-keys and spidev support.

Do not build a custom kernel or codec module. FATES0 adds one original overlay for RP1
GPIO controls and SPI0 userspace access. It uses the packaged `proto-codec` overlay for
audio.

## Provisioning and refusal posture

Provisioning must refuse:

- any board other than Raspberry Pi 5 Model B with 8 GB RAM;
- non-AArch64, wrong image, kernel or page size;
- a hardware profile other than explicit `v1.8.1`;
- a declared power source other than the Pi input;
- absent WM8731 acknowledgement at I2C address `0x1a`;
- ShieldXL, old Fates, UART-MIDI, overclock, or foreign overlay configuration;
- foreign replacement files or prohibited translation/runtime software.

Provision in two stages: enable only I2C, reboot and admit address `0x1a`, then install
the full audio/control/SPI configuration. Services remain disabled until ordered ALSA
verification succeeds. OLED failure must not prevent audio startup.

## Physical verification order

1. Clean Pi 5 baseline and thermal inventory without experimental overlays.
2. Bounded I2C presence at `0x1a`.
3. `proto-codec` overlay, WM8731 binding and stable ALSA identity.
4. Silent playback, low-level line playback, line capture and stereo physical loopback.
5. JACK2 at 48 kHz with 128/256/512/1024 frames and ordinary-user restart.
6. Three encoders, optional fourth encoder, and three buttons.
7. OLED clear/fill/text/changing-number and independent restart.
8. USB MIDI note, velocity, sustain and all-notes-off behavior.
9. Reboot persistence and complete integration contract.

Every physical run records temperature before/during/after, throttling flags, observed
CPU frequency, workload duration, cooling fixture, and whether Fates obstructs airflow.
Do not overclock or conceal throttling.

## Acceptance and nonclaims

FATES0 is accepted only after physical stereo playback/capture/loopback, stable JACK
ports, USB MIDI, all three required encoders, all three buttons, OLED output, service
restart, reboot persistence, and unthrottled bounded functional testing are retained.
Encoder 4 and UART MIDI are optional follow-ups.

Static DTBO compilation/application and local test success are preparation evidence,
not hardware acceptance. This lane does not install or run Box64, FEX, Wine, Proton,
Native Access, commercial software, a VST host or a DAW.

## Delivery

Retain the eventual machine-readable contract at
`evidence/fates0/hardware-contract.json`, a human report and RPI0 handoff beside it, and
one draft PR against `main` titled:

```text
Experiment: FATES0 Pi 5 hardware bring-up — do not merge
```

Stop at the complete hardware contract and draft experimental PR. Do not merge.
