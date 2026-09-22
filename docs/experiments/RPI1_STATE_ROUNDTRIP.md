# RPI1 editor-closed state round-trip — 2026-09-22

**One same-state save/restore passed: measured and audible stereo before and
after, unchanged playback routes, resumed transport and complete retirement.**
The operator confirmed: "Yes, I heard both".

No product code changed. This reused the native executable from source
`8eb89230410a1b42531bfb0fc252cddb166b1d5d`, tree
`ff138a43d6d9be9c517e8aa0209cf268690fa6c9`, with SHA-256
`2689b5a0af961d8637007c6be7a1af58ce55458b26ebb16662c2455007202553`.
The repository head at the attempt was the later documentation commit
`dc83017f48f1a777bff429eddd0253c5d8f69210`. The existing qualification client,
Windows host, Pigments, runner, bus contract and bridge delay were unchanged.

The one session ran on the already-connected MacBook supply, with the editor
closed and USB MIDI unplugged: 48 kHz, JACK 512, bridge delay 2048 and vendor
quantum 256 frames. Initialization observation completed in 5449 ms.

## State and audio result

The existing save command captured a 381,326-byte envelope containing 381,222
opaque host-state bytes. It was stored privately on the Pi with mode 0600; no
state payload was exported or committed. The existing restore path paused the
callback, stopped and deactivated the component, restored state, reactivated,
restarted and resumed. It returned a successful 381,326-byte receipt, and transport
advanced from epoch 1 to epoch 2.

The exact native left/right outputs remained connected to `system:playback_1/2`
before and after restoration. Native MIDI/audio ports remained present. The same
bounded qualification client ran once before and once after restoration; its own
MIDI/meter ports retired between measurements. This proves preservation of the
playback routes, not persistence of a fixture client that had already retired.

Each measurement on each channel recorded 240,000 samples, 72,928 nonzero samples,
144 nonzero windows, peak 0.19765145 and RMS 0.076079156529. These summary statistics
match exactly; no samplewise waveform comparison is claimed. All six MIDI messages
were accepted. Nonfinite samples, JACK xruns, process failures, callback deadline
misses and terminal transport faults were zero.

This restores the same state and proves the mechanical round-trip does not break
processing. It does not prove restoration of an editor parameter deliberately
changed afterward, restart persistence, preset fidelity or arbitrary vendor state.

## Retirement and residuals

The host processed 1322 blocks before restoration and 1166 after, totaling 2488
blocks / 636,928 frames. The final epoch's position was 298,496 frames; it is not
the overall completed-frame count. Both processing workers joined without an
exception. Final endpoint mapping was unmapped, native retirement reported all
milestones (127), and clean shutdown completed with outer exit 0.

Startup retained 2560 missing/expired frames in one gap episode. Restoration's
intentional pause accounted for 30,720 frames; priming totaled 4096 frames across
the two epochs. Timing/performance remains unqualified.

Across 31 one-Hz active samples, temperature reached 57.3 C and sampled voltage
ranged 4.77844–5.01696 V. Throttle flags remained `0x0`, no kernel undervoltage
event was recorded, and the Pi stayed responsive. MemAvailable stayed at least
5,578,276 KiB. Outer/translated memory peaks were 59,351,040 and 2,019,328,000 bytes.
Short-run supply margin is not promoted to sustained acceptance.

The successful transport directory, command FIFO, native/translated processes and
experiment JACK ports/routes retired. The private state file remains on the Pi.
Health and remote journal observers stopped. The approved temporary watchdog
override continued into the headless diagnostic gate, then the original 60-second
runtime watchdog was restored and read back. Earlier failed sessions remain preserved.

Sanitized receipts, lifecycle rows and measurements are retained in
`evidence/rpi1/state-roundtrip.json`. The separate ten-minute headless gate was then attempted and stopped early after
the operator reported a brief buzz; see [the interrupted soak record](RPI1_HEADLESS_SOAK_INTERRUPTED.md).
Editor, physical controller and the final combined appliance remain later gates.
