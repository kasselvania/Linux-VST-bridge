# Native process-call timing proposal

Base74705deb47d256326feaba8a11b4de4b39cfb0fa; pending source review, not implemented
or installed evidence. The operator requested complete call durations before any
scheduler/editor repair. Sample-accurate capture alignment is not a prerequisite.

The existing AP23 phase trace retains128 guard-held calls reaching the backend,
misses early admission/busy refusals, and stamps before guard cleanup. Keep it as
a separate optional phase diagnostic; it cannot provide the complete population.

## Measurement and ownership

Surround the existing native SDK process body with one prepared observer. Its
entry clock is also the existing containment origin. The body retains every
decision, wait, SDK sink and busy guard; all body locals/guard unwind before the
final clock. Every SDK result and exception unwind has a record. The final stamp
is the last controllable SDK return boundary after body cleanup. Fixed record
publication/function epilogue follows it: internal telemetry cannot observe the
caller instruction after return. Independent caller clocks bracket this tail;
report observer overhead, without instruction-perfect timing claims.

Opt-in LVB_PROCESS_CALL_TRACE=1 is sampled during non-audio instance construction;
default off. Allocate/prefault262,144 fixed record slots before activation (roughly
32MiB maximum; exact layout asserted). No resize/overwrite or live observer thread.
Proven lock-free atomics reserve unique slots and release/acquire immutable records,
including concurrent/reentrant busy refusals. Saturation, counter overflow, allocation
failure, incomplete writers and calls after sealing are explicit, never a complete
dataset. Storage remains owned until SDK object destruction. Normal terminate
seals/snapshots without waiting for an audio writer; refuse complete export if any
writer remains. Post-terminate calls are outside the ended SDK lifetime and cannot
be promoted as part of a complete lifetime. No callback lock/allocation/format/IO/free.

Each call retains instance identity/entry sequence, CLOCK_MONOTONIC entry/return,
actual calling namespace PID/TID, signed actual N, actual mode/precision and SDK
result or unwind outcome. Accepted rate/maximum/configuration revision, phase,
backend handle and available project sample position are captured under the existing
busy guard with validity bits. Busy refusal must not race configuration reads.
A fixed Linux calling-TID query is the only additional kernel metadata operation;
qualify its callback allocation/I/O posture. Retain every call, not only outliers.

Records/summary cross a separate versioned diagnostic C ABI as plain data, never
SDK/STL ownership. C++ owns SDK-edge capture and quiescent snapshot; Rust owns
bounded off-thread serialization beside the exact existing session report. Private,
exclusive, nonsymlink export reports all failures and does not enlarge the ordinary
report. Mandatory AP23 ABI2, IPC15 and processing policy remain unchanged. Off-RT
collection authenticates namespace/host PID/TID/starttime with session/engine.
A live task census tied to the recorded caller TID is role evidence, not names.

## Tests and installed comparison

Test production Processor success/zero/partial, validation/busy refusal, backend
failure, terminal containment, SDK sink failure and unwind against independent
caller clocks. Test saturation/no-overwrite, invalid clocks, allocation refusal,
quiescent/in-flight export, output custody and bounded/private export failure.
Callback audit with observer off/on retains allocation/I/O/wait counters. Keep
meaningful failing-before/passing-after evidence. Affected supported targets only;
inherited Pi/macOS limitations do not become this repair task.

Freeze one reviewed candidate; install normally and reprepare only Pure LoFi and
chosen first-party reference through offered actions. Verify loaded engine, exact
predecessors/settings, licensed state, original projects and unselected publications.
Three first-attempt180s conditions: Pure LoFi editor open; Pure LoFi editor closed;
first-party bridge reference. Same host/rate/capacity/delivery and equivalent MIDI
where applicable. Retain full output/pre/post-roll, graph/recorder counters, all
call records and normal retirement. No kernel trace or priority change. Read the
exact session-owned wineserver policy/dependency off RT.

Report whole-call distributions, every sequence/N, cadence exceedances and coverage
separately from signal/graph errors. Whole-call time includes fixture DSP; N/Fs or
graph512/48k does not prove a device miss. Several short calls can cumulatively
consume a graph period; one threshold is not a complete deadline oracle.
Do not replace the matched instrumented open baseline with older data, or call
different plug-in DSP an exact waveform match. Capture alignment limits per-gap
attribution, not measured call duration. Preserve first failures; no causal repair
or scheduling qualification from duration measurements alone.
