# HP0 clean-build comparison

Both builds started from absent clean build directories and the same clean
implementation commit and dependency/toolchain lock.

| Fact | Build 1 | Build 2 |
|---|---|---|
| Classification | `passed` | `passed` |
| Historical source commit | `e2d87997dc200cd49ec0e6fc2a0e7646b7cfed62` | same |
| Historical source tree | `3c60d7f48b78677f65c9f2879aa49a2c0d913db6` | same |
| Build location | `<REPO>/build/hp0-second-repair-1` | `<REPO>/build/hp0-second-repair-2` |
| Build receipt schema | `linux-vst-bridge-hp0-build/v2` | same |
| Build-source manifest schema | `linux-vst-bridge-hp0-build-source/v1` | same |
| Build-source manifest SHA-256 | `ee301e3e42b2381d5a9aa88ec64635482d098bc8cde50d33a57e6c397fb1e9ab` | same |
| Module SHA-256 | `3abaa8a2051ea179e28986cc6fbad4cde2d0d5e9a844b18aca91cdf7c59e69d7` | same |
| Bundle manifest SHA-256 | `ec8a4531a3b73da3c3e29c6e1f3c90c989fb61e3f3397af32d4c1576b3624ea9` | same |
| Module metadata SHA-256 | `b1d12fcda57f4b3249af447ac1f954164a118b7da4a1d05f08db0b132cffd911` | same |
| Validator SHA-256 | `cccae776eb87fbbbf6ac9c34c78bf1548937c48db3843e0cf0f12d312650466f` | same |

`cmp` confirmed both module binaries, validator binaries, module metadata files,
bundle manifests, and build-source manifests were byte-identical in this
two-build observation. The path-normalized validator logs were not
byte-identical: build 1 repeated the informational `Not all points have been
read via IParameterChanges` notice once, while build 2 emitted it once. Both
runs still registered the same 47 succeeded / 0 failed tests, factory/classes,
buses, parameters, state/sample-size results, and exit `0`. This is a bounded
semantic comparison, not a universal reproducible-build claim.

Each v2 build receipt was independently checked against a freshly generated
canonical path/mode/blob source manifest; exact Freedesktop and VST3 SDK
identities; compiler and build-tool versions; fixed class and parameter IDs;
bounded bundle/module paths; complete bundle manifest; module SHA-256; and
validator path/SHA-256. Both receipt checks passed before publication and the
app-sandbox run.

After the evidence-only amend, both receipts were checked again at the final
clean PR head. The current repository commit/tree had changed, the canonical
build-source manifest remained byte-identical at the digest above, and both
receipt verifications passed without rebuilding. A covered source/configuration
change and a stale receipt with edited historical commit/tree fields were
independently rejected by the deterministic suite.

Generated bundle roster:

```text
d Contents
d Contents/Resources
d Contents/x86_64-linux
f Contents/Resources/moduleinfo.json
f Contents/x86_64-linux/LabHostProbe.so
```

The module is 322224 bytes. `file` classified it as a stripped, dynamically
linked, 64-bit little-endian x86-64 ELF shared object with build ID
`be4ce73a2b209081cf3b83831d86e2b22eef3cf1`. `readelf` reported `DYN` and
machine `Advanced Micro Devices X86-64`.

Direct dependencies were `libstdc++.so.6`, `libm.so.6`, `libgcc_s.so.1`, and
`libc.so.6`, with the ordinary x86_64 loader. `ldd` resolved every dependency;
there was no `not found` entry.
