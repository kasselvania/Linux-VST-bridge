# Merged MF3 installation: readback blocked

The exact merged source `be9a7e9bd017a4d2739910f5fcbf71df7b10a8ec`
(tree `b6664b8f0972861309d62e7f59e99e35061a45c3`) was built and installed
through immutable setup. After the source's service-stop requirement was
reported, the operator explicitly authorized the idle stop/install/restart route.
The manager window was confirmed closed. The service and both keepers returned
healthy. No commercial instance or inspection ran.

**The requested post-install acceptance did not pass. LC2 has not started.**

## Exact installed bytes

| Artifact | SHA-256 |
| --- | --- |
| Manager | `ced6f44389b0999fd6ebbd1849873d6bb50f43755ea0e32613de316de37d9ca9` |
| Frontend | `621599fe477358dfe3265b87470b20525c3d87d6c791696c91b9281b1bfe6f2d` |
| Schema-2 kit | `3d8bc23bbd575881721752647e10dc74cbcf4b4ef7ac2b175f207de22ec0abf2` |
| Kit-owned native builder | `8af097a89211fd51596cb3f0f36495016f95576bc460d97f405a7d6c34c4b265` |
| Kit-owned descriptor generator | `05ab8308c50d48871818b6906e7c6a2023c453bf225ae7733dd408686f504970` |
| Kit Windows preparation/inspection host | `348a4bbc6ea34f57fc5899c279d9e43999bf9ecae4563a6cfee88967d36f66be` |
| Kit host-source manifest | `0ce0ecae2264bab24290712d0e3d2ed9e8c092ce6cb7a521fd0fac71ca010ac3` |
| Preserved default Windows host | `3ee36fd34384b2c282ec9cb6ae6c48d17f83940eaaf6d8f9a1fa130b65274939` |
| Preserved default host-source manifest | `6ca742eaaa2547eeea866bcd2e8121c9ee400c57d0394c752e3ac2a9794432b5` |

The installed artifacts and the entire kit file roster were hash-verified. The
kit recipe names the merged source, and its builder/generator bytes are inside
the archive. No Serum native proxy or Windows product host was rebuilt. Previous
immutable software remains retained. The merged software is still installed.

## Upgrade gap

Operator activity returns schema 4, but the full operator snapshot exits 1 with
`No such file or directory`. The pre-upgrade installation already retained
candidate A (`33f68a981984e147b7bb17564d8b101e3eeaa2596e4bdbcb61a22070e6f0aa9e`)
as a `retained_sv1` candidate record. It did not yet have the later legacy
provenance snapshots/seal or lineage record.

`preparation::candidates` treats any existing `retained_sv1` candidate as already
adopted, skipping `adopt_sv1` and its materialization owner. The history projection
then requires the absent `legacy/provenance.json`. Candidate A's bytes remain
unchanged; materialization did not complete. A bounded private file trace retained
the missing legacy-directory lookup, but the instrumented snapshot exceeded its
diagnostic deadline; it is not counted as successful readback. No diagnostic
process remains.

The next source repair must cover upgrading the older candidate-only state:
materialize verified immutable provenance and lineage without replacing the
candidate, preserve exact conflicts, and prove retry after interruption. A fresh
adoption fixture alone does not cover this upgrade.

## Unexpected existing publication

Canonical managed status also reports Serum 2 as experimentally published, with
an existing registry entry and physical Bitwig link. Its ordinary activation
permission is false. Its managed refusal is the missing-file error above.

The registry digest and physical link target are identical before and after this
installation. This publication therefore predates the installation; no publication
action was issued here. The expected unpublished state is **not** established.
Resolve the existing publication through its exact manager owner before claiming
that state. It was neither removed nor silently relabelled.

Serum 2 FX remains a separate installed/unqualified inventory class, with no
registry entry or physical publication. Both products retain version 2.1.5.

## Preservation and final state

All 261 compared retained records/artifacts and all five protected projects are
unchanged. Canonical managed readback confirms Pigments 18, exact rollback 11,
Pure LoFi 10 and Efx FRAGMENTS 10, with valid ordinary publications. Installer,
onboarding, environment manifests and existing publication history were preserved.

The original SV1 processing-restart failure remains original machine evidence;
normal retirement remains unproved. Positive terminal cleanup stays separate.
The complete MF3 history projection is unavailable until the migration repair.

- Service active; both keepers healthy.
- DSP and maintenance leases: 0.
- Pending transactions and stale transports: 0.
- Cleanup unconfirmed: false.
- Capture off; no service-resume record.
- No Bitwig/Serum launch, inspection, candidate build or publication mutation.

The accompanying JSON is the sanitized machine-readable installation/readback
record. Raw paths, process identifiers and diagnostic traces remain private.
