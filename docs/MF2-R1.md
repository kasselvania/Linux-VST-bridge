# MF2-R1 — operator validation lock coordination

Historical failure: `62f8756df21ebedd5f5a2fa963112f773dd255f9`.
`evidence/mf2/result.json` is unchanged. Import succeeded; environment creation
was refused after 64.2938 ms, before any environment, resume record or installer.
The historical lock holder remains unknown.

## Acquisition and ownership

`Manager::try_lock` returns acquired or typed busy; open, identity and real flock
errors remain errors. `Manager::lock` remains a single nonblocking exclusive
attempt, mapping only actual contention to `LockBusy`. Existing admission and
mutation callers do not wait. No code identifies contention by matching error
text. The legacy display text remains available to old error consumers.

The non-RT bounded helper retains a closed lock name, exclusive mode, purpose,
operation ID when applicable, bounded policy, elapsed microseconds, attempts,
timeout milliseconds, and acquired/timeout/error outcome. Canonical serialization
always uses `action_serialization`; ordinary registry projection uses
`operator_readback`; queued validation uses `operator_validation_readback`.
Holder is explicitly
unknown. It sleeps at most 10 ms between attempts; open/identity errors do not
retry. No path, command line, credential or asserted process identity is exported.

Validation order is:

1. Operator serialization (`operator-canonical.lock`, at most 10 seconds).
2. Registry authority (at most 10 seconds for the registry sampling phase).
3. Release readback authority, then enter the existing action-specific owner.

The worker retains the same operation while waiting. Under registry authority it
captures the registry, software/registry token and exact capacity owner census,
then explicitly releases the guard. Artifact hashing, runner verification,
systemd observation, history and installer/onboarding presentation happen outside
registry authority. `project_managed_registry` consumes the captured registry
without reacquiring it. A brief bounded registry recheck requires the same token
and owner census before the snapshot can authorize an action; drift refuses the
snapshot. Canonical serialization remains held throughout this projection.

Environment creation has a separate mutation boundary. Full imported-installer
and offered-runner digest verification happens outside registry authority. The
preparation retains file identities (device, inode, size, nanosecond modification
and change times, owner and mode) for the installer, its immutable record, runner
files, installed software record and catalogue. The existing vendor/onboarding
retirement checks also run outside registry authority; bounded metadata stamps
bind their durable inputs across that work.

The action samples LVC1 outside registry authority, then obtains **one** bounded,
operation-bound registry guard with purpose `environment_creation_admission`.
Under that same guard it compares the exact owner census, registry/software token,
durable retirement inputs, inactivity, cleanup and transaction posture. The
prepared installer/runner identities must remain unchanged. The onboarding owner
receives and validates that exact guard, creates one unpublished environment and
returns without reacquiring registry authority. No full digest, runner-wide
verification or systemd query runs inside this mutation guard. All ordinary
`Manager::lock` and `capacity::reserve` callers remain one-attempt fail-fast.

LVC1 itself needs the registry lock in the service, so it is never requested
while the caller owns registry authority. Capacity is sampled first and its
exact sorted owner census must still equal the durable census under the guard.
If the initial capacity read was unavailable during contention, the registry
wait can clear that contention, after which the guard is released for one fresh
LVC1 sample and reacquired within the remaining registry budget. An unavailable
or mismatched sample stays unsafe. This is readback resampling, not action retry;
no mutation is replayed. Existing IPC timeouts and artifact-validation costs
remain separate from the lock-acquisition deadlines.

### Lock-order audit

Snapshot, worker validation and operator environment creation use the same
serialization-to-registry order.
They do not hold registry authority while calling LVC1, waiting for action
completion, acquiring receipt ownership or entering service recovery. The new
per-operation worker guard is acquired fail-fast and is only a duplicate-worker
lifetime guard; no other owner waits for it.

Existing action-specific environment/resume owners are subsequent phases. Some
legacy action owners nest a **fail-fast** registry attempt under their
operation/resume lock; those calls have not become bounded waits. In particular,
`operator-resume.lock` still protects suspension/restore against stale cleanup.
No new registry wait occurs beneath an action-specific lock. This preserves the
existing mutation/recovery contracts and does not claim that every legacy lock
nest follows the new validation order. Generated concurrent readback/capacity/
environment-create coverage checks the affected waiting path for deadlock.

## Durable result and presentation

The schema-1 durable operation gains `waiting` and bounded `lock_waits` context.
Final completion retains acquired waits without calling them failures. A timeout
retains a schema-2 operator projection with:

- layer `manager_control_plane`;
- stage `operator_validation_readback`, or `environment_creation_admission`
  for the separate pre-mutation acquisition;
- code `registry_lock_timeout` (or a distinct serialization/access error);
- retryable true for timeout only;
- mutation_started, environment_created and installer_launched all false;
- the exact acquisition facts and unknown holder.

A bounded human reason is retained for older consumers. The additive typed
`Onboarding.failure` field attaches only to the exact imported installer named
by the retained operation request. The card explains that environment creation
did not start, the installer never launched, and retry is safe after healthy
canonical refresh. It does not describe this as an installer failure. The UI
never retries automatically. Older unstructured failures remain historical and
are not retroactively assigned invented wait/holder facts.

Terminal first-write preservation is unchanged. Waiting and late completion
cannot steal `operator/latest.json` from a newer operation. A duplicate worker
cannot execute an already completed/refused operation or create a second
environment.

## Generated proof and real boundary

The production worker fixtures use real registry/capacity/snapshot/import/
onboarding owners; only external service transport is substituted. They cover
brief contention and exactly one environment, timeout without mutation,
canonical state change while waiting, concurrent operator snapshot and capacity
read, duplicate-worker suppression, newer-latest preservation, structured card
projection, and fail-fast admission/mutation preservation. Frontend rendering
checks prove the manager/no-launch/retry explanation comes from typed fields.

No Deck mutation, GUI interaction, package installation, file picker, installer reimport,
vendor launch or live retry occurs in R1 before independent source review.
The installed manager/frontend identities remain those recorded in the historical
MF2 result. The next authorized live continuation reuses installer SHA-256
`507b726d97bf78920157f3817aff003b9ee38ee961f4efd318cf43216370f695`
(size 1,313,084,056 bytes); it must not reopen the source file picker.


A read-only Deck check after source verification confirmed the same imported
record, no onboarding environment, the previous installed manager/frontend,
service active, keeper 1, capture off and zero leases/transactions/stale transports.
It did not read the source installer or mutate any installed artifact.

## Review repair at the 44f251f boundary

The original barrier regression failed against reviewed source: while actual
installer verification was paused, another owner could not acquire registry.lock.
It passes after the split. A second regression changes the registry during that
unlocked expensive phase and requires the final guarded recheck to refuse it.

The production queued-worker mutation regressions introduce contention only after
validation has succeeded and the mutation capacity sample has been captured. A
brief hold continues the same operation with exactly one mutation acquisition and
one environment; a persistent hold produces the typed mutation-stage timeout and
no environment/resume/installer operation. Duplicate terminal dispatch remains
inert. Replacing installer, runner or catalogue files after verification is
refused before creation. Acquired and timed-out lock facts assert exact lock names
and purposes; the frontend covers both validation and creation-admission stages.

These are generated source tests, not another Xfer attempt. Historical MF2 and
prior R1 evidence are retained separately and unchanged. No installed software,
product or real environment changes in this repair pass.
