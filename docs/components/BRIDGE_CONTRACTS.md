# Retained bridge contracts

These active invariants accompany the governing [architecture](../ARCHITECTURE.md).
They preserve the SDK, transport, custody, state and transition contracts while
runtime delivery and the control plane change. This document is not a support
claim or permission catalogue. Source/tests below define current wire layouts
and implemented coverage; an unsupported required capability is reported specifically.
Historical per-campaign selections are not a new execution authority.

## SDK and language boundary

The native proxy exposes Linux VST3 interfaces; the Windows host loads the exact
Windows module/class. Retain factory metadata, original class IDs/categories,
processor/controller linkage, interface query behavior and reference ownership.
Friendly names never regenerate class identity. Discovery, descriptor, native
exposure and Windows implementation must agree; widening a parser or returning
a placeholder interface does not implement an SDK contract.

C++ stays at the SDK/Win32 edges. The Rust seam is versioned `extern "C"` with
fixed-width values, opaque handles, explicitly laid-out plain structs and bounded
borrowed buffers. Ownership, lifetime, release function and thread affinity are
explicit. No STL/Rust-owned layout or exceptions cross the boundary. Process
protocol versions, SDK ABI, admission and diagnostic formats are independent axes.

Supported adapters preserve connection points, parameter IDs/conversions, restart
notifications, bus activation/arrangements, latency/tail changes, component/controller
state order and bounded payloads. Calls may reenter through host callbacks; preserve
direction, correlation, bounded dispatch and thread affinity instead of assuming
strict request/reply alternation. Vendor DSP, main/GUI and state calls retain their
SDK threading/exclusion requirements. Do not make state restore concurrent with DSP
to hide contention. Editor refusal is distinct from corruption of an audio owner;
a shared-process crash cannot promise independent editor containment.

## Prepared processing and transport

The DAW owns sample rate, maximum/actual block lengths, precision, process mode,
transport and delay compensation. Prepare capacities while inactive: maximum M,
actual 0 <= N <= M, channel/bus/event capacity, delivery mode, bridge delay D and
vendor latency L. N is a call extent, not a new configuration. Report D+L and retain
the safe processor/controller latency notification route. Unsupported formats fail
explicitly rather than being substituted. Large-block adaptation preserves the
whole host call, context, event offsets, parameter curves and returned results.

Keep setProcessing lightweight, including on an audio thread. Setup/restore and
launch/storage work stay outside the callback. Legal stop/deactivate/setup/activate/
start transitions retire old work and prepare a new exclusive sample lease and
stream epoch. Real-time/prefetch may alternate per call where supported; crossing
offline follows inactive setup. Every call has its own completion ticket, including
successive zero-frame calls at the same position.

Audio memory is sized/mapped/prefaulted outside the callback, versioned and bounded;
validated offsets never become raw cross-process pointers. Preserve session, epoch,
sequence, position, extent and output ownership checks. Notifications are wake hints,
not execution authority: missed/raced hints must not lose mailbox work. Validate
message kind/version/length before allocation or lifecycle effects. Reject hostile
lengths, stale sessions and incompatible pairings; no untyped invocation escape hatch.

SameCallback uses an exclusive prepared AUDIO owner from publication through the
owned reply/sample copy. Actual Windows render work is the dependency, not a native
relay, mutable control Session, state store or notification socket. Shared Linux
futex words are not private futexes. Timeout/cancel does not return host-owned storage
to reuse; joins or proven supervisor containment precede releasing mappings/helpers.
START/epoch readiness precedes direct AUDIO. STOP is an owned bounded transition,
not a render-thread control-socket read. Peer death/cancellation wakes both direct
waiters independently of state parsing, retaining the first fault.

