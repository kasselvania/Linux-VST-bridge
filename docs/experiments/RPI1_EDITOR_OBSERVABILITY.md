# RPI1 restored-state editor attempt and observability pause — 2026-09-22

## Boundary and disposition

The editor-with-physical-playing gate is **incomplete**. Physical Pigments
qualification is paused while the existing signals are put on one timestamped
view: Pi heat and voltage, MIDI/audio production, bridge gaps and faults,
Pigments/editor lifecycle, local X11, and the existing RPI0 connection. The Pi
is idle. The operator has no
active cooler compatible with the present ShieldXL assembly; cooler fit is not
established. This is a pause in physical qualification, not a rollback of the
previously passed editor-closed audio and saved-state gates.

Both attempts executed repository head
`b2d121860bb4dc9849ca42281efb4c6aaed76399`, tree
`7538a81a67933d25ce5132b0b3defbe33fa711db`, with the unchanged native
binary SHA-256
`81a44a8eed7191b08655496dfe0b49c0c919208e9f7a600658bc37887a2006c7`.
The 381,326-byte saved state, 512-frame JACK period, 2048-frame bridge delay,
physical OMX-27 UART input and ShieldXL stereo playback route remained the
selected setup. Pigments Master Volume was read back at
`0.45000001788139343` immediately after restore in each attempt. No new state
was saved.

## What the two attempts showed

| Observation | First attempt | Second attempt with operator at the RPI0 window |
| --- | --- | --- |
| Editor lifecycle | `3` and `5`, both result `0` | `3`, `5`, then close `8`, all result `0` |
| Operator visual result | None; operator was away | Editor control responded, with visible delay |
| Operator audio result | None at the screen | Playing sounded normal before opening; audio sounded good with the editor open |
| MIDI and stereo | Last live MIDI `551`; nonzero stereo `2,255,967` per channel | Last live MIDI `292`; nonzero stereo `4,494,238` left / `4,494,236` right |
| Processing | Terminal fault `2`; `336` process failures | Fault `0`; zero process failures, xruns, deadline misses or nonfinite output |
| Callback gaps | Two gaps, `5,376` missing frames; `2,560` were startup | Three gaps, `197,120` missing frames; `2,560` were startup |
| Pi health | 123 samples at `0x0`; max `69.95°C` | 193 samples; max `83.7°C`; temperature-limit, frequency-cap and throttle flags occurred |
| Retirement | No clean-shutdown receipt; private transport retained | `RPI1_CLEAN_SHUTDOWN`, all 127 milestones, owned units and transport retired |

The first fault code is the backend's `OVERFLOW` class. The retained status
does not identify which internal overflow site published it. The editor opened
before the fault, but that ordering alone does not establish that the editor
caused it. The operator did not witness this first attempt at the screen. The
failed session's private transport was copied to the Mac before the exact stale
Pi directory was cleared for the requested retry. It is not being called a
clean retirement.

The second attempt did not reproduce a terminal host fault. It did reveal
`194,560` missing frames beyond startup, about `4.053` seconds at 48 kHz, while
the editor was open. The operator described the audio as good; the aggregate
callback counter and that listening report are retained separately. The source
did not expose the callback gap counter in live `status` at the time, so its
exact wall-clock onset relative to a mouse interaction or thermal limit is
unproven. The operator moved a visible control; a later Pigments Master Volume
readback was `0.54427075386047363` at revision 68. This supports interaction,
but no changed state was saved or reopened.

