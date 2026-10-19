# GE X11 touch correction

This component carries the retained released-contact and per-window XI2 touch
corrections into the maintained GE-Proton11-7 line. Serum's Deck waveform popup
is the regression case, not a product-specific branch. Incorrect graphic
transparency (#330) is separate.

`build.py` reconstructs GE's complete Wine patch sequence from pinned Wine, GE
and wine-staging archives. It restores the two generated files that GE's recipe
restores, prepares wineopenxr, and applies the same Wine patch section before
the two local touch patches. Generation and compilation run in the pinned
Steam Runtime 4 SDK, with two CPUs, 4 GiB memory and no network. A test compiles
against the actual production touch-flag header and verifies released contacts.
The output is the x64 Unix X11 driver; this does not qualify 32-bit touch paths.

Run on the builder under the project heavy-build lock:

```sh
python3 build.py --output /absolute/new/build --inputs /absolute/input-cache
python3 package.py --build /absolute/new/build \
  --ge-archive /absolute/GE-Proton11-7-x86_64.tar.gz --output /absolute/new/package
```

The optional input cache uses the names and exact digests in `INPUTS`. Existing
output directories are refused. No script touches an installed runner or prefix.
The manager composes the correction only into fresh private staging after
verifying the upstream archive and exact X11 driver preimage. Retain the UIA and
MSI components in the same runtime generation; never transplant a driver from
the older Proton touch candidate.

## Source and distribution

The driver is Wine code under LGPL-2.1-or-later. Original notices and COPYING.LIB
remain in the full patched source; locally modified files carry dated notices.
The component archive includes that source, its build script, pinned SDK,
upstream GE/staging patch inputs, exact local patches, construction record and
licence/notices. Corresponding source accompanies the binary in the same
archive. Unmodified GE and SLR remain acquired upstream with their own notices.
Wine code is not linked into the proprietary manager or native proxy. No vendor
binary, content or licensing material belongs in this component or repository.

A successful build and flag test do not establish useful touch behavior. Test the
finger-operated popup against the retained working configuration, then verify
playback, changed state, fresh-process recall and retirement. Other plug-ins
need their own physical checks before extending support claims.

## Replacement driver compatibility

Before packaging, `compatibility.py` compares the exact GE preimage with the
built driver, without loading either. Packaging refuses a mismatch in ELF
class/endianness/architecture/ABI/flags, SONAME, DT_NEEDED, loader search paths or
flags, exported names/types/bindings/visibility/data sizes, versioned undefined
symbols (including weak imports), embedded optional-library names, or compiler
identity. Real ELF negative tests exercise changed imports, exports, data size,
dependencies, SONAME, architecture and optional-library names.

For the shipped r7 driver (`c163fc9d…`), the exact GE preimage (`5acbf619…`) and
replacement both have two exported 8-byte Unix-call tables and 380 imports, six
DT_NEEDED entries, SONAME `winex11.so`, the same optional graphics-library names,
and GCC 14.2.0-19. The check passes without changing the distributed driver or
component archive. Run the same check on retained build outputs:

```sh
python3 compatibility.py --original /path/to/extracted/ge/winex11.so \
  --build /path/to/build > /path/to/retained/compatibility.json
```

The build configurations are deliberately not described as identical. GE uses
`--enable-werror --with-wayland --with-ffmpeg --disable-tests`, component dependency
paths, and architecture/optimization flags including `-march=nocona`,
`-mtune=core-avx2`, `-fwrapv`, section splitting and frame pointers. The bounded
build uses `gcc -m64`, `-g -O2`, Wine's generated PIC/hidden-visibility/unwind flags
and `-fno-strict-aliasing`. Both use the same pinned SDK and fully GE-patched Wine
source. The X11 boundary headers retain GDI/OpenGL/Vulkan driver versions
111/38/49; the touch patches do not change these headers or the Unix-call tables.
Architecture tuning, warning policy, tests and debug/section flags do not define
this exported interface. Equal interfaces do not prove all runtime semantics;
the real editor/audio/state checks remain required.

`--without-wayland` disables building the separate `winewayland.drv` module.
The overlaid `winex11.so` links X11/GLX, not Wayland/EGL: its Makefile has no
Wayland or FFmpeg dependency, and its source has no conditional on those feature
macros. The checker records every HAVE_/SONAME_ feature used by the X11 source
and refuses a missing feature. XInput2, Shape, shared memory, XFixes, Xinerama,
XRandR, XRender, GL and Vulkan remain enabled; optional library names also match
the original binary. Only winex11.so is overlaid, so GE's existing Wayland and
media modules remain intact. The configuration report records build variables,
X11 feature definitions and interface-header versions for review.
