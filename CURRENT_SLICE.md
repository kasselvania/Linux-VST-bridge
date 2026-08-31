# Current Slice: None Selected

## Status

```text
status: no_active_implementation_slice
implementation: forbidden
last completed slice: HP0 — Native VST3 Probe Build and Bitwig Flatpak Sandbox Load
```

No successor, Bitwig discovery/scan session, Flatpak override change, Serum operation, installer/authorization session, runner experiment, Windows host, proxy, IPC, shared-memory transport, manager, broker, editor, CLAP work, or compatibility claim is selected or implied.

## Last accepted slice

```text
slice: HP0
implementation PR: #3
reviewed amended head: ebe84cc9d450d960cceb7f584af520bb51ce8133
reviewed tree: 8ab8f70dee1092503ae92761be38095e10605aad
basis commit: 5614da2d769931f0aa26a8f902d14e911b39b085
basis tree: 6d3cea1638e12e65298d9496755d8cc4d0f2eeba
merge commit: ad2072b9dbb65e86819eae6ccbde7eadef9377f5
merge tree: 8ab8f70dee1092503ae92761be38095e10605aad
```

## Accepted claim

HP0 establishes the first project-owned native Linux VST3 boundary on the exact Steam Deck and Bitwig Flatpak fixture:

```text
repository-owned C++20 source
  -> exact user-scope Freedesktop SDK 25.08
  -> exact pinned official VST3 SDK and recursive submodules
  -> deterministic LabHostProbe.vst3 Linux x86_64 bundle
  -> official VST3 validation
  -> exact owned publication beneath <HOME>/.vst3
  -> exact published-bundle validation inside the exact system Bitwig app sandbox
```

This route was exercised without launching Bitwig Studio or changing its Flatpak overrides.

The retained packet is under:

```text
evidence/hp0-native-vst3-bitwig-sandbox/
```

The repository-owned source and tooling are under:

```text
native-probe/
tools/hp0-native-probe/
cmake/
```

## Accepted identities

- Probe module/display/vendor/version: `LabHostProbe` / `LAB Host Probe` / `Kasselvania Research` / `0.1.0`.
- Processor CID: `6F4E7A5392E54B54A98AD6F714E0C201`.
- Controller CID: `B9C42F0736C34E218E5A71D40C8F1B62`.
- Gain parameter: `0x4C485001`; bypass parameter: `0x4C485002`.
- Freedesktop SDK: `runtime/org.freedesktop.Sdk/x86_64/25.08`, commit `b90ed309cc1d505dea48b6a2121c5dcfac22868120eee643b0596d31f96b9bb8`.
- Official VST3 SDK: `3cdf9ca5d1f5b1b21e0a86832aa4abe55607bd96`, with the seven recursive submodules locked in `docs/HP0_DEPENDENCY_LOCK.md`.
- Build-source identity schema: `linux-vst-bridge-hp0-build-source/v1`.
- Canonical 19-path build-source manifest SHA-256: `ee301e3e42b2381d5a9aa88ec64635482d098bc8cde50d33a57e6c397fb1e9ab`.
- Module SHA-256 from both clean builds: `3abaa8a2051ea179e28986cc6fbad4cde2d0d5e9a844b18aca91cdf7c59e69d7`.
- Bundle-manifest SHA-256 from both clean builds: `ec8a4531a3b73da3c3e29c6e1f3c90c989fb61e3f3397af32d4c1576b3624ea9`.
- Official validator SHA-256 from both clean builds: `cccae776eb87fbbbf6ac9c34c78bf1548937c48db3843e0cf0f12d312650466f`.
- Each clean-build validator run: exit `0`, 47 passed, 0 failed.

The two validator logs differed only in repetition count for one retained informational parameter-change-consumption notice. HP0 does not claim byte-identical logs or sample-accurate automation.

## Accepted publication and sandbox facts

- Exact owned publication remains at `<HOME>/.vst3/linux-vst-bridge/LabHostProbe.vst3` for a later separately selected slice.
- Ordinary receipt remains at `<HOME>/.cache/linux-vst-bridge/hp0-publication.receipt` under schema `linux-vst-bridge-hp0-publication/v2`.
- Publication is an exact copied bundle, not an SDK-generated symlink.
- Publication replacement and receipt commit are transactional through final bundle/receipt readback; tested failures restore the exact prior bundle/receipt or exact prior absence.
- Existing receipts require exact HP0 ownership and current publication hashes; unrelated and alternate ordinary receipt destinations are refused.
- Dynamic test publication, receipt, build, and log paths require canonical no-symlink containment beneath one exact user-cache test root.
- Bitwig fixture: system `com.bitwig.BitwigStudio` 6.0.11, app commit `7b66aed37386ffff99e40bbd486f90ceee7ac1d5c59508c7952094122cebf50e`, no user-scope shadow.
- Bitwig runtime: `org.freedesktop.Platform/x86_64/25.08`, commit `bd44a6230581917d04f89812a4c21090c304d390edb73995af1c2f9fd8abf4e8`.
- The exact published bundle was visible/readable with resolved dependencies inside that app sandbox.
- The receipt-bound official validator was rehashed inside the sandbox and returned exit `0`, 47 passed, 0 failed.
- Effective `VST_PATH` remained `/app/extensions/Plugins/vst;=/usr/lib/`; effective `VST3_PATH` and `CLAP_PATH` remained empty.
- User and system override outputs, app/runtime identities, and no-Bitwig-process posture were exact before and after.

## Accepted negative posture

The retained suite passed 35 deterministic assertions plus exact override preservation. It covers build-source and provenance drift, SDK/toolchain mismatch, unknown publication/receipt ownership, alternate ordinary receipt paths, canonical test-root escapes, transactional failure points, exact Bitwig app/runtime drift, modified validator/module, wrong architecture, missing bundle, and Bitwig-running refusal.

Generated binaries, SDK checkouts, build directories, validator executables, published bundles, private paths, proprietary plug-ins, credentials, and license material remain untracked.

## Current unknowns

HP0 does not determine:

- whether Bitwig automatically discovers the owned `~/.vst3` publication while the effective `VST3_PATH` is empty;
- Bitwig scan result, class registration, or plug-in-host isolation behavior;
- whether Bitwig instantiates and destroys the probe correctly;
- whether its generic parameter view exposes gain and bypass correctly;
- whether audio, automation, state, project save/reopen, or crash containment work in Bitwig;
- whether the existing Serum proxy or Windows module is discoverable or operational;
- Serum build, license channel, authorization state, editor behavior, or content paths;
- any Windows host, bridge, IPC, shared-memory, Wine/Proton/UMU, performance, CLAP, or general-Linux capability.

## Nonclaims

HP0 is native build, official validation, owned publication, and explicit app-sandbox load evidence only. It does not claim Bitwig discovery, scanning, instantiation, project state, generic UI, audio or musical operation; Serum compatibility or authorization; a Windows bridge; real-time transport; manager/broker behavior; or general Linux support.

## Work selection

The technical lead must inspect this accepted boundary and present one bounded next decision. Until the operator approves that decision, do not launch Bitwig for retained work, change its Flatpak overrides, modify the owned publication, add a Windows host/proxy/IPC layer, touch Serum or its environment, or widen the build system.
