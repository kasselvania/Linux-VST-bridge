# AP12 unfinished delivery status

This is fault visibility, not a repair claim for the intermittent LoFi timeout.
The existing five-second reply deadline and 512-frame bridge delay are unchanged.

`ap12.status` is an independently versioned, 1024-byte, session-private mapping.
The non-callback native session creates its immutable `LVFS`, version 1, extent,
and 16-byte session header before publishing `ap1.control`. Older diagnostic
clients without this extension remain explicitly unavailable to the observer.
Registered current sessions create it without the heavy-trace flag.

Three single-writer lanes start at offsets 64, 384 and 704: native transport,
Windows delivery and Windows UI owner. Each lane has a 64-bit publication counter
and two 128-byte slots at lane+64 and lane+192. Slots contain ten atomic u64
fields: generation, epoch, request sequence, stream position, stage, detail,
clock ticks, frequency, thread ID, process ID.
The owner also atomically reads the existing mailbox request/reply flags (two
independent samples, not a simultaneous transaction). Thus a writer interrupted
between flag publication and its diagnostic update remains distinguishable. The Windows lanes copy a stable
native generation when available; zero means not yet published, not generation 1.
The random session and epoch distinguish endpoint replacement and processing
intervals. Owner activity has no invented per-request identity; correlate the
independently sampled lanes using their session and observation time.

Writers fill the inactive slot with sequentially consistent atomic stores, then
publish its counter. Every shared word is atomic on both sides: Rust AtomicU64,
Windows Interlocked, Linux libatomic. A reader checks the counter before and after
loading the slot and makes at most three attempts. A process stopped mid-write
leaves the previously published slot intact; a raced read is marked unavailable
or explicitly stale. No ordinary mutable C++ trace row is read across threads.
Counter wrap is outside practical lifetime (u64 publications); native overflow
fails explicitly. Supported transport target is x86-64 Linux/Windows.

Native stages: 1 preparing publication, 2 request sent/waiting for reply,
3 reply received (protocol validation follows). Windows delivery stages:
1 waiting for a request, 2 consumed/decoding request, 3 vendor process entered,
4 vendor process returned, 5 publishing response, 6 response published,
7 waiting for owner state service, 24 lifecycle call region. Stage 1 keeps the
last consumed identity; the native lane identifies a newer unconsumed request.
UI owner stages: 20 initialized, 21 controller parameter update (detail is ID),
22 editor service, 23 state operation (detail is protocol kind), 24 lifecycle or
trait query, 25 Windows message pump. Lifecycle call detail 1..5 denotes setup,
activate, start processing, stop processing, deactivate. Scoped owner activity
restores its caller after a successful return; an exception retains the failing
stage. UI/state records contain no parameter values, opaque state, audio or text.

Native clocks are Linux CLOCK_MONOTONIC nanoseconds (thread ID zero is unavailable).
The native PID is in the DAW's PID namespace, which can differ from the owner's.
Windows clocks are QPC with their declared frequency and Windows thread/process
IDs. Never subtract between those domains. The supervisor records its own Linux
monotonic observation time and actual Linux PID/start identities for containment.

The installed supervisor polls only the native lane in ordinary operation. A
request observed pending for one second causes one bounded early snapshot. With
the existing heavy-trace opt-in, it also samples at most 128 owned thread
stat/wait-channel records. This is a pending observation,
not a terminal failure declaration. It neither interrupts the worker nor alters
its deadline. Before any containment, the independent supervisor captures all
lanes again and attempts an atomic `.fault.json` write outside disposable session
files. The rich outcome also retains the snapshot. Reporting errors cannot skip
physical cleanup, native release or independent sibling ownership. Detailed
completed histories remain opt-in; successful close exports them before the
terminal scanner record rather than relying solely on a later destructor.

Fault snapshots also retain the existing UI header's independently atomic
open/close/failure, view teardown stage and exception code/instruction address.
This reads no parameter, event or state payload. It distinguishes an unfinished
editor SDK operation from an audio computation stall even if the host disappears
without a terminal report. The UI mapping is validated independently and its
absence is explicit. These scalars are not claimed to be one atomic transaction.

Coverage: the native regression executes production Session::process_events and
Mailbox::receive with the first silent request deliberately unconsumed. It keeps
the five-second failure and original identity without musical arming. The Windows
SDK producer test blocks a child at each of six delivery boundaries, including
inside a real SDK process override; its independent parent reads the production
atomic publisher before and after forced containment, without a shutdown dump.
Linux supervisor tests stop a peer at all six stages, check persistence before
kill and after transport-directory removal, bounded racing reads, identity
rejection, reporting failure and healthy-sibling cleanup. These are deliberate
fault instrumentation tests, not evidence that a commercial stall was repaired.
