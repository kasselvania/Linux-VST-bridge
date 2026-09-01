# WR0 Run 2 — promoted environment reuse

Classification: `passed`.

| Fact | Result |
|---|---|
| Run number | `2` |
| Nonce | `6f20368e0bf055a6a46ed691d97e0536` |
| Started | `2026-09-01T04:53:51Z` |
| Finished | `2026-09-01T04:53:54Z` |
| Duration ms | `3391` |
| Windows/supervisor exit | `0` / expected `0` |
| Timeout | `false` |
| Root start ticks | `4163033` |
| Root identity SHA-256 | `914f84847f23bd1b2f47a1c6cfaed1b0db5d64c03ed8dfcf5b3646988cfb46ca` |
| Environment identity SHA-256 | `447e6d4dfccfbebe7be44cc521e8ed192bfad4ab1fe1f16673d391161db73c24` |
| Clean descendants after completion | `0` |
| Steam game-launch ancestor | `false` |
| Process-role assertion | `passed` |
| Missing required roles | `[]` |
| Observed process scope | `complete_bounded_exact_root_descendants` |
| Descendants outside initial process group | `11` |
| Command first observed us | `3134553` |
| Fresh pre-gate revalidation us | `3193564` |
| Gate committed us | `3205709` |
| Command completed us | `3344689` |
| Causal gate order | `true` |
| Strict command vector | `true` |
| Contract-source SHA-256 | `887b148b7862038fd8fc0af52146ecc7452a5fbf82c22c5af8eaec6fe47504b8` |
| Tracked workload SHA-256 | `4518ca37b8d7e005f01b441de5273447b70fa7c8c9c3d2b9c194a91782cfecac` |

## Exact normalized stdout

```text
WR0_MAGIC=LINUX_VST_BRIDGE_WR0_V1
WR0_NONCE=6f20368e0bf055a6a46ed691d97e0536
WR0_RUN=2
WR0_ARCH=x86_64
WR0_READY=waiting_for_supervisor
WR0_GATE=accepted
WR0_RECEIPT=written_and_read
WR0_EXIT=0
```

- Raw stdout SHA-256: `fe0b8d04b54fc6bce43c20c51e0379a82c1899eca25ca1a5e336dd7391c5ff37`.
- Bounded stderr: `352` bytes / SHA-256 `776f568fe17292177657bc22a107fa8846c74d5d3418ed6f9e8071bd539e6eb6`; content is not retained.
- Internal receipt SHA-256: `81eb4884f7e066d79973af53cce85ac086c64c78879bea3d8c1c4372a5a5792b`.
- Internal receipt normalized SHA-256: `e5b13dc5b9d1313aa7bdf3a2c6a407ba843ee5374c4293c4c6f0cb3c808a6064`.

## Bounded owned process identities

| Label | Role | Safe executable | UID | Start ticks | Parent | Root PG | Root session | Identity SHA-256 |
|---|---|---|---:|---:|---|---|---|---|
| `RUNTIME_ROOT_1` | `runtime_root` | `pressure-vessel-wrap` | 1000 | 4163033 | `SUPERVISOR` | `true` | `true` | `914f84847f23bd1b2f47a1c6cfaed1b0db5d64c03ed8dfcf5b3646988cfb46ca` |
| `OWNED_DESCENDANT_1` | `owned_descendant` | `x86_64-linux-gnu-capsule-capture-libs` | 1000 | 4163105 | `RUNTIME_ROOT_1` | `true` | `true` | `64e6dd81ed5a67324483612f794c1f68b26e5465d6a62ac32d335ea71367eaae` |
| `OWNED_DESCENDANT_2` | `owned_descendant` | `i386-linux-gnu-capsule-capture-libs` | 1000 | 4163129 | `RUNTIME_ROOT_1` | `true` | `true` | `a902239181d4d4e9a46ee28b8eb2af21cad169aaa079ca8ece9e04508851eaf6` |
| `RUNTIME_COMPONENT_1` | `runtime_component` | `pv-adverb` | 1000 | 4163160 | `RUNTIME_ROOT_1` | `false` | `false` | `cbe289e90bdbe836c7b8c39753c9f962c19e5e3a4963b69fce142b903d557a70` |
| `PROTON_1` | `proton` | `python3.13` | 1000 | 4163168 | `RUNTIME_COMPONENT_1` | `false` | `false` | `bfcfbdf0c4a64a1b2b4ba234541b6ce087044eff80d93b6ac1c4eb5e4bde7158` |
| `WINE_1` | `wine` | `wine-preloader` | 1000 | 4163173 | `PROTON_1` | `false` | `false` | `9e8d9e9051a6cd0e11c4bcbb61101d0e2acbd5a37441fb47418be23136531a9b` |
| `WINESERVER_1` | `wineserver` | `wineserver` | 1000 | 4163174 | `RUNTIME_COMPONENT_1` | `false` | `false` | `a76f45b4daf32bce39ab1e2791b156926921de815b0b6b4a8e49a4c456f0f31f` |
| `OWNED_DESCENDANT_3` | `owned_descendant` | `wine64-preloader` | 1000 | 4163178 | `RUNTIME_COMPONENT_1` | `false` | `false` | `c61539a0972ec6a65853cdb35cb89c2c55911a7c530dcdac551a84c2689a66ae` |
| `WINE_SUPPORT_1` | `wine_support` | `wine64-preloader` | 1000 | 4163181 | `RUNTIME_COMPONENT_1` | `false` | `false` | `8eae585cea6fb879c7d12e5950e30f27d33b0926938c340a3ab5b9090548ef31` |
| `WINE_SUPPORT_2` | `wine_support` | `wine64-preloader` | 1000 | 4163184 | `RUNTIME_COMPONENT_1` | `false` | `false` | `65d56c3fd9817b4db9f8ee6fdfe8b2bbe04d42abd830a7b839007cc4374a9378` |
| `OWNED_DESCENDANT_4` | `owned_descendant` | `wine64-preloader` | 1000 | 4163187 | `RUNTIME_COMPONENT_1` | `false` | `false` | `3b8c0a7c05e097422d47d345b0fe142694b66295126af7e5733d78614490ee50` |
| `OWNED_DESCENDANT_5` | `owned_descendant` | `wine64-preloader` | 1000 | 4163220 | `RUNTIME_COMPONENT_1` | `false` | `false` | `5386f562e44d1e5ececae1073aec3068cc71fad3ee36269518eb6a4784b07639` |
| `WINE_SUPPORT_3` | `wine_support` | `wine64-preloader` | 1000 | 4163224 | `RUNTIME_COMPONENT_1` | `false` | `false` | `2b905decf0ca9c033f8d5e29f718d185c5851ee88a49660a3230f4ec76c4a8d7` |
| `WINDOWS_COMMAND_1` | `windows_command` | `wine64-preloader` | 1000 | 4163331 | `RUNTIME_COMPONENT_1` | `false` | `false` | `dfd5d780b00d07a1353facd481ee58c12f3f9dc0c65997772bf5e4897787f45b` |

No PID, command line, process environment, complete process map, or unrelated process is retained.
