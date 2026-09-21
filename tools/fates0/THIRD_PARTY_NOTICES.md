# FATES0 third-party provenance

The Fates repository at commit
`f7f4cf69a8886f77be943c564b69f925b95d2e88` was inspected as hardware prior art.
No repository-wide license or permission notice was found, so no source file from that
repository is copied or adapted here.

The pin assignments and component identities recorded in `pinned-inputs.json` are
factual hardware interface observations. `overlays/fates0-overlay.dts` is an original
implementation against current Raspberry Pi device-tree bindings.

The installed Raspberry Pi distribution supplies `proto-codec.dtbo`,
`snd-soc-rpi-proto`, and the Linux WM8731 driver under their distribution licenses.
They are inspected and invoked but not redistributed in this directory.

`oled_service.py` adapts the SSD1322 initialization sequence, display address window,
and framebuffer-to-wire mapping from:

- repository: `https://github.com/monome/linux`
- commit: `458e2253667a09bd01f1d50cc2b4063c14c491ec`
- file: `drivers/staging/fbtft/fb_ssd1322.c`
- SHA-256: `b26582cd845bec03ee8f11bf8a697210e92103342ae33511cb91322b41382cb7`
- original author recorded by the driver: Ryan Press
- license: GPL-2.0-only

That adapted file is explicitly marked GPL-2.0-only. A copy of the license is retained
at `licenses/GPL-2.0-only.txt`.
