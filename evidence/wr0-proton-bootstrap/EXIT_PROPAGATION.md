# WR0 expected exit propagation

Classification: `expected_failure`.

| Fact | Result |
|---|---|
| Run number | `37` |
| Nonce | `e923d07248d8dc20b68d9651e15c736a` |
| Started | `2026-09-01T04:53:54Z` |
| Finished | `2026-09-01T04:53:57Z` |
| Duration ms | `3146` |
| Windows/supervisor exit | `37` / expected `37` |
| Timeout | `false` |
| Root start ticks | `4163387` |
| Root identity SHA-256 | `5bd76ade8b2313c798eb62c897ab617da80043c5ca283bc77bd7fda968cbb6ca` |
| Environment identity SHA-256 | `447e6d4dfccfbebe7be44cc521e8ed192bfad4ab1fe1f16673d391161db73c24` |
| Clean descendants after completion | `0` |
| Steam game-launch ancestor | `false` |
| Process-role assertion | `passed` |
| Missing required roles | `[]` |
| Observed process scope | `complete_bounded_exact_root_descendants` |
| Descendants outside initial process group | `11` |
| Command first observed us | `2893430` |
| Fresh pre-gate revalidation us | `2949835` |
| Gate committed us | `2962222` |
| Command completed us | `3102508` |
| Causal gate order | `true` |
| Strict command vector | `true` |
| Contract-source SHA-256 | `887b148b7862038fd8fc0af52146ecc7452a5fbf82c22c5af8eaec6fe47504b8` |
| Tracked workload SHA-256 | `4518ca37b8d7e005f01b441de5273447b70fa7c8c9c3d2b9c194a91782cfecac` |

## Exact normalized stdout

```text
WR0_MAGIC=LINUX_VST_BRIDGE_WR0_V1
WR0_NONCE=e923d07248d8dc20b68d9651e15c736a
WR0_RUN=37
WR0_ARCH=x86_64
WR0_READY=waiting_for_supervisor
WR0_GATE=accepted
WR0_EXIT=37
```

- Raw stdout SHA-256: `007719df843bf00965844576ec8889c12510db2fe813518755a7a2f30544ec04`.
- Bounded stderr: `352` bytes / SHA-256 `776f568fe17292177657bc22a107fa8846c74d5d3418ed6f9e8071bd539e6eb6`; content is not retained.

## Bounded owned process identities

| Label | Role | Safe executable | UID | Start ticks | Parent | Root PG | Root session | Identity SHA-256 |
|---|---|---|---:|---:|---|---|---|---|
| `RUNTIME_ROOT_1` | `runtime_root` | `pressure-vessel-wrap` | 1000 | 4163387 | `SUPERVISOR` | `true` | `true` | `5bd76ade8b2313c798eb62c897ab617da80043c5ca283bc77bd7fda968cbb6ca` |
| `OWNED_DESCENDANT_1` | `owned_descendant` | `x86_64-linux-gnu-capsule-capture-libs` | 1000 | 4163439 | `RUNTIME_ROOT_1` | `true` | `true` | `384f5ca5cde149f314c7006f2779b2fee5993f58c2c83aa611b07d989fa24f1a` |
| `OWNED_DESCENDANT_2` | `owned_descendant` | `i386-linux-gnu-capsule-capture-libs` | 1000 | 4163459 | `RUNTIME_ROOT_1` | `true` | `true` | `74b3d90ca7bcae015a1cd06c3bab3eb77b5ec2035fa6d881307d1e87d7f1a9eb` |
| `RUNTIME_COMPONENT_1` | `runtime_component` | `pv-adverb` | 1000 | 4163490 | `RUNTIME_ROOT_1` | `false` | `false` | `ba2cca402e5c0bd10dbcc18c80bab7580b5cf41e6a8159d68873bad64dcb4305` |
| `PROTON_1` | `proton` | `python3.13` | 1000 | 4163498 | `RUNTIME_COMPONENT_1` | `false` | `false` | `5744178b5f84e13bc8777bae3b8baa5cab90289b3b358c1eedf6ec161c32853a` |
| `WINE_1` | `wine` | `wine-preloader` | 1000 | 4163503 | `PROTON_1` | `false` | `false` | `3e16294d52eb5b6981433aa47858f4b35382bd0ccd93e36792f99b9d303562cc` |
| `WINESERVER_1` | `wineserver` | `wineserver` | 1000 | 4163504 | `RUNTIME_COMPONENT_1` | `false` | `false` | `7c7607671d457a5db91a1fd6eb11f48adf578ae8f6ec11e09c068004f1044fd1` |
| `OWNED_DESCENDANT_3` | `owned_descendant` | `wine64-preloader` | 1000 | 4163509 | `RUNTIME_COMPONENT_1` | `false` | `false` | `412a9cf168f11bdea6dbd2cf0ff242af2ad709485b36bb7e1b4ee7eff88b85c4` |
| `WINE_SUPPORT_1` | `wine_support` | `wine64-preloader` | 1000 | 4163512 | `RUNTIME_COMPONENT_1` | `false` | `false` | `ed0a46024733115f69dc8df4a1ca277811219fb60fd1dcfc1f4d3270bcfb3962` |
| `WINE_SUPPORT_2` | `wine_support` | `wine64-preloader` | 1000 | 4163516 | `RUNTIME_COMPONENT_1` | `false` | `false` | `82c9adc5104530f432973fd9e4b86c7bb29b5f37865e5ac531e5f25e27cf58ea` |
| `OWNED_DESCENDANT_4` | `owned_descendant` | `wine64-preloader` | 1000 | 4163519 | `RUNTIME_COMPONENT_1` | `false` | `false` | `076acf8901d65ad7dfc3e0aa7fb964e1fdf3b76179ba6b2df29513f0c6cc839f` |
| `OWNED_DESCENDANT_5` | `owned_descendant` | `wine64-preloader` | 1000 | 4163551 | `RUNTIME_COMPONENT_1` | `false` | `false` | `f2ab2d377139149cb69691d9162a34c9360b3314213f5753d19ce8013073d3ec` |
| `WINE_SUPPORT_3` | `wine_support` | `wine64-preloader` | 1000 | 4163554 | `RUNTIME_COMPONENT_1` | `false` | `false` | `c38126b02abd427a65299692c3712425cb164dea62d7d4970c27700aa34e7a3e` |
| `WINDOWS_COMMAND_1` | `windows_command` | `wine64-preloader` | 1000 | 4163661 | `RUNTIME_COMPONENT_1` | `false` | `false` | `557de52238248354c1713cbdee897791b2a4121af7e27d4d3ce414c6ce93aa83` |

No PID, command line, process environment, complete process map, or unrelated process is retained.

The explicit Windows child exit `37` propagated exactly; the ordinary Run 2 receipt remained byte-identical: `true`.
