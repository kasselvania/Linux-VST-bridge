# Current Slice: HP1 — Bitwig Native Discovery and Instance Admission

## Status and exact basis

```text
status: active
slice: HP1
basis commit: 7cda2d85eb426c2ed6e4eb3d86e52c114c4aa1c4
basis tree: e9534822625ceff0f06549b78c85546050bddf42
branch: codex/hp1-bitwig-native-discovery-admission
```

## Exact accepted HP0 input

- Owned bundle: `<HOME>/.vst3/linux-vst-bridge/LabHostProbe.vst3`.
- Linux module: `Contents/x86_64-linux/LabHostProbe.so`, SHA-256 `3abaa8a2051ea179e28986cc6fbad4cde2d0d5e9a844b18aca91cdf7c59e69d7`.
- Ordinary publication receipt: `<HOME>/.cache/linux-vst-bridge/hp0-publication.receipt`, schema `linux-vst-bridge-hp0-publication/v2`, accepted SHA-256 `d328adb27fef326b8e5b1104f072c246a803389a77d35ee5babc9834067668d1`.
- Display/vendor: `LAB Host Probe` / `Kasselvania Research`.
- Processor/controller CIDs: `6F4E7A5392E54B54A98AD6F714E0C201` / `B9C42F0736C34E218E5A71D40C8F1B62`.
- HP0 build-source manifest: schema `linux-vst-bridge-hp0-build-source/v1`, SHA-256 `ee301e3e42b2381d5a9aa88ec64635482d098bc8cde50d33a57e6c397fb1e9ab`.
- Official validator identity retained from HP0: SHA-256 `cccae776eb87fbbbf6ac9c34c78bf1548937c48db3843e0cf0f12d312650466f`.

HP1 consumes this exact publication. Rebuild, replacement, relocation, and publication repair are not authorized.

## Exact host fixture

- Maintainer Steam Deck model `Galileo`, SteamOS `3.8.16`, x86_64, active KDE Wayland user session.
- System `com.bitwig.BitwigStudio`, stable/x86_64, version `6.0.11`, app commit `7b66aed37386ffff99e40bbd486f90ceee7ac1d5c59508c7952094122cebf50e`.
- System runtime `org.freedesktop.Platform/x86_64/25.08`, commit `bd44a6230581917d04f89812a4c21090c304d390edb73995af1c2f9fd8abf4e8`; no user-scope Bitwig shadow.
- User/system Flatpak override SHA-256: `1b4a6a6ed688f69dd3c36dcac8db008c5a41ed52170ea3e23dee984b0aae6a1e` / `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`.
- Effective `VST_PATH=/app/extensions/Plugins/vst;=/usr/lib/`; effective `VST3_PATH` and `CLAP_PATH` are present and empty.

## Primary claim

With the accepted HP0 publication and the exact current Bitwig Flatpak app/runtime/override fixture unchanged, a normal operator-controlled Bitwig launch discovers and registers the exact `LAB Host Probe` VST3 class; allows one instance to be inserted in an unsaved temporary Bitwig project; maps the exact accepted `LabHostProbe.so` into a process belonging to the exact Bitwig process tree; shuts down cleanly; and repeats discovery and instance admission after a second normal Bitwig launch without another scan or path change.

HP1 does not require audio or project-state proof.

## Allowed changed paths

Only these tracked paths may change:

```text
CURRENT_SLICE.md
tools/hp1-bitwig-admission/**
evidence/hp1-bitwig-admission/**
```

## Operator-assisted session law

Repository tooling performs every safe noninteractive preflight, exact-identity readback, bounded Bitwig-state baseline, process/module monitor, collection, and sanitization step. It then stops for a nonce-bound operator checkpoint. Only the operator launches Bitwig from its ordinary desktop/application entry, performs the bounded browser/scan/insert/remove/quit actions, and reports the exact nonce-bearing result. No success claim may be created from synthetic evidence, an unrelated process mapping, an ambiguous confirmation, or a reused nonce.

## Required discovery and instance evidence

- Retain the exact Bitwig root process start identity for each launch as volatile raw state and a PID-redacted committed label.
- Prove the exact mapping process is in the active Bitwig ancestry and mapped the accepted module path/hash; retain only the matching mapping metadata.
- Require a distinct Bitwig root start identity for Session 2.
- Prove Bitwig, its observed descendants, the exact module mapping, the official validator, and every forbidden compatibility-process class are absent after each required shutdown.
- Retain bounded, completion-aware Bitwig-owned log/cache/index readback for the exact class names, CIDs, vendor, module basename, and applicable scan errors.
- Treat every timeout, access failure, cap, parse failure, stale PID, or incomplete contributor as `unknown`, `search_incomplete`, `blocked`, or an explicit refusal—never as a false non-finding or success.

## Blocked-result law

If the exact probe remains absent after one normal launch/startup scan and at most one ordinary built-in rescan that changes neither plug-in locations nor Flatpak state, stop the success path and retain `HP1_HOST_PATH_BLOCKED`. The packet must state sandbox visibility, effective paths, applicable scan rejection/non-finding evidence, exact mapping/non-mapping result, forbidden-process result, and unchanged fixture/publication identities. Do not change a path, environment, override, location, bundle, or extension to force discovery.

## Expected Bitwig-owned mutation

The only expected application-state mutation is ordinary Bitwig plug-in scan/cache/index/log state produced by normal launch and, if needed, the one allowed built-in rescan. No project is saved.

## Explicit non-goals and nonclaims

HP1 does not authorize or claim audio routing or DSP correctness; automation, state, or project recall; a custom editor; Bitwig path/location or Flatpak override changes; publication or receipt changes; a probe rebuild; official-validator execution during graphical sessions; Serum launch or operation; Wine, Proton, UMU, or yabridge launch; `.wine` or compatdata mutation; Windows hosting; bridge IPC/shared memory; Rust, manager, broker, CLAP, discovery beyond this exact fixture, or general Linux compatibility.

## Cleanup and rollback posture

Raw session data stays beneath one canonical, symlink-free user-cache session root. Synthetic fixtures stay beneath a canonical cache test root and are removed. The final operator quits Bitwig normally without saving; no Bitwig descendant, exact module mapping, validator, or forbidden compatibility process may remain. The accepted HP0 publication is intentionally left unchanged for later separately authorized work.

## Review standard

Review the exact PR head against this one admission claim, exact fixture and identity locks, normal-launch operator attestation, process ancestry and start identities, exact module mapping, bounded readback completeness, forbidden-process fail-closed behavior, before/after identity preservation, nonce custody, sanitization, changed-path envelope, and explicit nonclaims. A browser sighting or process-name coincidence alone is insufficient.
