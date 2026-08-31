# HP0 findings

## Claim result

`passed` — repository-owned C++20 source repeatably built the same deterministic
Linux VST3 probe with the exact user-space Freedesktop 25.08 SDK and pinned
official VST3 SDK; the official validator passed both clean builds; an exact
owned copy was published beneath `<HOME>/.vst3`; and the same official validator
loaded and passed that exact copy from inside the current Bitwig Flatpak
application sandbox without launching Bitwig or changing its overrides.

## Consequential observations

- Both clean build modules were byte-identical at SHA-256
  `3abaa8a2051ea179e28986cc6fbad4cde2d0d5e9a844b18aca91cdf7c59e69d7`.
- Both v2 build receipts bound the canonical 19-path build-source manifest at
  SHA-256
  `ee301e3e42b2381d5a9aa88ec64635482d098bc8cde50d33a57e6c397fb1e9ab`.
  That manifest and both receipts remained valid at the final evidence-only
  amended head; covered source/configuration changes were refused.
- Each build-SDK validator run and the app-sandbox run reported 47 passed and 0
  failed tests.
- The app-sandbox validator was bound to the verified build receipt and rehashed
  inside the sandbox; a modified validator was refused before execution.
- Publication success now includes atomic v2 receipt commit and final bundle and
  receipt readback. Deterministic post-swap failures restored either the exact
  complete prior publication/receipt or prior absence without debris.
- Ordinary publication owns exactly
  `<HOME>/.cache/linux-vst-bridge/hp0-publication.receipt`; it refused an
  unrelated existing file without changing its bytes or metadata and refused
  alternate ordinary receipt paths. Alternate test roots and descendants are
  canonical, symlink-ancestor-free cache descendants; tested escape targets
  remained untouched.
- The owned publication remains at
  `<HOME>/.vst3/linux-vst-bridge/LabHostProbe.vst3` for HP1.
- The exact pinned system app/runtime fixture can read and dynamically load that
  exact path despite empty effective `VST3_PATH` and `CLAP_PATH` values; the
  effective `VST_PATH` remained `/app/extensions/Plugins/vst;=/usr/lib/`.
- The exact expected user and system override outputs and the exact app/runtime
  deployment identities were byte-identical before and after.
- The two exact regular Serum artifacts accepted from SR0 retained their hashes,
  sizes, and modification timestamps:
  `317d70f95a3c7559ff3d43b014c3e7b792fd03362d5e999d550a5566cf25b184`
  and `838bc7ab42d5d039156768680ffc3e0175d6e6bed9f99802989a01d695e13175`.

## Unknowns and claim ceiling

Bitwig discovery of `~/.vst3`, scan behavior, generic-host parameter view,
instantiation, audio, state in a Bitwig project, and musical operation remain
`unknown` because Bitwig was not launched. Serum authorization and operability
remain `unknown`. Windows-host, proxy, IPC, shared-memory, bridge, manager,
broker, Wine/Proton/UMU, CLAP, custom-editor, performance, crash-recovery, and
general-Linux claims are `explicitly_out_of_scope`.
