# SHIELDXL0 evidence status

Status: **pending physical fixture**.

The retained source/package admission and deterministic tooling are ready. No Raspberry
Pi hardware result is claimed yet. In particular, nulls and empty result arrays in
`hardware-contract.json` are deliberate stop markers, not implicit passes.

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
