# WF0 Reconnaissance — Supervised Windows VST3 Factory Census Probe

```yaml
slice: WF0
record_class: bounded_read_only_reconnaissance
selection_basis_commit: 745ca63bdd8641ade85cb9a024c1dc842681192d
selection_basis_tree: 275521adff574f165bb2b3e883c2ec909bae4c66
activation_head: 199569513ebba076e4fca90c75f7ae91f79c110d
activation_tree: 69d418d8c638bd8606b10d4e662d6c1a1cf25eff
authority_phase: reconnaissance_and_design
implementation_authorized: false
observed_at: 2026-09-01
result: WF0_DESIGN_RECONNAISSANCE_CLEAR
```

This record supports implementation design only. No compiler or Flatpak extension was installed; no source or binary was built; no Proton, Wine, Windows scanner, validator, Bitwig, or Serum workload was launched; no WF0 scan environment was created; and the accepted WR0 environment was inspected only through its bounded identity records.

## 1. Authority and preflight

The canonical Steam Deck repository design worktree began clean on `codex/wf0-windows-vst3-factory-census-design` at the exact activation head and tree above. The selection basis remained the fetched `main` identity. The activation branch contained exactly these two commits above the basis:

```text
9ba088fdb549b173987aeadb8bfaf4a918cbd805  Activate WF0 reconnaissance and design authority
199569513ebba076e4fca90c75f7ae91f79c110d  Record WF0 slice selection
```

Its cumulative basis diff contained only `CURRENT_SLICE.md` and `docs/slices/WF0/SLICE_SELECTION.md`. The authority card and receipt agreed on:

```text
authority_phase: reconnaissance_and_design
implementation_authorized: false
design_gate: required
successor_selection_authorized: false
```

No competing WF0 pull request or alternate WF0 branch owned this slice at preflight. Repository refs were transported without changing `origin` or recording credentials. The existing repository worktrees were not reset, cleaned, or overwritten; a unique clean WF0 design worktree was used.

### Live fixture readback

| Fact | Read-only result |
|---|---|
| Device family | Steam Deck Galileo |
| Operating system | SteamOS 3.8.16 |
| Architecture | `x86_64` |
| Graphical session | KDE Wayland |
| SteamOS root posture | read-only mode enabled |
| Linux harness interpreter | `/usr/bin/python3`, Python `3.13.5` |
| Forbidden workloads | Bitwig, validator, Wine/Wine64, wineserver, Proton workload, Steam Runtime workload, UMU workload, scanner, and yabridge host absent |
| Ordinary Steam UI services | present but outside the accepted WR0 workload guard; no Runtime 4 workload was active |
| WR0 transaction siblings | no stage, previous, retiring, or journal sibling |
| Accepted WR0 environment | only `wr0-proton11`; ownership record ready and cleanup held true |
| WR0 environment identity | `d3ed38d7ed53e9a5973cbf22fc504f2479dbdd15616fe1bfc1d5e1cd0bf5f4c4` |
| Runner/runtime digest | `2d64df1d36786ca2d0e955c553005423dc2b5bdd714bd0a17872622e33912547` |
| Contract-source digest | `c6543004fbcd70252393f0e0cab4f9ad7a31e85f433ce3e903e690244cc79541` |
| Protected-environment inspection | bounded marker/manifest readback only; no prefix traversal or mutation |

No hostname, network address, serial number, account value, credential, raw process identifier, or private absolute checkout path is retained here.

## 2. Flatpak SDK and runtime inventory

The exact installed inventory was read from user and system Flatpak installations. A commit below is an installed OSTree commit, not merely a branch name.

### User installation

