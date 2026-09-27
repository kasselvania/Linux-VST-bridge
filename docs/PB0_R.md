# PB0-R — installed authority convergence audit

PB0-R starts from public canonical main `80e66bc050975a35a80c85bb2a1da3d7738f5c63`,
tree `585214bbbd44401a085c85cd9efcd0a2f7cdb979`, after the independently
reviewed WD1 PR #169 merged. The installed private UI2 source selected for
comparison is `8bf70de6b442bcac4086de4e27dadbdc7e76ea43`, tree
`146931737cb74781d67c8a9d368ea6bcffeadf3c`. Their merge base is
`65f113b463ed1dbba7d142f1b833720575932a0d`.

This is a read-only classification, not permission to replace the Deck manager.
PB0 PR #179 remains draft at `2b0dba9c6ecc38972f308d951e5a72c31a172cab`.

## Exact comparison

The tree comparison has 120 changed paths. Every path is recorded in the
[classification ledger](PB0_R_DIFF.tsv). The dispositions are:

| Disposition | Paths | Meaning for PB0-R |
|---|---:|---|
| Already canonical or supplied by WD1 | 0 changed paths | WD1's FL schema-2 application and Serum product owner is in main. Private UI1 adds only setup grouping around it. |
| Shared durable authority candidate | 1 | The installed catalogue's typed `onboarding_runtime` format must be parsed and validated without silently enabling UI1's default-runner behavior. |
| Independent feature needing its own integration | 69 | UI1 onboarding and installer presentation; UI2 guided check/result records and recovery; BG1 runner transition/runtime; BETA0 package/readiness; RPR0 REAPER detection; DIST0/CACHY0 tools; mixed operator and frontend files. Whole-file or whole-tree import is prohibited. |
| Excluded private documentation/evidence | 50 | Private research, draft beta terms, physical evidence, screenshots and workstream documentation are not PB0-R source. Their relevant claim must be reviewed under the owning feature. |

`operator_model.rs`, `operator_cli.rs`, and frontend files contain changes from
several independent owners. A file-level merge would silently import unrelated
actions and claims. UI1's installer presentation sidecar and UI2's guided
result records are durable data, but the private changes also implement new
user workflows and mutation/recovery authority. They require separately
reviewed canonical owner PRs before PB0-R can claim to own their records.

## Current Deck readback

Bounded, read-only file inspection found:

- Managed catalogue schema 3, SHA-256
  `c0d848f3875d29e6532e7c11e6ecafa5ec975165ab1fdb34b5adc6ba650988ba`,
  with `onboarding_runtime` schema 1 and exact standard runner key
  `72d6da6c18cd30cb9027952cbbcc7ca2e7753c86933e310a7c8db32d8261be63`.
- Registry schema 1, revision 165, eight exact class entries, SHA-256
  `c2baa0894b20c00feec6c2f946c87287aea96b0f6280b49d171127a08a9b67db`.
  The six established native classes and two Lunacy classes remain selected as
  found; this audit performed no mutation.
- Current FL workspace schema 2, ID `762bafec213cefe91dbe14d67ee1b1c6`,
  revision 21, SHA-256
  `331e17e877de59c2f66b40eff09a445e8fca97d9ff4cff4cde3892019cc5b70d`.
  It retains two FL installation records, one uninstall record and one
  installed Serum product record. Its schema-1 runner-transition backup is
  historical, not the current workspace.
- One UI1 installer-presentation sidecar, one completed UI2 guided check with
  intent/inspection/candidate/completion stages, and one completed UI2 guided
  result. Their exact operation identities remain in manager custody.
- Managed software SHA-256
  `920b1282f13ee847eb91b13aa5f120d6f59469a64e8a2b6bb0a1cc5cb62c1b49`.
  The ordinary manager binary still resolves to immutable generation
  `6e4c53bf8533741d0e9423dacca9dd542b7329707c2748769cc495386a100385`.

The read-only operator snapshot command timed out after 20 seconds, so this
audit does not claim a fresh full schema-11 projection or a clean live-service
assessment. No record or package was rewritten.

## Integration boundary

Public main already owns WD1's durable FL records and the registry/publication
schema. It does not yet parse the installed catalogue field or project the
completed UI1/UI2 records. A catalogue-only decoder change would remove the
first error but would not establish ownership of those histories or preserve
the current product workflow. PB0-R therefore stops before a manager-code
change. UI1 and UI2 need separate reviewed canonical integration PRs, in that
order. BETA0 package binding needs a separate owner review before the eventual
normal immutable PB0 Deck install. BG1, RPR0, DIST0 and CACHY0 are not imported
by PB0-R merely because the installed private tree contains them.

After the owner PRs establish exact record readback and rollback continuity,
PB0-R may add only the remaining shared durable-format adaptation and its
preservation tests. Then PB0 PR #179 can merge canonical main normally and
resume its exact-head and physical gate. No PB0 or PB1 code belongs in this
audit.
