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

The record contains schema/session, generation, processing epoch, request
sequence, last completed end position, confirmed snapshot revision/generation/
through and SHA-256, closed failure class/status/domain, failed position,
native lifecycle phase and producer. No payload, pointer, vendor text or state
bytes are included. Status domains distinguish native fault, GUI failure,
Windows-host terminal status, outer launcher exit and Windows exception code.
The first observer is identified; it is not automatically the ultimate cause.

Only the native transport owner updates progress. Snapshot identity changes only
when the existing complete-state store confirms a capture. No new lock, I/O,
allocation, or observer call is added to the DAW callback or vendor process call.
The native shared owner retains the mapped custody after physical unlink, so a
bounded non-RT `if1_terminal` query can serve the controller after containment.

## Host-visible failure

AP10.poll emits one AP10.instance_failed message for a terminal generation.
The controller ends gestures/groups, cancels focus, retires the native editor
identity, refuses dead-instance commands, retains a bounded failure description,
and requests kReloadComponent once when a component handler exists. Reopening
shows the native bridge failure view, not a new vendor request. The existing
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

The one automatic UIO1 menu/resize check remains pending. No new candidate or
ordinary acceptance is claimed by source/fixture results. Pigments11 is active;
12/13 are retained unchanged, with 13 inactive failed ordinary history.
