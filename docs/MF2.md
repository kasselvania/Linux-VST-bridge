# MF2 — Managed Windows installer onboarding

The native manager frontend adds a file-picker workflow for a local Windows
installer. Operator model 2 is an exact manager/frontend generation. It retains
MF1's closed actions and adds installer environment creation, start, focus, stop
and scan. There is no arbitrary command, argument, environment, PID or path field.

## Import custody

The native picker uses RFD 0.17.2 (the desktop portal on Linux). Cancellation
returns without invoking the manager. The frontend opens the selected object
read-only, nonblocking and no-follow, rejects nonregular/foreign objects, and
passes that descriptor as stdin to the fixed `import-installer` command. The
manager independently checks descriptor type, access mode, ownership, extent and
metadata before/after copying. It does not accept a pipe as file authority.

Import is limited to 4 GiB, with 64 MiB additional free-space reserve. PE signature,
header extent, executable characteristics and x86/x64 machine type are checked;
compound-file signature, version, sector and extent checks select the MSI route.
These are format recognition, not trust or installer compatibility claims.
The source filename and path are not persisted. A generic display hint is used.
Vendor/product begin null. A private temporary file is hashed while streaming,
synced and renamed to a digest-derived immutable artifact. Identical bytes reuse
the verified existing artifact and record. The schema-1 import record retains
size, format, digest, import outcome and local creation time.

## Isolated environment and initial installation

The frontend offers only runners derived from the currently verified immutable
software catalogue. Selection uses the digest of the complete Runner record.
Each onboarding creates a new random environment with private home, configuration,
cache, data, temporary and runner surfaces plus its operation lock. No existing
Arturia, .wine or yabridge environment is selectable. The schema-1 onboarding
record binds installer, runner, environment, creation operation and exactly one
initial installation operation. It is explicitly unpublished.

The manager revalidates authoritative inactivity before mutation. Installation
uses the existing operation-bound service suspension/recovery and a dedicated
`linux-vst-bridge-installer-<operation>` unit. The existing --install supervisor
has a schema-2 managed-session route, using the same bounded cgroup/PID-start
containment owner as vendor applications. A launcher exit with surviving children
is unknown, not completion. The supervisor drains diagnostic pipes without
publishing installer output. It records completed, failed, cancelled or
cleanup-unconfirmed with exact operation, launcher status and cohort cleanup.
A one-hour bound applies; stop signals belong only to that exact unit.

Focus is an ordinary EWMH request for one unambiguous visible window belonging to
the exact current installer cohort. It does not inject input, select by title or
assume that an extracted installer child retains the original executable name.
Ambiguous windows refuse focus. Stop checks the exact onboarding/operation pair.
Frontend close has no cancellation authority. Worker termination attempts exact
installer retirement; absent positive retirement blocks service recovery.
Recovery remains bound to the suspending operation, so delayed older cleanup
cannot consume a newer recovery record.

## Scan and limits

After positive retirement, Scan uses the existing bounded VST3 traversal and
supervised factory/inspection owner. The new environment's private home applies
only to its isolated scanner; ordinary products retain their existing launch
environments. Factory classes and follow-on component refusal remain distinct.
Inventory currency includes environment, module, scanner host and source.
Unknown audio classes are Installed — unqualified; malformed/incomplete factory
results are quarantined, and an empty result is reported explicitly. No profile,
native proxy, candidate, registration or DAW publication is created.

A separate optional Inspect button is not introduced: the existing scanner already
retains its bounded follow-on component inspection result. SV1 remains responsible
for later candidate preparation and Bitwig testing. MF2 does not claim universal
installer support, silent installation, existing-environment updates or successful
authorization from installer completion. All installer, account, licensing and
consent interaction belongs to the human operator.

## Verification status

Generated import, environment, action, ownership, inventory and frontend checks
are part of the manager/frontend/runtime suites. The real Xfer workflow and exact
installed-software identities will be recorded separately after those checks and
CI pass. No real installer or product operation has run during implementation.
