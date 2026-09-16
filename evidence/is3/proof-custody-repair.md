# IS3 proof custody repair

The result at `ce0b9f9a9354ad2091898bb0d84e3c1792122ef8` remains credible and its
JSON is unchanged. This amendment supersedes its comparison custody, not its
physical observation.

## Exact anchors

The completed campaign executed committed source
`079e32f89d242beca9029bbae488bd1aa301df6d`, tree
`41be6da47b67362098546a3b4cb2972ecbcbd87c`.
The subsequent delivery commit adds only the manifest, result, readback and this
explanation. All packaged executable sources still match that source commit and
their manifest hashes.

- Manifest: `tools/is3/fixture-manifest.json`
- Manifest SHA-256: `5d4b7713912eb8741705aaecb2e1d1345aebe54110e92fa8fd786244efc087d5`
- Complete comparison identity SHA-256: `46763f344145d03633c7b279c40eebfa59ad1556355f88c87b72e7c997889353`
- Campaign-owned result: `evidence/is3/sealed-loader-authority.json`
- Result SHA-256: `1c31553c17734224c80bb7715f5e5ee82c8adce89b7c243b7c9aab31ab583979`

The manifest hashes every staged driver/owner source, the x86 payload and its
architecture. It binds the complete 31-file runner set, entry point, Proton,
both PowerShell images and sizes, all five relevant installed owners and the
complete installed software record. One runner member is legitimately empty;
it remains explicitly retained with size zero and the SHA-256 of empty bytes.
Entry-point and Proton executable artifacts cannot be empty.

The manifest's detached hash avoids a self-hash. The exact package directory is
the hashed required set plus the manifest. The campaign and staged supervisor
both require that set and seal. The result contains the manifest itself, the
source head/tree, and the identical complete envelope from all three sessions.
Baseline Windows override hashes match across the two outer sessions; the
Unix-override session's distinct Windows override is an explicitly declared
experimental difference.

## Completed comparison

| Independent session | PowerShell call result | Declared source-owned fallback |
| --- | --- | --- |
| Baseline | Launch success, exit 0, no requested side effect | Not selected |
| Unix target override after prefix initialization | CreateProcessW false, last-error 126; no returned helper exit | Launch success, exit 43 |
| Restored baseline | Launch success, exit 0, no requested side effect | Not selected |

Each session has exact IS2 root binding, positive process retirement and removed
scratch prefix. `campaign.py` consumed the immutable private proof files,
required the complete shared identity, projected bounded loader diagnostics and
atomically created the result without replacing any old evidence. No manual
JSON assembly selected the new disposition.

## Development boundaries retained

Two package attempts refused before any Windows launch: an unexpected compiler
PDB in the closed runtime set, then a validator that incorrectly rejected the
runner's declared empty member. The builder now keeps sidecars in a temporary
build directory and copies only the PE; the validator retains exact empty members.

The first sealed baseline completed with positive process cleanup but its driver
hit a collected-systemd-unit race at teardown. The campaign stopped before its
second session. Its result and hashes are retained in
`sealed-campaign-custody-failure.json`; its owned prefix was removed only after
exact inactive-unit and PID/start retirement verification. It was not used by the
completed comparison. `retire_unit()` now waits for a completed unit without
sending Stop, and requires positive cleanup even when a timed-out unit races
collection. The corrected, newly sealed campaign then ran exactly three sessions.

## Verification and preservation

24 IS3 tests passed locally and on the Deck. Each comparison-identity mutation
changes exactly one leaf; changed runner files, PowerShell images, adapter,
installed owners, each staged source, manifest seal, source generation, payload,
architecture and baseline environment all refuse. Additional tests cover exact
package sets, source/manifest tampering, runner ambiguity, atomic no-replace
results, campaign ordering/early stop and the collected-unit race.

Affected installer tests passed locally (46, with two Linux-only cases skipped);
AP12 runs the Linux suite. AP8 now writes its SHA256 inventory and architecture
record after both x64 and x86 builds/self-tests. AP8, AP12 and PX2 final-head
links/results belong on the PR so this retained result does not self-reference
a later evidence commit. AP10 is not applicable.

`sealed-final-readback.json` verifies installed software unchanged, all 307 retained
files and five projects unchanged, products/publications unchanged, service active,
two keepers healthy, capture off, zero DSP/maintenance leases, pending transactions
and stale transports. It verifies retirement and prefix deletion for the prior
four development sessions and all four sessions in this repair (the interrupted
baseline plus the completed trio).

No Native Access or other commercial session, interpreter installation, registry
mutation, replacement software, runner update or product publication occurred.
Native Access fallback and historical causality remain unproved. A genuine
interpreter remains deferred; the evidence proves only the source-owned honest
absence/fallback contract. Keep PR #114 draft and unmerged for rereview.
