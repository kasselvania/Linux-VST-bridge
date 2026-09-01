# WR0 Run 1 — staged initialization

Classification: `passed`.

| Fact | Result |
|---|---|
| Run number | `1` |
| Nonce | `6b35e29945d635948bf21712d88943bc` |
| Started | `2026-09-01T04:53:37Z` |
| Finished | `2026-09-01T04:53:50Z` |
| Duration ms | `13676` |
| Windows/supervisor exit | `0` / expected `0` |
| Timeout | `false` |
| Root start ticks | `4161644` |
| Root identity SHA-256 | `67e1564b6a301bdcfe445b8fed28a0607294b842349495f7cffcd375c61c1aa3` |
| Environment identity SHA-256 | `447e6d4dfccfbebe7be44cc521e8ed192bfad4ab1fe1f16673d391161db73c24` |
| Clean descendants after completion | `0` |
| Steam game-launch ancestor | `false` |
| Process-role assertion | `passed` |
| Missing required roles | `[]` |
| Observed process scope | `complete_bounded_exact_root_descendants` |
| Descendants outside initial process group | `21` |
| Command first observed us | `13329758` |
| Fresh pre-gate revalidation us | `13408644` |
| Gate committed us | `13416047` |
| Command completed us | `13612483` |
| Causal gate order | `true` |
| Strict command vector | `true` |
| Contract-source SHA-256 | `887b148b7862038fd8fc0af52146ecc7452a5fbf82c22c5af8eaec6fe47504b8` |
| Tracked workload SHA-256 | `4518ca37b8d7e005f01b441de5273447b70fa7c8c9c3d2b9c194a91782cfecac` |

## Exact normalized stdout

```text
WR0_MAGIC=LINUX_VST_BRIDGE_WR0_V1
WR0_NONCE=6b35e29945d635948bf21712d88943bc
WR0_RUN=1
WR0_ARCH=x86_64
WR0_READY=waiting_for_supervisor
WR0_GATE=accepted
WR0_RECEIPT=written_and_read
WR0_EXIT=0
```

- Raw stdout SHA-256: `4329cc9bbf97eb260930525bc92259606342989c5d8cae5388f97fe20b5707ad`.
- Bounded stderr: `523` bytes / SHA-256 `24ad5820789f1104f55882e85df985af800fd5e4f5457f6313d3b3354506993d`; content is not retained.
- Internal receipt SHA-256: `c3a9954ca7d9fb9f57ecc6bb50e204923132490952fc8b8eb714e4c05e3327c1`.
- Internal receipt normalized SHA-256: `9ac4cab255a597a01aaa747c486de5b267e33d6e499e839364a6a275662b22f9`.

## Bounded owned process identities

