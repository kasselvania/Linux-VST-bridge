# WR0 process guards and ancestry

| Fact | Result |
|---|---|
| Pre-run forbidden counts | `{"bitwig": 0, "proton": 0, "runtime": 0, "umu": 0, "validator": 0, "wine": 0, "yabridge": 0}` |
| Post-run forbidden counts | `{"bitwig": 0, "proton": 0, "runtime": 0, "umu": 0, "validator": 0, "wine": 0, "yabridge": 0}` |
| Ordinary Steam reported separately | `true` |
| Repository supervisor owned each observed runtime root | `true` |
| Steam game-launch ancestor observed | `false` |
| Final WR0 descendant count | `0` |
| Live held-command cleanup | `passed` |
| Held descendants outside initial group | `11` |
| Held final owned count | `0` |
| Unrelated same-comm sentinel survived | `true` |
| Disposable held stage removed | `true` |
| Successful final environment unchanged | `true` |
| Bitwig/validator/UMU/yabridge participation | `false` |

Ordinary Steam client/web-helper processes were neither terminated nor classified as WR0. Each observed launch used a newly created Unix process group, while the census followed the complete exact-root descendant tree across process-group/session boundaries. Retained identities use UID, `/proc` start ticks, safe basenames, exact parent labels, group/session membership classifications, and one-way identity hashes; PIDs, command lines, environments, maps, and unrelated process data are absent.

## Actual gate-withheld cleanup topology

| Label | Role | Safe executable | UID | Start ticks | Parent | Root PG | Root session | Identity SHA-256 |
|---|---|---|---:|---:|---|---|---|---|
| `RUNTIME_ROOT_1` | `runtime_root` | `pressure-vessel-wrap` | 1000 | 4165419 | `SUPERVISOR` | `true` | `true` | `5252045df74418de126f3f1236af53249e3dd8c1c678a72facd78ff19a51b46d` |
| `OWNED_DESCENDANT_1` | `owned_descendant` | `x86_64-linux-gnu-capsule-capture-libs` | 1000 | 4165471 | `RUNTIME_ROOT_1` | `true` | `true` | `4e33c6bf4608c02c3052f56ddc29b33e37d2c79484d9cd6d4dbe0adf4eb4cef8` |
| `OWNED_DESCENDANT_2` | `owned_descendant` | `i386-linux-gnu-capsule-capture-libs` | 1000 | 4165491 | `RUNTIME_ROOT_1` | `true` | `true` | `67e349f5dc8d9ec52c617029efe97f165c281a037c14498cd66a9f2ba3dd2d0e` |
| `RUNTIME_COMPONENT_1` | `runtime_component` | `pv-adverb` | 1000 | 4165522 | `RUNTIME_ROOT_1` | `false` | `false` | `b54264b98564867295d766c54d1da5c000b0921d7c1fd2da49a7f02878445792` |
| `PROTON_1` | `proton` | `python3.13` | 1000 | 4165530 | `RUNTIME_COMPONENT_1` | `false` | `false` | `62b726ee6b1432909d90947312a9379d125ed74d57c4aa79143f5e2f4f88908c` |
| `WINE_1` | `wine` | `wine-preloader` | 1000 | 4165535 | `PROTON_1` | `false` | `false` | `8a560df574159cd4850f47753e7ab57daecc10932c9e3fa94ef1a4977270b367` |
| `WINESERVER_1` | `wineserver` | `wineserver` | 1000 | 4165536 | `RUNTIME_COMPONENT_1` | `false` | `false` | `08ff5173a44f13de4aefe234a271c6f7ed6cb8fd63ad67897c23d34ed81d0261` |
| `OWNED_DESCENDANT_3` | `owned_descendant` | `wine64-preloader` | 1000 | 4165542 | `RUNTIME_COMPONENT_1` | `false` | `false` | `6a361ea9c88c820f882b45e01287bd8fb018b910a52708908af67f2c0755a3f2` |
| `WINE_SUPPORT_1` | `wine_support` | `wine64-preloader` | 1000 | 4165545 | `RUNTIME_COMPONENT_1` | `false` | `false` | `5d31a53fe0477033202bdc96cf759b7b620419c65dde5e9b466e0cf03fe2a324` |
| `WINE_SUPPORT_2` | `wine_support` | `wine64-preloader` | 1000 | 4165548 | `RUNTIME_COMPONENT_1` | `false` | `false` | `724e69e85179967d8146eaa3a1b12f7c23279713f725faa761fed029c46ef8fb` |
| `OWNED_DESCENDANT_4` | `owned_descendant` | `wine64-preloader` | 1000 | 4165551 | `RUNTIME_COMPONENT_1` | `false` | `false` | `314d241705d0813c826d9237f5c7d733c77829fe63b1eaf343b984b810067a34` |
| `OWNED_DESCENDANT_5` | `owned_descendant` | `wine64-preloader` | 1000 | 4165582 | `RUNTIME_COMPONENT_1` | `false` | `false` | `525cf71e8114edcd4bb7a33a3587962407fe7b6d997736e1bb6b1a92c4b0381d` |
| `WINE_SUPPORT_3` | `wine_support` | `wine64-preloader` | 1000 | 4165585 | `RUNTIME_COMPONENT_1` | `false` | `false` | `db52ac81d107e08a6ac04070fb2fce051c2a921672b19b16f7e3fd26b65d5fe7` |
| `WINDOWS_COMMAND_1` | `windows_command` | `wine64-preloader` | 1000 | 4165688 | `RUNTIME_COMPONENT_1` | `false` | `false` | `4f250bb6f86c256b65ea3466f2105a83e3ac8d65cdfdb359af432417e6e261c4` |

The sentinel used a separate process group/session and the same safe `python` comm family as the Proton script where available. Its PID was held only in raw local supervision state; evidence retains only its safe basename/start identity hash and the survival assertion.
