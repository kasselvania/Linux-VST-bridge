# WR0 expected exit propagation

Classification: `expected_failure`.

| Fact | Result |
|---|---|
| Run number | `37` |
| Nonce | `496771a945167f46a8aaa97aa6133633` |
| Started | `2026-09-01T05:33:35Z` |
| Finished | `2026-09-01T05:33:38Z` |
| Duration ms | `3106` |
| Windows/supervisor exit | `37` / expected `37` |
| Timeout | `false` |
| Root start ticks | `4401483` |
| Root identity SHA-256 | `2cebec2ae8921f851eb17e93d50f33a7330dd669127612e5edf3b327280ed0b9` |
| Environment identity SHA-256 | `d3ed38d7ed53e9a5973cbf22fc504f2479dbdd15616fe1bfc1d5e1cd0bf5f4c4` |
| Clean descendants after completion | `0` |
| Steam game-launch ancestor | `false` |
| Process-role assertion | `passed` |
| Missing required roles | `[]` |
| Observed process scope | `complete_bounded_exact_root_descendants` |
| Descendants outside initial process group | `11` |
| Command first observed us | `2855862` |
| Fresh pre-gate revalidation us | `2909331` |
| Gate committed us | `2920948` |
| Command completed us | `3062939` |
| Causal gate order | `true` |
| Strict command vector | `true` |
| Contract-source SHA-256 | `c6543004fbcd70252393f0e0cab4f9ad7a31e85f433ce3e903e690244cc79541` |
| Tracked workload SHA-256 | `4518ca37b8d7e005f01b441de5273447b70fa7c8c9c3d2b9c194a91782cfecac` |

## Exact normalized stdout

```text
WR0_MAGIC=LINUX_VST_BRIDGE_WR0_V1
WR0_NONCE=496771a945167f46a8aaa97aa6133633
WR0_RUN=37
WR0_ARCH=x86_64
WR0_READY=waiting_for_supervisor
WR0_GATE=accepted
WR0_EXIT=37
```

- Raw stdout SHA-256: `412a1106b823cb39e014f9ad1361ee31fa5d094bc06381a3556e17e4b72b0a33`.
- Bounded stderr: `352` bytes / SHA-256 `776f568fe17292177657bc22a107fa8846c74d5d3418ed6f9e8071bd539e6eb6`; content is not retained.

## Bounded owned process identities

| Label | Role | Safe executable | UID | Start ticks | Parent | Root PG | Root session | Identity SHA-256 |
|---|---|---|---:|---:|---|---|---|---|
| `RUNTIME_ROOT_1` | `runtime_root` | `pressure-vessel-wrap` | 1000 | 4401483 | `SUPERVISOR` | `true` | `true` | `2cebec2ae8921f851eb17e93d50f33a7330dd669127612e5edf3b327280ed0b9` |
| `OWNED_DESCENDANT_1` | `owned_descendant` | `x86_64-linux-gnu-capsule-capture-libs` | 1000 | 4401535 | `RUNTIME_ROOT_1` | `true` | `true` | `8dd49f9e13e6a3d065c5d572fe23298cd17c46b39b218a2d029e10b8fa86024e` |
| `OWNED_DESCENDANT_2` | `owned_descendant` | `i386-linux-gnu-capsule-capture-libs` | 1000 | 4401555 | `RUNTIME_ROOT_1` | `true` | `true` | `922b062ebee3410ac4d5be9d59f595976ca4e93fbe8b4c89eb6a5fe0edd8d796` |
| `RUNTIME_COMPONENT_1` | `runtime_component` | `pv-adverb` | 1000 | 4401586 | `RUNTIME_ROOT_1` | `false` | `false` | `f8458c2c7da34dd416d6f85dd2322383d95f2552119bff2b138b2d76b70c6962` |
| `PROTON_1` | `proton` | `python3.13` | 1000 | 4401594 | `RUNTIME_COMPONENT_1` | `false` | `false` | `668de040bef77fe40a671180070b474d134134b4ab263d0fa1a09266a309b842` |
| `WINE_1` | `wine` | `wine-preloader` | 1000 | 4401599 | `PROTON_1` | `false` | `false` | `7036aac7eabf31cdeae932118f2f6ee74cbd8f5ebaf7cccd47347c5924330562` |
| `WINESERVER_1` | `wineserver` | `wineserver` | 1000 | 4401600 | `RUNTIME_COMPONENT_1` | `false` | `false` | `365c3f6a340d4d13225be047ddc48115313a2114415a812f6a16a5f44d567e07` |
| `OWNED_DESCENDANT_3` | `owned_descendant` | `wine64-preloader` | 1000 | 4401604 | `RUNTIME_COMPONENT_1` | `false` | `false` | `9893d315dbfadbf48ccf71f95c9623dcd361a646cf413fb9fa5713b5f4ba4782` |
| `WINE_SUPPORT_1` | `wine_support` | `wine64-preloader` | 1000 | 4401607 | `RUNTIME_COMPONENT_1` | `false` | `false` | `b6f1e4f40fa1235e48e89b300faef0d50c9a0366c84c9bf44ccbab5e032ef994` |
| `WINE_SUPPORT_2` | `wine_support` | `wine64-preloader` | 1000 | 4401611 | `RUNTIME_COMPONENT_1` | `false` | `false` | `e62f5e0e6f744e1ed1acb1a0ebb513406ea600774aaf86cde0ff9a1383c0c6d4` |
| `OWNED_DESCENDANT_4` | `owned_descendant` | `wine64-preloader` | 1000 | 4401614 | `RUNTIME_COMPONENT_1` | `false` | `false` | `a420ce2efde6ad6f8a95bb2b029cd2b5c02679836340e4d61021514e7a49075b` |
| `OWNED_DESCENDANT_5` | `owned_descendant` | `wine64-preloader` | 1000 | 4401644 | `RUNTIME_COMPONENT_1` | `false` | `false` | `d5a8cf109a491a762205378ab7f845f3653c53b3fed28d7aade79d08d54d4f1b` |
| `WINE_SUPPORT_3` | `wine_support` | `wine64-preloader` | 1000 | 4401648 | `RUNTIME_COMPONENT_1` | `false` | `false` | `4c9d7eb28c8806d13a46479252589fe73ddc2aecc29d23dec516a107b032c79a` |
| `WINDOWS_COMMAND_1` | `windows_command` | `wine64-preloader` | 1000 | 4401753 | `RUNTIME_COMPONENT_1` | `false` | `false` | `e654e59f33c2fe1068983873c4cc38f1dd0c0f9f6abb8a8fd27913abe8e55965` |

No PID, command line, process environment, complete process map, or unrelated process is retained.

The explicit Windows child exit `37` propagated exactly; the ordinary Run 2 receipt remained byte-identical: `true`.
