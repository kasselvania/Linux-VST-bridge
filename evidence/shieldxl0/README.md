# SHIELDXL0 evidence status

Status: **SHIELDXL0 physical hardware contract accepted for bounded RPI0 integration;
sustained uncooled performance remains thermally unqualified**.

The retained source/package admission and deterministic tooling are ready. The exact
Pi 5 8 GB fixture has now built, loaded, bound, and rebooted the pinned CS4270 module on
the RPI0 4 KiB integration kernel, with the SHIELDXL ALSA card and JACK ports present.
See `rpi0-4k-integration.json` and `platform-physical.json`.

The OLED now has a separate physical receipt in `oled-physical.json`. After correcting
the SSD1322 D/C signaling for command parameters, the operator confirmed readable and
correctly oriented status text, a live text update, uniform grayscale fill, complete
clear, and return of `SHIELDXL0 READY` after a clean service restart. The bounded OLED
campaign observed 44.4-46.1 C and no throttling.

Direct ALSA playback, physical stereo input, physical stereo loopback, 20 playback and
capture reopen cycles, and the full JACK 128/256/512/1024-frame matrix now pass. The
128-frame run retained two XRUNs and the 256-frame run retained one; the conservative
512-frame recommendation completed with zero. See `audio-jack-physical.json`.

All three encoders emitted bidirectional typed events and all three buttons emitted one
press/release pair. See `controls-physical.json`. USB MIDI note-on/off, CC64, CC123,
hot-unplug, stable-identity reconnect, and a post-reconnect note passed with the Monolit
controller. See `midi-physical.json`. The three available controllers all emitted fixed
velocity, so the later RPI0 requirement for two distinct physical velocities remains an
explicit gap even though SHIELDXL0 note/control transport is accepted.

No active cooler was installed. All bounded acceptance runs retained temperature,
clock, and throttling observations without a throttle flag, but sustained performance,
low-latency behavior, and polyphony remain thermally unqualified.

## Retained pre-hardware result

The exact official Pi kernel package was downloaded and its published SHA-256 matched.
Its configuration provides the Pi I2S, simple-card, rotary-encoder, GPIO-key, and spidev
drivers, but explicitly disables `CONFIG_SND_SOC_CS4270`. The bounded response keeps the
distribution kernel and builds the unmodified upstream Linux v6.18 CS4270 source as one
external module. See `kernel-driver-admission.json`.

The Pi-5/RP1 overlay was also applied offline with DTC 1.7.2 `fdtoverlay` to the exact
Pi 5 base DTB extracted from the `rpi-2712` kernel package. Its CS4270-master clock
direction resolves to RP1's I2S clock-consumer controller, and all required targets and
nodes resolved. `overlay-base-admission.json` retains the hashes and the strict nonclaim
that this is not physical boot or probe evidence.

## Retained fixture refusal

The first powered candidate identified itself as a Raspberry Pi 3 Model B Plus Rev 1.3,
revision `a020d3`, with approximately 1 GB RAM. It remains outside the accepted Pi 5 8 GB
fixture, so provisioning stopped before any overlay, codec module, audio service, or
package change. `fixture-admission-failure.json` retains the sanitized result. Enabling
SSH and downloading the exact source archive were the only access/setup changes made
before the refusal.

## Physical receipts

The sanitized receipts omit hostnames, addresses, credentials, unrelated devices, and
arbitrary codec-register dumps. `hardware-contract.json` points to the platform, JACK
matrix/loopback, controls, OLED, and MIDI receipts and records the private fixture
package-manifest digest.
