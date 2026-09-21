# SHIELDXL0 evidence status

Status: **4 KiB platform enumeration, reboot, and OLED physical acceptance passed;
complete physical acceptance pending**.

The retained source/package admission and deterministic tooling are ready. The exact
Pi 5 8 GB fixture has now built, loaded, bound, and rebooted the pinned CS4270 module on
the RPI0 4 KiB integration kernel, with the SHIELDXL ALSA card and JACK ports present.
See `rpi0-4k-integration.json`. Nulls and empty result arrays in
`hardware-contract.json` remain deliberate stop markers, not implicit passes.

The OLED now has a separate physical receipt in `oled-physical.json`. After correcting
the SSD1322 D/C signaling for command parameters, the operator confirmed readable and
correctly oriented status text, a live text update, uniform grayscale fill, complete
clear, and return of `SHIELDXL0 READY` after a clean service restart. The bounded OLED
campaign observed 44.4-46.1 C and no throttling.

This is not complete hardware acceptance. Physical 4 KiB tone/loopback, the USB MIDI
event campaign, and the complete JACK period/XRUN matrix remain pending. JACK was
already inactive throughout the OLED-specific campaign, so that receipt does not claim
concurrent audio operation or boot-time OLED failure injection.

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

## Files populated after physical work

The private Pi run must be sanitized into this directory without hostnames, addresses,
credentials, unrelated devices, or arbitrary codec-register dumps. At acceptance the
contract will point to the exact package manifest, JACK matrix, loopback result, control
observations, MIDI result, OLED result, thermal record, and reboot identity comparison.
