# Proposed short/zero-frame completion contract — pending operator decision

Base: `35bbcac4f3399d18d4de57f10ff2848fe4414d1d`. This is a proposal, not
accepted architecture or permission to change production behavior.

Replace the real-time/prefetch `N/Fs` and zero-frame 1-ms success cutoffs with
**one five-second outer completion/host-health containment ceiling, measured
from native C++ callback entry**. Five seconds comes from the existing AUDIO
owner reply containment bound, not from a measured audio device or DAW deadline.
The same bound covers admission, earlier ordered work/control dependencies,
current request service, reply validation/publication, presentation and final
SDK event/parameter delivery. Rust and C++ consume the same prepared policy and
absolute origin; no stage, notification, retry or predecessor grants more time.
Each admitted AUDIO carries its originating callback's absolute containment bound,
including Buffered work serviced after that callback has returned. Keep an earlier
predecessor's existing absolute transport bound; never restart it at entry to a
later callback. The worker may not give an operation a bound later than its
originating callback ceiling. Offline retains its separately
declared 60-second bound. Startup 10 seconds and control 20 seconds are not audio
allowances and cannot extend a processing call.

SameCallback D0 succeeds only after its own ordered ticket is fully validated,
owned, published, presented and delivered to the SDK sinks. Zero-frame calls in
either mode use this exact ticket, even at a repeated sample position; they drain
prior FIFO dependencies before their supported results can be returned and do
not advance sample time. Legal short calls are ordinary requests with their
actual frame extent, not fixture-specific flush cases.

Buffered nonzero calls retain D and the exact due presentation frontier. They
need the earlier work that supplies that frontier, rather than completion of
the current request. Pending control is not permission to return successful
missing audio or omit due events/points. The callback waits within the same
ceiling for the due frontier and permitted serialized control dependencies;
priming silence and future results remain distinct from missing due results.
If the due predicate cannot be met by the ceiling, retain the explicit failure
and silence/containment posture instead of accepting incomplete output. Do not
run state restoration concurrently with vendor DSP to satisfy that predicate.
The retained Buffered SDK-success tail gap and exact final-N0 debt therefore
remain in acceptance alongside D0 short-tail and main-N64 cases; a D0-only
change would not establish this shared contract.

Cancellation, authenticated owner death, dead peer, terminal containment,
invalid reply or stale identity refuses earlier. A healthy result arriving
after N/Fs or 1 ms but before the containment ceiling may succeed; that alone
does not establish timely audio. A result arriving after the single ceiling,
including SDK sink work crossing it, cannot authorize success. On failure,
wake and cancel the existing owners, preserve the first request/result/predicate
failure, and keep storage owned until native workers are joined and supervised
Windows process/transport retirement is positively confirmed. Vendor process()
remains noninterruptible by this callback; cancellation is not retirement proof.

**Consequential tradeoff:** the requested absolute containment expiry is five
seconds from callback entry, unless an earlier owner/transport bound or
death/cancellation ends it. A hung host can therefore impose a seconds-long DAW
thread wait. This is not a guaranteed five-second wall-clock return: operating
system descheduling or a host SDK sink call is not preempted by the expiry check;
success is refused when bridge control returns after expiry. This replaces both
architecture 18.4's N/Fs/1-ms expiry clause and its explicit prohibition on using
the worker five-second timeout as a synchronous callback wait. It is a deliberate
behavioral amendment requiring operator decision. It is not a real-time deadline
policy, acceptable DAW stall target, device-underrun claim or release qualification.
The remaining external device-period/serial-graph budget is unobserved; no value
is inferred from N, M, D, ordinary scheduling, this test or fixture/plugin name.

Minimal implementation after acceptance: share the callback containment policy
and its final SDK check through the Rust/C ABI; bound current worker work by that
origin without renewing predecessor deadlines; preserve ticket/frontier separation
and remove the pending-control missing-result success bypass; retain bounded
request/result/predicate attribution through existing owners. No runtime,
graphics, privilege, epoch, identity, state schema or separate recovery engine
changes are proposed.

Acceptance: real IPC15 worker tests distinguish withheld render reply from
validated native reply awaiting publication; exact FIFO N0 and repeated-position
parameter/event results; Buffered due frontier with serialized control; mixed
short/full/N0 calls; death/cancellation, stale epoch, late publication and SDK
sink crossing the same bound. Then freeze a reviewed paired artifact and perform
the declared first-attempt installed physical comparison with diagnostics OFF
and separately ON, full captured audio/state and exact retirement. Retain all
prior failures and new first attempts. Timing distributions, native RT gap and
ordinary scheduling remain reported independently of full-output acceptance.
