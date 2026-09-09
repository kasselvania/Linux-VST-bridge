# AP13 evidence map

Read [docs/AP13.md](../../docs/AP13.md) for the bounded claim, causal finding, matched settings, failures and operating recommendation. Numerical records are allowlisted exports of private Deck reports. They contain no vendor state bytes, presets, binaries, audio waveforms, prefix export or authorization material.

- `traced-baseline.json`: instrumented, unrepaired 512/512 path; first request 21607 blocked by GetState. Native monotonic and Windows QPC clocks remain separate.
- `state-timing-closed.json`: owner component/controller/editor timing to test the cause. It is diagnostic evidence, not an untraced performance comparison.
- `baseline2-closed.json`, `async-untraced-closed.json`: before/after 512/512 complete sessions; their whole-session totals include actions beyond the matched 45-second intervals.
- `async-traced-closed.json`: repaired production vendor trace. It retains startup/vendor and later gaps, and the editor attempt that produced no changed value.
- `host256-delay512-closed.json`, `host256-delay256-closed.json`: repaired comparison at actual host maximum 256. The lower setting is not promoted.
- `removal-retry-closed.json`: independent LoFi sibling continues after effect-only removal. The previous extra-delete attempt remains in the preceding session records.
- `baseline-closed.json`: initially observed accepted AP12 binaries. Its long whole-session totals are not attributed to its shorter CPU interval.
- `interval-snapshots.json`: original independent counter and CPU endpoint samples, stripped of private locations. Counter vectors may straddle one parent callback. PID/start time pairs disambiguate CPU identities.
- `interval-results.json`: differences computed from those endpoints. Each device's gap/missing/expired counts remain separate. Steady comparison intervals are `baseline2-play45`, `async-untraced-play45`, `host256-delay512-play45` and `host256-delay256-play45`.
- `local-audio.json`: finite/nonzero output metrics from the Deck's local speaker sink monitor; no physical latency or listening claim.
- `installed-selection.json`: live active refusal, inactive selection, restoration and bridge-only deployment digests.
- `state-recall.json`: native state hashes and actual save/reopen observations, without state payloads.
- `installation-final.json`: final installed artifact identity, publication, project preservation, service and trace status; also the independently observed late CPUWeight change.

The original reports are hashed before filtering. Full native final lifecycle rows are genuinely absent from two early heavy-trace groups because the old C++ report cap was smaller than Rust's. Independent owner counters are retained, not converted into fabricated final lifecycle success. `raw_exit=-15` on a clean owned session is the supervisor's positive cleanup; inspect `error`, `cleanup_confirmed` and `transport_retired` separately.

Detailed trace collection is bounded and can fill before a later action. No missing row is evidence of a fast call. Coarse Windows thread CPU ticks cannot distinguish all execution, blocking and runnable delay. Commercial sample equality fields are not treated as an independent waveform oracle.