| Ref | Advertised version | Installed commit |
|---|---|---|
| `org.freedesktop.Platform/x86_64/25.08` | `freedesktop-sdk-25.08.16` | `bd44a6230581917d04f89812a4c21090c304d390edb73995af1c2f9fd8abf4e8` |
| `org.freedesktop.Platform.GL.default/x86_64/25.08` | `26.1.6` | `bcfd828b0c4739753cadb31964f8c1b0f90c8d2bed79bda2c2760438eb48c4b3` |
| `org.freedesktop.Platform.GL.default/x86_64/25.08-extra` | `26.1.6` | `37b03bfd88271f49ba6eaa00c07155716870497c66759972bc81d67153e35138` |
| `org.freedesktop.Platform.codecs-extra/x86_64/25.08-extra` | not advertised | `3f422b5306e0c1a1c6855fb36eeff26237d9d9c9a0abcb6d35bd9555ea75efed` |
| `org.freedesktop.Sdk/x86_64/25.08` | `freedesktop-sdk-25.08.16` | `b90ed309cc1d505dea48b6a2121c5dcfac22868120eee643b0596d31f96b9bb8` |
| `org.gtk.Gtk3theme.Breeze/x86_64/3.22` | `6.7.2` | `77495563abf1b523c653bf0b211c39fda1596cde833768baee4894f91b285c8c` |
| `org.kde.Platform/x86_64/6.10` | not advertised | `d0d8f7888350e93c0e6d009d79c5b143f6f6dde09de28ff93a7dc8a14a848c16` |
| `org.kde.Platform/x86_64/6.11` | not advertised | `3adaa41de78d95076617f743099ec3851b5ae4fcdadbaaa8b7df1412c705c56c` |

The selected SDK is a user installation from `flathub`, collection `org.flathub.Stable`, with installed size approximately 1.7 GB. Its exact ref is `runtime/org.freedesktop.Sdk/x86_64/25.08`, and the installed commit date is 2026-08-16. The full installed commit above is authoritative.

### System installation

| Ref | Advertised version | Installed commit |
|---|---|---|
| `org.freedesktop.Platform/x86_64/24.08` | `freedesktop-sdk-24.08.36` | `4ba2d4a7248a77f6510ffab405a729232105958d8347854ab8be74ec67229192` |
| `org.freedesktop.Platform/x86_64/25.08` | `freedesktop-sdk-25.08.16` | `bd44a6230581917d04f89812a4c21090c304d390edb73995af1c2f9fd8abf4e8` |
| `org.freedesktop.Platform.Compat.i386/x86_64/24.08` | not advertised | `88db92773b8ed6b2f712eb7e41be3568f20e408e68f610c06c27d843fd7e4e63` |
| `org.freedesktop.Platform.Compat.i386/x86_64/25.08` | not advertised | `49ccc93d00631b442eac263a96beabe35b8552d0604bb0a0c03688ec27f7bcf5` |
| `org.freedesktop.Platform.GL.default/x86_64/24.08` | `26.1.8` | `2a179c841db88673819276990b1b3dab8b0646ef0bcd43174bc5d85856443ba7` |
| `org.freedesktop.Platform.GL.default/x86_64/24.08extra` | `26.1.8` | `2669215ab0d8ee4bb3bbaabf0eb8be8f3bb4a1f572be7ce3ef1052e6c0158fa6` |
| `org.freedesktop.Platform.GL.default/x86_64/25.08` | `26.1.6` | `bcfd828b0c4739753cadb31964f8c1b0f90c8d2bed79bda2c2760438eb48c4b3` |
| `org.freedesktop.Platform.GL.default/x86_64/25.08-extra` | `26.1.6` | `37b03bfd88271f49ba6eaa00c07155716870497c66759972bc81d67153e35138` |
| `org.freedesktop.Platform.GL32.default/x86_64/24.08` | `26.1.8` | `cacee2454f6440eaa2de7ba794c5de4a68a0384b2d7aa2f9949fd05beef93624` |
| `org.freedesktop.Platform.GL32.default/x86_64/25.08` | `26.1.6` | `d83daada1d8407a685a0ad6d2cfacf2edb43709d8735c1a6581c705e96c5f3c9` |
| `org.freedesktop.Platform.codecs-extra/x86_64/25.08-extra` | not advertised | `3f422b5306e0c1a1c6855fb36eeff26237d9d9c9a0abcb6d35bd9555ea75efed` |
| `org.freedesktop.Platform.ffmpeg-full/x86_64/24.08` | not advertised | `f325cad5868fcc77c1433c84f98ac24ad1d161b528dfa7d53ba357f389a6441c` |
| `org.freedesktop.Platform.ffmpeg_full.i386/x86_64/24.08` | not advertised | `4ffc09f62d90d7373c94733e9927663d18ab2dce7ff8c18909864249ec1f5ae8` |
| `org.freedesktop.Platform.openh264/x86_64/2.5.1` | `2.5.1` | `0f52621e4540863ee86b1fe26216fff78fefa1096f367079692344139228e474` |
| `org.gnome.Platform/x86_64/50` | not advertised | `545da92354a265d2c3572c91c39ac14dd7e74f9d8f9b66744ad50f478d2497c5` |
| `org.gtk.Gtk3theme.Breeze/x86_64/3.22` | `6.7.2` | `77495563abf1b523c653bf0b211c39fda1596cde833768baee4894f91b285c8c` |
| `org.kde.Platform/x86_64/6.10` | not advertised | `d0d8f7888350e93c0e6d009d79c5b143f6f6dde09de28ff93a7dc8a14a848c16` |
| `org.winehq.Wine.gecko/x86_64/stable` | not advertised | `e764c7223a9a64d87b7811f3f4b60ea28dbe1ffa54f2a09b14671397ca6bb4f4` |
| `org.winehq.Wine.mono/x86_64/stable` | not advertised | `6d68124f0562dc042c8611d97b018b1243e033f4427a00d88fbdf04065cb10e9` |

