# K8I1 source tools

This directory contains the source-owned parts of the exact Kontakt 8 Player
8.13.1 adapter:

- `msi_shim.c` and the generated definition intercept only
  `MsiInstallProductA/W`; every other export forwards to `msi_lvb_real.dll`.
- `registry.cpp` exposes only the per-application MSI override and the four
  fixed Kontakt product values.
- `table_export.cpp` opens the exact pristine MSI read-only and projects its
  complete table roster and rows through the Windows Installer API.
- `compile_plan.py` invokes that exporter itself and compiles the selected
  files, destinations, registry state, mutation-table dispositions, custom
  actions, and normalized MSI properties from the package relationships.
- `package.py` validates the compiler result and disposable proof before it can
  seal built artifacts.

No adapter archive is checked into this repository. Caller-supplied plans and
caller-supplied table projections are rejected. Production sealing and renderer
admission are source-disabled until the exact reviewed compiler-result,
disposable-proof, and final-archive digests are pinned. The current source
candidate is therefore not installable and does not authorize a Native Access
Install click.

The real MSI library is built from Wine
`dc26e61847081a1b5cb0733dc30feba6ee575482` with the two ordered upstream
commits named in `THIRD_PARTY.md`. The resulting PE must be 32-bit and must
retain its corresponding source, exact applied patches, build record, and LGPL
notices. A build is not accepted by filename: `SOURCE.json`, the binary digest,
the exact export roster, and the private disposable-prefix proof all remain
required. No patched Wine binary or corresponding-source bundle has been
produced by this source-only change.

The next non-source gate is one disposable-prefix proof using the exact retained
Kontakt package. It must demonstrate a single interception, complete payload and
product-state verification, cold Native Access recognition, standalone launch,
VST3 enumeration, and a forced-failure rollback. The managed NI prefix remains
out of scope until that proof is reviewed.
