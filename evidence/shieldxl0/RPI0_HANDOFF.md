# RPI0 hardware integration handoff

RPI0 must first read `hardware-contract.json`. A complete SHIELDXL0 claim still requires
`acceptance_status` to be `accepted`, but the operator may explicitly defer OLED and
authorize bounded RPI0 audio/MIDI integration while that overall status remains pending.
In that case RPI0 must require the exact recorded kernel, CS4270 module, ALSA/JACK audio,
and MIDI results individually and must not promote the bounded handoff into a complete
SHIELDXL0 pass. It must consume recorded identities rather than selecting a similarly
named device.

The selected RPI0 fixture is Raspberry Pi 5 with 8 GB RAM on the pinned
`6.18.50+rpt-rpi-v8` integration kernel, which uses 4 KiB pages. The original
`6.18.50+rpt-rpi-2712` / 16 KiB hardware profile remains retained, but it is not the
translated-runtime profile. RPI0 must treat the 4 KiB page size and exact kernel identity
as part of its integration input.

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
