# WR0 Run 1 — staged initialization

Classification: `passed`.

| Fact | Result |
|---|---|
| Run number | `1` |
| Nonce | `672302cd4fcba209d7d83092c43b1495` |
| Started | `2026-09-01T05:33:18Z` |
| Finished | `2026-09-01T05:33:32Z` |
| Duration ms | `13793` |
| Windows/supervisor exit | `0` / expected `0` |
| Timeout | `false` |
| Root start ticks | `4399738` |
| Root identity SHA-256 | `efbc99e04babeb5ec6f82a9f3a9f2140a903440558bd4573e8f95df5f67c24bd` |
| Environment identity SHA-256 | `d3ed38d7ed53e9a5973cbf22fc504f2479dbdd15616fe1bfc1d5e1cd0bf5f4c4` |
| Clean descendants after completion | `0` |
| Steam game-launch ancestor | `false` |
| Process-role assertion | `passed` |
| Missing required roles | `[]` |
| Observed process scope | `complete_bounded_exact_root_descendants` |
| Descendants outside initial process group | `18` |
| Command first observed us | `13443470` |
| Fresh pre-gate revalidation us | `13520566` |
| Gate committed us | `13527137` |
| Command completed us | `13731099` |
| Causal gate order | `true` |
| Strict command vector | `true` |
| Contract-source SHA-256 | `c6543004fbcd70252393f0e0cab4f9ad7a31e85f433ce3e903e690244cc79541` |
| Tracked workload SHA-256 | `4518ca37b8d7e005f01b441de5273447b70fa7c8c9c3d2b9c194a91782cfecac` |

## Exact normalized stdout

```text
WR0_MAGIC=LINUX_VST_BRIDGE_WR0_V1
WR0_NONCE=672302cd4fcba209d7d83092c43b1495
WR0_RUN=1
WR0_ARCH=x86_64
WR0_READY=waiting_for_supervisor
WR0_GATE=accepted
WR0_RECEIPT=written_and_read
WR0_EXIT=0
```

- Raw stdout SHA-256: `faf3f9163e819c9aa7ca65ed0e3d9d72782af9bd9be9a97d008e8e353c3be964`.
- Bounded stderr: `523` bytes / SHA-256 `271b1252b1eac2ac272f1b0461109e35f22af317932c27e318cb5282531a2487`; content is not retained.
- Internal receipt SHA-256: `1d39ddd01bff03b84f7b90dead806cab6610bdf72c5578c3165a187b376feac5`.
- Internal receipt normalized SHA-256: `a694c72320267f13fa4c807c167479c6f13394e2977205741efef8bb739b0ed6`.

## Bounded owned process identities

