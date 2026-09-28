# PB0-R3 Deck read-only receipt — 2026-09-28

## Installed beforestate

- Selected generation:
  `6e4c53bf8533741d0e9423dacca9dd542b7329707c2748769cc495386a100385`.
- Selected manager/frontend SHA-256:
  `414a953b8e7b1186ae1bf3ce969f09be8c5a011b12dcbcfb83f1c1637449ad71` /
  `0963941036532478dab3c028e7b54cae8dca002f77a23bfa848e0dd60db797c9`.
- User service active/running and bound to that manager. Installed Activity
  schema 11: DSP 0/6, keepers 1, maintenance 0, pending transactions 0,
  stale transports 0, cleanup uncertainty false; latest operation completed.
- FL workspace schema 2: ready, cleanup confirmed, no active installer,
  session, uninstall or Serum product installation.
- No DAW or plug-in process was observed. No manager action was submitted.

The before inventory used the same bounded metadata and six-route method as
PB0-R2. It counted 4,262 JSON/TOML records. The SHA-256 of the sorted
path-to-digest map was
`ce9b45db560a683b6a994a6a9753dade67d03d272105c5f0005b365a9a8ee522`;
the SHA-256 of the six-route identity map was
`a227c04b7b950378c91b2e23f7388f92ef9222d26255a3defa4f426c6cf530d8`.
Both maps matched the PB0-R2 afterstate. Their private path lists and record
values remain in local temporary audit material and are not committed.

## Staged canonical readback

AP12 built the manager/frontend pair from post-BG1 main commit
`62d556cfea57b17b567c5374a7a42b78fa22780c`, tree
`f0f9c430e303447ea32129cb85d2532b0174a816`.
Their SHA-256 values were
`cfa47b7dc9febdb1b1dbe6831e0143384f79c83bd8977ddddebe0d9b137400ed` /
`5b44542748b9bdea06dfc2e73b567f471eef663e69bf11a1695045bc8e2b2f29`.
They were staged in temporary storage only; the selected commands and service
were not changed. Canonical Activity succeeded as schema 10 with the same
capacity state. Canonical Snapshot ran twice within a 90-second bound, about
36–37 seconds per read, and both replies were exactly equal. The installed
schema-11 Snapshot took about 39 seconds after an initial 30-second timeout.

The canonical and installed snapshots had equal state tokens and matched all
16 product identities and dispositions, four Setup records, five environments,
eight onboarding rows and one ready FL workspace. The installed projection's
additional `host_readiness` and `native_daws` fields were transient private
features, not a change to these retained product identities.

The source-owned audit probe accepted the exact retained BG1 V4 history and
both selected Lunacy class bindings, including pinned command-session component
SHA-256 `1fca37746647a975d80666f75f84c050770820b2c0f7de58352bbf7ed4f40c30`.
It decoded six imported installer records,
20 retained candidate identities, 28 observations, three reviews, one guided
check, one guided result and 177 historical requests, five of which used the
private schema-11 wire generation. Historical candidates were not misreported
as current after environment transitions.

The six-file internal candidate adoption fixture had manifest SHA-256
`a9a7223bcb545373f975e6b36e096b4c418705c25c21ab82d8f17811723a1612`.
The no-write PKG0 plan returned predecessor
`6e4c53bf8533741d0e9423dacca9dd542b7329707c2748769cc495386a100385`,
planned successor
`2bf3a3561c43c6f7c0146296deca1f99cfe23e6b9b4abea7d93a9afdd59dd11b`,
the unchanged catalogue digest, and six `exact` route classifications.
No package adoption, recovery, rollback, daemon reload or route switch ran.

## Final afterstate

The second bounded inventory again counted 4,262 files and produced the same
two aggregate hashes. All 4,262 file digests and six routes were identical to
beforestate. Installed manager/frontend digests, software selection,
catalogue, registry, FL workspace, preparations, UI2 histories, environments,
runner records and rollback files remained unchanged. The service remained
active on the original generation with DSP 0/6, keepers 1, maintenance 0,
pending 0, stale transports 0, cleanup uncertainty false and FL ready.
No package transition journal or BG1 command socket exists.

One old BG1 command-session descriptor remains from a terminal session; its
owner PID is dead, its report says cleanup confirmed, and there is no matching
socket or lease. Its bytes were present in the PB0-R2 afterstate and unchanged
here. It is retained as historical residue, not a newly created active owner.

No installed generation, publication, runner, environment, workspace, vendor
authorization, project, audio setting or rollback authority was selected or
modified. No BEAM, Bitwig or FL session was launched.
