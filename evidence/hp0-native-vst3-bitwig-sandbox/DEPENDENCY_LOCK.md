# HP0 VST3 SDK dependency lock

Classification: `observed`

```text
repository: https://github.com/steinbergmedia/vst3sdk.git
cache: <HOME>/.cache/linux-vst-bridge/dependencies/vst3sdk/3cdf9ca5d1f5b1b21e0a86832aa4abe55607bd96/
root commit: 3cdf9ca5d1f5b1b21e0a86832aa4abe55607bd96
description: VST3 SDK 3.8.1
license: MIT
copyright holder: Steinberg Media Technologies GmbH
root status: clean
```

Every entry emitted by `git submodule status --recursive` was initialized,
clean, and matched this lock:

| Path | Commit |
|---|---|
| `base` | `fcf9da0bd27a16f7f03773a3a39822f28f5c8477` |
| `cmake` | `054c9143cbb8d47fc4694e473f2ee3b4d951a8f5` |
| `doc` | `8bfca19d3b76a61d093951ba9297047f544caea1` |
| `pluginterfaces` | `4f547e8e102b47de4a8b8aaf343c73b700786372` |
| `public.sdk` | `586dc5e6c8012c3e4b01c79389375cbe96bdb1da` |
| `tutorials` | `33b73dfbb87f3fde3bce8c0a10cae934dc66ad34` |
| `vstgui4` | `5db272256172557818b6158cf0bb2c4410bddb25` |

The dependency verifier also required the pinned CMake plug-in helper, official
validator CMake target, `vstaudioeffect.h`, and `vsteditcontroller.h`. CMake
never downloaded a dependency and did not create a user plug-in symlink.

The unmodified pinned SDK adds the obsolete separate `stdc++fs` library on
Linux, while this GCC 15 SDK no longer ships it. Project-owned CMake removes
only that generated link item from the required SDK targets. The official
validator source, tests, registration, and output remain unmodified. The probe
also names the SDK's official `linuxmain.cpp` explicitly because the top-level
external-subdirectory integration does not propagate the sample-tree helper's
directory variable.