Buffered delivery retains D>=M, explicit FIFO/presentation ownership and D+M sample
history, including complete extra output planes. Partial/variable calls cannot be
counted as full blocks. SameCallback has D=0 and returns the current call's result;
neither is a zero-overhead claim. Keep returned events/parameter packets, capacities
and sink-overflow failures; no dropping due output while reporting complete success.
Preserve sample offsets, note IDs, deterministic ordering, parameter queues and
input/output changes. The existing negative-timestamp note-off recovery releases
its exact voice at offset zero in a nonempty call; it does not admit other invalid
timestamps or weaken wire validation.

## Completion, startup and observations

No post-activation allocation, file/network/process work, ordinary logging,
unbounded lock/wait or manager/editor dependency enters the callback. Preserve one
originating absolute expiry across queue debt, vendor rendering, reply validation,
presentation and final SDK sinks. No stage renews or stacks it; reject expired work
before publication and late results before success. Preserve the mandatory final
SDK finish check, first failure and explicit mode-specific result. Complete due
output and named terminal-failure silence remain different outcomes.

Current AUDIO host-health containment is five seconds from callback entry;
offline has one 60-second absolute bound and cooperative cancellation checks no
farther than 4 ms apart. Those are containment contracts, not acceptable audible
deadlines or scheduling guarantees. Vendor process/sink calls are not made
interruptible by a timeout. N/Fs, a bridge counter or a short isolated call cannot
prove the device/serial-graph deadline. Whole DAW-call timing and actual captured
audio establish advertised workload limits; death/cancellation can fail earlier.

Retain latency-aligned dry startup for admitted effects and silence for instruments,
the 50 ms sample-counted handoff after complete due wet output/vendor priming,
and no dry/fade in offline rendering. Editor visibility is not readiness. Preserve
ordered events/results, queue bounds and original expiries through startup/fade;
silence may be valid wet output. Retained parameter values use bounded zero-frame
synchronization without advancing sample position or replaying expired notes.
Offline synchronization shares the first render call's existing bound and adds no
host calls/tail blocks. A save is fresh only after its required synchronization
proof, including the existing epoch-bound post-STOP path.

Observations do not change processing policy. Whole-call observation uses prepared
finite storage, records complete SDK entry/exit and validates caller identity off
the callback. Drops, unfinished writers, saturation/overflow or identity/clock
failures refuse a complete-observation claim. Export/serialization happens after
quiescence. Scheduling assessment reports effective native/render-thread policy;
requested priority alone is not proof, and observing cannot silently enable a policy.

## State and project continuity

Keep stable DAW classes/automation IDs, opaque component/controller state and content
references. Exact recovery captures validate their producer digest. A DAW project
restore validates saved format/integrity/bounds/logical class, then offers unchanged
opaque state to the explicitly selected, independently admitted implementation.
The saved object never chooses an executable/runtime/publication. Only the selected
vendor implementation migrates its state; preserve the input saved object and error.

After restoration, use current authoritative vendor readback against the selected
descriptor. Historical parameter mirrors cannot invent inventory, values or
positional mappings; defer readback until connection where required. Changed IDs
are a separate automation-compatibility claim. Preserve owner-thread capture,
callback progress/barriers and freshness: later audio/parameter edits cannot reuse
an older capture. Parsing/store confirmation cannot gate prepared AUDIO replies.
Deterministic fixtures require byte-level round trips; vendor/project behavior
requires actual save, full exit and reopen tests. Failed/partial restoration is
not successful defaults; retire safely and offer a real retained predecessor.

## Control readback and observation validity

Ordinary readback validates bounded owned records, current bindings/transactions
and selected physical publication. It is not executable verification: do not walk
bulk runtime trees, hash payloads or execute preparation tools to display a product
or validate an offered choice. Offer admission rechecks its exact token, predecessor,
settings and affected owners. Queued worker executables are freshly verified;
mutation/launch owners retain deep payload verification and final identity checks.
A readable record or old successful observation never grants execution authority.

Incomplete retained history stays readable as needing recovery. A scoped operation
rechecks the selected binding and invokes its history owner; projection never
migrates records, invents predecessor lineage or repairs corrupt history. Preserve
independently valid publication/rollback facts and original operation bytes.

