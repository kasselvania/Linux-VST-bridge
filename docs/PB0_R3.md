# PB0-R3 — canonical Deck readback and package predecessor

## Claim and source boundary

PB0-R3 starts from public main merge `62d556cfea57b17b567c5374a7a42b78fa22780c`,
tree `f0f9c430e303447ea32129cb85d2532b0174a816`, after the reviewed
BG1-R0 integration. Its one claim is that current canonical source can read,
project and preserve the manager-owned state selected on the Deck, and can
compute one exact PKG0 successor with that state as rollback predecessor,
without changing the Deck. The [PB0-R2 owner ledger](PB0_R2_RECORDS.tsv)
remains the record classification; this gate does not relabel its 4,262 records.

The installed generation remains
`6e4c53bf8533741d0e9423dacca9dd542b7329707c2748769cc495386a100385`.
Its manager and frontend SHA-256 values remain
`414a953b8e7b1186ae1bf3ce969f09be8c5a011b12dcbcfb83f1c1637449ad71`
and `0963941036532478dab3c028e7b54cae8dca002f77a23bfa848e0dd60db797c9`.
The earlier private-source attribution remains commit
`8bf70de6b442bcac4086de4e27dadbdc7e76ea43`, tree
`146931737cb74781d67c8a9d368ea6bcffeadf3c`; `software.json` does not
itself embed that source commit. No new provenance is inferred from matching
binary digests.

## Canonical readback

The fixed installed service was active and idle at the canary: DSP 0/6,
maintenance 0, pending transactions 0, stale transports 0, cleanup uncertainty
false, and one existing keeper. FL Studio was ready with confirmed cleanup and
no active installer, session or uninstall. The selected catalogue remained
historical schema 3 with `onboarding_runtime`; the registry remained revision
165 with eight selected class entries.

The paired Linux manager/frontend built from the exact post-BG1 tree were
staged under temporary storage only. Canonical Activity returned operator
schema 10 and the same clean capacity state. Two canonical Snapshots completed
within a 90-second bound, in approximately 36–37 seconds each, and were
canonically identical, including their state token. The installed schema-11
Snapshot completed in approximately 39 seconds. A first 30-second installed
snapshot attempt timed out; PB0-R3 does **not** claim responsive navigation or
startup from these timings.

The canonical and installed snapshots have the same state token, 16 product
rows, four installer setups, five environment rows, eight onboarding rows,
one ready FL workspace, and zero active sessions. Every product's class ID,
name, version, environment, module digest and disposition matched. The
installed private projection has two additional transient fields,
`host_readiness` and `native_daws`. No retained record in the PB0-R2 ledger
depends on either field; they are not silently parsed as canonical schema-10
fields. PB0 must build its readiness projection on the current model, and
RPR0/BETA0 behavior remains outside this gate.

The source-owned, read-only audit probe separately decoded six imported
installer records, 20 immutable candidate identities, 28 observations, three
reviews, one guided check, one guided result, and all 177 historical operator
requests, including five private schema-11 requests. Older schema 1–9 requests
remain historical evidence; executable UI2 recovery retains its narrower
schema-10/11 authority. Historical candidates are decoded and their evidence
bound to their original identity. They are not verified against today's
environment revision as if they were current candidates.

The same probe verified the exact retained BG1 V1/V3/V4 transition chain,
selected environment `7d8fce354e68595cdc20485f754a892d`, V4 runner
`proton-11.0-2c-dcomp-bg1-v4`, pinned command-session component SHA-256
`1fca37746647a975d80666f75f84c050770820b2c0f7de58352bbf7ed4f40c30`, and both
selected BEAM class bindings. This was history verification only; no keeper,
runner, plug-in or DAW was launched.

## PKG0 predecessor plan

The planning function reuses PKG0's verified predecessor, unchanged host/source
pair, generation-record construction and six-route preflight before any staging
in ordinary adoption. It creates no directory, journal, generation or route.
Focused tests prove a no-write plan, exact adoption identity, same-generation
reuse, exact/missing/stale package-owned route classification, foreign-route
refusal and changed-host refusal.

