# SHIELDXL0 Raspberry Pi 4 hardware platform

This directory provisions only the Raspberry Pi 4 + ShieldXL hardware lane selected by
`docs/experiments/SHIELDXL0.md`. It does not install or run a VST host, DAW, Box64/FEX,
Wine/Proton, Native Access, commercial plug-in, or licensing software.

The accepted base is the official **Raspberry Pi OS Lite (64-bit), Trixie,
2026-09-15** image:

```text
2026-09-15-raspios-trixie-arm64-lite.img.xz
SHA-256 cdf4f3bfac35ae947b46e4e767f935453810549779ac3290e05a6754aee627e5
```

`pinned-inputs.json` records the image, kernel, firmware, exact added packages, source
commits, inspected-file hashes, and adapted-overlay hashes. `verify-image.sh` verifies a
download before it is written to a **fresh** SD card. Never point an imaging tool or any
script here at the retained working norns card.

## Why one external kernel module exists

The exact official Pi 4 kernel package was inspected before implementation. It ships
the I2S, simple-card, rotary-encoder, GPIO-key, and spidev drivers but has
`CONFIG_SND_SOC_CS4270` disabled. `kernel-driver-admission.json` preserves that concrete
deficiency. `build-cs4270-module.sh` builds the unmodified upstream Linux v6.18 codec
driver against the exact distribution headers; it does not replace the kernel image.

The old out-of-tree SSD1322 framebuffer driver is not carried. The display uses the
distribution spidev module through a bounded ordinary-user service. The adapted overlay
and upstream codec source remain GPL-2.0-only; see `THIRD_PARTY_NOTICES.md`.

## Two-stage, fail-closed provisioning

Run these from an exact checkout of the experiment branch on the fresh Pi image.

1. Before changes, run `./inventory.sh PRIVATE_OUTPUT_DIRECTORY`. Keep that output
   private until it has been reviewed and sanitized.
2. Run `sudo ./prepare-buses.sh`. It adds one managed boot include and enables only I2C.
3. Reboot the fresh card.
4. Run `sudo ./provision.sh --user YOUR_ORDINARY_USER`.
5. Reboot again, then run `./verify-platform.sh PRIVATE_OUTPUT_DIRECTORY`.

The split is deliberate: full provisioning will not proceed until the physical codec
acknowledges exactly address `0x48` through one SMBus Quick presence transaction. It also
refuses the wrong architecture, wrong board, wrong image, wrong kernel, wrong page size,
prohibited RPI0 software, unexpected boot configuration, package-version drift, or a
foreign replacement file.

The scripts modify only:

- one `include shieldxl0.conf` line in the discovered Pi boot config;
- the owned `shieldxl0.conf` fragment and `overlays/shieldxl0.dtbo`;
- one external codec module below `/lib/modules/<exact-kernel>/updates/shieldxl0`;
- named udev, ALSA, limits, systemd, `/etc/shieldxl0`, and `/usr/local/libexec/shieldxl0`
  files;
- membership of the selected ordinary user in `audio`, `input`, `spi`, and `gpio`.

It never replaces the complete boot config and never reboots automatically.

## Stable interfaces

- ALSA: `shieldxl` and `hw:SHIELDXL`, stereo playback/capture, acceptance target 48 kHz
  S16_LE.
- JACK2: `system:capture_1`, `system:capture_2`, `system:playback_1`, and
  `system:playback_2` unless the physical result records a different exact name.
- controls: `/dev/input/shieldxl-encoder-{1,2,3}` and
  `/dev/input/shieldxl-button-{1,2,3}`; `controls.py` emits typed JSON Lines with
  monotonic timestamps and is not an audio-callback component.
- OLED: `/run/shieldxl0/oled.sock`, 128x64 logical 4-bit grayscale, rotated 180 degrees,
  maximum 20 updates/second. The service accepts bounded `clear`, `fill`, `text`, and
  `status` JSON operations.
- USB MIDI: ALSA sequencer first, then JACK ALSA-sequencer MIDI. The exact controller and
  port identities are recorded only after the physical controller is observed.

The OLED and JACK services are independent. Failure of the OLED unit cannot stop JACK.

## Physical verification order

After platform enumeration, apply and retain the mixer state with `mixer-state.sh apply`.
JACK remains disabled so the direct ALSA device is available. Perform the direct ALSA
steps first, then run `sudo ./activate-services.sh YOUR_ORDINARY_USER` before JACK and
OLED verification. With operator confirmation between physical steps:

1. `audio-test.sh silent`;
2. low-level `audio-test.sh tone` and audible confirmation;
3. `audio-test.sh capture` with a physical input;
4. physical stereo cable loopback with `audio-test.sh loopback`;
5. `audio-test.sh reopen`;
6. activate services, then run `jack-matrix.sh --user USER --loopback-connected OUTPUT`
   for 128/256/512/1024 frames;
7. USB MIDI `midi-test.sh CLIENT:PORT OUTPUT`, unplug/reconnect, and identity comparison;
8. three-detent turns in both directions for each encoder and press/release for each
   button while `controls.py` records events;
9. OLED clear, fill, text, changing number, stop, and restart through `oled_client.py`;
10. bounded `thermal-observe.sh` run while native JACK loopback is active, normal service
    restart, and final reboot identity comparison.

Do not infer a physical pass from script availability or process liveness. Populate
`evidence/shieldxl0/hardware-contract.json` only from observed results.

## OLED examples

```sh
oled_client.py '{"op":"clear"}'
oled_client.py '{"op":"fill","gray":4}'
oled_client.py '{"op":"text","x":0,"y":0,"text":"SHIELDXL0"}'
oled_client.py '{"op":"status","preset":"TEST","macros":[0,25,50,100],"cpu_percent":12,"missing_frames":0,"temperature_c":52,"throttled":"NO"}'
```

## Rollback

`sudo ./uninstall.sh` stops and disables only the two owned services, removes exact
owned files, removes the managed boot include/fragment, rebuilds module dependencies,
and requests a reboot without initiating one. It retains Debian packages and group
memberships because they may have gained other users. Restore those manually only after
confirming they are unused. The pre-change boot config remains at
`/var/lib/shieldxl0/rollback/config.txt.pre-shieldxl0` for comparison; it is not blindly
copied over later operator changes.

Never run rollback against the working norns card. All board/image/kernel checks remain
active during uninstall.
