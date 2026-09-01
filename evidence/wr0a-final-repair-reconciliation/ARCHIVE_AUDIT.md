# WR0 final-repair archive audit

| Fact | Result |
|---|---|
| Archive | `52be94316664f88a19df630e164105a0ca50b875` / `21601814bd352114fe2e55833c88a81e47e13e41` |
| Canonical packet | `14` files / `13` payload hashes |
| Canonical hash-manifest blob | `5369c4cb3e55314fd2c2589c9394518b40f5023f` |
| Retained production cases | `80/80` |
| Non-live / actual live cases | `75` / `5` |
| Final-repair helper cases | `10` |
| Final WR0 result | `WR0_COMPLETE` |

The ten final-repair cases exercise the production predecessor backup and
pre-commit recovery helpers, including rename/fsync, verification, stale-event,
unknown-object, and guarded-success boundaries. This reconciliation does not
rerun a Windows workload and does not promote archive evidence into broader
product claims.
