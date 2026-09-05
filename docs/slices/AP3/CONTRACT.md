# AP3 — Sustained AGain preview

Implements the selected behavior in CURRENT_SLICE.md from base
5890e862fc91a9fec96a142cbc72916849b19b12. PRODUCT_CONTRACT_CHANGE is
explicitly authorized by AGENTS.md. AP2 remains accepted pending review.

The existing SDK edge, one Rust session, one file mapping, loopback control,
Windows AGain instance and owned supervisor are retained. The new
AGainQueuedBridge bundle exposes the existing reference processor identity
and bridge-owned gain controller D1444DE338814391A916DC9CFCC67008. The
AGainOfflineBridge bundle and protocol minors 1/2 retain their earlier modes.
Only float32 stereo at 48 kHz with 1..256 frames is supported for realtime
preview. Actual Windows ProcessSetup and ProcessData use kRealtime.

## Callback storage and ownership

Two native Rust SPSC queues contain 2048 descriptors each. Each descriptor
owns two fixed 256-float planes plus kind, extent, gain, input/output silence,
epoch and sample position. Allocation occurs during activation; the queues
consume 8,552,448 bytes (8.15625 MiB) together. The callback owns request publication and
result consumption; only the transport worker consumes requests/publishes
results. Acquire/release atomics transfer queue ownership. Native atomics
never cross the C ABI or process boundary. A try-only atomic registry guard
refuses overlapping API ownership; it never spins or waits. Worker owns no
DAW pointer. The callback snapshots in-place inputs before writing outputs.

Fixed additional latency is 1024 samples (21.333 ms at 48 kHz), reported via
VST3. Only startup positions below 1024 are intentional zeros. Thereafter
position s must receive actual Windows output for input s-1024, independent
of callback length. Missing due output latches failure and clears validated
output spans. Capacity overflow, stale epoch/position/sequence, malformed
silence claims, nonfinite output and dead/timed-out worker also latch failure.
No replay, reconnect, local gain replacement or acceptance of fault silence.

Minor 3 extends the existing wire messages: Activate adds the realtime mode;
Start/Stop and acknowledgements carry an eight-byte epoch; Process appends
epoch/sample position to its existing 32 bytes; Done appends both to its
existing 16 bytes. Existing mapping and token/sequence ownership are unchanged.
The transport worker serializes one outstanding mapping use, with the existing
bounded control I/O. Host callbacks use only preallocated copies and atomics.

Start/stop are ordered, nonblocking callback operations. A new processing
interval increments epoch and resets the delayed timeline, while retaining
logical gain in the same instance. Stop discards delivery of old results and
drains previously dispatched work before its acknowledgement. Deactivation
waits outside the audio thread; close joins the worker before freeing storage.
Failure teardown remains bounded by the existing transport and supervisor.

## Verification and claim ceiling

Local tests load the actual SDK bundle with a declared substituted peer and
instrument actual process/setProcessing calls for allocation/free, waits,
socket/file calls and logging. They exercise variable lengths, delayed gain,
zero gain, one-channel/all silence, in-place buffers, queue wrap, restarts,
stale/malformed/dead peer responses and unload. These tests are never live
AGain evidence. The finite queue unit test additionally checks overflow.

Fresh device measurements use host-paced 30 seconds at 128 frames and 30
seconds at 256 frames, plus delayed-tail silence. The SDK host selects inputs
after activation and independently compares every returned sample at the
reported delay. Compact counts, FNV-1a input/output digests, maximum error,
callback median/p99/max, overruns and queue/fault counters are retained.
Acceptance requires exact numerical output and zero unexpected callback
misses, underflows or overflows for these measured streams. The same build
must also pass controlled disposable Bitwig playback; desktop streaming is
an operational aid, not an audio/timing oracle. Primary timing runs have no
active Moonlight stream; stream-active interference is a separate observation.

No commercial plug-in, vendor editor, state/preset fidelity, arbitrary DAW or
machine support, multiple instances or universal realtime guarantee is claimed.
Existing protected evidence/installations remain unchanged. Diagnostics stay
acceptance-ineligible. Budgets are six Windows producers, ten diagnostics in
one AP3 campaign, two fresh acceptance candidates; no historical count resets.

## Controlled DAW batch

The completed plan uses four serial supervised Windows instances: primary SDK
streams, a separately confirmed Moonlight-active short stream, and two fresh
Bitwig sessions opening only the prepared disposable project. Every stage reuses
the existing companion/supervisor, exact mapping and owned cleanup. The companion
records descendants while the Windows supervisor runs, and preserves the previous
segment before moving its closed staging files. It never retries a failed segment.
An ordinary bounded file confirms the actual stream state before the short test;
agent-written, reservation-bound GUI notes describe only visible actions. They
cannot supply numerical/timing values or turn a failed native/Windows result into
success. The native shell optionally writes at most 8 KiB of termination-time
facts into the owned session directory because a DAW may redirect stdout. No
report writing occurs in processing or setProcessing. The native report includes
actual setup, frames, gain range, zero-gain blocks, queue fault and clean teardown.
The temporary plug-in publication must byte-match the admitted native artifact;
Bitwig uses its existing installation and a single exact per-launch stage mount.
Original preference bytes remain privately backed up until test settings are
restored. Old diagnostic observations remain permanently acceptance-ineligible.
