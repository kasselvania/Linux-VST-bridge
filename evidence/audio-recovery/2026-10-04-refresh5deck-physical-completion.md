# Refresh5deck physical completion comparison

**All 24 declared first-attempt lifetimes pass** complete captured audio, meaningful state recall and authenticated retirement on the physical Deck. The refreshed source repairs the approved ordered-completion contract at this first-party SDK scope. Native real-time scheduling and DAW/device timing remain unqualified.

[Exact sanitized evidence](2026-10-04-refresh5deck-physical-completion.json) records the frozen d970 source/artifacts, all per-call cadence audits, stage histograms, output/state/provenance and durable archive hashes. The original refresh4 failures remain retained.

The normal installer and one Update Bridge action selected refresh5 software/kit successfully, but **all 10 modern publications retained their old native engine and revision**. Only the two Completion fixtures were subsequently refreshed through offered rescan, reinspection, preparation and experimental replacement actions. All eight commercial registrations, preferences and current/predecessor artifacts stayed exact and retain their earlier engines. This remains an end-customer engine-update delivery gap under FC-MGMT-008; the fixture preparation does not close it.

The approved D-030 policy uses one originating native callback-entry absolute five-second AUDIO containment bound through Rust completion/presentation and final SDK sinks. Queued requests retain their original bounds, including predecessors. Requested expiry is not a guaranteed wall-clock return during descheduling or an unpreempted SDK sink. Seconds-long failure stalls were explicitly accepted; five seconds is not an audio performance or external deadline claim.

At 48kHz, each role/delivery/block/diagnostic cell ran 4,000 unpaced main callbacks, four initial N0 calls, a D+13-frame tail split at capacity, and final N0. Four output lanes produced **57,357,536 exact sample values**, 96,172 recorded callbacks, 96,192 returned events and 192,276 parameter points. All 24 captures/recalls passed with gain 0.625 and colour 0.125, exact component bytes and controller recall. The independent consumer, callback audit and full-output oracle were unchanged.

| Role | Delivery | N=M | Diagnostics | Main median / p99 / max (ms) | N/Fs exceedances main / all | Full lifetime | Refresh4 lifetime |
| --- | --- | ---: | --- | --- | ---: | --- | --- |
| instrument | SameCallback D0 | 256 | off | 0.236474 / 0.410647 / 1.954463 | 0 / 0 | PASS | FAIL |
| instrument | SameCallback D0 | 256 | on | 0.346336 / 0.565419 / 2.131227 | 0 / 1 | PASS | FAIL |
| instrument | SameCallback D0 | 128 | off | 0.312205 / 0.518659 / 1.172600 | 0 / 1 | PASS | FAIL |
| instrument | SameCallback D0 | 128 | on | 0.289355 / 0.476509 / 0.709642 | 0 / 0 | PASS | FAIL |
| instrument | SameCallback D0 | 64 | off | 0.233294 / 0.371227 / 0.599091 | 0 / 0 | PASS | FAIL |
| instrument | SameCallback D0 | 64 | on | 0.289435 / 0.490928 / 0.623931 | 0 / 1 | PASS | FAIL |
| instrument | Buffered D256 | 256 | off | 0.258955 / 0.440148 / 0.978237 | 0 / 0 | PASS | PASS |
| instrument | Buffered D256 | 256 | on | 0.176743 / 0.337886 / 0.500089 | 0 / 0 | PASS | FAIL |
| instrument | Buffered D256 | 128 | off | 0.178553 / 0.319955 / 0.674192 | 0 / 0 | PASS | PASS |
| instrument | Buffered D256 | 128 | on | 0.185283 / 0.326655 / 1.169230 | 0 / 0 | PASS | PASS |
| instrument | Buffered D256 | 64 | off | 0.193144 / 0.332816 / 1.139320 | 0 / 0 | PASS | FAIL |
| instrument | Buffered D256 | 64 | on | 0.187703 / 0.339286 / 0.891596 | 0 / 0 | PASS | FAIL |
| effect | SameCallback D0 | 256 | off | 0.257654 / 0.447468 / 0.770143 | 0 / 0 | PASS | FAIL |
| effect | SameCallback D0 | 256 | on | 0.320236 / 0.504779 / 0.807124 | 0 / 0 | PASS | PASS |
| effect | SameCallback D0 | 128 | off | 0.316926 / 0.491258 / 0.891846 | 0 / 0 | PASS | FAIL |
| effect | SameCallback D0 | 128 | on | 0.234454 / 0.379827 / 0.788774 | 0 / 1 | PASS | FAIL |
| effect | SameCallback D0 | 64 | off | 0.230595 / 0.369387 / 0.714423 | 0 / 0 | PASS | FAIL |
| effect | SameCallback D0 | 64 | on | 0.233164 / 0.361896 / 0.629321 | 0 / 0 | PASS | FAIL |
| effect | Buffered D256 | 256 | off | 0.173203 / 0.299995 / 0.522219 | 0 / 0 | PASS | PASS |
| effect | Buffered D256 | 256 | on | 0.178044 / 0.317205 / 0.750954 | 0 / 0 | PASS | PASS |
| effect | Buffered D256 | 128 | off | 0.178723 / 0.298425 / 0.650132 | 0 / 0 | PASS | PASS |
| effect | Buffered D256 | 128 | on | 0.177334 / 0.316436 / 0.641631 | 0 / 0 | PASS | PASS |
| effect | Buffered D256 | 64 | off | 0.183644 / 0.295736 / 0.618511 | 0 / 0 | PASS | FAIL |
| effect | Buffered D256 | 64 | on | 0.185623 / 0.336246 / 2.537056 | 2 / 2 | PASS | FAIL |

