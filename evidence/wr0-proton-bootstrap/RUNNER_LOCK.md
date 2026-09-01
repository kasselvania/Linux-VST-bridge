# WR0 runner/runtime lock

Classification: `passed`.

- Schema: `linux-vst-bridge-wr0-launch-critical/v1`.
- Canonical manifest SHA-256: `2d64df1d36786ca2d0e955c553005423dc2b5bdd714bd0a17872622e33912547`.
- Runner: `Proton 11.0` / `1787334450 proton-11.0-2-x86_64` / Steam app `4628710` build `24867889`.
- Declared runtime: Steam app `4183110` build `24599767`, depot `4.0.20260805.254769`.
- pressure-vessel: `0.20260805.0`.
- Before/after digest equality: `true`.

| Safe relative path | Type | Size | Mode | SHA-256 |
|---|---|---:|---|---|
| `runner/version` | `regular_file` | 32 | `0755` | `823833b4a22543efdd3b0822981a9518dff08282520eba16661bd9d59bf5e026` |
| `runner/toolmanifest.vdf` | `regular_file` | 156 | `0755` | `bb81d44a687cc6f4ea7ae05fa9f0502f35c48e462051809dce3b2a58492e3498` |
| `runner/proton` | `regular_file` | 93103 | `0755` | `ccb67e21ef0d81cc4142ee3af74a16702292623bad11ec7047dbd6c97be62eed` |
| `runner/steampipe_fixups.json` | `regular_file` | 96309 | `0755` | `1fc86009719ad059a6ef372e1f838c3672016dab421e5c7de1ea9c1fbe66b489` |
| `runner/steampipe_fixups.py` | `regular_file` | 3252 | `0755` | `90a40fe7b030ef94b138687ef4f0e45d1f716f70a83e5a55b36d382fbefa4faf` |
| `runner/filelock.py` | `regular_file` | 12778 | `0755` | `c50e07ad2bc2245c30037034f940581ad18b15d084b0702b33242fff7015ee34` |
| `runner/dist.lock` | `regular_file` | 0 | `0755` | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| `runner/files/steampipe_fixups_mtime` | `regular_file` | 19 | `0644` | `734a0f4cf2d544d84e428a698525d3e7bf3c24a3058bd945ab701fcc7731b078` |
| `runner/files/bin/wine` | `regular_file` | 16872 | `0755` | `7a6de49c00d8ed2ba55c6967d3643c5ef729f5e562fb51e47e4f41c6cdb5c92a` |
| `runner/files/bin/wineserver` | `regular_file` | 887688 | `0755` | `ae2550cd7c6c675128cffe938764f2f797eb7163ca6ca9bddf0014d7fb984b4b` |
| `runner/files/lib/wine/x86_64-unix/wine` | `regular_file` | 16816 | `0555` | `f96a2ac1fcb00f0317a48eaddeedfb8f170d42a6d5714b46a21dbde876649500` |
| `runner/files/lib/wine/x86_64-unix/wine-preloader` | `regular_file` | 19096 | `0555` | `e805dfb4c12973ea54d6161360d91e1e1ee9aba151b77b8f5b6a06910c86cecd` |
| `runner/files/lib/wine/x86_64-windows/cmd.exe` | `regular_file` | 1210529 | `0555` | `74a8fece1a1affc3ad06c82726f069ff7ca9ce0e6a703fe088d89aa38a6aaa4b` |
| `runner/files/share/default_pfx/system.reg` | `regular_file` | 3874912 | `0755` | `0effc7f846639fcd45aa9dce7a3fb6413776a9a47402e4ff20a590d5c87e4333` |
| `runner/files/share/default_pfx/user.reg` | `regular_file` | 27885 | `0755` | `f06c94392a8b4936342f18f2f1b8317f0bc259b1c09206ffc009957db14a0e5b` |
| `runner/files/share/default_pfx/userdef.reg` | `regular_file` | 4190 | `0755` | `ca8de447ded78d2cfbb415bb3d40edd58db9fd170063abbadb9b82fdf24cc739` |
| `runner/LICENSE` | `regular_file` | 20069 | `0755` | `e4829d4e560bd13d378d47592a15d5c7eb1b9a702279eeea9b2f0fc6e60e0b31` |
| `runner/LICENSE.OFL` | `regular_file` | 4414 | `0755` | `93fed46019c38bbe566b479d22148e2e8a1e85ada614accb0211c37b2c61c19b` |
| `runner/PATENTS.AV1` | `regular_file` | 5730 | `0755` | `335eca574598bf4ca181b12f708d6669e5a5e78c8e1513e5b35fa1f03901484b` |
| `runtime/VERSIONS.txt` | `regular_file` | 319 | `0755` | `ef61e9bce47d60d02707661d116c8048ffa39291b84acee687a1ed4e430af6be` |
| `runtime/toolmanifest.vdf` | `regular_file` | 227 | `0755` | `e1c2598db7ae7adfa780b51ee24f391ac0fa11f4049f49aaead0319f8c84d27f` |
| `runtime/_v2-entry-point` | `regular_file` | 8989 | `0755` | `caa39b5cde8ea955288b574b49b3416fd16e7be3f53a17249d673f5ecfdce2c1` |
| `runtime/run` | `regular_file` | 603 | `0755` | `d5069c037c53f204a9d7d0ecb501b430e57dc2d702d49c9cccd57d569ca87eec` |
| `runtime/steamrt4_platform_4.0.20260805.254769/metadata` | `regular_file` | 968 | `0755` | `87fd5d88a7f9439bd344d5984c0237d3ae90d9de0228d65c25c8a484dbd073b1` |
| `runtime/pressure-vessel/bin/pressure-vessel-unruntime` | `regular_file` | 3726 | `0755` | `9211c921bdb343ce1f5987d959b1ca443137a40c4ed44a486a9bd6b7718c0016` |
| `runtime/pressure-vessel/bin/pressure-vessel-wrap` | `regular_file` | 868056 | `0755` | `0439811997a081e4f90f898b7e98e34e46cff3745a0611c17309947fd47f4da9` |
| `runtime/pressure-vessel/bin/steam-runtime-supervisor` | `regular_file` | 137040 | `0755` | `fa0f6586b7ada3d0fd5b96a518a3bcb672810a605d7a543bec30a221d1018d7d` |
| `runtime/pressure-vessel/bin/steam-runtime-launcher-interface-0` | `regular_file` | 10552 | `0755` | `583c789cedd0443d36020aecf3ff0fa444ec663a0dc3f6be5381af55458dae58` |
| `runtime/pressure-vessel/bin/steam-runtime-launch-client` | `regular_file` | 148336 | `0755` | `b2e6dfeecc6581aa210adf1406da3208ed93eb2e8e62f041325ed77918d909bb` |
| `runtime/pressure-vessel/libexec/steam-runtime-tools-0/pv-adverb` | `regular_file` | 606184 | `0755` | `e1428cb8edd77303e5c7f4024da07212c5b88816bd8c9097ea1763474b93e501` |
| `runtime/pressure-vessel/libexec/steam-runtime-tools-0/srt-bwrap` | `regular_file` | 64624 | `0755` | `967bff17693c5b4c7a359a40204125439109bc8c3aa8ba38a556dbef415c070a` |
| `steamapps/appmanifest_4628710.acf` | `regular_file` | 710 | `0755` | `a67dc7a8f53b9f2f38b99a8694047755aff9fcb827dc466530f00e3717fb910a` |
| `steamapps/appmanifest_4183110.acf` | `regular_file` | 541 | `0755` | `8026b0384d21a5652d9d55d0680d7fa2481c5a031624821123e0132bdb820daa` |

No runner/runtime payload is retained. The Scout runtime is installed but undeclared and was not selected.
