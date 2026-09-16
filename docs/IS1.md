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
