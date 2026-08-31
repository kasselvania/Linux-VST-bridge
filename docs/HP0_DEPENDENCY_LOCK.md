# HP0 dependency lock

HP0 uses one third-party source dependency and does not vendor it.

## Official VST3 SDK

```text
repository: https://github.com/steinbergmedia/vst3sdk.git
root commit: 3cdf9ca5d1f5b1b21e0a86832aa4abe55607bd96
root description: VST3 SDK 3.8.1
license: MIT
cache: <HOME>/.cache/linux-vst-bridge/dependencies/vst3sdk/3cdf9ca5d1f5b1b21e0a86832aa4abe55607bd96/
```

Recursive root submodule lock:

| Path | Commit |
|---|---|
| `base` | `fcf9da0bd27a16f7f03773a3a39822f28f5c8477` |
| `cmake` | `054c9143cbb8d47fc4694e473f2ee3b4d951a8f5` |
| `doc` | `8bfca19d3b76a61d093951ba9297047f544caea1` |
| `pluginterfaces` | `4f547e8e102b47de4a8b8aaf343c73b700786372` |
| `public.sdk` | `586dc5e6c8012c3e4b01c79389375cbe96bdb1da` |
| `tutorials` | `33b73dfbb87f3fde3bce8c0a10cae934dc66ad34` |
| `vstgui4` | `5db272256172557818b6158cf0bb2c4410bddb25` |

Every submodule is initialized recursively so the dependency state is complete and inspectable even though HP0 disables SDK samples, unrelated hosts, and VSTGUI at configuration time.

The root `LICENSE.txt` identifies the SDK as MIT and requires preservation of its copyright and permission notice. HP0 compiles against the cached source but commits neither SDK source nor generated SDK binaries.

## Toolchain lock

HP0 builds only inside the user-scope Flatpak runtime:

```text
ref: runtime/org.freedesktop.Sdk/x86_64/25.08
version: freedesktop-sdk-25.08.16
commit: b90ed309cc1d505dea48b6a2121c5dcfac22868120eee643b0596d31f96b9bb8
origin: flathub
scope: user
```

The build refuses another runtime ref/commit, compiler identity/version, architecture, missing submodule, dirty checkout, or mismatched commit.

## Modern GCC link compatibility

The pinned SDK's `SMTG_PlatformToolset.cmake` adds the historical separate `stdc++fs` link library on Linux. GCC 15 provides `std::filesystem` from `libstdc++` and the Freedesktop 25.08 SDK no longer ships `libstdc++fs`.

`cmake/HP0ModernGcc.cmake` removes only that obsolete link entry from generated SDK targets when the verified compiler is modern GNU C++. It does not edit the SDK checkout, validator sources, test registration, or validator output.
