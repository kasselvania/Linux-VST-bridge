# WF0 Windows Build-Plane Reconciliation

## Reconciliation identity

```yaml
slice: WF0
reconciliation: wf0-windows-build-plane-reconciliation-v1
design_revision: wf0-design-v6
authority_phase: reconnaissance_and_design
implementation_authorized: false
accepted_basis_commit: 67026ad7160a584cbf9cdb4bf0db7b8dbfa60136
accepted_basis_tree: 2021dfcbf000d934a462d40efef6bbe7b54e0862
mandatory_stop_result: RETURN_TO_DESIGN_GATE
provisional_source_commit: 5b166f4902af5e1e3b9c2287512f1f8f099c51d0
provisional_source_parent: 67026ad7160a584cbf9cdb4bf0db7b8dbfa60136
provisional_source_tree: fb6a14a78bcab6dd11b5720a725e9df841714743
provisional_source_path_count: 26
provisional_evidence_path_count: 0
provisional_source_status: preserved_salvage_material_not_accepted_implementation
mingw_posture: installed_exact_but_not_selected_for_wf0_again
windows_build_plane: github_actions_windows_2022_msvc_2022
implementation_authorized: false
```

This record reconciles the exact stopped V5 implementation attempt with the
V6 build-plane correction. It records source custody and design facts only. It
does not authorize implementation, run a build, change the SDK, or establish
the WF0 primary claim.

## Live preflight and custody

The pre-mutation readback on the accepted Steam Deck fixture established:

- x86_64 Steam Deck fixture and the declared repository/worktree locations;
- the accepted basis object and tree exist locally;
- the exact provisional commit is a one-commit child of that basis;
- its tree is `fb6a14a78bcab6dd11b5720a725e9df841714743`;
- its delta contains exactly the approved 26 source/configuration paths and no
  evidence path;
- both the primary Deck checkout and provisional worktree were clean;
- prohibited Bitwig, validator, Wine, Proton, Runtime 4, UMU, yabridge, and WF0
  probe process counts were all zero;
- WF0 build, artifact, receipt, and scan-stage root counts were all zero;
- no WR0 transaction sibling existed;
- SteamOS read-only mode was enabled;
- the user-scope MinGW extension was present at exact commit
  `f15a5a88eb09557f645860bfcb3c4a6bc267683fce06cb68436bd76376fca694`;
- Bitwig `6.1`, its Flatpak application/runtime commits, scope, origin,
  user-shadow absence, override hashes, and permission hash matched V5; and
- the accepted WR0 environment, runner, contract-source, marker, and retirement
  identities matched V5 and were treated as protected read-only state.

No mismatch was repaired or softened. The Deck `main` branch has unrelated
later history, but the required accepted basis object and tree remain exact;
the V6 design branch is created directly from that immutable object, not from
the moving branch name.

## Provisional-source preservation

Before any V6 design edit, the Deck repository gained the immutable local ref:

```text
refs/archive/wf0-minwg-provisional-5b166f
    -> 5b166f4902af5e1e3b9c2287512f1f8f099c51d0
```

The exact ref was bundled at the private-path-free logical location:

```text
<DECK_HOME>/.local/share/linux-vst-bridge/handoffs/
  wf0-minwg-provisional-5b166f.bundle
```

Bundle readback:

```yaml
advertised_ref: refs/archive/wf0-minwg-provisional-5b166f
advertised_commit: 5b166f4902af5e1e3b9c2287512f1f8f099c51d0
bundle_sha256: c442d429ff20447d616c62c4273bbefaedbe4b28cd4631e3fc5683c4bb2d1143
bundle_size_bytes: 730167
git_bundle_verify: passed
history_shape: complete
parent_readback: 67026ad7160a584cbf9cdb4bf0db7b8dbfa60136
tree_readback: fb6a14a78bcab6dd11b5720a725e9df841714743
delta_path_count: 26
evidence_delta_path_count: 0
result: WF0_PROVISIONAL_SOURCE_PRESERVED
```

