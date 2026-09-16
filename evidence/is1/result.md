# IS1 source result — ready for independent review

IS1 establishes the **generic installer custody/reporting repair**. It does not
establish why Native Access failed. The two original attempts remain byte-for-byte
retained: successful bootstrap and prerequisites, partial installation, outer exit
2, unknown later failing stage, positive cleanup. No new Native Access attempt ran.

Implementation is `f3997a2b58dc7cf85d912fc2266146d6fc258a9d`, tree
`11449b3c7a2748a08b207e4015de4176cd5d75d8`, stacked on PR #111 at
`0d1c644a182c98e326ee4697f49b4d0c1dfc45a0`. PRs #110/#111 remain separate.

## Generated production-supervisor result

| Source-owned case | Retained result |
|---|---|
| Prerequisite 0, payload 0 | Outer 0; installed-file/registration witness |
| Prerequisite 0, payload 37 | Child 37 retained separately from outer 2 |
| Service creation/start failure | Child 73; exact service API evidence; installed witness retained |
| Installed application launch failure | Copied installed image digest and child 41; outer 2; installed witness retained |
| Outer exits first | Later successful child exit retained before cleanup |
| Child fails while outer lives | Child 39 precedes outer retirement; no premature completion |
| Short-lived child | Child 43 survives Wine parent reaping through separate Windows self-exit observation |
| Update/relaunch handoff | Root, updater and third-generation child creation/exit chain retained |
| Repeated image names | Distinct exact process generations and separate results |
| Cancellation after failure | Earlier child 39 survives later outer -15 and successful cleanup |
| Accessibility-like noise | Separate diagnostic sink; no selected cause |
| No installation side effects | Explicit not-installed witness result |
| Direct MSI | Outer 0; 5,271 private verbose-log bytes through bounded pipe |

The old production source was run against the same payload-failure fixture in a
separate scratch prefix: it retained outer 2 and `installer_launcher_failed`, with
no child attribution. The repaired collector retains the child result. Linux wait
statuses consumed by Wine remain unavailable; Windows trace IDs are never treated
as Linux ownership.

The twelve-case collector execution predates only the final in-progress
presentation correction. An additional production cancellation case and generated
tests cover that correction. Stronger oracle checks were applied to retained
records without repeating those runs. Complete identities, limits, loss counts,
execution provenance and private-proof digests are in [result.json](result.json).
Earlier development evidence remains separate and unchanged.

## Validation and preservation

125 manager-library tests, 64 manager-binary tests, 15 frontend tests, 91 Linux
runtime tests and both strict Clippy checks pass. AP8/AP12/PX2 apply; final-head
workflow results are attached to the draft PR. AP10 does not apply.

Final readback: service active, two healthy keepers, zero DSP/maintenance leases,
pending transactions and stale transports; cleanup not blocked; capture off.
All 29 generated units are retired and their scratch prefixes removed. Five
protected projects and 221 retained historical files verified unchanged. Both
Native Access environments, original records and observed files verified unchanged.

Pigments 18/rollback 11, Pure LoFi 10 and FRAGMENTS 10 remain unchanged. Serum B's
existing experimental publication remains exact. No manager/runtime installation,
product launch, runner replacement or compatibility-policy mutation occurred.

## Next authorized boundary

Independent source review comes first. Only after acceptance and immutable
installation may the operator perform one fresh linked Native Access attempt with
the existing import and bounded diagnostics. Its result must select any further
repair; IS1 does not guess a vendor, service, elevation or accessibility cause.
