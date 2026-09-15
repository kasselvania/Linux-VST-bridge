# SV1 — Serum 2 instrument preparation

MF2 merged at `3aa26d11f401abfabf86b0b574d4cc7f8436810a`, preserving the
accepted installer and inventory result. This continuation selects only Serum 2
instrument version 2.1.5. Serum 2 FX remains inventory, not a qualified product.

The explicit-class inspection passed on the accepted managed installation using
the installed Windows host and pinned runner. It initialized the component and
separate controller, enumerated stereo audio output and 16-channel event input
and output, enumerated 2,623 parameters, captured component state and synchronized
the controller. The component/controller terminated and module unloaded normally.
Supervisor cleanup and transport retirement were positive. The outer launcher was
terminated during owned cleanup; its -15 result is not the Windows inspection
result, which was zero. No GUI or authorization action occurred.

## Candidate custody

The sealed `compatibility/sv1` profile names exact instrument, module, runner,
host, descriptor and native artifacts. Its separate binding retains the accepted
onboarding/environment/inventory records and the selected inspection digest.
`ManagedInstallerV1` identifies the managed installation lineage without pretending
that Xfer belongs to the Arturia environment family. No caller-provided class,
profile, executable or environment can select this engineering route.

`qualify-instrument stage` verifies the package and creates immutable private
candidate custody. Staging creates no registry entry or Bitwig publication.
`publish` deliberately selects the exact review candidate through the existing
managed publication transaction. It uses the retained successful inspection after
revalidating its module, environment, runner, report and host; no repeat scan is
needed. Ordinary activation remains unavailable. The candidate has no fictional
ordinary parent: restore removes its physical publication while preserving history.
The existing publication interruption and reconciliation machinery remains owner.

All other registry classes are retained as the baseline. The registry's global
revision counter is excluded from that comparison because the candidate itself
advances it; every sibling entry remains exact. The service admits at most one
Serum engineering DSP instance, without expanding the global six-instance ceiling
or making any Serum capacity claim. The current manager must be installed before
this new candidate can be served.

Onboarding's private HOME is kept for its inspection, keeper and DSP/editor
session. The Rust owner validates the exact onboarding/environment record before
setting the flag. The Python owner refuses a nonprivate or aliased home. Arturia
and ordinary diagnostic-off environment behavior remain unchanged. No runner,
Windows host, accessibility override or audio implementation change is required.

## Test plan and limits

Preparation does not establish audio, current authorization, editor behavior or
state/project recall. After preparation, deliberately expose one candidate, then
have the operator launch Bitwig normally and load one Serum instrument in a
protected project. The operator owns every license/account screen. The bounded
check is one note, nonzero output, real authorized editor, one safe control and
normal retirement. Stop on a material failure and retain its evidence. Unpublish
the candidate after the check; never ordinary-promote it in this slice.

The source tests cover exact inactive staging, changed artifact/environment
refusal, compiled instrument/FX separation, ordinary-activation refusal,
engineering publication/retirement, all 16 publication interruption boundaries,
and private-home isolation. Existing manager/runtime suites and CI remain required.
The native artifact was built from the current registered backend and official
SDK with the new inspection-derived descriptor; no historical Serum proxy is used.

The initial-install owner continues refusing registered environments. Their
onboarding records remain byte-for-byte history; initial-install cards/actions
are omitted once qualification/product management owns that environment. The
candidate session instead validates its sealed installation binding. This avoids
reimporting or hashing the large installer during candidate startup. It does not
make the environment eligible for another initial installation.

## Preparation handoff

Implementation `5cd73b00098dc25f2948e9f07df40d20e3cc4aa3` passed AP12 and PX2,
including 97 manager library tests, 56 Linux CLI tests, 13 frontend tests, 70
runtime tests and strict manager/frontend Clippy. The subsequent documentation
head `32ea70c8bf2f99539e885c45957b99e589d197be` also passed both workflows.

After the operator closed the manager, the prepared manager/runtime package was
installed through the existing immutable setup route. Installed hashes were read
back and matched the built package; the frontend, Windows host, host source and
ordinary native catalogue remained exact. The installed manager then staged the
Serum instrument candidate inactive and verified its immutable artifacts. Exact
installed and staged artifact hashes are retained in `evidence/sv1/preparation.json`.

Staging created no Serum registry entry or Bitwig publication. The ordinary
registry, Pigments18/rollback11, LoFi10, FRAGMENTS10, imported installer record,
both onboarding histories, accepted inventory and protected projects remained
unchanged. Service and keeper were healthy; DSP/maintenance leases, pending
transactions and stale transports were zero, and capture was off. No Bitwig
session was launched. The next step is deliberate reversible test publication,
followed by the bounded human-owned instrument check described above. Audio,
authorization and editor usability remain untested.

## Operator test publication

After the operator explicitly requested making Serum available for their own test,
the installed manager published the exact instrument review candidate once through
its managed transaction. Canonical readback verified the physical publication,
module, native artifact, Windows host, environment and runner. Ordinary activation
remains forbidden; this is the explicit SV1 engineering route. Serum 2 FX remains
unpublished and the three Arturia registry entries remained byte-for-byte equal.
Service and keeper were healthy with zero leases, pending transactions and stale
transports; capture remained off at handoff. No Bitwig launch or input was sent by
the agent. The operator result is pending. The exact sanitized receipt is
`evidence/sv1/operator-publication.json`.

The first post-publication verifier used an incorrect projection field and stopped
after the successful manager transaction. A corrected read-only check verified
the already-published candidate; publication was not repeated.

## First operator session: terminal lifecycle failure

The operator reported a very responsive editor before a crash. The exact Serum
instrument session completed 16,508 processing blocks, and the editor retained
two complete parameter gestures (24 value events, two ends) with no editor
failure. Nonzero audio and authorization were not independently measured.

The processing worker stopped and joined normally, followed by deactivation,
setup (reported plug-in latency changed from 0 to 7 samples) and reactivation.
The newly started worker then reported `lifecycle correlation`, and the Windows
host terminated the failed processing session with status 90. The guard checks
message kind, session and sequence; these retained records do not identify which
field differed. This selects the lifecycle/protocol boundary for investigation,
not a Serum exception, graphics cause, latency-change cause or compatibility fix.

IF1 retained the first native-transport failure and confirmed-state identity.
There was no Collector rejection. The Windows controller and component terminated,
and supervisor cleanup and transport retirement were positive; outer cleanup
exit -15 is distinct from host result 90. CA1 was off, so exception attribution
and stack are unavailable. Bitwig main/audio-engine processes remained alive;
survival of its separate native plug-in-host was not established. Service was
healthy with two environment keepers, no DSP/maintenance leases, no pending
transactions or stale transports, and no cleanup block. No relaunch or repair
was attempted. See `evidence/sv1/first-operator-session.json`; raw diagnostics
remain private. The temporary candidate remains published pending operator
shutdown and cleanup handoff.