| Label | Role | Safe executable | UID | Start ticks | Parent | Root PG | Root session | Identity SHA-256 |
|---|---|---|---:|---:|---|---|---|---|
| `RUNTIME_ROOT_1` | `runtime_root` | `pressure-vessel-wrap` | 1000 | 4161644 | `SUPERVISOR` | `true` | `true` | `67e1564b6a301bdcfe445b8fed28a0607294b842349495f7cffcd375c61c1aa3` |
| `OWNED_DESCENDANT_1` | `owned_descendant` | `x86_64-linux-gnu-capsule-capture-libs` | 1000 | 4161703 | `RUNTIME_ROOT_1` | `true` | `true` | `9939b07756c7c5474114c105656ea9329c8d3f6829a6478efec855a5ae1edd33` |
| `OWNED_DESCENDANT_2` | `owned_descendant` | `i386-linux-gnu-capsule-capture-libs` | 1000 | 4161729 | `RUNTIME_ROOT_1` | `true` | `true` | `8d6844a0c6f25f503ec212d735e7766e7a2a05674d6da9f9f53dc1c4bdcd0f78` |
| `RUNTIME_COMPONENT_1` | `runtime_component` | `pv-adverb` | 1000 | 4161772 | `RUNTIME_ROOT_1` | `false` | `false` | `8bc0fd82252773d706a28841889842d3c16070a4711736f4c14b5ef7928e54a4` |
| `PROTON_1` | `proton` | `python3.13` | 1000 | 4161783 | `RUNTIME_COMPONENT_1` | `false` | `false` | `7913ac94eec0706b871c614e699892b0aabbac6dad8a03dc27c6eacc0b80fd26` |
| `WINE_1` | `wine` | `wine-preloader` | 1000 | 4161790 | `PROTON_1` | `false` | `false` | `9e26db05378a00359214f363d0db55cd18f113e25147022812644190618daf1b` |
| `WINESERVER_1` | `wineserver` | `wineserver` | 1000 | 4161791 | `RUNTIME_COMPONENT_1` | `false` | `false` | `5990bf4e497ce09f0b01a722ec79970261145d2d91d4ed0b83018770ee299206` |
| `OWNED_DESCENDANT_3` | `owned_descendant` | `wine64-preloader` | 1000 | 4161791 | `RUNTIME_COMPONENT_1` | `false` | `false` | `3db9999e2f5c3c4527f79a2262b25ef2b630570bfccfc42e8059c5d6e3027035` |
| `WINE_SUPPORT_1` | `wine_support` | `wine64-preloader` | 1000 | 4161795 | `RUNTIME_COMPONENT_1` | `false` | `false` | `4dbfc9ce2f83fe964d31ddf32463de199ebbbd53d2697689be6c488940ae1b79` |
| `OWNED_DESCENDANT_4` | `owned_descendant` | `wine64-preloader` | 1000 | 4161798 | `RUNTIME_COMPONENT_1` | `false` | `false` | `6aeee3f74a77ad8a770ec3910286b88b22f89ec13608dba7cbf9e23905f95921` |
| `WINE_SUPPORT_2` | `wine_support` | `wine64-preloader` | 1000 | 4161841 | `RUNTIME_COMPONENT_1` | `false` | `false` | `28b2e55f5dcaf852e3504ab6205e89646c20fdaed25b5643a7f6be51fad95495` |
| `WINE_SUPPORT_3` | `wine_support` | `wine64-preloader` | 1000 | 4161849 | `RUNTIME_COMPONENT_1` | `false` | `false` | `d591d420c9d84209dda7b5f1dae5c447185ea04f4a340a70a2d7bba4d2acf42f` |
| `WINE_SUPPORT_4` | `wine_support` | `wine64-preloader` | 1000 | 4161854 | `RUNTIME_COMPONENT_1` | `false` | `false` | `3c13cb5a9c2e9c650de567fc2ab3d8e86c5c63ccf2de7e3901a20308aef88f56` |
| `OWNED_DESCENDANT_5` | `owned_descendant` | `wine64-preloader` | 1000 | 4161857 | `RUNTIME_COMPONENT_1` | `false` | `false` | `336c2f90dbc621de4315e29fe3369b9dc68d849a652da36c9eccfcf8f56f208e` |
| `OWNED_DESCENDANT_6` | `owned_descendant` | `wine-preloader` | 1000 | 4162126 | `RUNTIME_COMPONENT_1` | `false` | `false` | `58eb13e9c47d3ce080bffaea6632e89d1e6ab90f32045aeaf144d3bf45aa4296` |
| `OWNED_DESCENDANT_7` | `owned_descendant` | `wine-preloader` | 1000 | 4162172 | `RUNTIME_COMPONENT_1` | `false` | `false` | `4302dc3698c9732da1f366487e9167ff0fb4a9b75ef297c9597738358ad0145c` |
| `OWNED_DESCENDANT_8` | `owned_descendant` | `wine64-preloader` | 1000 | 4162187 | `RUNTIME_COMPONENT_1` | `false` | `false` | `73e623d38d2d856431ec7f2b66b95d14c9c52c4bea7fc6103a46a146561c6276` |
| `OWNED_DESCENDANT_9` | `owned_descendant` | `wine64-preloader` | 1000 | 4162322 | `RUNTIME_COMPONENT_1` | `false` | `false` | `d9484cf64403192ddd008ed7dfbc7eefec851c2778d75a8481b93c9e9e10ebbd` |
| `OWNED_DESCENDANT_10` | `owned_descendant` | `wine64-preloader` | 1000 | 4162342 | `RUNTIME_COMPONENT_1` | `false` | `false` | `bce45c53498d6c54982adc38e8370724b8b53d73ef6255442914fbe0f4a12433` |
| `OWNED_DESCENDANT_11` | `owned_descendant` | `wine64-preloader` | 1000 | 4162346 | `RUNTIME_COMPONENT_1` | `false` | `false` | `d4e1d9f57ae5dd8691605053defd0d0586a95afe247cb0312c5a6729423c013a` |
| `OWNED_DESCENDANT_12` | `owned_descendant` | `wine-preloader` | 1000 | 4162361 | `RUNTIME_COMPONENT_1` | `false` | `false` | `e1b329c6e0861b9885ef703ce1fa6d589cf75beed011513568cd211a55851d39` |
| `OWNED_DESCENDANT_13` | `owned_descendant` | `wine-preloader` | 1000 | 4162838 | `RUNTIME_COMPONENT_1` | `false` | `false` | `8f3220b36beffeba6007ff684bb7b6a4b47719de63e25a52d6741cdfa2db4ecc` |
| `WINE_SUPPORT_5` | `wine_support` | `wine64-preloader` | 1000 | 4162865 | `RUNTIME_COMPONENT_1` | `false` | `false` | `3a0b5b3944f326c0899fbf99db956c1986b47adff9503ca528ab87d23c995ded` |
| `WINDOWS_COMMAND_1` | `windows_command` | `wine64-preloader` | 1000 | 4162960 | `RUNTIME_COMPONENT_1` | `false` | `false` | `dedf4b824f07bf0b60cf79379f6a0428d6017bf99760dbedea7825ad206b3c21` |

No PID, command line, process environment, complete process map, or unrelated process is retained.
