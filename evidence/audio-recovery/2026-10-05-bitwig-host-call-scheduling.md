# Bitwig host-to-call scheduling observation

The one declared 180-second editor-closed acquisition **failed at the unchanged compressed recording cap**. Its retained prefix nevertheless supports independently checked attribution of the actual SDK caller's measured calls and intervening time to scheduled, runnable or sleeping states. This is a bounded observation, not a complete recording, audio pass or cause assignment.

Frozen timing1deck remains source `d67b4bd5`, engine `82a78f2e`, Windows host `c8587d8c`. Pure LoFi 1.0.0.6121 uses the saved Little Pleasures device/S1 four-note clip at 110 BPM, SameCallback, editor closed; Bitwig Flatpak 6.1 and PipeWire graph 512/48k. All 79,649 lifetime calls are sealed/export-confirmed with actual N=512, Fs=48000, SDK success and zero observation-loss/invalid counters. Actual caller identity matches the immutable wake filters and sampled census: OTHER0/nice0/affinity 0–7. Native transport and Windows render workers remain RR|RESET_ON_FORK5; environment-bound shared wineserver is OTHER0. No priority changed.

The raw source event span is 155.746834 seconds; post-machine-start attribution spans 155.746471 seconds. CLOCK_MONOTONIC attributes, zero namespace offsets, all eight online CPUs and unchanged sampled thread generations are verified. Within those bounds, 14,601 measured bodies and 14,600 arrival pairs are fully contained. Median is sorted[count/2]; p99 is sorted[floor((count−1)×0.99)], without interpolation.

| Observation | Median / p99 / max, ms | Scope |
| --- | --- | --- |
| Measured SDK call | 3.877 / 5.870 / 8.376 | Two calls >8 ms |
| Entry-to-next-entry | 10.666 / 11.037 / 13.475 | 35 exceed nominal N/Fs by >1 ms |
| Prior return-to-next-entry | 6.794 / 8.227 / 9.149 | Includes normal idle/host work and observer return tail |
| Caller wake-to-run | 0.004 / 0.122 / 2.881 | 34,102 intervals; 39 >1 ms: 29 inside bodies, 10 outside |
| Caller runnable switchout-to-run | 0.057 / 0.226 / 1.477 | 4,670 intervals; four >1 ms: two inside, two outside |

The 8.050 ms call contains 3.608 ms caller runnable time and 4.352 ms sleeping; the 8.376 ms call contains 0.033 ms runnable and 8.240 ms sleeping. The longest arrival is a 6.290 ms measured body plus 7.185 ms outside it; that outside interval is 7.118 ms sleeping and 0.012 ms runnable. Native/Windows worker wake-to-run maxima are 0.094/0.109 ms. These findings distinguish state composition; sleeping does not identify a blocking primitive, and scheduled residency is not DSP time. All covered calls are below nominal 512/48k cadence (10.667 ms), which is not an observed device deadline. Collector effects on these durations are unquantified; this is not a controlled regression comparison to prior untraced runs.

Four identical rendered non-target lines remain; none affects the three target streams. One 73.121 µs wake/R+ interval is ambiguous and excluded from runnable-latency metrics while retained in its call body. Initial/final edges are censored at actual events, not an ACK. Nonmember-only CPU switches are omitted from the derivative; emitter fields/names do not establish a waker's role or IPC peer. Trace observer effects remain: perf/helper CPU totals 14.396/2.504 seconds.

Whole PCM is finite/aligned at 166.485 seconds. Start-byte-to-EOF is 157.173 seconds without an exact stereo-zero span, but recorder ERR rises 0→2 and capture aborts; no continuity pass follows. Pre-roll has three 512-frame stereo-zero spans and 1,024 unilateral-zero frames. Post-marker sampled Bitwig ERR 6→7 is +1; two further increments straddle start uncertainty. Missing end-byte marker and uncalibrated output timing prohibit per-gap mapping.

Offline export conserves all 27,267,467 native-counted events and retains 9,311,437 lines with verified gzip integrity. Original full-text cap, inherited reader timeout and unavailable-emitter-header refusals stay retained. Corrected export completes under a separately reviewed finite 240-second/6 GiB/zero-swap allowance; this changes analysis capacity only. Raw hashes, resources and failed predecessors are in the [sanitized evidence](2026-10-05-bitwig-host-call-scheduling.json).

Normal stop/save/quit, owner retirement and Moonlight disconnect complete. Original/prior projects, licensed environment, selections/preferences and predecessors are preserved; healthy service/keepers remain and the candidate stays installed. Owned perf FDs, offline cgroup and backed-up temporary PCM buffer are retired. The unresolved boundary is the host's blocking/admission mechanism and intended wake schedule. No further run, priority/engine repair, causal, deadline, continuity, soak or beta qualification follows.
