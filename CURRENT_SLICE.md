# RPI2 CPU efficiency — existing FEX statistics

Base commit: 4978d0f (full resolved identity retained with final evidence).
Base tree: 71d0d6eb2bcd684968293798b70007725d281135.
The operator authorizes autonomous continuation; this thread remains the sole
writer/live Pi owner. The coordinator reviewed the pinned FEX schema and reader.
The preceding multicore preference check is complete: startup OFF is overridden
by unchanged24AM full state ON, so no mislabeled multicore comparison will run.

## One bounded outcome

Determine whether the existing pinned ARM64EC FEX shared statistics are available
and whether their translation/cache activity distinguishes the first four-note
hold from its warmed repeat. No new runtime/native/Windows build, feature flags,
GUI, settings, state edits, latency, affinity, priority or governor changes.

Use the unchanged85780f4856f2bac9f605628ca7decd80568cca0ad36fa92db70960bf74dd1a62
native candidate explicitly; phase tracing OFF. Existing Pigments7.0.1.6772,
GE-Proton11-7 ARM/FEX prefix, 24AM Poly4/master0.35, notes60/64/67/71 velocity96,
48k/JACK512/vendor256/reserve2048. One fresh session: availability after READY,
then only the existing20-second warm-up and20-second measured capture if usable.
The same near-drained estimate and75C/current power-warning stops apply.

## Reader and interpretation

Source: FEX82510eb452b258959ef982be58a9c3c1bafc82a4 SHMStats.h and allocator,
ARM64EC Module.cpp and UnixLib. Version2/header64/slot112. Head/Next are byte
offsets from base; Linux getpid names the file but GetCurrentThreadId values are
Windows IDs. Validate exact owned Pigments mapping and process lifetime. Check
host /dev/shm and that process root/dev/shm using its NSpid when needed.

Read-only bounded snapshots no faster than2Hz; record observer CPU/time. Reject
unknown schema, invalid bounds/linkage, unstable topology, reset or churn for
attribution. No atomic whole-process claim or Linux/Windows TID guess. Retain
raw CNTVCT ticks; derived elapsed seconds may use kernel-reported54.00MHz reference
timer, never CPU GHz. JIT-path time includes lookup/locking/compile work and is
not CPU-only compilation. Avoid adding overlapping time counters.

If unavailable/unusable, retain the specific dependency and stop after this one
launch without benchmark or rebuild. If usable, compare sparse process-level
deltas with captured audio, whole held2–14including attack and stable3–14,
retaining failed warm-up. No per-audio-thread claim without verified identity.

## Scope, restoration and delivery

Use existing recorder_compare/trace_analysis helpers with narrowly scoped
reader/reducer/tests, this card, RPI2 documentation and sanitized evidence.
No new framework/dashboard. Keep proprietary captures/state and raw paths private.
Restore original user state/master, preference bytes and JACK graph; retire only
owned processes. Preserve generic9d0a611... and helper-default24e6ab17... binaries.
Commit/push final results to draftPR150 without merge/default replacement; stop
with the measured finding, limitations and next evidence-backed lever.

## Observed disposition

Existing FEX v2 counters were readable. One session and the two existing captures
completed; both had substantial output silence despite no thermal flags and
near2.4GHz samples. The repeated cutoff around2.39s preceded its later large
JIT burst, with zero compile attempts in intervals overlapping2–3s. A separate
Arturia-named process contributed additional repeat CPU, while Pigments-only CPU
also rose. No sole cause or reliable warmed baseline is claimed. Cleanup and
five focused reader/reducer tests passed. No further runtime experiments this turn.
