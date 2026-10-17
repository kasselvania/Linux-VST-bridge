# Common source integration comparison

This is a clean source comparison for #430, not the selected production base.
`source.json` pins the historical Proton tree, its matching SDK, DComp Wine
source, patch inputs and integrated source postimages. It defines build inputs
only; it is not an installed runtime catalogue, configuration or support record.

```sh
python3 tools/runtime/prepare.py --check-upstream
python3 tools/runtime/prepare.py --check-source /private/pristine-wine-c27f058
python3 tools/runtime/redist-inputs.py --check-upstream
# Reserved homelab builder (takes the shared lock and resource limits):
bash tools/runtime/build.sh /mnt/Storage2/LinuxPluginBridge/runtime-430-clean
# After both Wine architectures complete, finish the same private component set:
bash tools/runtime/complete.sh /mnt/Storage2/LinuxPluginBridge/runtime-430-clean
```

The checker copies only the named pristine source files to a temporary directory.
It applies all patches together and verifies the integrated postimages, leaving
the supplied source untouched. Preparation modifies only a pristine exact Git
build checkout. A changed prerequisite is an error, not permission to rebase it.

The build acquires pinned Proton recursively, replaces Wine **source** before
compilation and invokes upstream's complete Wine/dependency rules in a new object
tree. It never takes Wine DLLs or objects from an installed runner. The pinned
timestamp prevents the upstream default of using the current time. A pinned
[crane release](https://github.com/google/go-containerregistry/releases/tag/v0.22.1)
exports the exact SDK image directly under the output directory. Bubblewrap uses
that read-only SDK root; a transient user systemd scope caps the build at three
CPUs and 6 GiB, including a zero swap allowance, throughout acquisition and build.
The outer wrapper requires mounted Storage2 and an output under the project root
before any large write, and holds the shared build lock throughout. There is no global Docker image
cache write or daemon change. These host tools must already be available; the
recipe does not install system packages. Build logs and outputs stay outside Git. This
does not execute a vendor program, create a prefix, install into Steam or change
an existing managed setup. The script refuses an existing output directory.

Recursive gitlinks alone do not pin every input: the pinned Piper CMake recipe
downloads a moving phonemizer branch and other unhashed archives. `build-inputs`
pins its phonemizer, fmt, spdlog, espeak and Sonic source commits/archives, Microsoft's
ONNXRuntime 1.14.1 Linux auxiliary and Wine's expected Vulkan registry 1.4.335.
Nested CMake consumes those verified local inputs. Compilation has no network;
an additional hidden download fails visibly. This closes the inputs of the Wine
target, not every optional upstream `redist` target. ONNXRuntime is an upstream
prebuilt auxiliary, not a Wine DLL or a local source-build claim.

## Integrated source behavior

The original Steamuser, D3D11 interface, UIA guard and MSI reader patches are
reused from their recovered source directories. The UIA provider regression is
included. c27f058 already selects per-window XI touch; the adaptation resolves
the actual event window to its HWND, bounds virtual-desktop coordinates and
clears contact on release. The existing touch flags and popup fixtures remain
the regressions; physical touch acceptance still requires the real session.

c27f058 already retains the original authenticated-attribute encoding for
signature verification and has an unsorted-signature regression. Do not apply
the older MR11824 re-encoder over that implementation. The additional regression
corrupts only a signature byte and requires `NTE_BAD_SIGNATURE`, preserving the
requirement to reject invalid signatures. No production crypt32 code is changed.

The original account-free PE fixtures are built for both architectures by the
source-input workflow. `loader_fixture` takes the opposite-architecture fixture
path and checks native execution and same/opposite-architecture child creation
with bounded waits. `dcomp_fixture` checks the three device-creation exports;
it does not test composition or rendered pixels. `strref_fixture` takes a new
disposable directory: ordinary short and engine-written genuine long pools are
controls, while a separate short database advertises long references to expose
the historical reader defect. It refuses to overwrite fixture databases.

## Acquisition and delivery boundary

The virtual-desktop compositor's optional XDamage state uses the same feature
guard as its call sites. This preserves builds whose SDK architecture omits
XDamage without weakening compiler warnings or changing the enabled path.

Valve's [pinned source instructions](https://github.com/ValveSoftware/Proton/blob/5b89db940e0ebe3a137a6009a3589232fe084c09/README.md)
support recursive acquisition and rebuilding changed components. The SDK digest
is the pinned Proton tree's own default SDK tag. Its ICU tarballs are tracked
in Git, with [Unicode provenance and hashes](https://github.com/ValveSoftware/Proton/blob/5b89db940e0ebe3a137a6009a3589232fe084c09/icu/README.md).
The operator-selected composition retains complete Wine, graphics, media,
installer/service and accessibility components. It omits Steam-client/game and
overlay integration, the Steam executable intermediary and OpenVR/OpenXR relays.
Their build edges and required prefix copies are removed together. After normal
prefix initialization, `run` invokes the requested program through the pinned
native Wine dispatcher; `waitforexitandrun` preserves its wineserver wait first.
`runinprefix`, path conversion and the shared SLR command-session anchor retain
their behavior. Existing applied predecessor runtimes are not modified.
The source checks execute the actual launcher and prefix-preparation methods with
synthetic payloads: arguments/wait ordering, required-copy failures, architecture
placement and preserved identity/data are checked without starting Wine.

c27's [native loader dispatch](https://github.com/giang17/wine/blob/c27f058814b402a5709e073adccd42baa66810b9/dlls/ntdll/unix/loader.c#L479)
uses `wine` and `wine-preloader` in each architecture's own library directory.
The pinned Proton launcher already uses those paths. Its two additional
Valve-specific `wine64` compatibility copies are removed before the Wine target;
c27 lacks the matching cross-directory alias dispatch. Both native library trees,
the x64 public `bin/wine` dispatcher and server remain in the common composition.
No loader is renamed, taken from another build or moved into the other
architecture's directory. Normal dual-architecture and forced new-WoW64 launches
are distinct smoke tests; source layout alone does not qualify either execution.

`build.sh` builds Wine and its dependencies. `complete.sh` uses the same source
and object tree for the declared composition's `redist` target, including graphics,
media, fonts and an unlicensed generated default prefix. It
pins matching Mono 10.4.1, both Gecko 2.47.4 packages and Xalia 0.4.9, and retains
the corresponding Mono/Gecko release-source archives outside Git. Cargo first
fetches the unchanged upstream workspace with `--locked`, then compilation runs
`--frozen` in the network-isolated SDK. Cache-only workspace sources do not add
extra runtime components. The build timestamp/version is explicit.

The upstream generated-prefix cleanup is extended to remove device and broken
drive symlinks as well as directory links. This runs only on the disposable build
prefix. Its regression preserves unrelated files and links; no user-prefix
cleanup or lifecycle changes are included.

This remains a private comparison. The recipe preserves exact Xalia and bundled
dependency notices, Mono's inventory/component licence and patent notice,
Gecko's aggregate licence, and the ONNX auxiliary's original notices beside the
redist output. `redist-inputs.json` maps those texts to the exact inputs. The
complete Mono/Gecko source archives retain additional component licences and
exceptions, including SharpZipLib's classpath exception. Wine Mono is not one
uniform licence. MIT notices, Apache attribution (including Superpower, Datalust
and Sprache contributors), and zlib source notices must be preserved as applicable.
Each component's licences, modification notices and required corresponding
source must accompany distribution, alongside Valve's notices. A completed
`redist` directory alone does not discharge those obligations, publish a runtime
or qualify this combination. The broader source/distribution package is separate
from this private build.

The completed composition emits `wine-build.json`. Its release-builder facts bind
the prepared recipe/patch roster, pinned SDK, generated compiler configuration for
both native architectures, explicit invocation and separate original helper
source/ABI identities. The recorded `winebuild` is that build's own executable.
`tools/mf3/prebuilt_kit.py --runtime-build ... --winebuild ...` checks these exact
helper sources and marker bytes before executing any build tool, then binds the
declaration digest in its schema2 helper manifest. Shared command-session schema2
also binds that digest; runtime delivery seals both declarations. Launch verifies
the selected pair and refuses missing/mismatched facts rather than selecting a
substitute. Retained schema1 predecessors keep their existing interpretation.
The declaration has no qualification or readiness meaning.

After the source companion completes, package the final redist without changing
its bytes or the original declaration:

```sh
python3 tools/runtime/package.py --build-root "$OUTPUT" \
  --source-companion "$OUTPUT/source-companion" --source-sha256 "$SOURCE_SHA256" \
  --wine-build-sha256 "$BUILD_SHA256" --slr-archive "$SLR_ARCHIVE" --output "$ARCHIVE"
```

The source digest is the collector's existing SHA256 over canonical sorted JSON
mapping relative filenames to their size and SHA256, with a final newline. No
new manifest or installation record is written. The producer verifies that exact
collection, original recipe/helper/configuration bytes, unchanged runtime notices
and declaration. It writes a deterministic gzip tar with the generic
`Common-Proton-x86_64/` root, the entire unchanged `source-companion/`, and the
schema2 command-session declaration required by `runtime_delivery`'s WholeArchive
layout. That declaration binds the original Wine declaration and exact paired
SLR launch-client/service hashes. Existing outputs, changed inputs and unsafe
symlinks refuse. Run archive production with the same shared lock and resource
limits as collection. This command does not select, install or publish a runtime.

SLR remains the existing separate direct Valve acquisition. The producer reads
only its fixed catalog archive (4.0.20260805.254769, 164895128 bytes,
SHA256 `3226d8234e7c0542ee767837832bfb1dad5e5e2dc944ec97eb221b437f6b9349`);
it does not put SLR binaries in the common archive. Existing delivery preserves
Valve's supplied component licences and notices. The matching
[Valve release](https://repo.steampowered.com/steamrt4/images/4.0.20260805.254769/)
provides the runtime manifests, source-required index and
[corresponding sources](https://repo.steampowered.com/steamrt4/images/4.0.20260805.254769/sources/).
Those source-availability and notice obligations remain applicable to that
separately acquired component. The common archive preserves its own recorded
component notices, licence exceptions, modification notices and corresponding
source/build scripts, including the GPLv3-linked speech closure.

WebView2 is intentionally absent from this engine build. The historical .37
official archive URL and hash are now recovered and independently verified in
`../runtime-dcomp/recipe.json`. The operator selected a managed setup dependency:
its installed dependency set advances through explicit setup revisions, and the
configuration owner binds an exact version for each resolved launch. The common
Wine engine remains independent. Acquisition versus redistribution, user
terms and servicing still need a valid delivery basis before implementation.
The historical extracted Deck browser is never a build input.

`build-regression-fixtures.py --output NEW_DIRECTORY` builds the existing UIA
argument/valid-provider case and the signed/tampered crypt32 case as standalone
PE32 and PE64 tests. It verifies pinned source headers and the complete prepared
certificate test; `--prepared-wine PATH` reuses the common build source offline.
