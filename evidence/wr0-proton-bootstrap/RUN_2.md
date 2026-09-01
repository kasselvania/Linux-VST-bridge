# WR0 Run 2 — promoted environment reuse

Classification: `passed`.

| Fact | Result |
|---|---|
| Run number | `2` |
| Nonce | `5c169d4b749e45723dd9503254516d10` |
| Started | `2026-09-01T05:33:32Z` |
| Finished | `2026-09-01T05:33:35Z` |
| Duration ms | `3315` |
| Windows/supervisor exit | `0` / expected `0` |
| Timeout | `false` |
| Root start ticks | `4401138` |
| Root identity SHA-256 | `a03020c4163f9b78a52da6e08013c089762de2398c61edb15721e395ffb0e9c9` |
| Environment identity SHA-256 | `d3ed38d7ed53e9a5973cbf22fc504f2479dbdd15616fe1bfc1d5e1cd0bf5f4c4` |
| Clean descendants after completion | `0` |
| Steam game-launch ancestor | `false` |
| Process-role assertion | `passed` |
| Missing required roles | `[]` |
| Observed process scope | `complete_bounded_exact_root_descendants` |
| Descendants outside initial process group | `11` |
| Command first observed us | `3058416` |
| Fresh pre-gate revalidation us | `3114641` |
| Gate committed us | `3125614` |
| Command completed us | `3272143` |
| Causal gate order | `true` |
| Strict command vector | `true` |
| Contract-source SHA-256 | `c6543004fbcd70252393f0e0cab4f9ad7a31e85f433ce3e903e690244cc79541` |
| Tracked workload SHA-256 | `4518ca37b8d7e005f01b441de5273447b70fa7c8c9c3d2b9c194a91782cfecac` |

## Exact normalized stdout

```text
WR0_MAGIC=LINUX_VST_BRIDGE_WR0_V1
WR0_NONCE=5c169d4b749e45723dd9503254516d10
WR0_RUN=2
WR0_ARCH=x86_64
WR0_READY=waiting_for_supervisor
WR0_GATE=accepted
WR0_RECEIPT=written_and_read
WR0_EXIT=0
```

- Raw stdout SHA-256: `a45e6b5d2766dd975634a60256c8a796fb4f0dfe48f45b0e6ecba913da958cfd`.
- Bounded stderr: `352` bytes / SHA-256 `776f568fe17292177657bc22a107fa8846c74d5d3418ed6f9e8071bd539e6eb6`; content is not retained.
- Internal receipt SHA-256: `42b31ae82585537c43a7bdf1b5cdb0ad2f4335cf53209b34343a8c8c26430453`.
- Internal receipt normalized SHA-256: `b8396af6da378cfb698428fe5c363ac3161a82fd8a861898757da6a14bd2bda4`.

## Bounded owned process identities

| Label | Role | Safe executable | UID | Start ticks | Parent | Root PG | Root session | Identity SHA-256 |
|---|---|---|---:|---:|---|---|---|---|
| `RUNTIME_ROOT_1` | `runtime_root` | `pressure-vessel-wrap` | 1000 | 4401138 | `SUPERVISOR` | `true` | `true` | `a03020c4163f9b78a52da6e08013c089762de2398c61edb15721e395ffb0e9c9` |
| `OWNED_DESCENDANT_1` | `owned_descendant` | `x86_64-linux-gnu-capsule-capture-libs` | 1000 | 4401210 | `RUNTIME_ROOT_1` | `true` | `true` | `7856b6d123a1b2141726f530f28c3603dbaf6b2051e2266c52969ed544dd6894` |
| `OWNED_DESCENDANT_2` | `owned_descendant` | `i386-linux-gnu-capsule-capture-libs` | 1000 | 4401231 | `RUNTIME_ROOT_1` | `true` | `true` | `b07f1fe3e3d8ad4b44a9e0225e33daf4377d1675dedcf9fa8ac9a6373f23e404` |
| `RUNTIME_COMPONENT_1` | `runtime_component` | `pv-adverb` | 1000 | 4401261 | `RUNTIME_ROOT_1` | `false` | `false` | `152839b840b33a0fa802eda259721b820169d20de2b3d5d4856c12d002cad17e` |
| `PROTON_1` | `proton` | `python3.13` | 1000 | 4401269 | `RUNTIME_COMPONENT_1` | `false` | `false` | `9b5d7517e944eb242605a2f015cda0f3c15fedc2db3bade694a0dd0887833c5d` |
| `WINE_1` | `wine` | `wine-preloader` | 1000 | 4401274 | `PROTON_1` | `false` | `false` | `60cbe49368278421841bb619704bea8d8d53c1b5420e0b329d9f676657644b02` |
| `WINESERVER_1` | `wineserver` | `wineserver` | 1000 | 4401275 | `RUNTIME_COMPONENT_1` | `false` | `false` | `1aed3407cc696bad5aacc32b5d3bdb8837e8d4d016901536d1b2f5d3a4825f7a` |
| `OWNED_DESCENDANT_3` | `owned_descendant` | `wine64-preloader` | 1000 | 4401281 | `RUNTIME_COMPONENT_1` | `false` | `false` | `8aa81d0cd46cc62511811ec6c7f9e486e47a4c0d0b1f89eff2ae76c37b23a09b` |
| `WINE_SUPPORT_1` | `wine_support` | `wine64-preloader` | 1000 | 4401283 | `RUNTIME_COMPONENT_1` | `false` | `false` | `e179bd8eb62b2702bc5dfcf55094c97f86cd1ee93eec087b996d2587373ee48c` |
| `WINE_SUPPORT_2` | `wine_support` | `wine64-preloader` | 1000 | 4401287 | `RUNTIME_COMPONENT_1` | `false` | `false` | `3bea2461a144f3996e66035db661e15a035a6c3ae44bb5d8ed38ad732bdb8cf2` |
| `OWNED_DESCENDANT_4` | `owned_descendant` | `wine64-preloader` | 1000 | 4401290 | `RUNTIME_COMPONENT_1` | `false` | `false` | `eb864559c8f358bbfa5c4316d5a03cf43dd5421fe44b272cb0feaa164b8d05fa` |
| `OWNED_DESCENDANT_5` | `owned_descendant` | `wine64-preloader` | 1000 | 4401320 | `RUNTIME_COMPONENT_1` | `false` | `false` | `bd413b85cdd4ded094f22aaa7b3fd203054c3033dbdfe8df6c213133ef3ea4fa` |
| `WINE_SUPPORT_3` | `wine_support` | `wine64-preloader` | 1000 | 4401324 | `RUNTIME_COMPONENT_1` | `false` | `false` | `dd32e17e7e25d6bfa5c4bf2cc92ef70ecfe39d0e0f5644102df50974de2cac69` |
| `WINDOWS_COMMAND_1` | `windows_command` | `wine64-preloader` | 1000 | 4401432 | `RUNTIME_COMPONENT_1` | `false` | `false` | `9d17e682fc7496e6ebea5f4f0672a5ca4fd65cf4ae923d3b2635184dba8c2d35` |

No PID, command line, process environment, complete process map, or unrelated process is retained.
