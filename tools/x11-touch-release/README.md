# Exact Serum X11 touch-release runner

This directory contains source only. Do not commit the patched Wine source,
compiled runner, SDK image, private build logs, Proton distribution or Serum
files. The installed Proton runner remains untouched.

The patch applies only to Wine commit
`dc26e61847081a1b5cb0733dc30feba6ee575482`, tree
`da4b1eb3b7f209eb4a971d4b5929fcda08ff6b4e`. The Proton distribution
source is `5b89db940e0ebe3a137a6009a3589232fe084c09`. The offline Steam
Runtime 4 SDK image is pinned by digest
`sha256:97526b794ce1a9bed5f891084462260b3a02399569f7438a3a57b5a253001db9`.

Apply `wine-x11-touch-release.patch` to a separate checkout of that exact Wine
commit. Compile and run `test_touch_flags.c` against the resulting header.
Configure Wine in the pinned offline SDK with a private `--prefix` and
`--enable-archs=i386,x86_64`, then run the complete build and install to that
private prefix. Retain the configure, build and install logs and actual exit
statuses. Do not execute any commercial plug-in in this proof.

After successful build/install, `seal_candidate.py compose` copies the exact
existing Proton distribution to a new private candidate tree and replaces
only its version file and the x86-64 Unix `winex11.so` image. This WoW64
reference build has no separate i386 Unix driver output; copied 32-bit files
remain unchanged. It verifies all 31
predecessor runner artifacts and records the complete tree. The separate
`seal` phase exercises the candidate through the pinned Steam Runtime entry
point with an isolated, unlicensed `cmd.exe` prefix; it verifies scoped
shutdown and unchanged tree bytes, then records the private build receipt and
manifest. Both phases refuse reuse of an existing output.

The manager accepts only manifest kind
`x11_touch_release_reference_runner` with the exact patch, source, SDK,
base-runner, tree and changed-artifact identities. `experimental-runner`
requires a stopped bridge, no active owner, the exact removed Serum revision,
the exact candidate-B predecessor, and a single compare-and-swap environment
advance. It creates candidate C with a provenance-recorded carry-forward of
B's existing factory and selected-class result. The old inventory remains
stale; no fresh scan is claimed. The read-only
`x11_touch_candidate_preview` example can calculate the prospective C
identity after sealing, before any physical transition.

Physical Serum use and candidate-C publication are a separate post-review
step. Candidate B and the installed runner remain the rollback authority.