The established SSH route copied the bundle to
`<MAC_PROJECT>/handoffs/wf0-minwg-provisional-5b166f.bundle`. The Mac SHA-256
equals the Deck SHA-256, `git bundle verify` passes, and the existing Mac clone
imports the same commit under the same archive ref. Mac readback reproduced the
exact parent, tree, 26-path delta, and zero evidence paths. The archive ref was
not pushed. The ordinary remote implementation branch was not moved.

## Exact upstream source identity

The failed route used the clean VST3 SDK root commit
`3cdf9ca5d1f5b1b21e0a86832aa4abe55607bd96`, tree
`38343890fd1a0cedd48b7ec80ef17da15231b6c8`, and its seven exact submodules.
The failure-controlling sources read by immutable Git object were:

| Component commit | Path | Git blob | SHA-256 |
|---|---|---|---|
| `3cdf9ca5d1f5b1b21e0a86832aa4abe55607bd96` | SDK root `CMakeLists.txt` | `f006a70c9b5d115928cd14d0580ca541d72717b0` | `5f3d95f1037d501e5c0018e282cff2c5e4a0f35e4c7f0c20ee6cf5d3357d6497` |
| `054c9143cbb8d47fc4694e473f2ee3b4d951a8f5` | `modules/SMTG_VstGuiSupport.cmake` | `0c63c1c2322c9e1374dd45b297e2508a0f93e8bc` | `4c99377c14b253aa7f7da699e7376db5239a129823516a6eaa89ec78c2ea25b5` |
| `5db272256172557818b6158cf0bb2c4410bddb25` | VSTGUI root `CMakeLists.txt` | `8fa41c406756788e44c7c301c3bb9d9b1ede5335` | `f8c6bacb19cbc7d309326ac407db68049028aa5579489121d331b541843a730a` |
| `5db272256172557818b6158cf0bb2c4410bddb25` | `vstgui/standalone/CMakeLists.txt` | `a1bbc822ea153d1ef1036241da79b437c82ddf40` | `70269facc46bb4094a93feccf0947e9efe92bfa76c2293612fbee857a06a0942` |
| `586dc5e6c8012c3e4b01c79389375cbe96bdb1da` | `samples/vst/again/CMakeLists.txt` | `f2616195f4f0b92b55ade45ac2fa448a4674ec79` | `b3b6865609cfe50338f210cc90b7f6d137204b145e05e01402baac19f95abc89` |

## Exact failed-build diagnosis

The failure chain is closed by the pinned sources:

1. With `SMTG_ENABLE_VSTGUI_SUPPORT=ON`, the SDK root calls
   `smtg_enable_vstgui_support`.
2. Exact `SMTG_VstGuiSupport.cmake` unconditionally sets
   `VSTGUI_STANDALONE ON` before adding the exact VSTGUI subdirectory.
3. Exact VSTGUI `vstgui/standalone/CMakeLists.txt` defines common, macOS,
   Win32, and GDK source lists, but populates `vstgui_standalone_sources` for
   Windows only inside `if(MSVC)`.
4. A MinGW Windows cross-compile is neither `MSVC`, `CMAKE_HOST_APPLE`, nor
   `LINUX`, so no applicable branch assigns sources before
   `add_library(vstgui_standalone STATIC ${vstgui_standalone_sources})`.
5. CMake therefore stops exactly with:

   ```text
   No SOURCES given to target: vstgui_standalone
   ```

6. Disabling VSTGUI cannot preserve the selected positive fixture. Exact
   AGain `CMakeLists.txt` returns immediately when
   `SMTG_ENABLE_VSTGUI_SUPPORT` is false and the `again` target explicitly
   links `vstgui_support` when enabled.
7. A local SDK/VSTGUI patch, copied SDK source, replacement standalone target,
   or handwritten substitute AGain build would move positive-fixture and build
   ownership outside the approved V5 design.