In the second health record, 184 samples were `0x0`, three were `0x80008`,
three `0x80000`, and three `0xe0000`. An additional bounded gate check read
`0xe0006` and stopped the session. These flags show a thermal limit and later
frequency capping/throttling, with **no recorded undervoltage bit in this
attempt**. Voltage ranged `4.80122–5.05448 V`. This does not resolve the
earlier separate whole-Pi power failure, nor prove thermal limiting caused the
audio gap. A one-hertz sampler can miss brief active flags, as this attempt
demonstrated. Raspberry Pi's [firmware flag definitions](https://www.raspberrypi.com/documentation/computers/os.html#get_throttled)
distinguish current bits from historical bits, and its
[Pi 5 thermal guidance](https://www.raspberrypi.com/documentation/computers/raspberry-pi.html#frequency-management-and-thermal-control)
describes progressive Arm throttling around `80–85°C`.

The close command received lifecycle `8` result `0`. MIDI and nonzero stereo
counts increased in subsequent live reports, and the session shut down
normally. The operator was not asked to confirm post-close playing before the
thermal stop, so the full UX gate is not claimed.

## Monitoring change prepared for review

The new read-only `rpi1/live_observer.py` runs on the Mac. It reads the existing
Pi health source and the exact RPI1 user-unit journal through the already
established SSH control connection. It checks the **existing** RPI0 Screen
Sharing TCP connection, queries the Pi's local X11 server, and shows separate
panels for Pi power/heat, MIDI and audio, bridge continuity, Pigments/editor
lifecycle, and the visual path. Operator buttons timestamp what is actually
seen in the existing RPI0 window; they do not send commands to the Pi. A
loopback-only browser view and a new mode-`0600` JSONL receipt on the Mac keep
the observer outside the translated cohort and off the audio callback.

The RPI1 standalone's existing `status` command now also prints
`RPI1_CALLBACK_LIVE` from the already allocated atomic callback counters. This
does not change the callback, queue, processing, editor, or command semantics.
It makes a new gap visible before shutdown **when an external diagnostic harness
requests `status`**. The old Pi binary has not been replaced; this source
addition is compile-tested, not physically qualified.

The local display query proves that X11 responds and whether a Pigments-titled
window exists. The TCP check proves only that Screen Sharing is connected.
Neither proves that a frame was rendered promptly or delivered to the operator;
the operator's timestamped report remains a separate measurement. An X11
response time includes local X11 work, while the separate SSH plus X11 time
also includes wireless transport. Their difference is not a calibrated VNC
frame-latency measurement.

| Next-run pattern | Narrow interpretation |
| --- | --- |
| Undervoltage or thermal bit changes before audio counters change | Pi platform event is observed; causality still needs timing and repeat evidence |
| Live missing-frame or gap count rises while hardware flags remain clear | Bridge/audio continuity is the first recorded failure region |
| Local X11 response slows, with audio counters steady | Pi-local graphical path is implicated |
| Local X11 stays responsive, RPI0 connection breaks or operator sees delay | Wireless/VNC path needs separate frame-level measurement |
| Editor lifecycle succeeds while operator sees no usable editor | Lifecycle and visible UX have diverged; do not count an editor pass |

An idle read-only smoke check on the actual Pi showed fresh health, `0`
translated-cohort processes, a responsive local X11 query, and the original
RPI0 Screen Sharing connection still established. It did **not** start
Pigments, send MIDI, open the editor, or validate live callback records.
The smoke observer was then stopped. The Pi remained idle.

For a later run, start the Mac observer **before** the RPI1 unit. Supply the
existing Pi SSH target/control socket, the absolute copied `rpi1/health.py`
path on the Pi, the PID of the **existing** RPI0 Screen Sharing window, and a
new absolute private Mac receipt path. The observer serves only
`http://127.0.0.1:8765/`. The run harness must request the RPI1 `status`
command periodically; the observer itself sends no product commands. Record
what the operator sees using the viewer buttons at the moment of a visual
change. Stop the observer after owned retirement and keep its Mac JSONL file
with the private session journal. The browser page is a monitor, not a second
screen-sharing connection.

## What the next review must decide

The monitoring change is ready for source review. A later physical gate needs
a cooling arrangement that physically fits the ShieldXL assembly and an exact
observer run started before Pigments. Keep the old saved state and audio
configuration. Run only one bounded restored-state editor session, capturing
live gap-counter deltas, temperature/firmware flags, Pi-local X11 responsiveness,
the existing RPI0 connection, and operator visual/audio reports on a shared
timeline. Stop on the first newly observed fault or thermal limit; preserve the
private logs and normal retirement when possible. Do not claim the editor UX
gate until post-close physical playing is heard and the run retires cleanly.

The observer does not identify the exact backend overflow branch or the exact
Windows process within a populated cohort, measure GPU render or VNC frame
time, meter analog output, or establish that cooling repairs transport. Those
remain distinct questions for evidence-driven follow-up.

The sanitized measurements are in
[`editor-observability-pause.json`](../../evidence/rpi1/editor-observability-pause.json).
Raw journals, transport files, Xauthority, configuration, and saved vendor
state remain private and are not committed.
