# Current Slice: HP0 — Native VST3 Probe Build and Bitwig Flatpak Sandbox Load

## Status and basis

```text
status: active
basis commit: 5614da2d769931f0aa26a8f902d14e911b39b085
basis tree: 6d3cea1638e12e65298d9496755d8cc4d0f2eeba
branch: codex/hp0-native-vst3-bitwig-sandbox-probe
target: main
```

## Primary claim

From repository-owned C++20 source, the project can repeatably build one deterministic no-editor Linux VST3 probe against the exact pinned official VST3 SDK using a user-space Freedesktop 25.08 SDK; validate it with the official VST3 validator; publish an exact owned copy beneath the user's standard VST3 area; and load and validate that exact published bundle from inside the current Bitwig Flatpak sandbox without launching Bitwig or changing Bitwig's Flatpak overrides.

## Exact fixture

- Steam Deck model `Galileo`, SteamOS `3.8.16`, x86_64, KDE/Wayland, read-only mode enabled.
- Bitwig Flatpak `com.bitwig.BitwigStudio` version `6.0.11`, system scope, stable x86_64 branch, app commit `7b66aed37386ffff99e40bbd486f90ceee7ac1d5c59508c7952094122cebf50e`.
- Bitwig runtime `org.freedesktop.Platform/x86_64/25.08`, system commit `bd44a6230581917d04f89812a4c21090c304d390edb73995af1c2f9fd8abf4e8`.
- Existing override state: user output SHA-256 `1b4a6a6ed688f69dd3c36dcac8db008c5a41ed52170ea3e23dee984b0aae6a1e`, empty system output SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`; effective `VST3_PATH` and `CLAP_PATH` empty; effective `VST_PATH=/app/extensions/Plugins/vst;=/usr/lib/`.
- Build toolchain: user-scope `org.freedesktop.Sdk/x86_64/25.08`; exact installed commit is retained in HP0 evidence.
- Official VST3 SDK: `steinbergmedia/vst3sdk` commit `3cdf9ca5d1f5b1b21e0a86832aa4abe55607bd96`, with recursive submodules locked in `docs/HP0_DEPENDENCY_LOCK.md`.

## Allowed tracked paths

- `CURRENT_SLICE.md`
- `CMakeLists.txt`
- `cmake/**`
- `native-probe/**`
- `tools/hp0-native-probe/**`
- `docs/HP0_DEPENDENCY_LOCK.md`
- `evidence/hp0-native-vst3-bitwig-sandbox/**`

No other tracked path is authorized.

## Intended route

```text
repository-owned C++20 source
  -> user-scope Freedesktop SDK 25.08
  -> exact pinned official VST3 SDK
  -> LabHostProbe.vst3 Linux x86_64 bundle
  -> official validator inside the build SDK
  -> exact owned copy at <HOME>/.vst3/linux-vst-bridge/LabHostProbe.vst3
  -> explicit-path official validation inside the Bitwig Flatpak sandbox
```

## In scope

- One original deterministic stereo gain effect named `LAB Host Probe` with separate processor/controller classes, fixed class and parameter IDs, versioned state, bypass, 32-bit and 64-bit processing, and no custom editor.
- Exact dependency and user-space toolchain verification.
- Deterministic build-affecting tracked-source manifest binding build receipts
  across evidence-only amended heads while rejecting every covered source or
  declared-directory path-set change.
- Two clean out-of-tree builds and stable semantic/metadata comparison.
- Official validator execution in both build environments and against the published copy inside the existing Bitwig sandbox.
- Exact owned publication with staged verification, atomic replacement, safe inspection, and documented exact removal.
- One ordinary project receipt path with strict prior-receipt ownership, plus
  canonical no-symlink containment for every alternate test root and path.
- Read-only sandbox visibility, readability, ELF architecture, hash, dependency-closure, effective environment, override-preservation, and process-state observations.
- Sanitized retained evidence for this exact fixture and claim.

## Explicit non-goals and nonclaims

HP0 does not launch Bitwig, request a scan, change a plug-in path or Flatpak override, or claim automatic `~/.vst3` discovery. It does not load, inspect, authorize, or change Serum, Wine, Proton, UMU, yabridge, `.wine`, or Steam compatdata. It does not implement a Windows host, bridge IPC, shared-memory transport, manager, broker, Rust component, CLAP plug-in, editor, compatibility profile, or real-time bridge claim. It does not generalize this exact fixture to Linux support.

## Evidence requirements

Retain the exact repository basis, host/Bitwig fixture, Freedesktop SDK identity and before/after state, official SDK root/submodule lock and license, source/class/parameter IDs, two-build receipts and hashes, bounded official-validator summaries, bundle structure and dependencies, publication receipt, sandbox load receipt, negative-test ledger, override byte identity, no-Bitwig-process checks, sanitization review, and explicit claim ceiling.

Build evidence also retains the canonical build-source path/mode/blob manifest
schema and digest, historical exercised commit/tree, final-head manifest
equality, and successful final-head receipt verification. Publication evidence
retains the exact ordinary receipt path, prior-receipt ownership validation,
unrelated-receipt refusal, and canonical test-root/symlink-escape results.

Generated binaries, SDK checkouts, build directories, validator executables, published bundles, private paths, hostname, proprietary material, and license/account state remain untracked.

## Negative acceptance

Acceptance must fail closed for a wrong or dirty SDK, absent exact user SDK,
changed/missing/unexpected build-source identity, a stale receipt with edited
historical provenance, unknown publication or receipt destination, alternate
ordinary receipt, escaped/symlinked test path, any failed post-swap publication
or receipt operation, absent bundle, modified published module or validator,
wrong architecture or unresolved dependency, Bitwig app/runtime/scope drift, a
shadowing user app installation, running Bitwig process, validator failure,
changed or unexpected Flatpak override bytes, an unexpected effective
VST/VST3/CLAP path, or unsanitized/unhashable evidence.

## Cleanup and rollback

- The Freedesktop SDK may remain as the declared user-space development fixture.
- Third-party source and build products remain beneath the user cache or ignored `build/` tree and may be removed without touching tracked source.
- Publication removal is limited to the exact manifest-verified HP0-owned bundle.
- Failed publication or receipt commit removes only the transaction-owned replacement, restores the exact prior complete bundle and receipt bytes (or prior absence), and leaves no stage or backup sibling after a successful rollback.
- HP0 leaves the verified owned bundle published for HP1.

## Review standard

Review the exact PR head against this one claim, the exact SDK/toolchain locks, original source and fixed identities, validator results, publication ownership/rollback law, sandbox evidence, unchanged overrides/process state, negative tests, sanitization, and explicit nonclaims. A successful explicit-path validator run is not Bitwig discovery or musical-operation evidence.
