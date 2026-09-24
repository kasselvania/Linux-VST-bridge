# RPI2 Digitalis timing selector

Base: `a1af1c2634267381ca35cfbebd88634302158ecd`; base tree `1d08a05ef0ddbbe16e4c8df9c63f6eb0ed666fa3`.
Branch: `codex/rpi2-digitalis-all-128`, isolated from PR #159 and the canceled multicore/UI checkout.
Basis: `AGENTS.md` “Work to an outcome”, “Keep the engineering safeguards”, and “Verification and review”; `GOVERNANCE.md` “What evidence means”; `docs/ARCHITECTURE.md` “7.3 Audio shared memory” and “7.4 Real-time failure posture”; and the operator's request for a simple Digitalis boot and timing selector.

## Claim and fixture

On the Pi 5/ShieldXL, the owner can select 128, 256 or 512 frames with one private command. The selection moves JACK's period, the generic native bridge reserve and the Digitalis processing quantum together, reopens the exact saved sound in the editor, and routes stereo ShieldXL input/output. A supervised stop or size switch retires the old session and restores the 512-frame JACK service period between runs. The same Windows host, Digitalis v1.1 module, pinned runtime, native observer and saved state are used throughout.

This is an experimental timing selector for owner audition, not a claim of gap-free performance or imperceptible latency. The operator subsequently measured an approximately 90 ms round trip after isolating a substantial Bitwig contribution from an earlier 275.79 ms HW FX readout. The exact measurement trace and isolation procedure are not retained in this repository; this is an operator-reported physical result, not a source-test result.

## Source and boundaries

Allow 128/256/512 values only through the existing setup, queue and latency validation. Preserve the host-maximum-versus-reserve check, real-time callback behavior, state identity, vendor-reported latency, and explicit loss/fault counters. Build the native module privately with the same map-v2 512-capacity feature as the retained Windows host. Do not replace the working native module, Windows host, runner, plug-in, prefix or saved sound. No FEX/Wine, kernel, multicore, quality, sample-rate, affinity, priority or UI-automation work.

The Pi-only selector and configs remain private because they bind the licensed environment and device paths. The user-visible command is `digitalis-live 128|256|512`; `status`, `save` and `stop` are explicit. Switching reopens the last saved sound slot; `save` must be requested before switching if current editor changes should persist. The live session is bounded to two hours and retains the existing 75°C/current-power-warning stops. It restores JACK 512 when stopped or timed out.

## Observed status

Focused source-owned bridge tests pass for 128-frame quantum/reserve and event ordering. The map-v2 native candidate built on the Pi and the private selector opened 256, then 128, then 512, then 128 with the editor and exact JACK/quantum/reserve values read back. An editor-closed 128 source-owned stereo capture completed with finite audio, zero capture xruns and normal session shutdown. Its bridge observer nevertheless recorded 1,024 missing frames and three gaps; another 128 run recorded 1,024 missing frames and two gaps. The matched 256 live pass recorded 768 missing frames and two gaps. A later live 128 status read recorded 3,456 cumulative missing frames, 19 gaps and one JACK xrun with zero process/callback failures. These are delivery failures, not clean performance results. The operator considers this Digitalis development check complete for now; it does not qualify a low-latency pedal setting. See `evidence/rpi2/digitalis-timing-selector.md`.

Cleanup after the bounded source-owned sessions and size switches restored the original 512-frame JACK period. The later editor session was owner-supervised with restoration at stop or timeout; its final retirement was not independently rechecked for this review. Preserve private recordings and logs; publish no vendor binary, preset, state or authorization material.
