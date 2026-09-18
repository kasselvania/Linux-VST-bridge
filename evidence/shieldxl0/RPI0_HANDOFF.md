# RPI0 hardware integration handoff

RPI0 must first read `hardware-contract.json` and refuse integration unless
`acceptance_status` is `accepted`. It must consume the recorded identities rather than
rescanning and selecting a similarly named device.

The selected fixture is Raspberry Pi 5 with 8 GB RAM on the pinned `rpi-2712` kernel,
which uses 16 KiB pages. RPI0 must treat that page size and the contract's exact kernel
identity as part of its integration input rather than assuming the former Pi 4 shape.

The intended interfaces, pending physical confirmation, are:

- audio: JACK2 at 48 kHz, S16_LE, with the exact capture/playback ports recorded in the
  contract; the backing stable ALSA alias is `shieldxl` / `hw:SHIELDXL`;
- MIDI: the exact accepted JACK MIDI input port recorded under `midi.usb.jack_port`;
- encoders/buttons: execute `/usr/local/libexec/shieldxl0/controls.py --format json` off
  the audio callback and consume bounded JSON Lines events;
- OLED: send one JSON object per connection to `/run/shieldxl0/oled.sock`; supported
  operations are `clear`, `fill`, `text`, and `status`, with a hard 4096-byte request
  bound and a maximum 20 Hz update rate.

The control reader emits zero-based indices:

```text
{"type":"encoder","index":0..2,"delta":signed_integer,"monotonic_ns":integer}
{"type":"button","index":0..2,"pressed":boolean,"monotonic_ns":integer}
```

The OLED service is independent of JACK. OLED failure must not block or stop audio. RPI0
must not open evdev, SPI, GPIO, or ALSA card numbers directly when the accepted contract
provides the stable interface.

RPI0 must also consume the retained thermal classification. A functionally accepted
hardware contract may still mark sustained performance `thermally_unqualified` when the
open, unenclosed, no-active-cooler fixture throttles.
