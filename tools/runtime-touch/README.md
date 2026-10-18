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
