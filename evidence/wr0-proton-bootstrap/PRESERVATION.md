# WR0 protected-fixture preservation

Classification: `passed`; exact before/after equality: `true`.

| Fact | Result |
|---|---|
| Bitwig version/app commit | `6.0.11` / `7b66aed37386ffff99e40bbd486f90ceee7ac1d5c59508c7952094122cebf50e` |
| Bitwig runtime/commit | `org.freedesktop.Platform/x86_64/25.08` / `bd44a6230581917d04f89812a4c21090c304d390edb73995af1c2f9fd8abf4e8` |
| Bitwig user/system override SHA-256 | `1b4a6a6ed688f69dd3c36dcac8db008c5a41ed52170ea3e23dee984b0aae6a1e` / `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| HP0 module SHA-256 | `3abaa8a2051ea179e28986cc6fbad4cde2d0d5e9a844b18aca91cdf7c59e69d7` |
| HP0 publication receipt SHA-256 | `d328adb27fef326b8e5b1104f072c246a803389a77d35ee5babc9834067668d1` |
| HP0 retained build receipt SHA-256 | `2fae3eb22cd6ee0725703d4b507525cae427bcaa01412dd80f879832c5951b4e` |
| HP0 build-source identity | `linux-vst-bridge-hp0-build-source/v1` / `ee301e3e42b2381d5a9aa88ec64635482d098bc8cde50d33a57e6c397fb1e9ab` |
| Accepted packet sr0-steam-deck-fixture-reconnaissance | `e0b7b45da7a88313bcb21d56fd9ea90a6bfa657bdcd70dd5e115b3673b56e6ba` / `10` files |
| Accepted packet hp0-native-vst3-bitwig-sandbox | `beaa49577d0e964a1d4a5ea915fb210ca85ca84e57a79eb937ff84ccece7b532` / `12` files |
| Accepted packet hp1-bitwig-admission | `3f064735b78347839f3cb99c09f0fd930ff28680d0b3e740fbc5e80d9f63e827` / `12` files |
| SR0 Serum existing_native_proxy | `317d70f95a3c7559ff3d43b014c3e7b792fd03362d5e999d550a5566cf25b184` / `88888` bytes / mtime ns `1745989109370060240` |
| SR0 Serum existing_windows_module | `838bc7ab42d5d039156768680ffc3e0175d6e6bed9f99802989a01d695e13175` / `18062336` bytes / mtime ns `1745689532000000000` |
| Existing .wine system.reg | `2e0d2eb3b1bf446d93a66a7566738704072db083e7242d87c0bc1f0697738a88` / `4397430` bytes |
| Existing .wine user.reg | `8a34a08acfe6efb340e264cad90e70fd1586f25eb6bf90c49dde6fc78264fab9` / `117505` bytes |
| Existing .wine userdef.reg | `0299356f52d87c08134b7da63db359db20806f8731fd4f008985d8fd0df364bf` / `4026` bytes |
| Steam compatdata immediate roster | `be754ce016292277476721d282adba873495e34f8b218512572024189cf94967` / `33` directories |
| Runner/runtime lock | `2d64df1d36786ca2d0e955c553005423dc2b5bdd714bd0a17872622e33912547` |

Composite before SHA-256: `3fcb16af33518797020855ef214b7b55b2f006838e2ed1e24337caf0474fcb9c`. Composite after SHA-256: `3fcb16af33518797020855ef214b7b55b2f006838e2ed1e24337caf0474fcb9c`. Steam background state is not claimed globally immutable; the exact listed fixtures and immediate compatdata roster are the preservation boundary.
