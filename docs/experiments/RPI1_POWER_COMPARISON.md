# RPI1 final power comparison — 2026-09-22

**The Pi remained responsive without recorded undervoltage, but this attempt
stopped on an initial setup-response timeout before audio started.** It cannot
qualify the new supply under sustained processing. The prior measured/audible
audio and clean-retirement pass remains unchanged.

The operator requested one final test after changing to a MacBook Pro supply.
The Pi accepted a normal shutdown before the swap. It initially remained
unreachable; once the operator reported SHIELDXL0 READY, direct SSH from the Mac
connected to a fresh boot. The homelab was used only for a bounded reachability
cross-check while the Pi was offline; all test control and recording used the Mac.
The exact adapter model and its 5 V rating were not provided or inferred.

## Exact attempt

The native executable was reused byte-for-byte from source
`1db3478292fda7839d051f703992c8ddc0040653`, tree
`f7edd886464f5791b86332375661248f8ffc6fd9`. Repository head at the attempt was the
result-only commit `d24c3d881fba818cdc831d8c36acf2a96852f99f`. Digests and sanitized
records are in `evidence/rpi1/power-comparison-startup-timeout.json`.

JACK was available at 48 kHz / 512 frames without restarting its service. The
bridge delay remained 2048 and intended vendor quantum 256. USB MIDI remained
unplugged and the editor stayed closed. Persistent prior-boot journal access was
verified, and independent one-Hz health plus remote journal/ping recording ran.
Only the temporary runtime watchdog disable was applied; its original 60-second
setting was restored afterward.

Exactly one Windows/Pigments launch ran. The architecture/host handshake passed,
the exact module loaded, and component initialization, parameter inspection and
initial state capture succeeded. The native standalone then reported
`RPI0 setup failed: 2` and exited 1 before publishing `RPI1_READY`. The deterministic
MIDI/audio fixture was never launched. No audio block was processed.

## Diagnosed boundary

The retained native fault is operation 20 (Configure), fault 3,
`WouldBlock: Resource temporarily unavailable (os error 11)`, with zero request,
result and processing progress. The Windows journal contains no processing-setup
receipt and no plug-in terminal exception.

The source returns a native session after the mapping handshake for protocol
minor >= 4. RPI1 immediately requests setup; `Session::exchange` uses a 10-second
response deadline. The Windows owner finishes plug-in initialization and
inspection before it can service Configure. The recorded monotonic milestones are:

| Milestone | Seconds since boot |
| --- | ---: |
| Mapping ready | 133.221075 |
| Component initialization began | 133.712907 |
| Component initialization succeeded | 143.010976 |
| Initial inspection/state capture complete | 143.328038 |
| Cohort stop began | 143.356396 |

Initialization alone took 9.298069 seconds; mapping-to-inspection took 10.106963
seconds. The failure operation, socket error, source order and timing identify
an initialization/readiness versus Configure-deadline problem. Exact socket-send
and timeout timestamps were not recorded; the attribution uses the combined
source and timing evidence. The 44-frame latency was observed before setup and
does not establish rejection of the bus contract or configured 48-kHz latency.

The next repair should give plug-in initialization its own bounded readiness
contract before the ordinary Configure response budget begins, preserving
cancellation, failure reporting and cleanup. A blind sleep, repeated launch or
general relaxation of audio/control deadlines is not a diagnosed remedy. No
source change or additional physical attempt was made after this result.

## Power and retirement limits

Across 25 one-Hz active samples, voltage ranged 4.83338–5.02098 V and temperature
peaked at 51.8 C. All sampled throttle registers were `0x0`; the kernel contained
no undervoltage event. MemAvailable remained at least 5,718,424 KiB. Outer memory
peaked at 163,352,576 bytes and translated-cohort memory at 2,094,239,744 bytes.
The Mac received all 120 ping replies. No restart occurred during the attempt.

This is better observed voltage behavior than the preceding run's recovered
undervoltage event, but the workloads differ: this attempt never reached audio.
It does not prove that power caused or now resolves the earlier whole-machine
failure, or qualify a sustained audio load.

The native process and translated cohort stopped, experiment JACK ports/routes
and command FIFO disappeared, and health/remote recording stopped. The failed
outer-unit status and private transport directory are preserved. No normal
product-retirement receipt was reached; this is startup-failure containment.
The Pi remains available with the original watchdog restored. Stop after this
one comparison; state, soak, editor, physical MIDI and overall RPI1 remain open.
