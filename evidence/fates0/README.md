# FATES0 evidence status

Status: **pending physical fixture**.

Offline inspection confirms that the pinned distribution kernel supplies the WM8731
and Raspberry Pi PROTO audio path. The original FATES0 RP1 overlay compiles
deterministically and applies together with the official `proto-codec` overlay to the
exact packaged Pi 5 DTB.

This does not establish boot, I2C acknowledgement, ALSA/JACK operation, physical audio,
controls, OLED, MIDI, restart, reboot persistence, or thermal acceptance. Those fields
remain null or empty in `hardware-contract.json` until observed on the Pi 5.
