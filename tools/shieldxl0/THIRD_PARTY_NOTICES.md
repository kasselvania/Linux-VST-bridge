# SHIELDXL0 third-party provenance

The experiment is original project tooling except for the clearly separated Linux
kernel material below. The repository-level proprietary notice does not change these
third-party terms.

## monome Linux prior art

The device-tree design was adapted from these files at monome/linux commit
`458e2253667a09bd01f1d50cc2b4063c14c491ec`:

- `arch/arm/boot/dts/overlays/monome-snd-4270-overlay.dts`
- `arch/arm/boot/dts/overlays/norns-buttons-encoders-overlay.dts`
- `arch/arm/boot/dts/overlays/ssd1322-spi-overlay.dts`

The adapted overlay remains licensed `GPL-2.0-only`. A copy of the license is in
`licenses/GPL-2.0-only.txt`. Changes are identified in the overlay header.

## upstream Linux CS4270 driver

`kernel/cs4270.c` is an unmodified copy of
`sound/soc/codecs/cs4270.c` from Linux tag `v6.18`, commit
`7d0a66e4bb9081d75c82ec4957c50034cb0ea449`. It is retained because the selected
official Pi kernel package disables `CONFIG_SND_SOC_CS4270`. The file is licensed
`GPL-2.0-only`; the local Kbuild wrapper does not change that license.

## shieldXL repository

The hardware pin map and historical installation posture were inspected from
okyeron/shieldXL commit `eace215e99e8e0c3cb06dcd029ff14964da433c2`, licensed
GPL-3.0. No source from its installer is copied. The new provisioning implementation
does not retain its unpinned clones, ARMHF package, hard-coded user, broad mode changes,
or complete boot-configuration replacement.
