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

The compiled overlay was also applied offline with DTC 1.7.2 `fdtoverlay` to the exact
Pi 4 base DTB extracted from that kernel package. All required targets and nodes resolved;
`overlay-base-admission.json` retains the hashes and the strict nonclaim that this is not
physical boot or probe evidence.

## Files populated after physical work

The private Pi run must be sanitized into this directory without hostnames, addresses,
credentials, unrelated devices, or arbitrary codec-register dumps. At acceptance the
contract will point to the exact package manifest, JACK matrix, loopback result, control
observations, MIDI result, OLED result, thermal record, and reboot identity comparison.