No system `org.freedesktop.Sdk` and no user or system MinGW extension was installed.

## 3. Exact Windows cross-toolchain route

### Availability and identity

Read-only Flathub metadata established this available but absent extension:

```yaml
id: org.freedesktop.Sdk.Extension.mingw-w64
ref: runtime/org.freedesktop.Sdk.Extension.mingw-w64/x86_64/25.08
arch: x86_64
branch: 25.08
advertised_version: 14.0.0
origin: flathub
collection: org.flathub.Stable
download_size: 283.8 MB
installed_size_estimate: 1.1 GB
remote_commit: f15a5a88eb09557f645860bfcb3c4a6bc267683fce06cb68436bd76376fca694
remote_parent: f4617c2f95d603df2a379cf94ddfaf6f493932f77ae6e38842d4eb8371886877
remote_commit_date: 2026-08-26T11:11:59Z
```

The Flathub build source is [`flathub/org.freedesktop.Sdk.Extension.mingw-w64`](https://github.com/flathub/org.freedesktop.Sdk.Extension.mingw-w64/tree/345e5766c6cf8016cc62cb7755bd1711a7d739aa) at source commit `345e5766c6cf8016cc62cb7755bd1711a7d739aa`, branch `branch/25.08`; the manifest blob observed at that commit is `f9fa5d5261516151412ff294b978594172d971af`. The Flathub OSTree commit subject explicitly binds that source commit.

The manifest declares:

| Component | Exact version / shape |
|---|---|
| Extension prefix | `/usr/lib/sdk/mingw-w64` |
| MinGW-w64 | `14.0.0` |
| GCC | `15.2.0`; C, C++, and LTO; POSIX threads; shared and static runtimes; no multilib |
| GNU binutils | `2.45.1`; deterministic archives enabled; `x86_64` and `i686` targets |
| Headers/CRT | all SDK headers; x86_64 CRT selected for WF0 |
| Thread runtime | winpthreads shared and static variants |
| Tools | `gendef`, `genidl`, `genpeimg`, and `widl` for x86_64/i686 |
| Extension mount contract | `ExtensionOf=org.freedesktop.Sdk/x86_64/25.08`, beneath `/usr/lib/sdk` |

Expected selected executable paths after an exact installation are:

```text
/usr/lib/sdk/mingw-w64/bin/x86_64-w64-mingw32-gcc
/usr/lib/sdk/mingw-w64/bin/x86_64-w64-mingw32-g++
/usr/lib/sdk/mingw-w64/bin/x86_64-w64-mingw32-ar
/usr/lib/sdk/mingw-w64/bin/x86_64-w64-mingw32-ld
/usr/lib/sdk/mingw-w64/bin/x86_64-w64-mingw32-nm
/usr/lib/sdk/mingw-w64/bin/x86_64-w64-mingw32-objdump
/usr/lib/sdk/mingw-w64/bin/x86_64-w64-mingw32-readelf
/usr/lib/sdk/mingw-w64/bin/x86_64-w64-mingw32-strip
/usr/lib/sdk/mingw-w64/bin/x86_64-w64-mingw32-windres
/usr/lib/sdk/mingw-w64/bin/x86_64-w64-mingw32-widl
```

The manifest does not provide a project-specific CMake toolchain file. WF0 therefore needs a repository-owned toolchain file that names the full compiler, resource compiler, and binutils paths, sets `CMAKE_SYSTEM_NAME=Windows`, sets `CMAKE_SYSTEM_PROCESSOR=x86_64`, and constrains `CMAKE_FIND_ROOT_PATH` to the MinGW prefix. Ambient `PATH` or compiler auto-detection is not trusted.

The extension is exposed automatically at `/usr/lib/sdk/mingw-w64` when a command runs inside the matching installed SDK. The future execution shape is:

```text
flatpak run --user --devel \
  --filesystem=<SOURCE_ROOT>:ro \
  --filesystem=<BUILD_ROOT>:rw \
  --command=/usr/bin/cmake \
  org.freedesktop.Sdk//25.08 \
  <exact CMake arguments>
```

Compiler identity probes use the same SDK invocation with `--command=/usr/lib/sdk/mingw-w64/bin/<tool>`. No host package-manager route is needed.

### Future exact installation and trust gate

Installation is not authorized by this record. If separately authorized, the design selects only this user-scope sequence:

```text
flatpak install --user --noninteractive flathub \
  org.freedesktop.Sdk.Extension.mingw-w64//25.08

flatpak update --user --noninteractive \
  --commit=f15a5a88eb09557f645860bfcb3c4a6bc267683fce06cb68436bd76376fca694 \
  org.freedesktop.Sdk.Extension.mingw-w64//25.08
```

Before any build is trusted, readback must prove the SDK commit `b90ed309cc1d505dea48b6a2121c5dcfac22868120eee643b0596d31f96b9bb8`, the extension commit `f15a5a88eb09557f645860bfcb3c4a6bc267683fce06cb68436bd76376fca694`, each full tool path, compiler/binutils version output, x86_64 target triple, sysroot/search directories, and runtime DLL identities. A different deployed commit is `WF0_TOOLCHAIN_INSTALL_BLOCKED`; it is not silently accepted because the branch still says `25.08`.

The install is user-scope, exact-branch, noninteractive, reversible with an exact-ref user uninstall, and independent of the immutable SteamOS base. The implementation design does not automatically uninstall an exact extension that another task may use; it records preexistence and never changes a wrong preexisting deployment.

Expected non-system runtime DLL candidates are `libgcc_s_seh-1.dll`, `libstdc++-6.dll`, and `libwinpthread-1.dll` under the x86_64 target prefix. Their existence, hashes, dependency closure, and license material remain future post-install readbacks, not current observations.

## 4. Pinned VST3 SDK

The HP0 checkout was located without retaining its private absolute path. It was clean, detached at the required root, and every recursive submodule was present and clean:

| Repository | Commit |
|---|---|
| root | `3cdf9ca5d1f5b1b21e0a86832aa4abe55607bd96` |
| `base` | `fcf9da0bd27a16f7f03773a3a39822f28f5c8477` |
| `cmake` | `054c9143cbb8d47fc4694e473f2ee3b4d951a8f5` |
| `doc` | `8bfca19d3b76a61d093951ba9297047f544caea1` |
| `pluginterfaces` | `4f547e8e102b47de4a8b8aaf343c73b700786372` |
| `public.sdk` | `586dc5e6c8012c3e4b01c79389375cbe96bdb1da` |
| `tutorials` | `33b73dfbb87f3fde3bce8c0a10cae934dc66ad34` |
| `vstgui4` | `5db272256172557818b6158cf0bb2c4410bddb25` |

The root tree was `38343890fd1a0cedd48b7ec80ef17da15231b6c8`. No generated file, local patch, or missing submodule was required for inspection.

### Source identities and findings

| Subject | File blob | Finding |
|---|---|---|
| root CMake | `f006a70c9b5d115928cd14d0580ca541d72717b0` | SDK project `3.8.1`; plugin examples and VSTGUI enabled by option |
| MinGW platform setup | `d1d0db26f8a4be1fe87146b50de5baf2881464ed` | explicit `SMTG_WIN && MINGW` branch; GNU flags and no-undefined linking |
| Windows bundle layout | `61ce003169b2ad1db892eb6578f5c670b1eec52a` | MinGW removes `lib` prefix; emits `Contents/<arch>-win` when bundle creation is enabled |
| validator/module-info hooks | `6cc4c01015a67cc3dbd37bc2473c813372fa88ed` | post-build validator and module-info generation default on; both must be disabled for cross-build WF0 |
| Windows validator target | `391623664128d9d141abe3c14887c1359b70407a` | a Windows hosting target using `module_win32.cpp`; it instantiates classes and exceeds WF0's claim |
| official Win32 module helper | `7616bd8566141c8f63413869ebe46d2938b67b28` | loads module, requires `GetPluginFactory`, optionally calls `InitDll`/`ExitDll`, but collapses stages and ignores the exit boolean |
| Windows entry/exit exports | `c32d1374b55d5db24f002aa97cdf92cda31f0ab7` | `InitDll` and `ExitDll` return `bool` and are exported by the SDK target |
| factory implementation | `bf41a9cb885c364d24bb1ac132b8d8bf4279c4dc` / `a50c5000c1215ced5ae920a07c12153edb9b2498` | official `IPluginFactory1/2/3` implementation and release semantics |
| factory structures | `859424bfc7f14b61df4b209a85c09d413513127a` | exact bounded arrays and flags for `PFactoryInfo`, `PClassInfo`, `PClassInfo2`, and `PClassInfoW` |
| TUID/FUID layout | `3f6de83b104484e09097411417b37bcd28a46a1c` | Windows is COM-compatible; in-memory TUID byte order differs from four-word printable FUID form |
| VST factory defaults | `e20ef5f429349bdccf55367d45051fcdb0ff97c5` | `BEGIN_FACTORY_DEF` selects `PFactoryInfo::kUnicode` |
| VST version string | `ec4ff4021a4afd960d061d5e0ba32e0618136a9f` | exact SDK string `VST 3.8.1` |

The official helper is evidence for correct ordering, not the selected scanner implementation: it cannot independently report all WF0 stages, treats missing required export inside one load result, and does not expose `ExitDll=false`. The implementation design therefore selects explicit Win32 loading while using only the official VST3 interface definitions.

The root, `public.sdk`, `pluginterfaces`, and `cmake` license files are MIT at these commits. `vstgui4/LICENSE` permits source and binary redistribution under its notice, disclaimer, and no-endorsement conditions. Binary redistribution is not part of WF0; locally built scanner, AGain, and runtime DLLs remain untracked.

## 5. AGain reference fixture

### Exact source

The selected target definition is `public.sdk/samples/vst/again/CMakeLists.txt`, blob `f2616195f4f0b92b55ade45ac2fa448a4674ec79`. Its complete AGain-specific input roster is:

| Repository-relative path | Blob |
|---|---|
| `public.sdk/source/vst/utility/test/midiconverttest.cpp` | `fbed2e2373f2f61b933d5155189f411b6da07bcf` |
| `public.sdk/samples/vst/again/source/again.cpp` | `4676454679e37f188b99c2ec6e6def6b825da173` |
| `public.sdk/samples/vst/again/source/again.h` | `061c0d4afb5f59410e75681d9998871f32fb1e72` |
| `public.sdk/samples/vst/again/source/againcids.h` | `d32d1640ea187e718baba9cbb039d3a436d53cce` |
| `public.sdk/samples/vst/again/source/againcontroller.cpp` | `5e2fe55cc68b94386a2034f3aad672566941d2c6` |
| `public.sdk/samples/vst/again/source/againcontroller.h` | `9182b7cf2969fd2d46234387a587cf5516999cc4` |
| `public.sdk/samples/vst/again/source/againentry.cpp` | `13b920b4b7a74137301bf213cf048e96e82861d4` |
| `public.sdk/samples/vst/again/source/againparamids.h` | `6ea6d64190158f139fafbea21bb3b5fd9b065d22` |
| `public.sdk/samples/vst/again/source/againprocess.h` | `7f285ec9998fcb37c141410fb556f7937ef275ab` |
| `public.sdk/samples/vst/again/source/againsidechain.cpp` | `7c8e457227055e5c3bfebe8a80562ca498545b08` |
| `public.sdk/samples/vst/again/source/againsidechain.h` | `85c932b383da75cae643b4a8acd35f5d6d68f1e0` |
| `public.sdk/samples/vst/again/source/againuimessagecontroller.h` | `c6d5a5794cb70287d2e9449f58cedfd87127e52d` |
| `public.sdk/samples/vst/again/source/version.h` | `dc7d37921973bc3edaa3a1376b38b5e68d5d0ed8` |
| `public.sdk/samples/vst/again/resource/again.rc` | `d6f2a0b81aae83d840a36f96e5591a046297e890` |
| `public.sdk/samples/vst/again/resource/again.uidesc` | `440c3ee71f01df9d5708af69a2ec3f94084501fa` |
| `public.sdk/samples/vst/again/resource/background.png` | `a37d9cae57f88ac587be00e52047cbf3fcaf4b59` |
| `public.sdk/samples/vst/again/resource/slider_background.png` | `56716b9cd6cc6f97ec83dd4cc360d98fad3b52c4` |
| `public.sdk/samples/vst/again/resource/slider_handle.png` | `cb0d5e7c0aacae639cd1b954133db962a9192892` |
| `public.sdk/samples/vst/again/resource/slider_handle_2.0x.png` | `08f6a9d7da4fe5a91cd1ea922909f09631530297` |
| `public.sdk/samples/vst/again/resource/vu_on.png` | `38d8408d7ceb9310b79e229ff6609b437529dd68` |
| `public.sdk/samples/vst/again/resource/vu_off.png` | `b4abf97a2cc6ac61a35ebac2ce73710f54d38c1d` |
| `public.sdk/samples/vst/again/resource/84E8DE5F92554F5396FAE4133C935A18_snapshot.png` | `5eb2d4d3b62aa8b17e4a5cd8ee27c308b5ba097f` |
| `public.sdk/samples/vst/again/resource/84E8DE5F92554F5396FAE4133C935A18_snapshot_2.0x.png` | `515dd9928c072389791681342ee0cd07087537fa` |
| `public.sdk/samples/vst/again/resource/41347FD6FED64094AFBB12B7DBA1D441_snapshot.png` | `5eb2d4d3b62aa8b17e4a5cd8ee27c308b5ba097f` |
| `public.sdk/samples/vst/again/resource/41347FD6FED64094AFBB12B7DBA1D441_snapshot_2.0x.png` | `515dd9928c072389791681342ee0cd07087537fa` |

Transitive SDK/VSTGUI input identity is the exact recursive submodule lock above; the future build receipt expands its actually compiled source/dependency closure rather than inventing a smaller source claim.

The target requires `sdk` and `vstgui_support`. It includes the Windows resource file when `SMTG_WIN`, and the official CMake bundle code selects `x86_64-win` from the 64-bit target size. It does not require changing upstream source. `SMTG_RUN_VST_VALIDATOR=OFF` is mandatory because the cross-built validator cannot run during the Linux build and would exceed WF0; `SMTG_CREATE_MODULE_INFO=OFF` likewise prevents a cross-built helper from being executed on the Linux build host. The exact target build is `again`, not an all-target build.

### Expected factory metadata

| Field | Exact source-derived value |
|---|---|
| vendor | `Steinberg Media Technologies` |
| URL | `http://www.steinberg.net` |
| email | `mailto:info@steinberg.de` |
| factory flags | decimal `16`, `PFactoryInfo::kUnicode` |
| class count | `3` |
| plug-in version | `3.8.1.0` |
| SDK version | `VST 3.8.1` |

Each AGain `PClassInfo2` vendor field is source-empty; the normalized census must not silently replace it with the factory vendor.

### Expected ordered class census

The printable ID is the four-word FUID form. `raw_tuid_hex` is the exact 16 bytes returned by the Windows COM-compatible `TUID` array.

| Ordinal | Printable class ID | `raw_tuid_hex` | Name | Category | Subcategory | Cardinality | Class flags |
|---:|---|---|---|---|---|---:|---:|
| 0 | `84E8DE5F92554F5396FAE4133C935A18` | `5FDEE8845592534F96FAE4133C935A18` | `AGain VST3` | `Audio Module Class` | `Fx` | `2147483647` | `1` (`kDistributable`) |
| 1 | `D39D5B65D7AF42FA843F4AC841EB04F0` | `655B9DD3AFD7FA42843F4AC841EB04F0` | `AGain VST3Controller` | `Component Controller Class` | empty | `2147483647` | `0` |
| 2 | `41347FD6FED64094AFBB12B7DBA1D441` | `D67F3441D6FE9440AFBB12B7DBA1D441` | `AGain SideChain VST3` | `Audio Module Class` | `Fx` | `2147483647` | `1` (`kDistributable`) |

The controller name has no space before `Controller` because the source concatenates adjacent string literals. The registration ordinal is authoritative; normalization must not sort classes.

### Expected bundle and dependencies

```text
again.vst3/
  Contents/
    x86_64-win/
      again.vst3
    Resources/
      again.uidesc
      background.png
      slider_background.png
      slider_handle.png
      slider_handle_2.0x.png
      vu_on.png
      vu_off.png
      Snapshots/
        84E8DE5F92554F5396FAE4133C935A18_snapshot.png
        84E8DE5F92554F5396FAE4133C935A18_snapshot_2.0x.png
        41347FD6FED64094AFBB12B7DBA1D441_snapshot.png
        41347FD6FED64094AFBB12B7DBA1D441_snapshot_2.0x.png
  PlugIn.ico
  desktop.ini
```

The icon and desktop metadata are added by SDK CMake from blobs `a31b6293884365e5cc314433d304a6a0e2dfb0aa` and `6e483d85482cf07247ab626edc4f1506621b55cf`. The same post-build rule invokes the Windows `attrib` command even during a Linux-hosted cross-build. WF0 must place a repository-owned, exact-argument host shim named `attrib` ahead of `/usr/bin` for this build only. It accepts only `attrib +s <existing-path-inside-the-owned-AGain-build-root>`, performs no unsupported Linux approximation of Windows Explorer attributes, and rejects every other invocation. This drives the pinned target without patching its source; folder presentation is outside the factory claim.

The exact generated roster must be read back after building; unlisted files are rejected unless this design returns for review. Required exports are `GetPluginFactory` and, for this exact SDK-built target, expected `InitDll` and `ExitDll`. WF0 treats the latter two as optional at the scanner boundary even though the selected positive fixture is expected to export them.

The source uses Windows Direct2D/DirectWrite/WIC/DWM/IME/Direct3D support through VSTGUI. Several source files rely on MSVC `#pragma comment(lib, ...)`, which GCC does not use as the authority for linking. The repository toolchain must therefore supply a declared Windows system import-library closure without patching VSTGUI. The candidate set is `d2d1`, `dwrite`, `windowscodecs`, `dwmapi`, `shlwapi`, `imm32`, `d3d11`, `dxgi`, `ole32`, `oleaut32`, `uuid`, `comdlg32`, `comctl32`, `shell32`, `user32`, `gdi32`, `advapi32`, and `winmm`; the build receipt must prove the final PE import table and reject undeclared non-system dependencies.

Dynamic GCC runtime DLLs are preferred over silently changing upstream link semantics. Only exact runtime DLLs from the locked extension may be copied beside the untracked scanner/module artifacts, and each copy must be hashed. This choice and the system-import list are future build gates, not a claim that linking has already succeeded.

### Build-feasibility result

```text
WF0_AGAIN_BUILD_FEASIBILITY: CLEAR_FOR_IMPLEMENTATION_PROOF
```

The pinned SDK has explicit MinGW toolset handling, Windows resource compilation, no-`lib` prefix behavior, and Windows VST3 bundle layout. The available extension declares the necessary x86_64 C/C++ compiler, binutils, resource compiler, headers, CRT, and runtime libraries. The selected route needs a repository-owned toolchain/lock, explicit system libraries, and the bounded host `attrib` shim described above, but no upstream SDK source change. Actual configure, compile, link, PE inspection, runtime-DLL closure, post-build invocation verification, and two-build comparison remain mandatory implementation proofs. Failure of any of those exact gates returns to the design gate; another fixture is not substituted.

## 6. Factory API and loading design findings

The narrow observable route is:

```text
LoadLibraryExW(exact module path, constrained search)
  -> optional GetProcAddress("InitDll") and boolean result
  -> required GetProcAddress("GetPluginFactory")
  -> GetPluginFactory(), non-null IPluginFactory
  -> getFactoryInfo
  -> query only IPluginFactory2 and IPluginFactory3
  -> countClasses
  -> ordered getClassInfoUnicode / getClassInfo2 / getClassInfo fallback
  -> release queried factory interfaces and base factory
  -> optional GetProcAddress("ExitDll") and boolean result
  -> FreeLibrary
```

The base factory returned by `GetPluginFactory` is required. Factory 2 and 3 support are observations. No class is instantiated, and no component, controller, view, bus, parameter, event, state, process, or host-context interface is queried. The fixed SDK buffers make bounded validation possible: factory vendor 64 bytes, URL 256, email 128; class category 32, name 64, vendor/version/SDK version 64, and subcategories 128; Unicode class fields use the corresponding 64-code-unit limits.

Every buffer is zero-initialized before the call. A missing terminator, invalid UTF-8/UTF-16, invalid count, duplicate 16-byte class ID, impossible result/pointer pair, or output-cap breach is a stage-specific failure. Exact occupied source bytes are retained beside strict normalized text; padding and uninitialized bytes are never retained.

## 7. Proposed disposable environment and supervision

The design can reuse the accepted WR0 Runtime 4/Proton 11 identity and process laws without reusing or modifying its environment. The exact proposed root template is:

```text
<HOME>/.local/share/linux-vst-bridge/environments/
  .wf0-factory-census.stage-<32-hex-run-id>/
    wf0-environment.json
    compatdata/pfx/drive_c/wf0/
      bin/
      fixture/
      session/
    runtime-var/
    host-cache/
    host-config/
    host-data/
    receipts/
```

Scanner and fixture artifacts are copied into `drive_c`; no arbitrary host-drive mapping is needed. Their Windows paths are `C:\wf0\bin\wf0-factory-probe.exe` and `C:\wf0\fixture\again.vst3\Contents\x86_64-win\again.vst3`. The stage root must be initially absent, carry an exact ownership marker before use, and be removed after bounded evidence is captured and all owned descendants reach zero. An unknown preexisting object is refused, not repaired or deleted.

The scanner announces a nonce/session/build/module-bound ready record before `LoadLibraryExW`, then waits for the matching atomic gate. The Linux supervisor verifies the exact Runtime root, Proton process, Wine-hosted scanner command vector, both ancestry chains, ready record, stdout stage, hashes, and continued gate absence immediately before publishing that gate. From then on, the last validated scanner stage owns timeout/crash classification.

The process tree is observed from the exact Runtime root PID/start identity, not by name. Cleanup signals only the isolated owned group and freshly revalidated escaped descendants, and completion requires no owned descendant. An unrelated same-family sentinel must survive negative cleanup proof. Raw PIDs and command lines are transient and sanitized from evidence.

## 8. Licensing, security, and redistribution

- SDK root, `public.sdk`, `pluginterfaces`, and SDK CMake material are MIT at the pinned commits; retained source use must preserve applicable notices.
- VSTGUI uses its pinned permissive license with notice, disclaimer, and no-endorsement conditions.
- The Flathub extension advertises `ZPL-2.1 and GPL-3.0-or-later and LGPL-3.0-or-later and GPL-2.0-or-later and LGPL-2.0-or-later and LicenseRef-Public-Domain`. Exact runtime-DLL notices and any source-offer obligation must be captured from the installed extension before distribution is considered.
- WF0 performs local build/test use only. It does not commit or distribute the SDK, AGain binary, scanner binary, MinGW runtime DLLs, complete prefixes, or proprietary material.
- No network is permitted during configure/build/scan. The separately authorized Flatpak installation is the only contemplated networked acquisition.
- The scan environment contains only open reference/test instrumentation. It must contain no credential, browser state, vendor installer, license material, Serum data, or Bitwig project data.

## 9. Material unknowns and implementation gates

These are gaps to be closed by a separately approved implementation, not softened into current claims:

1. The extension's executable version output, runtime-DLL roster, and hashes have not been observed because the extension is absent.
2. The pinned AGain target and scanner have not been configured, compiled, or linked through this exact extension.
3. The exact MinGW system-library link closure for current VSTGUI has not been exercised.
4. PE32+ architecture, exports, imports, and bundle roster have not been read from built artifacts.
5. Raw byte reproducibility has not been established; the design therefore permits only a declared normalized PE comparison if byte identity fails solely in explicitly named fields.
6. AGain's expected `InitDll`/`ExitDll` exports and factory data are source-derived, not binary/runtime observations.
7. A Wine-hosted scanner process topology under the accepted Runtime/Proton lane has not been observed; WR0 proved only the accepted command workload.
8. Real `FreeLibrary` failure may not be deterministically inducible. Only a bounded internal loader-adapter fault may prove the error mapping; the live claim remains successful AGain unload.
9. No current fact establishes class instantiation, audio, GUI, Bitwig, Serum, general VST3, or product-runner behavior.

If actual implementation needs a different compiler route, upstream SDK patch, different reference fixture, different runner/runtime, accepted-WR0 mutation, persistent product environment, wider process owner, class instantiation, new IPC, native proxy, audio, GUI, Bitwig, Serum, or paths outside the approved envelope, it must stop and return to the design gate.

## 10. Verdict and no-mutation confirmation

```text
WF0_DESIGN_RECONNAISSANCE_CLEAR
```

The exact user-scope 25.08 MinGW route is available and statically capable of supporting the selected narrow implementation. The pinned SDK and AGain source are clean, complete, licensed for the proposed local proof, and expose the required factory boundary without an upstream patch. The remaining uncertainties are executable implementation gates already owned by the design; none requires changing the selected primary claim or fixture today.

During this reconnaissance:

```text
package_or_extension_installed: false
windows_artifact_built: false
windows_or_proton_workload_launched: false
scan_environment_created_or_mutated: false
accepted_wr0_environment_mutated: false
bitwig_or_serum_accessed: false
repository_product_source_modified: false
```
