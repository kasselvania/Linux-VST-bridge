# Current task: complete native VST3 process-call timing

## Outcome and source

Reliably observe every native VST3 process() call from SDK entry through its return
boundary, including zero/partial calls, early refusals and exception exits. Use
permanent opt-in, prepared bounded storage; export only off the audio thread.
Base 74705deb47d256326feaba8a11b4de4b39cfb0fa, tree
43b2e347a11b79e1e0c06e0d96804d818c57bfbd, branch codex/audio-callback-timing.
The prior completion endpoint/#207 remains intact. Basis: current operator order,
AGENTS.md real-time/ownership laws, GOVERNANCE.md evidence/completion, architecture
18.1–18.4 and D-030. Completion/containment, IPC15, runner and scheduling stay unchanged.

## Contract and acceptance

Retain all call sequences, actual N/mode/precision, accepted rate/capacity,
instance/configuration context, monotonic entry/return stamps and actual calling
PID/TID. Map namespace identities to exact host PID/TID/starttime off RT; names
are not role proof. Allocation refusal, saturation, dropped/unfinished observations,
invalid clocks and export failure are explicit. No RT allocation, filesystem/logging,
ordinary lock or observer thread. The [proposal](evidence/audio-recovery/2026-10-05-process-call-timing-proposal.md)
defines the measurement edge and observer tail; review before implementation freeze.

Test all SDK exit categories, zero/partial calls, backend/sink failure, unwind,
reentrant/concurrent refusal, saturation, lifecycle export and callback allocation/
I/O posture. Independent caller clocks must enclose the measured interval.
On one reviewed installed candidate, retain three first-attempt180s Bitwig
conditions: Pure LoFi editor open, Pure LoFi editor closed, first-party bridged
reference. The added open baseline makes the editor comparison use one instrumented
build. Same declared host/audio settings and clip where semantically applicable;
different plug-in DSP/audio is not an exact waveform match. Retain whole output,
all call records, exact engine/process identities, scheduling, graph/recorder errors
and retirement. Sample-accurate capture alignment is not an entry gate; report limits.

## Limits and custody

Whole-call time includes vendor DSP, not isolated bridge overhead. N/Fs and
graph512/48k are cadence context, not proven device budgets; several short calls
can consume one graph period. No causal gap, dependable continuity, RT/DAW,
interaction/soak, update-UX completion or beta claim. Previous evidence remains;
the [scheduler window](evidence/audio-recovery/2026-10-04-paced-scheduler-window.md)
does not authenticate a callback thread or align individual output gaps.

Execution/source/SSH: Sol6.1 xhigh; GUI: Sol6.1 high with explicit lease; review:
Astra6 xhigh; root orchestrates. Refresh inactivity before installation; preserve
predecessors/settings, all unselected publications, licensed state and original
projects. Use working copies and normal offered preparation. Read the exact owned
wineserver policy during normal setup; no priority mutation or kernel trace.
Reuse enrolled Keychain access. One builder/VM at a time, reserve Audiobookshelf
headroom; stop idle builder before timing. Retain first outcomes, no rerun until
green. Commit/push and create a focused draft PR stacked on #207; no merge.
