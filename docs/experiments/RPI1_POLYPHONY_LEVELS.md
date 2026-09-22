# RPI1 exact polyphony ladder at 50% and 45% master — 2026-09-22

The same source-owned 30-second 1/2/4/6/8-note ladder completed at normalized
master 0.5, repeated at 0.5, then completed at the operator-requested 0.45.
The operator heard both follow-up passes clean and chose 45% for future sessions.
The initial 50% tail distortion and an objective gap in the 50% repeat remain
separate retained limitations.

## Exact execution

Source `1173e816bdea7f334347e411ddb15b438f81467b`, tree
`d97e60f8ab04fc0e1604fd975a465f365db4bc4c`, was already built and tested.
Native SHA-256: `81a44a8eed7191b08655496dfe0b49c0c919208e9f7a600658bc37887a2006c7`.
Fixture SHA-256: `a3fb302d8f648742a853d85cd4d6f3b8b0d46ecc70259d56272137553b19e861`.
Repository head at these attempts was `4493c7337cf932e076762b098f850b4a1028a51e`.
Neither binary changed. The editor stayed closed. JACK remained 48 kHz / 512
frames / 3 periods, bridge delay 2048, vendor processing quantum 256.

The exact qualification client supplied MIDI and measured both returned JACK
outputs while the same outputs fed ShieldXL playback. Physical UART MIDI was
not routed. Each six-second window held 1, 2, 4, 6 or 8 notes for three seconds;
channel 1, velocity 96, pitches 48/52/55/59/62/65/69/72, note-on at 0.5 seconds,
note-off at 3.5 seconds and CC123 at 4 seconds. All 47 messages were sent and
accepted in each pass. Each complete private recording contains 1,440,000 stereo
frames / 11,520,000 bytes, transferred to the Mac and hash-verified.

## Per-chord measurements

Peaks were identical between left and right in each recording. No recorded
sample reached normalized full scale (1.0), and no nonfinite sample occurred.

| Held notes | Initial 50% peak | Repeated 50% peak | 45% peak |
| --- | ---: | ---: | ---: |
| 1 | 0.076683 | 0.076683 | 0.056541 |
| 2 | 0.152277 | 0.152277 | 0.112280 |
| 4 | 0.301430 | 0.301430 | 0.222255 |
| 6 | 0.421685 | 0.421685 | 0.310925 |
| 8 | 0.523957 | 0.523957 | 0.386334 |

The first 50% run completed with zero xruns and no additional gap beyond the
known 2560-frame startup gap. The operator heard slight distortion near the end
of the eight-note chord. This remains an audible artifact despite the measured
headroom; the output measurements alone do not identify downstream distortion.

The requested 50% repeat also completed with zero xruns and no terminal fault,
but retained one additional transport gap: 18,944 missing frames beyond startup,
about 0.395 seconds. The two-note capture window contained 130,351 nonzero samples
per channel instead of 144,928. Its first late request recorded 341,474 us in
send and 2,138 us in reply; these wall times include scheduling and do not isolate
a cause. The bounded gap trace retained 32 request records. The operator's later
clean-audio impression does not erase this objective interruption.

The 45% sequence completed with zero xruns, deadline misses, processing failures
or terminal faults, and no additional gap beyond startup. Peak was 0.386334;
aggregate per-channel RMS was 0.0572365. The operator answered: "They both sounded
perfect. I am more than happy to leave it at 45%" when comparing the two follow-up
passes, including the end of the eight-note chord.

## Readback precision and chosen level

The operator explicitly selected five percentage points of control travel,
50% to 45%. After the 50% repeat, restoration of the saved baseline succeeded,
and Pigments returned 0.44999998807907104 for the requested 0.45. The private
harness incorrectly required a tighter-than-float32 match and timed out before
any 45% notes. That session retired normally. The check was corrected to accept
the exact requested value or its exact float32 rounding. Only the remaining 45%
sequence was launched, in a fresh session. No product source repair is claimed.

The verified 45% state is saved privately (381,326 bytes), and a private playing
preference records it as the operator's chosen level. Future supervised launches
should apply and verify 0.45 before connecting MIDI/audio. This does not install
automatic startup or prove reopening that saved envelope. Normalized control
positions are not linear-amplitude percentages or a claimed dB reduction.

## Health and retirement

All health samples had `0x0` flags: 73 for the initial pass, 79 for the repeat
session and 68 for the 45% session. Their voltage ranges were respectively
4.84142–5.05448 V, 4.84142–5.05448 V and 4.87090–5.07056 V; peak temperatures
were 61.7 C, 61.7 C and 65.0 C. These are one-Hz observations, not proof of
sustained supply margin.

All three native owners exited 0 with all 127 retirement milestones. Windows
workers retired normally. Exact transports, FIFOs, fixture and Pigments JACK
ports, native/translated/fixture processes and observers were absent afterward.
The original outer unit and 60-second watchdog were restored. Raw audio and
machine records remain private; only sanitized measurements are published.

The chosen 45% setup passed this bounded eight-note sequence and operator
listening. The earlier whole-unit failure, first-pass audible artifact, repeat
transport gap, sustained operation and broader preset/polyphony qualification
remain distinct issues. See [prior master correction](RPI1_MASTER_LEVEL.md) and
[sanitized evidence](../../evidence/rpi1/polyphony-levels.json).
