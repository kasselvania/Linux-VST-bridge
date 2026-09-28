# PB0-R2 — installed-state convergence after UI1, UI2 and PKG0

## Scope and source

This read-only audit starts from post-PKG0 public main
`a35604781a703f35691bf193c8f301d94a02ed33`, tree
`189ae05f3b3917b653575add43b17362ee856ec6`. PKG0 merged with the
approved tree unchanged. [PB0-R PR #180](https://github.com/kasselvania/Linux-VST-bridge/pull/180)
and its 120-path classification are historical input. They are not a patch to
apply over canonical UI1, UI2 and PKG0. PB0 PR #179 remains draft.

The older audit's installed-source attribution was private UI2 commit
`8bf70de6b442bcac4086de4e27dadbdc7e76ea43`, tree
`146931737cb74781d67c8a9d368ea6bcffeadf3c`. This Git object and tree
still resolve exactly. The current Deck manager and frontend digest and
generation match that retained attribution; `software.json` does not itself
embed a source commit, so the commit-to-binary association still rests on the
retained earlier build/installation evidence. No new source provenance is
inferred from a digest alone.

The [record-owner ledger](PB0_R2_RECORDS.tsv) groups the current durable
records by owner and schema. A bounded before/after hash inventory and the
[sanitized Deck receipt](../evidence/pb0-r2/deck-readonly-2026-09-27.md)
record exactly what was inspected. The inventory hashes metadata bytes and
route identities; it does not copy record payloads, prefixes, installers,
projects, vendor account state or binaries into this repository.
The ledger's counted categories account for all 4,262 hashed JSON/TOML files,
plus six user routes and one selected external runner component. Generated
runtime metadata is classified separately from manager authority.

## Clean read-only canary

The installed immutable generation is
`6e4c53bf8533741d0e9423dacca9dd542b7329707c2748769cc495386a100385`.
The manager SHA-256 is
`414a953b8e7b1186ae1bf3ce969f09be8c5a011b12dcbcfb83f1c1637449ad71`;
the paired frontend SHA-256 is
`0963941036532478dab3c028e7b54cae8dca002f77a23bfa848e0dd60db797c9`.
The user service is active at that same generation. Activity readback is
operator schema 11, DSP 0/6, one normal keeper, maintenance 0, pending
transactions 0, stale transports 0 and cleanup confirmed. The FL workspace
reports no active session, install, uninstall or Serum product installation,
with cleanup confirmed. The latest operator operation is terminal. No manager
command capable of mutation was issued.

Current software, catalogue, registry and FL workspace metadata hashes match
the earlier PB0-R audit: `920b1282…`, `c0d848f3…`, `c2baa089…` and
`331e17e8…` respectively. The current catalogue is historical schema 3 with
the exact UI1 onboarding runtime field. The registry is schema 1, revision
165, with eight selected class entries. These include the six established
native products and two Lunacy entries. The workspace is schema 2, revision
21; WD1 Serum product state remains installed. This read did not qualify a
plug-in or reinterpret old observations.

## Package predecessor: structural checks and the exact gap

All selected `Software` artifacts reported in `software.json` are present,
digest-exact and non-writable. Manager/frontend pairing is exact; the selected
catalogue is digest-exact and non-writable. The user manager and frontend
links target their selected artifacts. The desktop, callback and service
files carry the existing owner markers, and the loaded user service points to
the current manager. No package transition journal exists. These are
**read-only structural predecessor facts**, not a package-adoption dry run.

PKG0's source model can retain a pre-PKG0 `Software` record as a predecessor
and copy the native catalogue byte-for-byte; its synthetic tests exercise a
populated catalogue and user-record preservation. A real successor package
would also have to carry the unchanged selected Windows host/source pair.
No package archive was built or installed for this audit, and no route plan
was applied on the Deck.

The selected Lunacy environment is the blocking owner. Environment
`7d8fce354e68595cdc20485f754a892d` at revision 4 uses runner
`proton-11.0-2c-dcomp-bg1-v4`, policy
`dcomp_wine_builtins_reference_v1`. Its exact runner file roster includes one
`native-command-session.json` component, SHA-256
`1fca37746647a975d80666f75f84c050770820b2c0f7de58352bbf7ed4f40c30`.
The current registry selects that environment for both vanilla BEAM class
`ABCDEF019182FAEB4C756E6170726F43` and BEAM Filter class
`ABCDEF019182FAEB4C756E61666C744C`. Both retain exact managed publication
revisions. This is current selected execution authority, not an unused
private source file.

Public canonical source can deserialize the runner policy and verify its
pinned files. It does **not** contain the private UI2/BG1 native Proton
command-session handling in `bridge-manager/runtime/session.py` or the
matching remote process-group retirement in
`bridge-manager/runtime/ownership.py`. The private source diff contains the
component admission, keeper endpoint, instance launch and exact retirement
behavior. Its `experimental_runner.rs` also contains BG1 V4 candidate and
transition bindings absent from public main; the retained BG1 transition and
rollback records therefore need an exact owner audit. Merely accepting the
component as another runner file would silently
lose the selected runtime's execution and cleanup contract when PKG0 replaces
the supervisor scripts. This is a **BG1 shared-runtime source-owner gap**.

Consequently the full canonical readback/projection gate, exact package
predecessor preservation gate and PB0 merge-forward permission are **not
passed**. A read-only snapshot from a staged public binary could at most
prove decoding; it could not prove that this selected runner remains usable
or safely retired. No staged manager was run against the Deck after this gap
was established. No `package-adopt`, rollback, recovery, route switch or
service restart occurred.

## Required next integration

Integrate and independently review the selected BG1 native command-session
runtime and retirement owner, plus the exact retained BG1 V4 transition and
rollback interpretation needed by the current runner. It must bind the
component and runner exactly, preserve all six established
publications and the FL workspace, and demonstrate launch and retirement
under source-owned fixtures. Do not import all private BG1 experiments,
graphics evidence, RPR0, BETA0 or distribution work. After that integration,
refresh this owner ledger and run the exact installed-state canonical snapshot,
Activity and package-predecessor preservation gates. The schema-11 operation
requests are historical recovery inputs under canonical UI2; new live
requests remain schema 10. Any further exact retained-record gap discovered
then requires its own owner before PB0 can replace the Deck generation.

No PB0, PKG0 or manager generation was installed. No physical readiness or
support-export claim follows from this audit.
