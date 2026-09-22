# RPI1 fresh-process Pigments state reopen — 2026-09-22

The operator-selected 45% Master Volume state was reopened in a fresh Pigments
process, confirmed by exact parameter readback, played through the physical
ShieldXL UART MIDI and stereo audio route, and retired normally. The operator
reported, “It is great. You can close it.” This closes the bounded saved-state
reopen and physical-playing gate; it does not install automatic boot startup.

## Execution and result

Repository head at execution was `1d4ee3ddd633d5eab7a397f1aa37f82806d4279f`
with tree `4be2dbe99ae097458d030b8d342f7ee6f59d1b55`. The unchanged native
binary has SHA-256 `81a44a8eed7191b08655496dfe0b49c0c919208e9f7a600658bc37887a2006c7`.
The private selected state was 381,326 bytes. The editor remained closed; the
JACK period was 512 frames and the bridge delay remained 2048 frames. No
Proton, Box64, plug-in, bridge, bus, or source binary was changed for this gate.

On the successful attempt, a fresh process first reported Pigments parameter ID
0, **Master Volume**, at `0.67308074235916138` (revision 1). The existing
restore command reported `RPI1_STATE_RESTORED bytes=381326`. A read-only
parameter refresh then returned `0.45000001788139343` (revision 2), one binary32
step above the prior saved-state comparison value `0.44999998807907104`. Both
values bracket the requested normalized 0.45. No master write was issued.

Only after this confirmation, the exact JACK graph connected
`system:midi_capture_2` (ShieldXL UART) to `lvb-arm-pigments:midi_in`, and the
Pigments left and right outputs to `system:playback_1` and `system:playback_2`.
The OMX-27 was the physical controller. The final live record showed 58 MIDI
events accepted, 468,720 nonzero output samples on each channel, peak
`0.37648812` on each channel, and zero nonfinite samples, xruns, process
failures, or terminal transport faults. The callback summary had zero deadline
misses and one 2,560-frame startup gap, with no additional gap after startup.
The operator reported good audible playing and asked to close the session.

The native process exited successfully, recorded all 127 retirement milestones,
and published `RPI1_CLEAN_SHUTDOWN`. The Windows worker stopped and unmapped its
endpoint. The exact session unit and observer were inactive afterward, the
Pigments JACK ports and command FIFO were absent, the original user-unit
configuration was restored, the exact session transport directory was absent,
and the runtime watchdog was restored to 60 seconds.
All 161 one-Hz health samples recorded `get_throttled=0x0`; voltage ranged
4.86554–5.05850 V and the highest observed temperature was 61.15°C. These
samples do not establish sustained power margin or resolve the earlier whole-Pi
failure.

## Preserved first attempt

An earlier fresh-process attempt restored the same state and returned the same
`0.45000001788139343` value, but its private harness demanded only the lower
adjacent binary32 value. It stopped before connecting MIDI or playback; zero MIDI
events were accepted and zero output samples were produced. That session also
exited normally with 127 retirement milestones. The successful retry changed
only the private confirmation predicate to accept the two adjacent binary32
representations around 0.45. This was a diagnostic check correction, not a
product or plug-in repair. The previously observed product-facing set-and-confirm
refresh issue remains a separate follow-up.

The private state, raw journal, account-bearing environment and machine records
stay on the fixture. The [sanitized result](../../evidence/rpi1/saved-state-fresh-launch.json)
contains only bounded measurements and identities. Automatic startup, editor
operation with this restored state, long-soak stability, all presets, and overall
RPI1 qualification remain open.
