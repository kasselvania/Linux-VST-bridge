# Recovered DComp and BG1 source inputs

This directory restores the missing source inputs behind the Deck's established
Blackhole/Nibbi DComp runtime and its BEAM successor. It selects no common runtime
base, changes no product profile or owner, and installs nothing. A fresh build
has **not** been performed. The recorded inputs are a build basis, not proof of
a reproducible or distributable runtime.

## Source and build basis

`recipe.json` pins giang17 Wine `c27f058814b402a5709e073adccd42baa66810b9`,
its tree, the historical Steam Runtime SDK digest and both exact patches.
`proton-steamuser-policy.patch` is the retained build input, checked against the
Deck's patched source. It carries Valve's existing `steamuser` username policy
into that Wine lineage; it is not a new change to any installed setup.
`wine-video-interface-coherence.patch` is recovered byte-for-byte from Git
`8bf70de6b442bcac4086de4e27dadbdc7e76ea43:tools/bg1/`. Its hash matches the
retained BG1 D3D11 build receipt, and its resulting source matches the retained
Wine checkout. It stops advertising VideoDevice1 when the corresponding
VideoContext1 contract is absent.

The historical full Wine build configured `--enable-archs=i386,x86_64`, then ran
`make` and `make install` into a private stage under the pinned SDK. The DComp
reference applies only the Steamuser patch; the BG1 successor applies both.
The original BG1 incremental build reused the reference's existing object tree
and rebuilt both D3D11 DLLs. Those old objects are not a fresh-build input or a
reproducibility claim. A future clean build must configure its own object/stage
directories, observe the declared CPU/RAM limits, and compare its outputs.

Check the recovered files without downloading or executing a runtime:

```sh
python3 tools/runtime-dcomp/check.py
python3 tools/runtime-dcomp/check.py --wine-source /private/pristine-wine-c27f058
```

The second command reads only the two named pristine Wine files, checks their
exact preimages, applies the patches in a temporary copy and checks the exact
postimages. It never patches the supplied checkout. Neither command compiles
Wine, launches a vendor program or accesses an installed prefix.

## Historical composition steps

The recovered basis records these operations, not a product delivery command:

1. Start with the byte-verified historical Proton 11.0-2c distribution. Replace
   `files/bin`, `files/lib/wine` and `files/share/wine` with the staged full Wine
   build. Retain the distribution's ICU/OpenVR auxiliaries and Mono/Gecko.
   Wine Mono 10.4.1 was separately supplied to match the new Wine build.
2. Generate the default prefix using that Proton source's `default_pfx.py`,
   confined to the candidate tree, removing host-device links. The process
   policy used `PROTON_DISABLE_NVAPI=1`, `PROTON_USE_WINED3D=1` and
   `WINEDLLOVERRIDES=d2d1,d3d11,dxgi,dcomp=b`. These historical settings are not
   promoted to a new product default here.
3. BG1 V1 changed the DComp reference's version only. V2 replaced both D3D11
   DLLs with the coherence build. V3 added fixed WebView2 154.0.4258.37, retained
   the original launcher as `proton-original`, and generated
   `runtime-components.json` from the acquired file roster. It substituted that
   manifest's digest into `reference/fixed_webview_entry.py` and used the result
   as the launcher. V4 added the pinned `native-command-session.json` and
   changed the version; it did not introduce another Wine engine build.

The launcher template is retained original project source, not an installed
entrypoint or a proposed permanent product abstraction. The checker reproduces
the exact retained launcher and command-session marker bytes. The historical
builders in Git `8bf70de6...:tools/bg1/{build_video_coherence.py,
bundle_content_runtime.py,bundle_command_session.py}` coupled construction to
one machine and its selected environment. They are not imported as product
operations. Existing [BG1 custody](../../docs/BG1_RUNTIME_OWNER.md) remains
separate from this source recovery.

## Remaining acquisition and distribution gaps

The Wine source commit and SDK are pinned. The complete historical Proton
binary distribution was inherited from an installed build, rather than acquired
from an independently pinned archive here. Its launcher hash alone does not
identify every inherited auxiliary. Recover that acquisition/source/notices
basis and the exact composition inputs before claiming a fresh reconstruction.
Do not substitute customer-installed Steam as a product requirement.

The exact official WebView2 154.0.4258.37 CAB URL and archive hash were recovered
from Git `82bdf081:evidence/bg1/content-startup-attribution.sanitized.json`.
An independent stream from the official Microsoft CDN on 2026-10-17 read
307,889,499 bytes and matched SHA-256
`143da7f7c4939fddd3875ed918e44022d7eb87063bf912fe3e32df37c6b0b8c3`.
`recipe.json` now pins that acquisition input alongside the retained extracted
roster. This closes the archive-provenance gap; it does not add payload delivery
or an independent extraction comparison.

The operator selected WebView2 as a managed setup dependency, independently
serviced through setup revisions, rather than part of the common Wine engine.
That decision does not accept Microsoft publisher terms. Before delivery, record
the applicable [Fixed Version terms](https://developer.microsoft.com/microsoft-edge/api/eula/webview2?locale=en-us&fixed=true),
protective application/distributor terms, notices, platform/distribution scope
and [servicing responsibility](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/distribution).
Preserve exact predecessor bindings and browser user data; retained browser
binaries alone do not demonstrate usable rollback. Do not copy the Deck browser.

Wine/Proton and inherited components also require their corresponding source,
licences, modification notices and distribution obligations before delivery.
This source-only recovery does not discharge those obligations.

Existing [MSI](../runtime-msi-reader/), [UIA](../runtime-uia-guard/),
[touch release](../x11-touch-release/), [touch routing](../touch-routing/) and
[crypt32](../wd0/seal_fl_crypt32_runner.py) inputs remain the source of those
corrections. Nothing here combines their prebuilt DLLs across Wine lineages.
