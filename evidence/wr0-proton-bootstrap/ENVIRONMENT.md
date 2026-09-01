# WR0 owned environment

| Fact | Result |
|---|---|
| Classification | `created_by_slice` |
| Path | `<HOME>/.local/share/linux-vst-bridge/environments/wr0-proton11` |
| Marker schema | `linux-vst-bridge-wr0-environment/v1` |
| Transaction ID | `wr0-20260901T053317Z-183afd5bbf0736e4` |
| Created at | `2026-09-01T05:33:17Z` |
| Runner binding | `2d64df1d36786ca2d0e955c553005423dc2b5bdd714bd0a17872622e33912547` |
| Contract-source binding | `linux-vst-bridge-wr0-contract-source/v1` / `c6543004fbcd70252393f0e0cab4f9ad7a31e85f433ce3e903e690244cc79541` |
| Tracked workload binding | `4518ca37b8d7e005f01b441de5273447b70fa7c8c9c3d2b9c194a91782cfecac` |
| Environment identity SHA-256 | `d3ed38d7ed53e9a5973cbf22fc504f2479dbdd15616fe1bfc1d5e1cd0bf5f4c4` |
| Reviewed predecessor identity | `447e6d4dfccfbebe7be44cc521e8ed192bfad4ab1fe1f16673d391161db73c24` |
| Replacement phase | `evidence_finalized` |
| New environment durably committed | `true` |
| Predecessor retired | `true` |
| Rollback available | `false` |
| Retirement pending | `false` |
| Guarded predecessor rename | `true` |
| First parent fsync inside rollback guard | `true` |
| Exact backup snapshot verified | `true` |
| Durable commit-record schema | `linux-vst-bridge-wr0-replacement-commit/v1` |
| Durable commit-record SHA-256 | `917c939c605f457376b0bdd53e06d252d4aecee4b39728850b2e1a61aa102b3e` |
| Run 1 / Run 2 identity agreement | `true` |
| Prefix device/inode | `66312` / `5866359` |
| Promotion | `verified atomic sibling rename` |
| Stage/backup siblings after completion | `0` |

Bounded prefix top-level roster: `[{"name": ".update-timestamp", "type": "regular_file"}, {"name": "dosdevices", "type": "directory"}, {"name": "drive_c", "type": "directory"}, {"name": "system.reg", "type": "regular_file"}, {"name": "user.reg", "type": "regular_file"}, {"name": "userdef.reg", "type": "regular_file"}]`.

Selected registry hashes are retained in `fixture.json`; no registry content, machine GUID, SID, credential, browser state, or complete prefix roster is retained.

The predecessor transaction `wr0-20260901T045337Z-caf9f4eaf52d2d34` was moved to one exact transaction-derived recoverable sibling under a single guarded operation covering rename, the first parent-directory fsync, exact backup verification, and exact restoration on every post-rename failure. Outer pre-commit recovery classifies the physical destination/backup state and never treats the in-memory `backup_created` event as authority. The repaired environment was promoted only after Run 1; the predecessor remained untouched through Run 2, exit 37, the two actual bad-gate paths, held-command cleanup, preservation comparison, and commit-ready evidence validation. An atomically written, file-fsynced, directory-fsynced, read-back commit record made the new environment authoritative before predecessor deletion began. Predecessor retirement was then proved by parent fsync, absence readback, exact final-environment readback, and a retired commit-record update before final evidence was published.