One **internal adoption fixture**, not a customer Arch package, was built from
the exact paired post-BG1 Linux binaries, Python 3.13 bytecode compiled from
that commit's canonical `session.py` and `ownership.py`, and the unchanged
selected Windows host/source pair. Its six-file roster and archive were
re-read byte-for-byte. The externally selected Proton/SLR runners were reduced
to a path-free identity/digest manifest; no runtime was bundled. The fixture
builder is [predecessor_fixture.py](../tools/pkg0/predecessor_fixture.py).
The candidate manifest SHA-256 is
`a9a7223bcb545373f975e6b36e096b4c418705c25c21ab82d8f17811723a1612`;
the internal archive SHA-256 is
`403551a6dae4e4e57fcaf5219d02136616967c0de58cec8de21f66e0cb0f0628`.
Neither archive nor selected host bytes are committed.

Against the installed predecessor, the audit computed successor identity
`2bf3a3561c43c6f7c0146296deca1f99cfe23e6b9b4abea7d93a9afdd59dd11b`.
The plan identifies the selected generation separately, confirms this is a
new successor rather than a same-generation no-op, and makes the exact
selected generation its rollback predecessor. The native catalogue
digest remains `c0d848f3875d29e6532e7c11e6ecafa5ec975165ab1fdb34b5adc6ba650988ba`.
All six current routes classified `exact`. The installed registry, all selected
publications and revision ancestry, environments, native runner references,
installer/onboarding history, UI2 evidence, FL/WD1 state, projects,
preferences and vendor authorization were outside the plan's write set.
No `/usr` files, service unit, command link or `software.json` were changed.
This is a predecessor/adoption-plan result, not a signed complete release
package or physical package switch.

## Preserved physical state and one retained residue

The exact PB0-R2 inventory method found 4,262 bounded JSON/TOML metadata
files before and after. Every file hash and all six route identities matched,
including the PB0-R2 afterstate. The selected service still points to the
original manager and remains active. The FL workspace, catalogue and registry
high-value hashes match PB0-R2. No package-transition journal, new candidate,
new operation or command socket appeared. The sanitized
[Deck receipt](../evidence/pb0-r3/deck-readonly-2026-09-28.md) and
[machine-readable result](../evidence/pb0-r3/result.json) contain the exact
hash and count comparison without private paths or payloads.

One **preexisting** BG1 `command-session.json` remains in a retired session
directory. Its recorded owner PID no longer exists, its result reports
`ready=false` and `cleanup_confirmed=true`, and it has no matching command
socket or lease. It was already included in the byte-identical PB0-R2/PB0-R3
inventory and was not created or removed by this gate. The canonical runtime
ignores a dead owner while seeking exactly one live keeper. This residue is
retained honestly; PB0-R3 does not claim a new BEAM launch or repair it.

## PB0 forward decision

PB0 PR #179 is still draft at `2b0dba9c6ecc38972f308d951e5a72c31a172cab`.
Its merge base is private commit `65f113b463ed1dbba7d142f1b833720575932a0d`;
its operator contract was schema 8. Current canonical source is schema 10 and
now owns UI1, UI2, PKG0 and BG1. A direct merge of the old branch would mix
old operator assumptions with these accepted owners. A replacement PB0
integration from current main is cleaner: carry forward the bounded platform
collector, four-outcome readiness model, product contract and sanitized export;
bind them to current UI1/UI2 and PKG0 authority; and address the previously
reviewed mixed-product aggregation, offered SupportExport and explicit
authority-failure findings. No PB0 source was imported here.

**Disposition:** this read-only convergence gate passed. PB0 may begin a
separate current-main source integration after independent PB0-R3 review.
No PB0 installation or physical readiness/export acceptance is authorized by
this result. Clean-machine distro support, customer packaging, BEAM graphics
or audio performance, and new plug-in qualification remain separate claims.
