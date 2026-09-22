# RPI1 reload with an inline power meter — 2026-09-22

The operator explicitly requested another Pigments load after recovery and after
connecting a USB-C meter at the Pi. The editor-closed session accepted physical
MIDI, returned stereo samples and retired cleanly. The operator observed the
audio interface overloading as more notes were added. No level was reduced.

## Executed identity and scope

The run reused executable source `8eb89230410a1b42531bfb0fc252cddb166b1d5d`,
tree `ff138a43d6d9be9c517e8aa0209cf268690fa6c9`, binary SHA-256
`2689b5a0af961d8637007c6be7a1af58ce55458b26ebb16662c2455007202553`.
Repository head was `1173e816bdea7f334347e411ddb15b438f81467b`; the newer
master-control binary from the interrupted comparison was not used. The existing
OMX-27 Type B / ShieldXL UART route was resolved by its exact JACK alias.
Stereo outputs connected to `system:playback_1/2`. No synthetic notes, editor,
Master Volume change, installer, authorization, runner or audio-setting change
was introduced. JACK remained 48 kHz / 512 frames / 3 periods, bridge delay 2048,
processing quantum 256 and reported vendor latency 48 frames.

The operator's meter was at the Pi input and read 5.02 V / 0.63 A at idle.
No under-load meter reading was supplied. Exact cable identity or change was not
established. This is not a controlled cable or adapter comparison.

## Audio and health result

Pigments accepted 62 MIDI events and returned 615,120 nonzero samples per channel.
Peak was 1.4802243 on both channels, exceeding normalized digital full scale.
It completed 27,546 blocks / 7,051,776 frames (146.912 seconds). No callback
deadline misses, xruns, process failures, nonfinite samples or terminal transport
faults were reported. The existing contiguous 2560-frame startup gap remained;
no later gap was recorded. No independent JACK meter or waveform capture ran.

The operator said: "We have not lowered any level. I just saw the audio level
build higher as I added more notes and the audio interface was hot and hitting
beyond peak". This supports downstream clipping as the leading explanation for
the distortion. It does not identify the exact clipping stage or prove that
reducing Pigments' master clears the artifact. That comparison remains next.

All 172 one-Hz health samples reported `0x0`; voltage ranged 4.86152–5.05582 V
and maximum temperature was 58.4 C. Direct Mac Wi-Fi SSH retained journal and ping
records. The private harness checked flags during startup and playing and would
request normal retirement after a new voltage/throttle indication. That branch
was not exercised; no installed product power guard or hard-hang prevention is
claimed. The brief session does not establish sustained power margin.

## Retirement and preserved failure

Normal quit completed with exit 0 and all 127 retirement milestones. The Windows
worker joined, processing stopped and its mapping was unmapped. Native and
translated processes, exact session transport, command FIFO and Pigments JACK
ports were absent afterward. The previous outer-unit definition was restored,
recorders stopped and the original 60-second watchdog was verified. UART remained
active and enabled; its startup after reboot is now physically observed.

The earlier [interrupted headroom comparison](RPI1_HEADROOM_INTERRUPTED.md)
remains a real whole-unit failure with active undervoltage evidence. This clean
reload does not erase that result or prove a cable/supply repair. Artifact-free
polyphony, lowered-master behavior, sustained operation and overall RPI1 remain
unqualified.

Sanitized evidence: [`meter-retry.json`](../../evidence/rpi1/meter-retry.json).
