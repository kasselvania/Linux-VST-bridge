# WF0 Windows build-plane lock

WF0 V7 has one supported producer: the private repository workflow at
`.github/workflows/wf0-windows-msvc-build.yml`, triggered by an exact-path
push to `codex/wf0-windows-vst3-factory-census-v7`.

The selected service label is `windows-2022`; it is a mutable selector, not a
durable identity. Each accepted receipt therefore records the observed image
OS/version, Windows product/version/build, runner architecture, Visual Studio
installation/version, VSCMD version, complete `cl /Bv` digest, linker
version, CMake version, and selected SDK. The required compiler shape is Visual
Studio 2022 Enterprise, generator `Visual Studio 17 2022`, toolset `v143`,
platform `x64`, Windows SDK `10.0.19041.0`, Release configuration, and the
static `MultiThreaded` MSVC runtime.

Dependency acquisition is limited to the official Steinberg VST3 SDK root and
its seven locked recursive submodule remotes. Every root and recursive
submodule materialization runs with command-scoped `core.autocrlf=false` and
`core.eol=lf`; those values are also written and read back as repository-local
configuration before later Git use. Each locked source requires agreement
among its commit object, its unfiltered worktree Git object, and its canonical
raw SHA-256. After the exact root, tree, gitlinks, submodule commits, clean
states, and reconciliation source blobs are verified, configure and build
perform no dependency-network operation. The SDK and VSTGUI are not patched,
copied into repository source, or replaced by a shadow target.

Build roots A and B are distinct and initially absent. Each build maps its
distinct physical root to the same compiler-visible `C:\wf0\build` prefix;
the fixed source-date environment, `/Brepro`, and path-independent embedded
PDB name complete the deterministic input contract. This is build-time prefix
mapping, not post-build PE normalization. Only
`wf0-factory-probe`, `wf0-loader-adapter-tests`, the approved fault family,
and the exact upstream `again` target are built. Produced PE files are parsed
and inspected but never executed on Windows. Complete transferred file rosters
must be byte-identical.

The job uploads once. Its envelope contains only `wf0-payload.zip`,
`WF0_WINDOWS_BUILD_RECEIPT.json`, and
`WF0_WINDOWS_BUILD_RECEIPT.sha256`. The workflow requests only
`contents: read`; it uses no secret, attestation, signing, package, release,
pull-request, issue, check, deployment, or write permission.

Actions run and artifact hashes establish bounded custody joins only. They do
not establish cryptographic provenance, a trusted builder, SLSA, code signing,
release suitability, Windows runtime behavior, Proton behavior, class
instantiation, audio, GUI, Bitwig, Serum, or general plug-in compatibility.