Every main window contains 4,000 calls. Median=sorted[count//2]; p99=sorted[((count−1)×99)//100], without interpolation. The six nonzero cadence exceedances comprise four N13 tails and two effect Buffered N64 ON main callbacks (1.413705 and 2.537056 ms). N0 has no N/Fs cadence. The independent integer test is elapsed_ns×Fs>N×1,000,000,000; it never uses the new containment allowance as a timing-pass threshold. No SDK call refused, and no retained raw SDK span exceeded five seconds; raw SDK spans include work outside the stored native origin and do not prove its expiry checks or a guaranteed return bound.

The refresh4 matrix had 8 passes / 16 failures, 92,193 recorded calls, 17 nonzero cadence exceedances (one main) and 15 SDK refusals. Both generations began instrument D0/N256 OFF cold, with 23 later cells using the managed warm keeper. These single first-attempt observations establish the declared complete-output comparison, not causal diagnostics, repeatability, intrinsic latency improvement or performance qualification. The prior main N64 failure, N13 output/refusal and final N0 failures remain independently retained.

All 24 native workers reported ordinary SCHED_OTHER policy 0/priority 0. RTKit commands exited 1 with acceptance/refusal and exact DBus reason unavailable. Windows RR/reset-on-fork policy 1073741826/priority 5 has effective readback in all 24 cells. Readback does not prove scheduling continuity. No scheduling policy was forced and no global privileges changed; only test consumers received the recorded RTTIME 200,000 µs soft/hard limit.

ON request histograms describe observer-delivered requests beyond fixed main callbacks, require process_ns, and may omit failed/unpublished work. Their quantiles are upper bucket bounds at eight buckets per octave; maxima are exact. Per-stage counts plus dropped/unpublished/negative-residual counters are retained. Bounded phase traces do not observe every tail. OFF/ON is a first-attempt comparison without causal attribution. Callback audit was enabled for all cells and reported zero forbidden callback effects.

The first Linux source attempt failed strict clippy on a redundant offline field; its exact logs remain. The successor then passed Linux 164/165 tests (two ignored each), strict registered clippy, 13 actual native SDK tests and both SDK oracle unit tests. The first package pairing refused the old Windows source manifest after AP23 advanced to ABI 2; rebuilding the exact frozen Windows source restored pairing without weakening the guard. Private helper syntax, historical reinspection and evidence-label refusals are retained separately; none was a failed audio invocation or erased/replayed lifetime.

All 24 test DSP owners and transports retired with cleanup confirmation. The final service is active, DSP/maintenance counts are zero, and the healthy product-owned inspection keeper remains. Both first-party fixtures finish on SameCallback D0 with remembered Buffered 256. Commercial preferences are unchanged. Manager and Moonlight closed normally, builders/capacity VM are stopped, and Audiobookshelf remains running. No automatic rollback was performed.

This completes the approved bounded first-party SDK correctness comparison. FC-AUDIO-001 remains open for actual scheduling/timing and the missing native RTKit reason; FC-MGMT-008 remains open for modern-publication engine-update delivery and deferred restoration. Commercial new-engine processing, real DAW/project/reboot, full recovery, soak, platform/catalogue coverage and beta remain unqualified.
