# Refresh4deck physical SDK timing

The normal Deck update passed and preserved all eight commercial publications, preferences and predecessor artifacts. The declared 24-cell first-attempt timing matrix is complete: **8 full lifetimes pass; 16 fail**. This installed candidate fails full audio acceptance. Native real-time scheduling was not achieved.

[Exact sanitized evidence](2026-10-04-refresh4deck-physical-timing.json) retains identities, all-stage request histograms, output/state results, failure witnesses and durable archive hashes. Complete raw recordings and original failures are retained privately.

At 48 kHz, each row planned 4,000 unpaced fixed-size main callbacks, four initial N0 calls, the D+13-frame tail and final N0. Four output lanes were captured. Main median/p99/max below are complete SDK callback durations, including recorded failures. Median uses sorted[count//2]; p99 uses sorted[floor((count-1)*0.99)], without interpolation. The 34-call row is a truncated observation; its p99 is the second-largest observation. Local N/Fs comparisons do not establish a DAW graph deadline.

| Role | Delivery | N=M | Diagnostics | Recorded main | Main median / p99 / max (ms) | Full lifetime | State |
| --- | --- | ---: | --- | ---: | --- | --- | --- |
| instrument | SameCallback D0 | 256 | off | 4000 | 0.236265 / 0.389527 / 0.730074 | FAIL: 13 frame tail refusal | not reached |
| instrument | SameCallback D0 | 256 | on | 4000 | 0.361877 / 0.570991 / 0.931698 | FAIL: 13 frame tail refusal | not reached |
| instrument | SameCallback D0 | 128 | off | 4000 | 0.342185 / 0.558038 / 0.772352 | FAIL: 13 frame tail refusal | not reached |
| instrument | SameCallback D0 | 128 | on | 4000 | 0.345485 / 0.542159 / 0.943574 | FAIL: 13 frame tail refusal | not reached |
| instrument | SameCallback D0 | 64 | off | 4000 | 0.339046 / 0.521958 / 0.777892 | FAIL: 13 frame tail refusal | not reached |
| instrument | SameCallback D0 | 64 | on | 34 | 0.407747 / 0.679721 / 1.416452 | FAIL: main callback refusal | not reached |
| instrument | Buffered D256 | 256 | off | 4000 | 0.263126 / 0.478069 / 0.669523 | PASS | exact recall |
| instrument | Buffered D256 | 256 | on | 4000 | 0.264705 / 0.476800 / 1.015069 | FAIL: 13 frame tail output gap | not reached |
| instrument | Buffered D256 | 128 | off | 4000 | 0.264666 / 0.455459 / 0.865457 | PASS | exact recall |
| instrument | Buffered D256 | 128 | on | 4000 | 0.268764 / 0.469147 / 0.598699 | PASS | exact recall |
| instrument | Buffered D256 | 64 | off | 4000 | 0.272874 / 0.444916 / 0.968034 | FAIL: final zero frame refusal after correct audio | not reached |
| instrument | Buffered D256 | 64 | on | 4000 | 0.273574 / 0.505898 / 0.945134 | FAIL: final zero frame refusal after correct audio | not reached |
| effect | SameCallback D0 | 256 | off | 4000 | 0.335986 / 0.533720 / 0.918365 | FAIL: 13 frame tail refusal | not reached |
| effect | SameCallback D0 | 256 | on | 4000 | 0.345336 / 0.535149 / 1.509666 | PASS | exact recall |
| effect | SameCallback D0 | 128 | off | 4000 | 0.339656 / 0.528809 / 1.134540 | FAIL: 13 frame tail refusal | not reached |
| effect | SameCallback D0 | 128 | on | 4000 | 0.345146 / 0.542430 / 0.833305 | FAIL: 13 frame tail refusal | not reached |
| effect | SameCallback D0 | 64 | off | 4000 | 0.333166 / 0.530590 / 0.676162 | FAIL: 13 frame tail refusal | not reached |
| effect | SameCallback D0 | 64 | on | 4000 | 0.341416 / 0.526499 / 0.669522 | FAIL: 13 frame tail refusal | not reached |
| effect | Buffered D256 | 256 | off | 4000 | 0.255194 / 0.429837 / 0.926096 | PASS | exact recall |
| effect | Buffered D256 | 256 | on | 4000 | 0.256184 / 0.436488 / 0.643480 | PASS | exact recall |
| effect | Buffered D256 | 128 | off | 4000 | 0.264115 / 0.441977 / 0.997047 | PASS | exact recall |
| effect | Buffered D256 | 128 | on | 4000 | 0.266095 / 0.458098 / 0.650831 | PASS | exact recall |
| effect | Buffered D256 | 64 | off | 4000 | 0.269745 / 0.447038 / 0.714282 | FAIL: final zero frame refusal after correct audio | not reached |
| effect | Buffered D256 | 64 | on | 4000 | 0.271255 / 0.472958 / 1.319142 | FAIL: final zero frame refusal after correct audio | not reached |

All 23 completed main windows contain exact captured audio: 4,096,000, 2,048,000 or 1,024,000 sample values according to N. The truncated instrument D0/N64 ON row retains 34 main calls, including one failed call and its 256 unexpected zero values. Its matching phase witness records an unsatisfied completion predicate and a 1,378,162-ns wait against 1,333,333 ns. It does not attribute the Windows service/scheduler cause.

Thirteen-frame instrument tail losses contain 48 unexpected zeros among 52 returned values: the final four values were expected zeros. The effect tail losses contain 52 unexpected zeros. The Buffered instrument N256 ON tail gap was returned by a successful SDK call but failed the actual audio/event assertions. All four Buffered N64 lifetimes returned complete correct audio, then refused final N0; state capture was not reached. These are distinct observed failure outcomes.

Native workers remained SCHED_OTHER policy 0, priority 0. Their RTKit command exited 1 with acceptance/refusal unknown; the exact DBus reason is unavailable. Windows RR/reset-on-fork policy 1073741826, priority 5 was observed in 23 cells; the short failed instrument D0/N64 ON lifetime has no effective Windows readback. These observations do not prove scheduling continuity. No privilege or wait-budget changes were made.

ON request histograms describe observer-delivered requests, including request scopes beyond fixed main callbacks, and can omit failed/unpublished requests. Their quantiles are bucket upper bounds at eight buckets per octave; maxima are exact. Per-stage counts and dropped/unpublished/negative-residual counters are preserved in the JSON. Bounded phase traces omit late tail clocks in long lifetimes. OFF/ON differs on some first attempts; this is not causal evidence that diagnostics repaired or caused a failure. The first invocation also started its managed environment keeper; later invocations used that warm owner.

All 24 authenticated DSP owners retired with transport/cleanup confirmation. The service remains healthy, DSP and maintenance counts are zero, and the intended managed environment keeper remains. Commercial settings are unchanged. Both first-party fixtures remain on SameCallback D0 with remembered Buffered 256. Manager and stream are closed; builders and the capacity VM are stopped; Audiobookshelf remains running.

FC-AUDIO-001 remains open. The next bounded architectural target is the shared legal short/zero-frame completion and containment contract, with request/result/predicate attribution and a justified deadline policy. Attribution must distinguish unfinished rendering from completed work blocked by presentation/control ordering. Review one callback-entry time origin without resetting or stacking allowances; this does not make the observed N/Fs or 1 ms constants authoritative general policy. The deadline policy needs DAW/device justification. Retain the observed main N64 failure. Native scheduling failure-reason observability remains a gap. The current evidence does not authorize another source repair, allowance change, repeated run, recovery campaign, DAW/soak or beta promotion.
