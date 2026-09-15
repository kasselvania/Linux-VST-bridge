# MF2-R2 — recovery controls and private-home installer startup

Basis: evidence head `a0e82845a9b5b8795d5036889d077c4a2b6b409f`, installed
implementation `8e50cd3a6552c9d5317fdd3b5280e04d1acc6c1f`, same draft PR108.
The original import, unpublished environment, cancelled attempt, screenshot and
historical evidence remain unchanged. No product/native/profile change.

## Frontend reconciliation

The production reply handler now observes Snapshot.operation before releasing
feedback against that snapshot. Activity-first and Snapshot-first are equivalent;
an equal later Activity needs no additional transition. Exact operation identity
still separates old terminal receipts from the current request. The egui test
executes the reply handler and actual Stop button across Down/activity-poll/Up.

An uncertain acknowledgment remains uncertain, not terminal. A subsequent
canonical snapshot may establish the exact live onboarding owner and its offered
Stop action; it does not resend Start or infer installer success. A different
environment, absent owner, stale operation or disabled manager action cannot
release authority. The UI shows its bounded 12-transition history, identity and
pending/disabled reason. Duplicate Start is absent from live canonical controls
and is refused by the existing reserve owner. Generated tests also retain the
ordinary Activity-first and background-poll click cases.

## Pinned bootstrap comparison

The installed Proton script SHA-256 is
`787504a79bacf6b303984a9a846cf47463f36599248e8306b26d5faf78267aad`.
It equals the [proton-11.0-2c script at 5b89db9](https://github.com/ValveSoftware/Proton/blob/5b89db940e0ebe3a137a6009a3589232fe084c09/proton)
byte-for-byte. `run` initializes the prefix and starts the built-in Steam helper.
`runinprefix` calls Wine directly but skips prefix-file setup. The path-conversion
verb `getcompatpath` performs that setup without the Steam-helper launch. MF2 now
runs `getcompatpath /`, checks successful completion and the initialized prefix,
then runs the installer with `runinprefix` in the same owned cohort.

The [reference Unix library](https://github.com/ValveSoftware/Proton/blob/5b89db940e0ebe3a137a6009a3589232fe084c09/lsteamclient/unixlib.cpp)
uses HOME's architecture-specific Steam SDK path and resolves native exports.
The deployed ELF64 x86-64 library contains the corresponding lookup/export
strings. Its digest is
`742e115bbb32b907c3e349bfed1c8d898f621a8dd2031d1d4479274dde5002c2`.
This does not prove complete C-source/binary equivalence: the screenshot's
assertion at line375 differs from that reference C file. No historical stack or
native-loader status is invented.

The fresh private-home source-owned baseline emitted the actual deployed
`steamclient_init` diagnostic: native steamclient could not load. The library path
was absent inside the observed container; no target-ready marker appeared.
The same harmless payload under initialized `runinprefix` reached ready, exited0
and retired positively. A distinct hold-payload check reached ready and cancelled
with exit-15, zero owned processes and positive cleanup. No Steam account/library
bind, real-HOME exposure, accessibility override, runner replacement or assertion
suppression was added. This proves the selected bootstrap contract for the fixture,
not Xfer completion or a reconstructed cause of the historical screenshot.

## Startup and cancellation custody

The existing installer result includes a bounded operation-bound startup object:
48 scalar stages, first problem with source class, independently retained
cancellation, up to eight exact mapped helper identities, and target observation.
Each stream has its own bounded line assembly. Target/helper identity requires
mapped device/inode and process-start identity; names alone do not identify the
target. Only closed diagnostic signatures enter structured status. The existing
private tail continues draining; dropped bytes/stages and unavailable evidence
remain explicit. Commercial ready state and exception stack remain unknown.

The source-owned fixture's fixed ready marker is interpreted only by its private
test harness, not as commercial installer authority. A modal live-process problem
is retained before cancellation. Normal cancellation during bootstrap creates no
invented failure. Outer exit, cancellation and positive cleanup remain separate.
A wrapper originally attempted Stop after a completed unit had already been
collected; its existing successful result was finalized without a new run.

## New isolated attempt

A closed InstallerNewAttempt action selects an exact failed/cancelled predecessor
and currently offered runner. Positive predecessor retirement is required.
Preparation verifies the retained import/runner and predecessor records outside
registry authority; metadata and canonical ownership are rechecked under the
same bounded creation guard. A new environment retains previous_attempt;
the cancelled parent's operation/result never change. Only one direct successor
can be created; a further explicit attempt must select its own retired predecessor.
No reimport, qualified-environment update, candidate or publication occurs.

## Verification and remaining product gate

Generated frontend reply/button tests, actual creation-owner history tests and
runtime tests cover cancellation, diagnostic saturation, stream separation,
unknown evidence, launch failure and exact cgroup containment. AP8 builds and
executes the harmless Windows payload; AP12 and PX2 cover affected owners.
No native/audio source changes require AP10. Current source and installed hashes,
CI and the next human-only installer outcome are recorded in the PR continuation.
