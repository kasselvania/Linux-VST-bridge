# RPI1 lower master volume clears the reported crackles — 2026-09-22

Pigments' exact Master Volume parameter was lowered and read back successfully.
The operator repeated the chord with unchanged interface gain and answered
"Yup! No crackles. Perfect audio". Measured stereo peak was 0.5704991, compared
with 1.4802243 in the preceding overloaded session. Normal retirement passed.
This identifies a working level correction for the reported symptom on this
fixture; it does not qualify arbitrary presets or establish a voice ceiling.

## Executed source and parameter

The run used the already-tested master-capable binary from source
`1173e816bdea7f334347e411ddb15b438f81467b`, tree
`d97e60f8ab04fc0e1604fd975a465f365db4bc4c`, SHA-256
`81a44a8eed7191b08655496dfe0b49c0c919208e9f7a600658bc37887a2006c7`.
Repository head was `4003e5853d752af28ba6a8d0647c204eb92eb0d0`.
No source, runner, plug-in, bus, buffer or interface-gain change was made.
The editor stayed closed. JACK remained 48 kHz / 512 frames / 3 periods,
bridge delay 2048 and vendor processing quantum 256.

The parameter is ID 0, title `Master Volume`, units `dB`. Original normalized
value was 0.67308074235916138; the accepted lower value was 0.5, read back at
revision 4 before the physical MIDI route was connected. These are normalized
control positions, not linear gain percentages or a measured dB change.
A 381,312-byte state envelope containing the reduced setting was saved privately
before playing. This envelope has not been restored in another session, and no
automatic startup default was changed.

## Preserved confirmation failure

The first attempt queued the volume write but timed out awaiting its readback.
The initial 4446-parameter snapshot was still streaming after its first Master
Volume record. Existing `EditorSession::request_refresh` accumulates only flags
when a stream is active; a zero-flag overlapping request leaves no pending
refresh. The private harness had requested the second snapshot too early.

No MIDI or playback routes were connected in that attempt. It accepted zero
MIDI events, produced zero samples, reported no transport/audio faults and
retired cleanly with all 127 milestones. All 34 health samples had `0x0` flags.
The retry issued the volume write once and used bounded read-only refresh
queries to obtain a subsequent exact readback. It did not replay the write or
change Windows-host source. The companion's pending-refresh bookkeeping remains
a known control-path gap; the harness change is not a product repair claim.

## Physical comparison and retirement

The existing OMX-27 Type B / ShieldXL UART alias fed `lvb-arm-pigments:midi_in`;
stereo outputs fed the same `system:playback_1/2` ports. No automatic notes or
editor were introduced. The operator was explicitly asked to use the same chord
and unchanged interface gain, then confirmed the overload/crackles were absent.

The completed session accepted 110 MIDI events and produced 564,207 nonzero
samples per channel, with peak 0.5704991 on both channels. It processed 13,468
blocks / 3,447,808 frames (about 71.83 seconds). Xruns, callback deadline misses,
process failures, nonfinite samples and terminal faults were zero. The known
2560-frame startup gap remained; state capture also recorded 512 paused frames.
These sessions used human playing, not identical recorded MIDI, so peak ratios
are not a calibrated gain-transfer measurement. The exact clipping stage and
maximum simultaneous voice count were not measured.

All 90 health samples reported `0x0`; voltage ranged 4.86554–5.05850 V and
maximum temperature was 63.9 C. Normal quit exited 0 with all 127 milestones;
the Windows worker joined and its mapping was unmapped. Processes, session
transports for both attempts, command FIFO and Pigments JACK ports were absent
afterward. The prior outer unit and original 60-second watchdog were restored,
and recorders stopped. The successful level correction does not resolve the
[earlier whole-unit failure](RPI1_HEADROOM_INTERRUPTED.md).

Previous level observation: [inline-meter reload](RPI1_METER_RETRY.md).
Sanitized evidence: [`master-level.json`](../../evidence/rpi1/master-level.json).
