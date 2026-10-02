# Native worker scheduling preparation

The physical recovery4 run retained two missing presentation blocks before the
native worker consumed their requests. An external scheduler capture contains a
23.273160 ms preemption of one native transport worker across both deadlines.
The first collector did not retain its namespace mapping, so it cannot prove the
numeric per-session TID association. A separate intervention on both exactly
mapped workers accumulated no further bridge gaps for ten minutes. Its startup
gap and unexplained speaker-monitor silence remain failures, not an acceptance
pass.

The worker now requests the same RR priority 5 capability as the existing Windows
render-thread helper. This runs once on worker entry, before it handles setup,
state or processing. It does not run in `process` or `setProcessing`. Existing
policies and all DAW process limits are preserved. Denial, unavailable capability,
bad identity, timeout or ineffective readback are recorded as unavailable; they
do not manufacture scheduling success or change audio error handling.

The already authenticated owner supervisor supplies the host PID and process
start identity from its retained socket peer context. The native worker supplies
its own namespace PID, namespace TID and thread start ticks. The Rust helper
requires that exact peer, the session's mapped status inode, matching namespace
IDs, expected worker name and unchanged thread generation. It never searches
unrelated processes or chooses among identical thread names by order. RealtimeKit
remains the permission authority. The native DAW must already have a nonzero
hard RTTIME budget of at most 200,000 microseconds; the helper never sets a
native process limit. The existing owned Windows-process policy is unchanged.

This is a versioned, single-request preparation extension in the existing private
session directory, independent of the audio protocol and retirement socket:

| File | Exact bytes |
| --- | --- |
| `native-scheduling.supported` | `LVNS`, little-endian u32 version 1, 16 session bytes (24 bytes total) |
| `native-scheduling.request` | same header; little-endian u32 namespace PID, u32 namespace TID, u64 thread start ticks (40 bytes) |
| `native-scheduling.reply` | same header; little-endian u32 result: 1 effective, 2 already effective, 3 unavailable (28 bytes) |

Files are private and published by rename. A missing or incompatible advertisement
means no request and no wait, preserving older supervisor compatibility. The
native preparation waits at most three seconds for the reply, then independently
reads its effective scheduling policy. The supervisor submits at most one native
request per session to the existing bounded Rust helper. Its helper process and
bus call retain their existing timeouts. Session retirement removes these files
with the existing transport directory; no second lifetime authority is added.

The earlier direct Flatpak Realtime portal probe failed on this Deck's namespace
mapping. It is retained privately as a failed experiment and is not a production
fallback. No privileges, sandbox permissions, kernel settings, runtime version,
buffer size or thread affinity are changed by this repair. Scheduling is not a
continuity guarantee; the installed full-lifetime comparison remains necessary.
