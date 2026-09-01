# WR0 owned environment

| Fact | Result |
|---|---|
| Classification | `created_by_slice` |
| Path | `<HOME>/.local/share/linux-vst-bridge/environments/wr0-proton11` |
| Marker schema | `linux-vst-bridge-wr0-environment/v1` |
| Transaction ID | `wr0-20260901T045337Z-caf9f4eaf52d2d34` |
| Created at | `2026-09-01T04:53:37Z` |
| Runner binding | `2d64df1d36786ca2d0e955c553005423dc2b5bdd714bd0a17872622e33912547` |
| Contract-source binding | `linux-vst-bridge-wr0-contract-source/v1` / `887b148b7862038fd8fc0af52146ecc7452a5fbf82c22c5af8eaec6fe47504b8` |
| Tracked workload binding | `4518ca37b8d7e005f01b441de5273447b70fa7c8c9c3d2b9c194a91782cfecac` |
| Environment identity SHA-256 | `447e6d4dfccfbebe7be44cc521e8ed192bfad4ab1fe1f16673d391161db73c24` |
| Reviewed predecessor identity | `c301db6f41257925ab7bec9e1f64118e7a176a5b79bf97b8bdbfb98a40bcb497` |
| Replacement phase | `evidence_finalized` |
| New environment durably committed | `true` |
| Predecessor retired | `true` |
| Rollback available | `false` |
| Retirement pending | `false` |
| Durable commit-record schema | `linux-vst-bridge-wr0-replacement-commit/v1` |
| Durable commit-record SHA-256 | `aba3913537590b6c1302cfd473cb29e201b68342b58e8d92cdc6d09fe61ec6f1` |
| Run 1 / Run 2 identity agreement | `true` |
| Prefix device/inode | `66312` / `5835828` |
| Promotion | `verified atomic sibling rename` |
| Stage/backup siblings after completion | `0` |

Bounded prefix top-level roster: `[{"name": ".update-timestamp", "type": "regular_file"}, {"name": "dosdevices", "type": "directory"}, {"name": "drive_c", "type": "directory"}, {"name": "system.reg", "type": "regular_file"}, {"name": "user.reg", "type": "regular_file"}, {"name": "userdef.reg", "type": "regular_file"}]`.

Selected registry hashes are retained in `fixture.json`; no registry content, machine GUID, SID, credential, browser state, or complete prefix roster is retained.

The predecessor transaction `wr0-20260901T040545Z-faed70d04e3b9661` was first moved to one exact recoverable sibling. The repaired environment was promoted only after Run 1; the predecessor remained untouched through Run 2, exit 37, the two actual bad-gate paths, held-command cleanup, preservation comparison, and commit-ready evidence validation. An atomically written, file-fsynced, directory-fsynced, read-back commit record made the new environment authoritative before predecessor deletion began. Predecessor retirement was then proved by parent fsync, absence readback, exact final-environment readback, and a retired commit-record update before final evidence was published.
