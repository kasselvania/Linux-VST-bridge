# IF1 — terminal instance failure propagation

The accepted input-fairness repair is unchanged. Two subsequent ordinary
Pigments13 sessions lost their Windows endpoint; native processing refused
further work, but the controller could still present an editor for that dead
instance. A later state request failed. This is a terminal-failure propagation
gap, not evidence that a save directory or installation is wrong.

Existing incident custody and exact rollback are in `evidence/if1/`. Raw receipts
and bounded Bitwig log tails are private. Missing at-failure process/window
censuses and an independently identified Windows executable exit are recorded
as missing; the outer launcher exit 5 is not assigned to an SDK call.

## Custody and ownership

`if1.terminal` is an independent 2048-byte LVIF v1 mapping, mode 0600, created by
the native session before Windows admission and exposed through the existing
exact tmpfs Windows view. It does not change the audio/result protocol. Its
immutable header carries the exact session. All mutable words use atomic access.

Three producers own disjoint 256-byte slots: native transport, Windows UI owner,
and Linux supervisor. Each copies a fixed 24-word record before competing for a
single atomic commit. The first complete publication wins; no writer reserves
a global lock before copying. A killed writer cannot strand custody or replace
a previous complete record. Two native-progress slots preserve the last complete
context if a progress update is interrupted. Readers try at most three times.
An unstable read is retryable, not a completed failure publication. Windows and
supervisor producers keep their first pending status/domain until a stable
copy is committed or another producer has committed first.

The record contains schema/session, generation, processing epoch, request
sequence, last completed end position, confirmed snapshot revision/generation/
through and SHA-256, closed failure class/status/domain, failed position,
native lifecycle phase and producer. No payload, pointer, vendor text or state
bytes are included. Status domains distinguish native fault, GUI failure,
Windows-host terminal status, outer launcher exit and Windows exception code.
The first observer is identified; it is not automatically the ultimate cause.

Only the native transport owner updates progress. Unchanged idle turns do not
publish, and identical complete context/state records do not advance the counter.
A newly confirmed snapshot still publishes even when processing context is unchanged. Snapshot identity changes only
when the existing complete-state store confirms a capture. No new lock, I/O,
allocation, or observer call is added to the DAW callback or vendor process call.
The native shared owner retains the mapped custody after physical unlink, so a
bounded non-RT `if1_terminal` query can serve the controller after containment.

## Host-visible failure

AP10.poll emits one AP10.instance_failed message for a terminal generation.
The controller ends gestures/groups, cancels focus, retires the native editor
identity, refuses dead-instance commands, retains a bounded failure description,
and requests kReloadComponent once when a component handler exists. Reopening
shows the native bridge terminal failure view, not a new vendor request. The existing
snapshot identity can be displayed, but no state is silently substituted. The
view directs the user to reload/remove or reopen a saved project; it does not
invent a successful in-place recovery of opaque commercial state.

Normal process-scoped retirement still requires all seven milestones. An
unexpected exit or fatal editor/controller result is terminal_instance_failure,
not clean SDK destruction. The supervisor retains exact process containment,
transport retirement and lease receipt ownership.

## Verification boundary

Generated tests cover interrupted custody, first-write retention, actual native
worker peer loss and query after unlink, processor/controller notification,
root subprocess exit through the supervisor, unaffected unrelated processes,
and Windows fatal EditorSession destruction in a child. The Windows parent
reads custody after the child terminates; normal SDK close must leave it empty.

The single automatic check delivered the menu Down/Up to the vendor child.
The private harness then stopped on a filename collision before resize. During
subsequent cleanup the Windows endpoint exited: IF1 retained the first native
transport fault and complete state identity, and Bitwig displayed its crashed
plug-in status. No repeated interaction or reload was attempted. The exact
Windows exit cause and the resize result remain unresolved. Generated tests,
not a live notification counter, prove exact-once controller/reload behavior.

Ordinary Pigments11 is restored; 12/13/14 are inactive immutable history.
Service and keeper are active with zero DSP leases, pending transactions or
stale transports. See `evidence/if1/live-check.json`, `installed-final.json` and
`validation.json`. IF1 is not claimed fully qualified and remains draft.

## Engineering publication

Pigments revision 14 is the exact IF1 native/Windows candidate. `qualify-failure` uses the existing sealed publication and rollback owners with `if1_failure`; its immediate physical parent must be ordinary revision 11. Revisions 12 and 13 remain inactive immutable history. Ordinary activation and older acceptance commands cannot promote this candidate. `uio1 admit-if1` permits observation only of its exact physical candidate publication.

## Review-only custody repair

Concurrent generated tests force progress to recycle both slots between context
copy and commit validation on all three read attempts. They require no terminal
publication from the unstable read, a successful later retry with the original
failure status, coherent context and unchanged confirmed state identity. A
competing failure producer cannot be overwritten. The idle regression requires
byte-identical progress slots/counter across unchanged turns.

The commercial surface is a **terminal failure view**. It reports failure and
confirmed state identity; it is not an in-place recovery implementation. The
existing reference-fixture recovery controls remain a separate mode.

These review changes do not modify retained evidence, profiles, artifact identities
or rollback records. They are not installed or product-retested. Ordinary
Pigments 11 remains active; revisions 12–14 remain inactive. PR #98 stays draft
and unmerged pending source rereview.
