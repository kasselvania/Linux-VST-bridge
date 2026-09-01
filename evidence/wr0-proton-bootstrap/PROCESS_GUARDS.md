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
| `RUNTIME_ROOT_1` | `runtime_root` | `pressure-vessel-wrap` | 1000 | 4403501 | `SUPERVISOR` | `true` | `true` | `5ca718656a22579a0ad88024a5c71c8ba153cc00a60608f73ab910de206b9dc0` |
| `OWNED_DESCENDANT_1` | `owned_descendant` | `x86_64-linux-gnu-capsule-capture-libs` | 1000 | 4403552 | `RUNTIME_ROOT_1` | `true` | `true` | `60b73d4ab2e523cee54666b29ac334f3139ee996451bc438f25d534647bc390d` |
| `OWNED_DESCENDANT_2` | `owned_descendant` | `i386-linux-gnu-capsule-capture-libs` | 1000 | 4403572 | `RUNTIME_ROOT_1` | `true` | `true` | `75cbe5d5972a69f8122cf5ac518082b77942cd2816246197bd865ff7f4441d91` |
| `RUNTIME_COMPONENT_1` | `runtime_component` | `pv-adverb` | 1000 | 4403603 | `RUNTIME_ROOT_1` | `false` | `false` | `4a4c26a142f268d6b07327a5734c3af579ff898968681745b76665e2ff81e5cd` |
| `PROTON_1` | `proton` | `python3.13` | 1000 | 4403611 | `RUNTIME_COMPONENT_1` | `false` | `false` | `ff1afd9317cf011dcb9dbeab2d093a37d055106c2743992adefd8791bf756f61` |
| `WINESERVER_1` | `wineserver` | `wineserver` | 1000 | 4403617 | `RUNTIME_COMPONENT_1` | `false` | `false` | `3aeb848d1255c335637be66aa54c39e7737018f9fd470b85c9329754f4a0f27d` |
| `WINE_1` | `wine` | `wine-preloader` | 1000 | 4403617 | `PROTON_1` | `false` | `false` | `bd4c862cb511c79705ccb9597aa4a536145556e92f91530158e1f88d18161b9b` |
| `OWNED_DESCENDANT_3` | `owned_descendant` | `wine64-preloader` | 1000 | 4403622 | `RUNTIME_COMPONENT_1` | `false` | `false` | `0d6e85fa7e4b5c5d1e83c688412c3eb0adf5a9b772cc36ab1456eadd79f9216a` |
| `WINE_SUPPORT_1` | `wine_support` | `wine64-preloader` | 1000 | 4403625 | `RUNTIME_COMPONENT_1` | `false` | `false` | `2ae35187c4e51062decd3183e28f567c4cb30406af267018306e6f973b60468f` |
| `WINE_SUPPORT_2` | `wine_support` | `wine64-preloader` | 1000 | 4403628 | `RUNTIME_COMPONENT_1` | `false` | `false` | `b1373c6a7fb385910864caaf108adba68f8327a5559fd2d94a48c47633111578` |
| `OWNED_DESCENDANT_4` | `owned_descendant` | `wine64-preloader` | 1000 | 4403631 | `RUNTIME_COMPONENT_1` | `false` | `false` | `bd803d76153ae607b9d2b858cce0c6e1ff244494f23f439352c093c9ed984e40` |
| `OWNED_DESCENDANT_5` | `owned_descendant` | `wine64-preloader` | 1000 | 4403661 | `RUNTIME_COMPONENT_1` | `false` | `false` | `8f5adc8a6b3f5e6c102e9260e7cad918c660f47480106ea340324bd6dd9484ff` |
| `WINE_SUPPORT_3` | `wine_support` | `wine64-preloader` | 1000 | 4403665 | `RUNTIME_COMPONENT_1` | `false` | `false` | `e30841675dc3208be57856ac30bb173f2ea1d56b978ee8cafdc850371c9258af` |
| `WINDOWS_COMMAND_1` | `windows_command` | `wine64-preloader` | 1000 | 4403757 | `RUNTIME_COMPONENT_1` | `false` | `false` | `2d74d3a964497e6c13f3a0ad8548d5ecd204d9aa326d7f9548b0e72699236430` |

The sentinel used a separate process group/session and the same safe `python` comm family as the Proton script where available. Its PID was held only in raw local supervision state; evidence retains only its safe basename/start identity hash and the survival assertion.