Invalidate observations by actual inputs: module changes affect inspection;
runtime/dependency changes affect compatibility; driver/display/host changes affect
consuming-context checks. Keep unchanged facts reusable. Support prose, observation
refresh or a preference change does not justify an engine rebuild/prefix recreation.
Capability snapshots are not licensed machine identity. Absent live DAW format
remains unknown, rather than guessed from PipeWire or device defaults.

Requested graphics/probes do not prove an editor's renderer. Retain the editor's
loaded-library fingerprint before independent probes and teardown. Attribute only
libraries newly loaded with that editor or imported by its module; a stand-in
adapter name is not the rendering device. Preserve other composed DLL choices
when validating the requested graphics assignments. A close failure may preserve
useful observations but remains a separate failed/uncertain editor retirement.

## Loaded-native admission and update continuity

Descriptor C ABI v2 binds an anchor in the executing factory to its opened image
and reads the descriptor through that same resolved directory handle. The factory
caches its engine/descriptor digest pair. LVB5 carries that pair with logical class
and vendor-module identity; the manager checks selected registration and the peer's
executable mapping before starting resources and again before admission. Retain
one authenticated peer process generation across those checks and graphical-context
capture. Loader provenance uses the retained opened-file identity, not map presence
alone. Derive mapping coordinates from a pinned file mapping where stat devices
differ. Recovery uses the original caller binding, never a fresh hash of the path
that happens to be selected. These non-RT contracts do not change Windows IPC15.