| Label | Role | Safe executable | UID | Start ticks | Parent | Root PG | Root session | Identity SHA-256 |
|---|---|---|---:|---:|---|---|---|---|
| `RUNTIME_ROOT_1` | `runtime_root` | `pressure-vessel-wrap` | 1000 | 4399738 | `SUPERVISOR` | `true` | `true` | `efbc99e04babeb5ec6f82a9f3a9f2140a903440558bd4573e8f95df5f67c24bd` |
| `OWNED_DESCENDANT_1` | `owned_descendant` | `x86_64-linux-gnu-capsule-capture-libs` | 1000 | 4399798 | `RUNTIME_ROOT_1` | `true` | `true` | `35806fc94e9311a285331a330c4498067e74ddb206b83d06b28ff0acec5f62d3` |
| `OWNED_DESCENDANT_2` | `owned_descendant` | `i386-linux-gnu-capsule-capture-libs` | 1000 | 4399824 | `RUNTIME_ROOT_1` | `true` | `true` | `147826e4194090dc73cfa67cd337ad0d23a0141f3e102e4555dfb2215405c9cc` |
| `RUNTIME_COMPONENT_1` | `runtime_component` | `srt-bwrap` | 1000 | 4399866 | `RUNTIME_ROOT_1` | `true` | `true` | `214a6e63476cf7770e7c5556ed3aa053f62223bc34cc11e6f9e72b084a719fb6` |
| `PROTON_1` | `proton` | `python3.13` | 1000 | 4399877 | `RUNTIME_COMPONENT_1` | `false` | `false` | `1a681cac5fbce4abe8164648fe4cc6dab543321da5c1aa318a271d352d5140d5` |
| `WINE_1` | `wine` | `wine-preloader` | 1000 | 4399885 | `PROTON_1` | `false` | `false` | `c43b7daf2464620820ed8d17b3c2f1310816c127d0cd762619ce76a6f1f4d5a6` |
| `WINESERVER_1` | `wineserver` | `wineserver` | 1000 | 4399885 | `RUNTIME_COMPONENT_1` | `false` | `false` | `f30c425b51c6914643474720b852075b85eb0bd25bc5e9b10bda062c1429343d` |
| `OWNED_DESCENDANT_3` | `owned_descendant` | `wine64-preloader` | 1000 | 4399886 | `RUNTIME_COMPONENT_1` | `false` | `false` | `25558bb383ead6f9aa9a9ecf69f4ff457064fa3e38761c172a94e719cd96d5f7` |
| `WINE_SUPPORT_1` | `wine_support` | `wine64-preloader` | 1000 | 4399890 | `RUNTIME_COMPONENT_1` | `false` | `false` | `90d88ed046e672bf88c34fa7aa363622aa74636e23729f07da482e3269bcd5e6` |
| `OWNED_DESCENDANT_4` | `owned_descendant` | `wine64-preloader` | 1000 | 4399893 | `RUNTIME_COMPONENT_1` | `false` | `false` | `c656f009fe93595f07b9beca8e9916d3a49b0cf2fe11bc06047d4e306439e08c` |
| `WINE_SUPPORT_2` | `wine_support` | `wine64-preloader` | 1000 | 4399939 | `RUNTIME_COMPONENT_1` | `false` | `false` | `b8a3ced479400255754a30acfceca049ab8ba14dda05e0cdaf4ed2f1f0ebb773` |
| `WINE_SUPPORT_3` | `wine_support` | `wine64-preloader` | 1000 | 4399948 | `RUNTIME_COMPONENT_1` | `false` | `false` | `727d4c792a6ebd2878afd3dd317eb05ee702add3a569cc456d663842cf49492f` |
| `WINE_SUPPORT_4` | `wine_support` | `wine64-preloader` | 1000 | 4399954 | `RUNTIME_COMPONENT_1` | `false` | `false` | `4d799cd32c9ed43696898388b513716340a21c12153dec7bbe1f90b5dac8eaed` |
| `OWNED_DESCENDANT_5` | `owned_descendant` | `unavailable` | 1000 | 4399957 | `RUNTIME_COMPONENT_1` | `false` | `false` | `75ad0fe7d392be4fd2400daa25872b7a2c87c9df95c0611fc29723816d671e03` |
| `OWNED_DESCENDANT_6` | `owned_descendant` | `wine-preloader` | 1000 | 4400229 | `RUNTIME_COMPONENT_1` | `false` | `false` | `eb578f22119f6590a6b32466fb3083739b76c7adade76ae037566f25145c8038` |
| `OWNED_DESCENDANT_7` | `owned_descendant` | `wine64-preloader` | 1000 | 4400289 | `RUNTIME_COMPONENT_1` | `false` | `false` | `ec365fa4c8463dec08bdd5e9a15452cfe07f5ede43cb209d8d18d92c2ebc0a49` |
| `OWNED_DESCENDANT_8` | `owned_descendant` | `wine64-preloader` | 1000 | 4400448 | `RUNTIME_COMPONENT_1` | `false` | `false` | `297e9e9bafa0b33b53b40d35af9e05b7b84e68df18cafbbae1b4a0fc7fbeb4b4` |
| `OWNED_DESCENDANT_9` | `owned_descendant` | `wine64-preloader` | 1000 | 4400453 | `RUNTIME_COMPONENT_1` | `false` | `false` | `822f739971d953d148b7a26aa77b5c98bad12a8bb834775919e09d1697dfd75b` |
| `OWNED_DESCENDANT_10` | `owned_descendant` | `wine-preloader` | 1000 | 4400469 | `RUNTIME_COMPONENT_1` | `false` | `false` | `89a58e45d066ba67ca91ccc81a374b04154783844358646fae2726e42b5a4072` |
| `OWNED_DESCENDANT_11` | `owned_descendant` | `unavailable` | 1000 | 4400939 | `RUNTIME_COMPONENT_1` | `false` | `false` | `137a50be0dc027d3f979c0ae2009a35f7c5db2072dc566bb2e17d3d67782ad29` |
| `WINE_SUPPORT_5` | `wine_support` | `wine64-preloader` | 1000 | 4400967 | `RUNTIME_COMPONENT_1` | `false` | `false` | `5d3f8ac14a598bf8fb9d6ec28633d45c084ec20e9aa7b00c99ea280ba88bcca7` |
| `WINDOWS_COMMAND_1` | `windows_command` | `wine64-preloader` | 1000 | 4401065 | `RUNTIME_COMPONENT_1` | `false` | `false` | `f7c5c10892f6f6617afaee0ccdf0405b5ff228820508a8a58d9ccb6e4c61cf64` |

No PID, command line, process environment, complete process map, or unrelated process is retained.
