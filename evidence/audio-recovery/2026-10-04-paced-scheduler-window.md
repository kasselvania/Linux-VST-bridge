# Paced Bitwig scheduler observation on the frozen engine

The machine-defined scheduler observation completed **180.036106856 seconds**.
Commercial continuity remains **unqualified**: the paired written-byte window
contains nine both-channel zero spans of 512 frames, Bitwig ERR increases by eight
inside sampled observations or nine across enclosing observations, and recorder
ERR increases by one. Sampled bridge gap/missing/expired counters remain zero.
Commercial DSP is opaque, so the zero spans are observations rather than an
independent dropout oracle or proof of bridge origin.

The [sanitized evidence and retained hashes](2026-10-04-paced-scheduler-window.json)
bind source d970eda9, engine add49d7a, paired host c8587d8c and Pure LoFi
1.0.0.6121. The same one-instance S1 four-note clip at 110 BPM and Little Pleasures
editor were visibly playing before collection began. GUI response time was removed
from acquisition: the enable acknowledgement establishes a machine start, and
disable/collector retirement and paired recorder stop occur automatically. Later
normal GUI stop at displayed 4:34.879 is outside the captured lifecycle. Neither
onset nor a whole session or user interaction is claimed.

The complete output retains 212.181333 seconds, including pre/post-roll. The
written-byte window represents 179.925333 seconds; its 110.774 ms difference from
the monotonic observation is observed and unresolved. Buffering/capture latency
is uncalibrated, which limits interpretation without proving that difference's
cause. Pre-roll has
two additional 512-frame stereo-zero spans and 1,024 unilateral-zero frames;
post-roll has none. Full output is finite and retained. Graph 512/48k, negotiated
VST3 maximum 512 and actual per-call N/device deadline remain distinct; the latter two are unobserved.

The collector uses isolated owned perf event FDs/rings, rather than a tracefs
instance. It retains unfiltered switches on eight eligible CPUs and exact-worker
wake/waking/migration events, with monotonic timestamps and live Zstd level 1.
The 118,520,170-byte binary stays below the unchanged 256 MiB live cap. Perf used
6.619673 CPU seconds (about 3.68% of one core); helper CPU was 0.557757 seconds.
This measures collector cost, not isolated bridge overhead or proof of negligible
perturbation. Both authenticated workers retain sampled RR|RESET_ON_FORK5,
nice 0, affinity 0–7 and RTTIME 200000 µs. Thread membership does not establish a
functional role from its name.

Offline decoding occurs after normal retirement. The native reader's compressed
block mapping behavior requires a finite 132,434,108,416-byte virtual allowance,
separate from an enforced 4 GiB physical/zero-swap cap and 120 second transient
service bound. Its 63.397 second audit peaks at 3,157,266,432 bytes without memory
events. The separately authorized 512 MiB offline derivative allowance retains
342,571,550 compressed bytes; it does not enlarge the live cap. Raw data stays exact.
Intentional SIGINT exit -2 follows disable acknowledgement and native finalization;
exit status alone is not completeness proof. Native record accounting reconciles
17,136,165 records and 16,990,926 four-event samples with no recorded loss/throttle
types; the independent whole-stream audit verifies CRC and sample conservation.
Ten identical rendered nonworker switch lines and corresponding CPU-chain anomalies are
retained with their origin unassigned. They do not touch either target worker.
The decoded event span is 180.002992 seconds; the last event precedes the end
acknowledgement by 33.572825 ms. The acknowledgement tail is not event coverage,
and censored worker states receive no manufactured duration through it.

Native wake-to-run has median 4.320 µs, p99 13.901 µs and maximum 1.394097 ms
(one interval above 1 ms). Windows has median 4.231 µs, p99 14.911 µs and
maximum 108.932 µs (none above 1 ms). Runnable switch-out maxima are 6.980 µs
native and 42.711 µs Windows. Sleeping/blocked maxima around 4.05 ms and Windows
scheduled-residency maximum 5.748192 ms are neither runnable delay nor DSP time.
These are scheduler-event intervals, not callback latency or isolated bridge
overhead, and the worker event counts do not count audio callbacks. Reported
quantiles use nearest rank. Censored
boundaries and ambiguous CPU context remain explicit in the record. In the single
1.394097 ms native wake-to-run interval, an unassigned priority 120 task remained
scheduled on requested/resumed CPU 2 until an R+ switch to the native worker at
kernel priority 94. Its task role, interrupt contribution and causal obstruction
are unproved; it is not identified as recorder/helper or a target-process member.

The next bounded observation target is the actual callback/output-to-graph
boundary: bind callback/request identity, output/sample position and graph error
witnesses to a common clock with stated capture latency. Current byte snapshots
cannot establish that alignment. This result supports no blanket priority repair
or per-zero-span cause; no additional run or source change follows here.

Every earlier failure remains: first text collection exhausted its cap while
arming; the first binary invocation failed optional-argument parsing; the next
binary run timed out awaiting GUI coordination with no playback. Its failed
256 MiB/1 GiB reader attempts and original strict field-warning failure remain
separate from later complete sample reconciliation. Minor post-audit archive
packaging refusals are retained; neither audio nor native decoding was repeated.

Normal save/quit/disconnect and authenticated native/Windows retirement pass.
DSP/maintenance are zero; the healthy managed keeper remains. Exact software,
selections/preferences/predecessors, nine unselected published siblings and the
original project are preserved. Frozen audio source, priority, runtime and global
tracing remain unchanged. Scheduling causation, actual device deadlines,
thirty-minute interaction, soak, beta and engine-update UX are not qualified.