LVB1–4 cannot prove cached descriptor provenance and cannot execute under the
repaired manager. Retained codecs/files are compatibility history, not permission
to weaken admission. [D-029](../DECISION_REGISTER.md#d-029--managed-refresh-of-pre-contract-native-publications)
requires normal Update Bridge to prepare/verify affected selected publications
before switching, with a DAW-close request when needed. Failure preserves prior
selection and exact vendor/user state. Restoring a vendor/configuration predecessor
prepares a compatible bridge pair; full package restoration restores coherent
manager/publication components. Never fabricate lineage, silently use old proxy
admission or substitute a missing vendor build. Extend the existing transition
and history owners. Remaining installed update/recall, failed/interrupted recovery
and predecessor-restoration acceptance stays explicit under FC-MGMT-008 in the
[failure classes](../FAILURE_CLASSES.md); source binding alone does not deliver it.

## Custody, cleanup and admission

Authenticate the final Windows host, not an assumed pre-exec launcher. Existing
kernel-authenticated writer credentials, pinned PID/start/process handle and exact
mapped session identity establish custody independently of vendor/audio readiness.
Recheck render-generation binding. Claimed PIDs/names, a keeper sibling or diagnostic
line cannot grant ownership. Close custody admission before retirement; diagnostic
draining cannot expand the ownership set afterward. Missing/ambiguous identity
remains cleanup uncertainty; never delete its record to make recovery look successful.

Retire only exact owned generations/groups and their transport, preserving siblings,
keepers and unrelated services. Local command-client and remote Windows-family
custody are different facts. Existing shared-runtime marker/tool digests, endpoint,
nonce, graphical-context and keeper checks remain; missing custody never falls back
to another runtime or guesses a process group. Pre-ready failure must be containable
without pretending it ever became audio-ready.

A live native consumer owes the independent retirement acknowledgement R even after
an F failure notice, within the original deadline. Unknown bytes, EOF and timeout
do not prove that handshake. If the authenticated native generation ended, exact
Windows cleanup and transport retirement establish cleanup without requiring an
acknowledgement from the dead process. Cleanup never changes the original failed
audio/state result. Kernel-generation-ended recovery retains its actual evidence;
manual ownership deletion is never a recovery method.

Admission checks exact applied/publication/artifact identity, current process
generation, shared resources and structural capacity. Unknown support history is
not a permission failure. An idle/missing consumer observation is unknown, not an
invented failure. Maintenance retires actual affected owners; independent instances
are not global collateral. Preserve current broader installer/scanner/runtime
exclusion until narrower resource contracts are implemented and proven.

## Publication and revision transitions

Keep immutable publication/predecessor history and stable native class identity.
Changes bind an expected current revision, affected users and a real successor;
changed module bytes require observation/refresh, not execution under an old receipt.
Install intent may authorize ordinary discovery/preparation/publication after
required vendor steps, but never supplies missing custody/admission authority or
silently replaces an existing working setup. Qualification history is not required
to try a structurally valid unfamiliar class.

Before shared setup/runtime/dependency mutation, identify affected siblings and
retire their owners. Preserve setup identity, root, installation history, licensed
state and old publication records; advance the setup revision and rescan. Old
execution bindings cannot masquerade as current. A rollback is an owned successor
transition and new checks, not erasing history or claiming reversal of vendor state.
Retained bytes/evidence do not create an executable rollback operation by themselves.
Never promise an unavailable vendor build can be restored.

Applied snapshots and publications are immutable. A current user override is not
silently replaced by an old publication's historical setting; restoration resolves
and commits a compatible successor under the governing configuration lifecycle.
Keep unresolved legacy/new-model correspondence explicit until that transition is
implemented. Replacement of old acceptance paths must preserve identity, ancestry,
exclusion, state, cleanup and rollback tests before retiring their original owners.

## Live source and tests

These references locate retained contracts; campaign-like filenames do not give
their fixture identities authority over ordinary unfamiliar plug-ins.

| Boundary | Current implementation / checks |
| --- | --- |
| SDK and Rust ABI | [proxy](../../native-vst3-proxy/), [Windows host](../../windows-factory-probe/source/), [completion ABI v3](../../native-vst3-proxy/include/ap23_backend.h), [SDK fixtures](../../windows-fixtures/beta/) |
| Wire and audio | [IPC minor15 codec](../../native-audio-client/src/lib.rs), [LVB5 admission](../../native-audio-client/src/admission.rs), [notification schema1](../../native-audio-client/src/notification.rs), [direct mailbox layout4](../../native-vst3-proxy/backend/src/direct_audio.rs), [completion regressions](../../native-vst3-proxy/backend/src/completion_contract_tests.rs), [long blocks](../../native-vst3-proxy/backend/src/long_block_tests.rs) |
| Loaded factory provenance | [descriptor ABI v2](../../native-vst3-proxy/include/runtime_descriptor.h), [loader replacement regression](../../native-vst3-proxy/backend/src/descriptor.rs), [pinned-map identity regression](../../bridge-manager/src/mapped_file.rs), [same-peer admission/rechecks regression](../../bridge-manager/src/main.rs) |
| State | [native state owner](../../native-vst3-proxy/backend/src/state.rs), [bounded streams](../../vst-state/), [Windows instance owner](../../windows-factory-probe/source/component_instance_session.cpp), [startup synchronization](../../native-vst3-proxy/backend/src/startup.rs) |
| Custody | [session supervisor](../../bridge-manager/runtime/session.py), [ownership and negative-identity tests](../../bridge-manager/runtime/), [retained command-session contract](../BG1_RUNTIME_OWNER.md) |
| Publication and transitions | [publication owner](../../bridge-manager/src/publication.rs), [preparation transitions](../../bridge-manager/src/preparation_cli.rs), [package/update authority](../../bridge-manager/src/package_authority.rs), [beta journeys](../INTEGRATED_BETA_DELIVERY.md#acceptance-method) |

Current paired product routes retain IPC minor15, direct mailbox layout4 (legacy
worker layout3), notification schema1, LVB5 admission, descriptor C ABI v2 and
completion C ABI v3. Retained LVB1–4 formats cannot authorize a new launch under
the repaired manager; predecessor execution follows the coordinated refresh above.
Their compatibility is checked independently; old executable publications retain
their identities/limits. Updating a version requires both edges and relevant
contract tests, not documentation-only promotion of support.
