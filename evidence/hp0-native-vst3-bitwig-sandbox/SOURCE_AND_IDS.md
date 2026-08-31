# LabHostProbe source and fixed identities

Classification: `observed`

| Item | Value |
|---|---|
| Internal module | `LabHostProbe` |
| Displayed plug-in | `LAB Host Probe` |
| Vendor | `Kasselvania Research` |
| Version | `0.1.0` |
| Format | Linux x86_64 VST3 |
| Processor CID | `6F4E7A5392E54B54A98AD6F714E0C201` |
| Controller CID | `B9C42F0736C34E218E5A71D40C8F1B62` |
| Gain parameter | `0x4C485001`, normalized, default `0.5` |
| Bypass parameter | `0x4C485002`, default off, bypass flag set |
| Processor state | magic `LHP0`, schema version `1` |
| Controller state | magic `LHC0`, schema version `1` |

The original project source defines one processor and one edit controller, one
stereo input, one stereo output, 32-bit and 64-bit processing, in-place and
separate-buffer operation, zero-sample handling, bounded missing-bus handling,
versioned state, and no custom editor.

The callback uses fixed object state and caller-owned buffers. It performs no
heap allocation, ordinary logging, filesystem access, network access, process
creation, worker-thread work, or synchronization setup. The fixture contains
no environment, runner, Windows-module, IPC, shared-memory, manager, broker,
Serum, or yabridge logic. It is test instrumentation, not the future bridge
proxy.

## Build-affecting source identity

Canonical schema: `linux-vst-bridge-hp0-build-source/v1`

Canonical manifest SHA-256:
`ee301e3e42b2381d5a9aa88ec64635482d098bc8cde50d33a57e6c397fb1e9ab`

Each manifest entry is the stable sorted tuple `tracked mode`, `Git blob ID`,
and repository-relative path. The exact covered path set is:

```text
CMakeLists.txt
cmake/HP0ModernGcc.cmake
cmake/HP0ToolchainGuard.cmake
cmake/HP0Vst3SdkLock.cmake
docs/HP0_DEPENDENCY_LOCK.md
native-probe/CMakeLists.txt
native-probe/README.md
native-probe/include/lab_host_probe/ids.h
native-probe/include/lab_host_probe/state.h
native-probe/source/controller.cpp
native-probe/source/controller.h
native-probe/source/factory.cpp
native-probe/source/processor.cpp
native-probe/source/processor.h
native-probe/source/version.h
tools/hp0-native-probe/build.sh
tools/hp0-native-probe/common.sh
tools/hp0-native-probe/validate.sh
tools/hp0-native-probe/verify-dependency.sh
```

Generation rejects missing or untracked expected paths, duplicates, unmerged
entries, dirty covered files, non-regular/symlinked sources, and any unexpected
tracked path beneath `cmake/` or `native-probe/`. Evidence-only changes do not
alter this identity. The final-head regeneration was byte-identical to both
receipt-bound manifests.
