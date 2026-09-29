# RPI0 hardware integration handoff

RPI0 must first read `hardware-contract.json`. SHIELDXL0 is accepted for bounded RPI0
integration on the exact fixture, with sustained uncooled performance explicitly
thermally unqualified. RPI0 must consume recorded identities rather than selecting a
similarly named device.

The selected RPI0 fixture is Raspberry Pi 5 with 8 GB RAM on the pinned
`6.18.50+rpt-rpi-v8` integration kernel, which uses 4 KiB pages. The original
`6.18.50+rpt-rpi-2712` / 16 KiB hardware profile remains retained, but it is not the
translated-runtime profile. RPI0 must treat the 4 KiB page size and exact kernel identity
as part of its integration input.

The physically accepted interfaces are:

- audio: JACK2 at 48 kHz, S16_LE, with the exact capture/playback ports recorded in the
  contract; the backing stable ALSA alias is `shieldxl` / `hw:SHIELDXL`;
- MIDI: the exact accepted JACK MIDI input port recorded under `midi.usb.jack_port`;
- encoders/buttons: execute `/usr/local/libexec/shieldxl0/controls.py --format json` off
  the audio callback and consume bounded JSON Lines events;
- OLED: send one JSON object per connection to `/run/shieldxl0/oled.sock`; supported
  operations are `clear`, `fill`, `text`, and `status`, with a hard 4096-byte request
  bound, a maximum 20 Hz update rate, and a 60-second idle blank. The next valid request
  redraws the display; OLED blanking remains independent of JACK and audio.

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

The accepted Monolit controller proved note-on/off, sustain CC64, all-notes-off CC123,
hot-unplug, stable-identity reconnect, and a post-reconnect note. It emits a fixed
velocity, as did the two other available controllers. RPI0 must not claim its complete
physical MIDI sequence until one physical controller establishes two distinct note-on
velocities.
