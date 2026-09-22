# RPI1 headless soak stopped for reported buzz — 2026-09-22

**The ten-minute soak did not pass.** The operator reported occasional audio
quirks, clarified as "audio buzz briefly" during a note. The native owner was asked to quit
normally during note 27; it retired cleanly. The cause of the buzz is unresolved.

The preceding [same-state round-trip](RPI1_STATE_ROUNDTRIP.md) remains a separate
measured and operator-confirmed success.

## Exact attempt

No product source or binaries changed. This reused executable source
`8eb89230410a1b42531bfb0fc252cddb166b1d5d`, tree
`ff138a43d6d9be9c517e8aa0209cf268690fa6c9`, native SHA-256
`2689b5a0af961d8637007c6be7a1af58ce55458b26ebb16662c2455007202553`.
The repository head at the attempt was documentation commit
`dc83017f48f1a777bff429eddd0253c5d8f69210`. The existing qualification client,
Windows host, Pigments installation, runner and configuration were unchanged.

The editor stayed closed and USB MIDI stayed unplugged on the already-connected
MacBook supply. JACK remained 48 kHz / 512 frames, bridge delay 2048 frames and
vendor processing quantum 256 frames. The planned test was 600 seconds with one
five-second measurement window every ten seconds. Each window sent note 60 on
channel 1 at velocity 96, then note-off and CC123. External metering covered those
windows, not the quiet gaps between them. Native processing and health recording
continued between windows.

## What the records establish

The first 26 note windows had identical per-channel summaries: 240,000 samples,
72,928 nonzero samples, 144 nonzero windows, peak 0.19765145 and RMS
0.076079156529. All 78 messages in those complete windows were accepted. This is
not a samplewise waveform comparison and cannot rule out a short buzz.

Shutdown cut short the 27th note. Its still-running meter recorded 35,855 nonzero
samples per channel and lower RMS. The fixture sent its remaining messages after
the native owner had stopped, leaving 81 sent versus 79 accepted overall. Those
differences are attributed to the deliberate stop, not a spontaneous failure.
The orchestration script subsequently exited 1 because its native owner had
retired; the native unit itself exited 0 with `Result=success`.

The Windows host completed 49,120 blocks / 12,574,720 frames, equivalent to about
261.97 seconds of processing. There were no reported JACK xruns, callback deadline
misses, process failures, malformed/unsupported/overflow MIDI events, nonfinite
samples or terminal transport faults. All 2560 missing/expired frames were in
the single startup gap, at frame positions 0–2559. No later gap was recorded.
These results narrow the investigation but do not certify artifact-free audio.

Across 281 one-Hz samples while the outer owner existed, temperature reached
59.5 C, voltage ranged 4.78112–5.02902 V, throttle samples stayed `0x0`, and no
new kernel undervoltage event was observed. MemAvailable stayed at least
5,575,136 KiB. The translated cohort's memory peak was 2,095,116,288 bytes.
The Pi stayed responsive. Sampling does not exclude a subsecond power excursion
and does not resolve the historical whole-machine failure.

## Cleanup

The processing worker joined without an exception, the endpoint mapping was
unmapped, all 127 retirement milestones were reported, and clean shutdown
completed. The successful transport, command FIFO, native/translated processes
and experiment JACK ports/routes were removed. The prior inactive unit
configuration was restored. Health and remote journal/ping observers stopped.
The original runtime watchdog was restored and read back as 60,000,000 microseconds.
Earlier failed sessions and private diagnostics remain retained.

## UART input and next diagnostic boundary

The operator wants the ShieldXL physical TRS MIDI input, fed by a Monolit, Chord
ATK or OMX-27. That route is not implemented or physically qualified. The
SHIELDXL0 hardware record explicitly deferred UART MIDI. Live inspection found
only `ttyAMA10` exposed, with `serial0` pointing to it and a serial-console getty
active. GPIO14 and GPIO15 had no UART function. No UART-to-MIDI service or external
ALSA MIDI input was present; the visible JACK MIDI pair came from MIDI Through.

The installed Pi 5 overlay documentation offers `uart0-pi5` on GPIO14/15.
The [Raspberry Pi UART documentation](https://www.raspberrypi.com/documentation/computers/configuration.html#configuring-uarts)
distinguishes the Pi 5 debug UART from the header UART. The pinned
[ShieldXL installation](https://github.com/okyeron/shieldXL/blob/eace215e99e8e0c3cb06dcd029ff14964da433c2/install/systemd/ttymidi0.service)
uses historical clock/baud configuration and is prior art, not a Pi 5 installation
recipe. No boot configuration, UART service or physical controller connection
was changed here.

The operator subsequently suspected their audio interface and explicitly deferred
further audio diagnosis to prioritize TRS/UART MIDI. No cause was established.
A future audio follow-up needs a bounded recording of actual samples from the same
deterministic note sequence. If digital samples remain clean while the buzz is
heard, compare the codec/analog listening path. UART input bring-up must separately
prove incoming notes through the physical jack before combining it with Pigments.
Do not promote the current aggregate measurements to a buzz diagnosis, soak pass,
editor pass or full appliance qualification.

Sanitized measurements and lifecycle records:
[`headless-soak-interrupted.json`](../../evidence/rpi1/headless-soak-interrupted.json).