The mandatory `RETURN_TO_DESIGN_GATE` was therefore correct. This is a bounded
finding about pinned AGain plus its pinned VSTGUI integration. It does not
classify the exact MinGW installation as generally unusable for all future
Windows utilities.

## Supported build-plane selection

V6 selects the explicit GitHub-hosted x64 label `windows-2022`, never
`windows-latest`. GitHub's current official runner list exposes that versioned
label for Windows Server 2022 x64, and the current image inventory documents
Visual Studio Enterprise 2022, x64 C++ tools, CMake, Ninja, and multiple
Windows SDKs. Steinberg's exact pinned README uses the
`Visual Studio 17 2022` generator with `-A x64`; current Steinberg system
requirements identify MSVC 2022 as supported for Windows x86_64. Microsoft
documents the x64 native developer environment and its version-specific tool
discovery.

Design-time official sources, checked 2026-09-01:

- [GitHub runner labels](https://github.com/actions/runner-images/blob/main/README.md)
- [GitHub Windows Server 2022 image inventory](https://github.com/actions/runner-images/blob/main/images/windows/Windows2022-Readme.md)
- [GitHub hosted-runner selection](https://docs.github.com/en/actions/how-tos/write-workflows/choose-where-workflows-run/choose-the-runner-for-a-job)
- [Steinberg VST3 system requirements](https://steinbergmedia.github.io/vst3_dev_portal/pages/What%2Bis%2Bthe%2BVST%2B3%2BSDK/Index.html)
- [Steinberg Windows build instructions](https://github.com/steinbergmedia/vst3sdk/blob/master/README.md)
- [Microsoft C++ command-line build environment](https://learn.microsoft.com/en-us/cpp/build/building-on-the-command-line?view=msvc-170)

The design-time image document reported image `20260824.284.2`, Windows
`10.0.20348` build `5499`, Visual Studio Enterprise 2022 `17.14.37614.0`,
CMake `3.31.6`, Ninja `1.13.2`, and installed Windows SDKs including
`10.0.19041.0`. These values prove present availability only. They are not an
accepted future build identity. Every accepted workflow run must retain its
own observed image, OS, Visual Studio, `cl /Bv`, Windows SDK, CMake, and
generator receipts; a label/image change creates a new receipt and artifacts.

V6 selects:

```yaml
workflow_label: windows-2022
architecture: x86_64
visual_studio_generation: Visual Studio 2022
platform_toolset: v143
generator: Visual Studio 17 2022
generator_platform: x64
windows_sdk: 10.0.19041.0
cmake_minimum: 3.25.0
```

The exact compiler minor version and runner image release are observed build
facts, not silently pinned by the mutable workflow label.

## Corrected ownership split

```text
MacControlPlane
    -> repository, GitHub, exact workflow run selection, artifact download,
       provenance verification, SSH handoff, evidence retrieval/publication

WindowsBuildPlane (`windows-2022`, MSVC 2022)
    -> exact checkout and SDK acquisition, two clean builds, PE inspection,
       canonical manifest, bounded bundle, publication and attestation

SteamDeckExecutionPlane
    -> content-addressed local import, Runtime 4 / Proton 11 execution,
       failure proofs, cleanup, sanitized evidence and SSH return
```

The Deck has no ordinary GitHub credential, API, artifact-download, push, or
pull-request dependency. The Windows plane receives no Deck credential and
does not execute Proton or establish live Deck truth. The Mac does not claim
Windows compilation semantics or live Deck behavior merely because it moves
and verifies bytes.

## No-build and no-workload confirmation

This reconciliation performed source reads, identity checks, archival Git
operations, SSH bundle transfer, official-document research, and design work
only. It did not run GitHub Actions, compile with MSVC, retry MinGW, patch or
copy SDK/VSTGUI source, launch Proton, Wine, a scanner, validator, Bitwig, or
Serum, create a WF0 scan environment, or mutate the accepted WR0 environment.

```text
RETURN_TO_DESIGN_GATE retained
WF0_PROVISIONAL_SOURCE_PRESERVED
implementation_authorized=false
```
