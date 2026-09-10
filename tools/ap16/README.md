# Bounded development observation

`thread-sample-v3.rs` is the exact external sampler source used for the AP16 causal baseline. It is not installed software and is never called by an audio callback. It reads two exact native worker identities and two existing AP12 status mappings, samples for at most 300 seconds, and retains at most 128 predecessor + 64 follow-up rows around the first missing-frame increment. It does no periodic output/file persistence.

The argument order is `PID TID_A START_TICKS_A TID_B START_TICKS_B STATUS_A STATUS_B`. Thread order and status order are independent and must be recorded explicitly. `state="?"` means that stat/wchan details were not sampled on that row; its zero minor/major fields are not observed zero fault counts. A zero status row means a bounded atomic read did not yield a stable row. Rows outside the ring are unretained; the baseline record gives total samples and overwritten predecessor count. No output payload or opaque state is read.

The companion perf command used only the two recorded TIDs:

```
perf record --clockid mono -c 1 -m 64 -e minor-faults:u \
  --call-graph dwarf,4096 -t TID_A,TID_B -o OWNED_RAM_CAPTURE -- sleep 305
```

Stop perf after the bounded sampler completes and retain sanitized event/identity information, not raw stacks or the capture. The recorded sampler binary SHA-256 was `3bc65b160ac7f32d0c8b06594cbe85bece97381f1766ba167d2024c61b29ad09`. [Evidence](../../evidence/ap16/causal-baseline.json) states the clock brackets, observed cost and limits. This helper is observation only, with no production authority and no hard real-time claim.
