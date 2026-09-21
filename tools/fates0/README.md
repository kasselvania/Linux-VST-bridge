# FATES0 Raspberry Pi 5 hardware platform

This directory prepares only the Raspberry Pi 5 8 GB + Fates v1.8.1 hardware lane
defined by `docs/experiments/FATES0.md`.

The accepted base image is:

```text
Raspberry Pi OS Lite (64-bit), Debian Trixie, 2026-09-15
2026-09-15-raspios-trixie-arm64-lite.img.xz
SHA-256 cdf4f3bfac35ae947b46e4e767f935453810549779ac3290e05a6754aee627e5
```

The exact distribution kernel already contains the WM8731 codec and Raspberry Pi
PROTO machine drivers plus `proto-codec.dtbo`. FATES0 therefore builds no kernel and
vendors no Fates source. Its original `fates0-overlay.dts` exposes v1.8.1 controls and
SPI0 through current Pi 5/RP1 bindings.

## Power rule

Use the official 5 V / 5 A-class supply through the Raspberry Pi 5 power connector.
Leave the Fates USB-C connector disconnected. Provisioning requires the explicit
declaration `--power-input pi` and refuses every other value; this records the
operator assertion but cannot electrically detect the cable.

## Ordered use

From an exact checkout of this branch on the fresh card:

1. With no FATES0 configuration installed, retain a private baseline:
   `./observed-run.sh --label baseline-inventory --board-airflow STATUS --cooling none --output RUN_DIR -- ./inventory.sh INVENTORY_DIR`
2. Run `sudo ./prepare-buses.sh --board-revision v1.8.1 --power-input pi`.
3. Reboot.
4. Run `sudo ./provision.sh --user YOUR_USER --board-revision v1.8.1 --power-input pi`.
5. Reboot.
6. Run `./verify-platform.sh PRIVATE_OUTPUT_DIRECTORY`.
7. Run `./mixer-state.sh apply`, then follow `audio-test.sh` in order: `silent`,
   `tone`, `capture`, `loopback`, `reopen`.
8. Only after ALSA succeeds, run `sudo ./activate-services.sh YOUR_USER` and then the
   JACK, control, OLED and MIDI tests.

Every command that exercises physical hardware must be launched through
`observed-run.sh` with the actual airflow and cooling declarations. This includes each
audio phase, JACK matrix, control observation, OLED operation and MIDI observation; it
retains temperature, throttling, CPU frequency and exact workload duration. The
operator interaction remains one requested action at a time.

Do not perform a physical step until the operator has confirmed the immediately prior
instruction. Do not use the working norns card.

## Rollback

Run `sudo ./uninstall.sh` from the same exact checkout. It removes only hash-matching
FATES0-owned files and boot configuration. Added Debian packages and group membership
remain because they may be shared. Reboot after uninstall. Modified or foreign files
are refused rather than deleted.

## Evidence boundary

`test.sh` validates deterministic local behavior and static device-tree application.
It does not prove that the Pi booted, the WM8731 bound, audio passed, controls moved,
the OLED rendered, MIDI arrived, or thermal throttling was absent.
