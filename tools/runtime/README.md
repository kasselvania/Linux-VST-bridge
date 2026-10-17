# Common source integration comparison

This is a clean source comparison for #430, not the selected production base.
`source.json` pins the historical Proton tree, its matching SDK, DComp Wine
source, patch inputs and integrated source postimages. It defines build inputs
only; it is not an installed runtime catalogue, configuration or support record.

```sh
python3 tools/runtime/prepare.py --check-upstream
python3 tools/runtime/prepare.py --check-source /private/pristine-wine-c27f058
# Linux builder, under the shared heavy-build lock and resource limits:
bash tools/runtime/build.sh /mnt/Storage2/LinuxPluginBridge/runtime-430-clean
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
CPUs and 6 GiB, including a zero swap allowance. There is no global Docker image
cache write or daemon change. These host tools must already be available; the
recipe does not install system packages. Build logs and outputs stay outside Git. This
does not execute a vendor program, create a prefix, install into Steam or change
an existing managed setup. Callers must hold the shared heavy-build lock on the
homelab; the script acquires that lock before compilation. The script refuses an
existing output directory.

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

## Acquisition and delivery boundary

Valve's [pinned source instructions](https://github.com/ValveSoftware/Proton/blob/5b89db940e0ebe3a137a6009a3589232fe084c09/README.md)
support recursive acquisition and rebuilding changed components. The SDK digest
is the pinned Proton tree's own default SDK tag. Its ICU tarballs are tracked
in Git, with [Unicode provenance and hashes](https://github.com/ValveSoftware/Proton/blob/5b89db940e0ebe3a137a6009a3589232fe084c09/icu/README.md).
OpenVR auxiliaries come from the pinned official submodule; vrclient is rebuilt.
These official upstream auxiliaries are distinct from transplanted Wine DLLs.

This command builds the Wine engine and its dependencies, not a distributable
runtime. Upstream `redist` separately acquires Gecko, Mono and Xalia. Its Mono
11.2.0 differs from this Wine source's expected 10.4.1; composition must pin
the matching dependency and all archive hashes before delivery. Preserve each
component's licences, modification notices and required corresponding source,
alongside Valve's distribution notices. A successful build is not discharge of
those obligations or qualification of this combination.

WebView2 is intentionally absent from this engine build. The historical .37
official archive provenance remains a gap. A newer official dependency can be
tested in isolation; its placement in runtime versus setup, acquisition versus
redistribution, user terms and servicing need explicit decisions before delivery.
The historical extracted Deck browser is never a build input.
