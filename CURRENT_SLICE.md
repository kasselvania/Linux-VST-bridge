# RPI2 Digitalis JACK-period descent

Base: `223e02ddc3c24ef47ef00da7e3af95adb93f4df1` (draft PR #159).
Base tree: `4656219dca3db9a693f9e3a6dfd96b0634b7cfab`.
Branch: `codex/rpi2-digitalis-latency`, separate from PR #159 and the canceled multicore/UI checkout.

Basis: `AGENTS.md` real-time, fixture, privacy and slice laws; `GOVERNANCE.md` evidence and safety; `docs/ARCHITECTURE.md` host/process timing boundary; the 2026-09-24 device-agent handoff's shared-appliance and latency direction; and the operator's request to test Digitalis at JACK 256 and possibly 128 frames.

## Primary claim and fixture

On the Pi 5/ShieldXL with the exact privately staged Digitalis v1.1 Windows x64 module and the saved user-approved effect slot from PR #159, compare short stereo delivery and operator feel at JACK 512 and 256 frames. Keep the 256-frame vendor processing quantum, 2,048-frame bridge reserve, reported 4,096-frame vendor latency, native observer, Windows host, Wine/Proton/UMU/FEX runtime, and physical route fixed. A 128-frame run was initially planned, then canceled by the operator before capture in favor of an interactive 256-frame editor session.

This slice is an evaluation, not a promise of a low-latency pedal result. A smaller JACK period does not imply a smaller vendor or bridge delay. Report JACK-port-relative timing separately from analog input/output latency; an analog claim requires physical loopback evidence.

## Scope and acceptance

- Use one Pi owner, an isolated branch, the existing private runtime and state. Inspect current JACK service and ports first. Make only a reversible period change; restore the original 512-frame service/graph and verify it after testing.
- Run one same-slot 512 reference and one 256 session. Use fresh processes, editor closed, the same source-owned physical stereo signal/capture route, normal low-rate status reads, and identical settings except JACK period. The requested interactive 256-frame editor session is separate from the matched capture.
- Qualify the existing source-owned JACK helper for 128/256/512 if needed. Record input/output relationship at JACK ports, completed/delivered/missing frames, longest gap where observable, xruns, process/callback faults, temperature, power flags, and clean retirement. Do not infer audible quality from CPU percentage or sample count alone.
- Ask the operator for a bounded listening comparison if they are available. Label this subjective. Stop on setup mismatch, thermal 75 C, power/current warning, terminal fault, or sustained delivery loss. If 256 fails, do not run 128.
- Retain private audio/logs. Commit only sanitized evidence and source changes. No vendor installer, serial, state, preset, binary, or audio in Git.

## Non-goals

No bridge reserve descent, vendor preset/quality change, sample-rate change, plugin reauthorization, working-runner replacement, FEX tuning, Wine yield change, multicore/UI campaign, priority/affinity/governor experiment, kernel work, new dashboard, or broad latency framework. No near-imperceptible or sub-10-ms claim without measured end-to-end evidence.

## Observed disposition

Both matched five-second physical stereo captures completed with nonzero audio, zero capture xruns/bad blocks, zero reported missing frames/gaps/process or callback failures, and normal clean retirement. The 256-frame capture remained healthy. The operator first said it felt the same as 512, then corrected that impression: 256 felt snappier. Both sessions still reported 4,096 vendor frames plus a 2,048-frame bridge reserve, so JACK period reduction alone has not demonstrated a measured wet-path latency improvement. No analog or JACK-port-relative loopback latency result is claimed. The 128-frame process reached ready and exited cleanly after the operator redirected the task; no 128-frame capture or listening result exists.

The separate live 256-frame editor session closed cleanly at the operator's request and restored JACK 512. An interactive 128-frame session then opened briefly with the vendor quantum and reserve still at 256/2,048; the operator canceled that unmatched setting before testing, and it also closed cleanly and restored JACK 512. Details are in `evidence/rpi2/digitalis-jack-period-512-256.md`.
