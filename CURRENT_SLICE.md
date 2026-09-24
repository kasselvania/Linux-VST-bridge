# RPI2 remaining CPU cost

Base: `6297332ef7e4f797d79bfe103c82b6a172b68a9a` (draft PR #153).
Base tree: `bd2832d59dbd4fec82fd6074410923fc607ed9f7`.
Work on `codex/rpi2-kernel-codegen-profile`, stacked on #153. The outcome is
one bounded explanation of the Pigments processing worker's kernel CPU and
the emitted ARM code for the already identified caller arithmetic-loop family,
followed by one justified semantics-preserving optimization or a precise
capability limit. The coordinator independently reviews pinned FEX source and
handles any legitimate privileged bootstrap.
Basis: `AGENTS.md` real-time, fixture, evidence and privacy laws;
`GOVERNANCE.md` slice execution; `docs/experiments/RPI2.md` “Critical-path
profile of the 256-frame Pigments candidate” and this follow-on. The pinned
runner remains
GE-Proton11-7-aarch64 / UMU 1.4.4 / steamrt4-arm64 /
FEX 2609-41-g82510eb on Pi 5 kernel 6.18.50+rpt-rpi-v8.

## Fixture and boundaries

Reuse the private native/Windows candidate pair c9709c05/9a1e93f5 at vendor
quantum 256, map v2/capacity 512, Pigments 7.0.1.6772, 24 AM Poly 4, master
0.35, automated notes 60/64/67/71 velocity 96, 48 kHz, JACK 512 and reserve
2048. Keep native phase tracing off and the editor closed. No GUI/computer
control, runtime/kernel rebuild, broad layer swap, changed quantum/latency,
preset/polyphony/quality, licensing or state manipulation. Retain first-use
failures, private vendor artifacts and exact identity evidence.

## Capability first

Check staged `perf` kernel cycles and stack capture for the exact active worker
TID without lowering global perf security. If an authenticated narrowly owned
privileged sampler is unavailable, record the denial and stop. Check whether
the pinned FEX map and in-memory JIT header/tail can identify and extract only
bounded emitted ARM bytes for the prior arithmetic-loop family. Bind PID start
time, mapping, sample timestamp and read time; reject stale or ambiguous code.
Do not dump the code cache, enable heavy GDB/IR output or create a generic
profiler. Prove the capture path with cheap preflight before a live session.

If both capabilities hold, use one owned warm-up and one measured 20-second
four-note capture. Sample kernel cycles/stacks modestly on the worker and user
cycles on the caller, with explicit SIGINT/flush and an observer-CPU stop if
instrumentation becomes significant. Compare CPU with aligned completed frames,
delivered frames and losses; scheduler runnable wait is distinct from blocked
time and system CPU. Stop at 75°C or current power flags. Preserve original
state/master, preference bytes and metadata, governor, scheduler statistics,
graph, binaries, runtime and environment; clean up only owned processes.

## Delivery

Publish sanitized evidence and a concise optimization-ledger update. If a
clear low-risk project-owned fix emerges, test one isolated before/after with
matching work and diagnostics-disabled confirmation. Otherwise state the
precise source or measurement limit without claiming a gain. Commit, push and
open one draft PR stacked on #153; do not merge.

## Observed completion boundary

One initial startup failed before READY or notes because its configuration
selected the old Windows AP1 layout. That failure remains private. The
corrected session used the hash-pinned prior configuration; its warmup lost
650,240 frames without perf, and its measured capture stopped after a
temporary audio-statistics failure at graph-relative +12.085 s, adding
265,728 missing frames. The completed, loss-free +4.258–6.130 s bracket
supports CPU per completed frame and kernel/JIT mechanism attribution only.
The worker's sampled kernel cycles included `getrusage`, `sched_yield` and
futex wait; the first two match a verified installed Wine yield sequence.
Bounded JIT reads identified unchanged emitted ARM for one sampled arithmetic
block, including scalar FP work and lane preservation. No safe optimization
or diagnostics-disabled gain was established. Original state/master,
preference bytes, settings and graph were restored. Sanitized evidence and
limits are in `evidence/rpi2/pigments-remaining-kernel-codegen.json` and
`docs/experiments/RPI2.md`.
