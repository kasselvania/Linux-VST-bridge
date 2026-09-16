# IS1 — Multi-stage installer custody

IS1 stacks on integration `0d1c644a182c98e326ee4697f49b4d0c1dfc45a0`
(PR #111, dependent on #110). Those PRs and their product evidence remain
separate. No installed manager, runner, audio, product profile or publication is
changed during source qualification.

## Retained Native Access boundary

[Offline comparison](../evidence/is1/offline.json) was captured read-only before
implementation and fixture execution. Both isolated attempts initialized,
launched the real installer, installed VC++ 14.44.35211 prerequisites, and later
returned outer exit 2. They left prerequisite packages and Native Access version
markers, but no application executable or NTK service was found in the inspected
surfaces. Both are partial installations. No complete pre-attempt filesystem
snapshot exists; an exact historical full-prefix delta is unavailable.

The imported PE is an x86 NSIS Native Access 3.26.0 bootstrapper declaring
requireAdministrator. That declaration does not prove an elevation failure.
Xalia errors concerning already exited processes do not establish causality.
No historical exception, stack, or post-prerequisite failing child is invented.

## Ownership and observation

Only the exact installer unit cgroup plus PID/start identity grants process
ownership. The installer remains a subreaper. Its private ledger records identity,
verified parent identity where available, first/last observation, cgroup, observed
state, launcher/descendant relationship, phase, image evidence and exit custody.

The order is sample, merge, WNOWAIT observation, durable custody attempt, waitpid
reap, resample. No `Popen.poll()` runs ahead of the installer ledger. Known zombies
are checked independently because cgroup.procs can omit them. Unseen adopted
children are checked while WNOWAIT preserves their `/proc` identity. A child
already reaped by its own parent has an explicitly unavailable exit; an unknown
exit cannot acquire a role or borrow the outer result.

Linux wait status and a Windows DWORD exit are separate domains. The first
observed nonzero owned process exit is preserved as evidence with underlying
cause unestablished. Cancellation exits cannot replace it. A reporting write
failure is counted and does not obstruct reaping or containment. Cleanup requires
an empty observed owned cohort, including retained zombies, rather than merely a
dead launcher.

This is supervisor-side control-plane work, never audio callback work. Existing
ASC and DSP retirement owners are unchanged.

## Records and limits

The small existing schema-2 installer receipt retains legacy compatibility. An
optional schema-1 `transaction` binds the exact operation and projects outcome,
outer exit, durable installation category, first process failure, observed and
unavailable exit counts, diagnostic loss and cleanup. Manager readback rejects
unknown transaction versions, operation mismatches and outcome classes.

The private schema-1 transaction contains the full process ledger, stage
projections, before/after installation manifest, registry witness subsets and
delta. Roles are unknown unless independently evidenced; names alone confer
neither ownership nor stage authority. The report separates environment bootstrap,
target runner, descendants, cancellation, durable side effects and cleanup.

Limits: 512 ledger rows; 8 mapped PE images per process; bounded maps reads;
64 MiB per image and 256 MiB hashing budget; 8,192 filesystem entries per snapshot;
8 MiB per registry file; 32 changed log tails, at most 64 KiB each and 1 MiB total.
Bound exhaustion and unavailable observations are explicit. Paths and raw frames,
IDs and logs remain private. No prefix or account-data archive is produced.

Only allowlisted executable/package metadata, uninstall registration fields and
service configuration fields are inspected. Service command arguments are not
retained. Component-wise no-follow file opening rejects prefix aliases. Durable
application registration plus matching executable files witnesses installation,
not authorization, dependency health or successful first launch. Outer exit 2
cannot undo those facts. File changes alone can establish partial installation.

Runner diagnostics use bounded pipes, never PROTON_LOG or relay logging. Process,
service and MSI channels are installer-only. Accessibility-observer lines have a
separate bounded sink and no causal authority. Direct MSI uses a private manager
pipe for verbose diagnostics; arbitrary EXE bootstrappers receive no added flags.

## Product presentation and recovery

The installer card displays outcome, durable side effects, outer status, prior
failure and cleanup independently. Retained MF2 `installer_launcher_failed`
records are read as a legacy unknown-stage outer result, without rewriting them.
An installed witness prevents another automatic new-attempt/reinstall offer.
A managed first-launch crossing must be explicit; installation does not authorize
an unreviewed application launch, a candidate, or DAW publication.

## Qualification gate

Generated source-owned Windows cases use the actual installer supervisor, pinned
runner, private HOME and dedicated scratch cgroups. Fixture stdout is an oracle
for comparison, not a production process-identity or role authority. No vendor
files, credentials or GUI input are involved.

A fresh Native Access attempt remains prohibited until independent source review,
immutable installation and idle readback. Existing environments, both historical
attempts, the imported installer, Serum publication and ordinary products remain
unchanged. No runner, UAC, accessibility, graphics or vendor workaround is selected
without evidence. No host-root or operator-HOME expansion is authorized.

## Windows observations versus Linux exit custody

The pinned fixture demonstrated that Wine reaps many Windows children itself;
Linux subreaper collection cannot recover those already-consumed wait statuses.
IS1 therefore also parses bounded Wine `CreateProcessInternalW` observations and
final `NtTerminateProcess` self-pseudo-handle observations. Creation ordinal plus
runner-launch epoch separates Windows PID reuse. Creator thread and successful
creation rows establish a diagnostic process chain rooted in the exact imported
launch request. Arguments and environment values never enter that structured
chain. A Windows numeric PID is never joined to a Linux PID by equality.

These rows are diagnostic attribution, not cgroup ownership or signal authority.
Unmatched creation, nesting, missing identity and truncated observations remain
unavailable. A final self-exit observation is not an exception stack. A launch
request's image digest is labelled separately from an observed mapped-file digest.

Exact service API observations on a created process can establish that it performed
service/dependency work. A service-server return without exact RPC identity stays
separate; it does not silently acquire a client/child association. Other roles
remain unknown unless evidenced. No product-name rule selects a role.

The first nonzero child observation remains available even after outer exit,
cancellation and cleanup. An unknown numeric child status is not automatically
an MSI error or fatal cause. In particular, an outer-success transaction retains
its child observations without being converted into an overall failure by a
possibly informational/reboot-required value. Dependency health and application
first launch remain independently unproved.

## Generated execution and scope

`tools/is1/bootstrapper.cpp` is an account-free Windows fixture. It covers
prerequisite/payload results, actual service-create/start failure, installed-image
launch failure, outer-first retirement, a failure while the parent remains alive,
a zero-delay child, updater/relaunch, repeated images, cancellation preserving an
earlier child exit, accessibility-like noise and durable/no-install side effects.
The same cancellation case also proves cleanup does not erase prior failure.
`direct_msi.cpp` constructs an empty source-owned MSI with the official Windows
Installer API for the private verbose-log path.

`tools/is1/fixture.py` uses the actual `--install` production supervisor, current
pinned runner and dedicated installer cgroup. It refuses an occupied managed
capacity boundary. Generated scratch prefixes are removed only after positive
retirement and identity checks; proof records remain private. Test oracle markers
do not feed production role classification. No commercial installer, plug-in,
credential, operator input or installed-software mutation belongs to this harness.

Execution evidence is recorded separately from fixture design. The retained
[development matrix](../evidence/is1/generated-progress.json) predates the final
source verification; its historical nonclaims and pending checks remain explicit. The
[sanitized example](../evidence/is1/sanitized-example.json) demonstrates separate
child/outer custody without private IDs, paths, arguments or proprietary data.

## Final source verification

[Result and limitations](../evidence/is1/result.md) and the
[machine-readable record](../evidence/is1/result.json) supersede the pending status
of the earlier development matrix without rewriting that history. Direct MSI's
private FIFO was verified on the pinned runner. The updater fixture retained the
third-generation process; the post-launch fixture executed the copied installed
image. Neither oracle's semantic labels are admitted as production authority.

The public transaction outcome `in_progress` distinguishes an ongoing owned
installer from terminal `cleanup_unconfirmed`. Exact Stop/Focus remain offered
while supervision is active. A terminal failure observation remains visible in
both states; cancellation is a separate fact.

Record boundaries:

| Record | Schema and authority |
|---|---|
| Existing installer result | Schema 2; operation, state, outer status, startup observations, cancellation and cleanup |
| Public transaction | Schema 1; exact operation, outcome, durable witness classification, first nonzero observation, loss counts, safe next action |
| Private transaction | Schema 1; Linux ledger, Windows diagnostic chain, before/after witnesses, stage rows and diagnostic custody |
| Linux ledger row | Exact cgroup and PID/start; verified parent where available; phase, first/last monotonic time, state, wait source/domain/status, images, role or unknown |
| Windows trace row | Launch epoch and creation ordinal, creator relationship, image-request evidence, self-exit DWORD and evidence-backed role or unknown; no inferred Linux identity |

A missing stage is explicitly unproved; it is not inferred from a filename,
parent's exit, service-server response without RPC identity, or fixture stdout.
No generic first-launch operation or automatic reinstall is authorized by the
installation witness. A future first-launch crossing must retain its own exact
application selection and process ownership.

## Focused rereview repairs after `55ffc7c`

New isolated attempts use one manager eligibility rule for both preparation and
offered frontend actions. A positively retired, empty transaction with durable
`not_installed` or `partial_installation` can have one explicit successor even
when its top-level state is `completed`. Installed, unresolved, in-progress,
cleanup-unconfirmed and still-owned outcomes cannot. Legacy receipts without a
transaction retain their failed/cancelled retry rule. The creation owner checks
the prior result and linked successor again under its existing registry guard,
using only small custody records. It never rehashes the installer or runner there.
The new attempt has a new environment and operation; prior records remain intact.

Windows diagnostic history and active process authority are separate. A self-exit
retires that generation from the active map and discards its unfinished creates.
A create completion can inherit a target relationship only from the same active
creator ordinal and launch epoch that issued its request. A known exited PID
cannot masquerade as an unobserved outer launcher. Fully observed PID reuse gets
a new ordinal, while retired rows and their role evidence remain unchanged.
Windows IDs still confer no Linux ownership or signal authority.

Added, changed **or removed** allowlisted executable/package, uninstall and
service witnesses count as durable mutation. Removal-only results are at least
`partial_installation`; a stronger installed registration/image witness retains
precedence. Exact removal deltas remain available. Diagnostic-log changes alone
do not count as installation.

The frontend renders a first failure's phase, role (or unknown), relationship,
domain and numeric status, with cause explicitly unestablished. Outer exit,
cancellation and cleanup stay separate; numeric values receive no inferred
MSI, vendor or Windows error meaning.

Focused regressions exercise completed/no-install and partial-install retry,
installed/cleanup/live/duplicate refusal, two prevalidated successor requests,
snapshot and button behavior, exited/reused process generations, pending creates,
service attribution and each removal-only surface. These are source-only tests;
they do not replace or replay the accepted pinned-runner matrix or either retained
Native Access attempt.
