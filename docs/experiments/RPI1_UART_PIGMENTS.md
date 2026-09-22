# RPI1 OMX-27 UART input into Pigments — 2026-09-22

Physical MIDI now drives the real Pigments instance through ShieldXL, with
operator-confirmed audible output and clean retirement. The operator reported
crackles around 5–6 simultaneous notes. This is an input/audio-path pass with an
unresolved polyphonic-audio limitation, not an artifact-free appliance pass.

## Exact setup and route

The RPI1 executable was unchanged from source
`8eb89230410a1b42531bfb0fc252cddb166b1d5d`, tree
`ff138a43d6d9be9c517e8aa0209cf268690fa6c9`, binary SHA-256
`2689b5a0af961d8637007c6be7a1af58ce55458b26ebb16662c2455007202553`.
Repository head at the attempt was `55c95855a6e6941c4163e8e703781299e9082e80`.
The separately installed UART implementation is recorded in
[hardware PR #141](https://github.com/kasselvania/Linux-VST-bridge/pull/141),
head `c6b7903d15c9025a3244210c672cf804c0d5634c`, tree
`067ea914b212718fbedd858e83eb70a8a686d8bd`.
No Proton, Box64, Windows-host, Pigments, bus-contract or audio-setting changes
were made. The editor stayed closed and USB MIDI stayed disconnected.

The OMX-27 remained separately powered and connected by the operator's working
Type B setup. The exact current JACK port was resolved from
`ShieldXL-UART:midi/playback_1` and connected to `lvb-arm-pigments:midi_in`.
Pigments' left/right ports connected to `system:playback_1/2`. The native tone
fixture stopped before Pigments was connected. No synthetic MIDI or reference
tone was sent during this session. The existing OLED showed readiness and the
live accepted-MIDI count.

JACK remained 48 kHz / 512 frames, bridge delay 2048 frames, processing quantum
256 frames, reported vendor latency 48 frames. The same MacBook supply and
listening setup were used. Independent one-Hz health recording and remote
journal/ping recording over direct Wi-Fi SSH were active. The runtime watchdog
was temporarily disabled using the earlier operator authorization and restored
to its original 60 seconds afterward.

## Result and limitation

Pigments accepted **114 MIDI events**. Each channel returned **1,227,888 nonzero
samples**, with maximum absolute peak **1.4293827**. The operator answered
"Yes, Pigments responds" and then "audio absolutely works!", while identifying
crackles around 5–6 simultaneous notes.

The host completed **15,716 blocks / 4,023,296 frames**, about **83.82 seconds**
of processing. There were zero reported xruns, callback deadline misses, process
failures, nonfinite output samples, unsupported/malformed/overflow MIDI events
or terminal transport faults. The single startup gap retained 2560 missing and
expired frames at positions 0–2559. No later gap was recorded.

The output peak exceeded normalized digital full scale. Downstream clipping is
therefore a possible contributor to the reported crackles; this run does not
diagnose clipping or establish a CPU/polyphony ceiling. The bounded returned-audio
audit retained 32 nonzero windows near the first notes, not a full waveform of the
later chords. No independent external JACK meter ran in this session. A later
comparison should separate output level from processing load before changing
latency or declaring a performance limit. The operator deferred that work.

Across 100 active health samples, maximum temperature was 58.95 C,
voltage ranged 4.75834–5.02232 V and firmware flags remained
`0x0`. No kernel undervoltage event was observed. These samples do not prove
power margin or resolve the earlier machine-wide failure.

## Retirement and remaining scope

After operator confirmation, the native owner received normal `quit`, exited 0
and reported all 127 retirement milestones. The Windows worker joined without
an exception, processing stopped and its mapping was unmapped. Native and
translated processes, exact session transport, command FIFO and Pigments JACK
ports/routes were absent afterward. The previous inactive outer-unit definition
was restored. Recording stopped and the watchdog readback was 60,000,000 us.
The UART input service remains active and boot-enabled; the OLED shows
`PIGMENTS STOPPED`. Boot persistence has not been reboot-verified.

This advances the earlier [interrupted soak record](RPI1_HEADLESS_SOAK_INTERRUPTED.md)
without rewriting it. The existing [state round-trip](RPI1_STATE_ROUNDTRIP.md)
remains separate. Artifact-free polyphony, CC/sustain behavior, sustained soak,
editor with physical MIDI, cold-start reliability and overall RPI1 remain open.
Monolit and Chord ATK are not qualified by the OMX-27 result.

Sanitized evidence: [`uart-pigments.json`](../../evidence/rpi1/uart-pigments.json).
