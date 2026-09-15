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

## Installed R2 continuation

Implementation `3c23d8ef00440b0464231fee81104e44185ba314` passed AP8, AP12
and PX2. Linux validation passed 92 library, 54 binary, 13 frontend and 69 runtime
tests, with strict manager/frontend Clippy. Rust binaries were built from
`e3e7b98`; the later commit changes only runtime diagnostic parsing/tests.

The immutable setup installed the repaired manager, frontend and supervisor while
reusing the exact installed Windows host/native catalogue artifacts. An initial
staging omission of the retained host/source files caused setup to refuse before
stable authority changed; service was restored. Supplying those exact existing
inputs allowed setup and all post-install identity/preservation checks to pass.
No installer was launched during setup. The retained import and original cancelled
environment remain byte-for-byte unchanged. Service/keeper are healthy; DSP,
maintenance, transactions and stale transports are zero, and capture is off.

See `evidence/mf2/r2-recovery-startup.json`. The source-owned comparison was run
in development; final diagnostic refinements were covered by generated tests.
The final runtime digest was not captured at the start of those earlier runs and
is not retroactively asserted. Human continuation begins with one **New isolated
attempt** from the existing import, then waits for exact environment readback
before Run installer. Xfer completion and scan remain pending.

### Human continuation: startup passed, Focus not confirmed

One human New isolated attempt created `4db060b14388e41103834fc4dfdd023a`,
linked to the unchanged original cancelled environment. Registry validation
contention cleared in 80.961ms; the same operation completed. One human Start
launched the exact imported target, with successful prefix initialization and no
startup diagnostic. Closing/reopening the frontend retained the same running
installation and exact offered Focus/Stop controls.

One human Focus request returned `window_manager_refused`. This name denotes
absence of active-window confirmation within 750ms, not an explicit negative WM
acknowledgment. Subsequent read-only observation found one exact owned normal
window, DEMANDS_ATTENTION, and an active window outside that ownership. The
operator was unsure whether focus moved. No retry occurred, and no exact desktop
policy cause is claimed. The installer remains live under its original operation;
startup success is retained separately from this recovery-control boundary.
See `evidence/mf2/r2-human-start-focus.json`.

### Human installation and discovery completed

The same human-operated Xfer installer finished with outer exit0, zero owned
survivors, positive cleanup and no startup problem or cancellation. Its exact
operation survived manager close/reopen. No agent sent installer/account/license
input, and no account or authorization material was inspected.

The first Scan click targeted the historical cancelled environment. Its missing
VST3 directory produced an OS error2 refusal. This is separate from the successful
new environment. The next human click targeted that completed environment and
completed one factory scan: Serum2.vst3 SHA-256
`501e7bb3dd9cafe416b3412df3d4e084c01b7468201e5690b9009d7ecd4e5283`,
19,236,864 bytes, Xfer Records version2.1.5, four exported classes: instrument,
instrument controller, FX, and FX controller. The two audio products project as
**Installed but unqualified**, current evidence, no active revision, and no
activation/publication permission.

The generic inspector then refused to select automatically between two audio
classes: `multiple audio classes require explicit selection`, exit90. Factory
enumeration was complete; module exit/unload and supervisor cleanup/transport
retirement succeeded. This is not a crash or component-lifecycle qualification.
No extra class inspection, candidate, native proxy or Bitwig launch was attempted.
The final report retains the supervisor's separate cleanup exit -15 without
confusing it with inspector exit90 or the successful installer's exit0.

The user deferred the history-card/result-visibility issue. Retain that follow-up,
the unconfirmed Focus request and the generic inspection-warning wording. No UI
or focus workaround was introduced during this session. Final readback verifies
healthy service/keeper, capture off, zero DSP/maintenance/transactions/stale
transports, no resume record, and no installer/generated-fixture unit active.
Existing products, imported installer record and protected project hashes match
the pre-session state. See `evidence/mf2/r2-installed-scan.json`.
