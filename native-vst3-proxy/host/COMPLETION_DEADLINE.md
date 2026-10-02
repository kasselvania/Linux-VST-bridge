# Queued output completion deadline

The recovery5 Deck comparison reproduced a missing 512-frame Pure LoFi result
at the last callback before close. The next host callback arrived 1.34 ms after
admission; Windows processing took 1.94 ms, and publication followed the gap by
0.89 ms. No control operation occupied this request. The full captured recording
has no internal exact-zero span during its musical signal; this is a measured
delivery defect during shutdown, not a claim of an audible dropout in that run.

Protocol 14 keeps its existing delayed sample timeline, `D >= M`, output slots,
ordered returned events and epoch fencing. When output already due on that
timeline is absent, the callback may wait for native completion publication.
It never waits merely to fill future delay capacity or to finish the current
request when that request is not yet due.

The budget is `N / Fs`, measured from entry to the Rust processing call and
shared across all its subblocks. `N` is the actual host block and `Fs` comes from
accepted inactive setup. At 48 kHz/512 it is 10.667 ms; within current format
limits its maximum is 23.220 ms at 44.1 kHz/1024. Both currently accepted modes
(realtime and offline) use this finite budget. This is a local completion bound,
not a guarantee of the DAW's device deadline or unlimited offline processing.
Validation and copying consume that same elapsed-time allowance; the final
bounded copies and normal OS scheduling can extend observed callback duration.

The Linux callback waits on one preallocated native futex sequence. Publishing
the existing result queue advances that sequence and wakes its sole consumer.
Snapshot-before-inspection plus the futex comparison closes the lost-wake race.
Spurious wakes and interrupts retain the same absolute deadline. There is no
mutex, heap allocation, transport operation or callback logging. Linux resolves both imported functions
during inactive allocation, before a callback can enter the wait. Non-Linux
source tests use a bounded polling fallback; they do not qualify another OS.

Zero-frame and priming calls never wait. Pending control, terminal failure,
worker failure or cancellation prevents entry to the wait. If the budget ends
or the wait is unavailable, a final queue inspection precedes the existing
counted silence posture. Late audio still expires at its original position;
the callback neither moves it forward nor destroys the instance. The worker's
five-second transport containment timeout remains separate. State operations,
manager work, Windows render scheduling and runtime selection are unchanged.

Fixed callback counters retain wait attempts, waits ending without a result,
maximum successful Rust processing-call duration and a bounded histogram.
They export only after retirement. These counters are not complete C++/DAW
callback timings and do not establish an OS worst-case scheduling bound.

The causal regression uses a peer taking 2 ms per request while host callbacks
run without artificial pacing. The old path loses 7,680 of 8,192 frames; the
repair must deliver the exact delayed signal with zero callback allocations.
Separate cases exercise timeout silence, expiry, control exclusion, zero-frame
calls and publication racing with wait entry. Existing event, epoch, extra-output
and allocation cases remain required. Physical comparison and diagnostics-off
acceptance remain necessary. This does not implement the stage-4 `D = 0` path,
change the Windows mailbox wake mechanism, or qualify low blocks, reconfiguration
or the beta workflow.
