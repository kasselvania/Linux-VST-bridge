# Current Slice: None Selected

## Status

```text
status: no_active_implementation_slice
implementation: forbidden
last completed slice: HP1 — Bitwig Native Discovery and Instance Admission
```

No successor, Windows runner experiment, Windows scanner, proxy/factory crossing, Flatpak override change, Serum operation, installer/authorization session, audio transport, IPC, shared memory, manager, broker, editor, CLAP work, or compatibility claim is selected or implied.

## Last accepted slice

```text
slice: HP1
implementation PR: #5
reviewed head: 282a19936e7a3037c197df03ce3552acd6214e16
reviewed tree: 77bfeff33f991fe724cd3808f0ede8e3e742b7dc
basis commit: 7cda2d85eb426c2ed6e4eb3d86e52c114c4aa1c4
basis tree: e9534822625ceff0f06549b78c85546050bddf42
merge commit: 185a372e7cb20e539857b22579ee77cecb9f6469
merge tree: 77bfeff33f991fe724cd3808f0ede8e3e742b7dc
```

## Accepted claim

HP1 establishes the first actual native-DAW admission of the accepted HP0 fixture on the exact Steam Deck and Bitwig Flatpak host:

```text
exact owned HP0 publication
  -> normal operator-controlled Bitwig launch
  -> native VST3 discovery and class registration
  -> one operator-inserted instance
  -> exact LabHostProbe.so mapping by a proven Bitwig descendant
  -> clean shutdown
  -> second normal Bitwig launch
  -> discovery without another rescan
  -> second exact Bitwig-descendant module mapping
  -> clean shutdown
```

The retained packet is under:

```text
evidence/hp1-bitwig-admission/
```

The reusable operator-assisted session tooling is under:

```text
tools/hp1-bitwig-admission/
```

## Accepted fixture and native identities

- Host: Steam Deck model `Galileo`, SteamOS `3.8.16`, x86_64, KDE Wayland.
- Bitwig: system `com.bitwig.BitwigStudio` `6.0.11`, stable/x86_64, app commit `7b66aed37386ffff99e40bbd486f90ceee7ac1d5c59508c7952094122cebf50e`.
- Runtime: `org.freedesktop.Platform/x86_64/25.08`, commit `bd44a6230581917d04f89812a4c21090c304d390edb73995af1c2f9fd8abf4e8`.
- User/system override SHA-256: `1b4a6a6ed688f69dd3c36dcac8db008c5a41ed52170ea3e23dee984b0aae6a1e` / `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`.
- Effective `VST_PATH=/app/extensions/Plugins/vst;=/usr/lib/`; effective `VST3_PATH` and `CLAP_PATH` remained empty.
- Accepted module: `<HOME>/.vst3/linux-vst-bridge/LabHostProbe.vst3/Contents/x86_64-linux/LabHostProbe.so`.
- Module SHA-256: `3abaa8a2051ea179e28986cc6fbad4cde2d0d5e9a844b18aca91cdf7c59e69d7`.
- Processor CID: `6F4E7A5392E54B54A98AD6F714E0C201`.
- Controller CID: `B9C42F0736C34E218E5A71D40C8F1B62`.
- Display/vendor: `LAB Host Probe` / `Kasselvania Research`.
- HP0 publication receipt SHA-256: `d328adb27fef326b8e5b1104f072c246a803389a77d35ee5babc9834067668d1`.
- HP0 build-source identity: `linux-vst-bridge-hp0-build-source/v1` / `ee301e3e42b2381d5a9aa88ec64635482d098bc8cde50d33a57e6c397fb1e9ab`.

## Accepted discovery and instance evidence

- Session 1 Bitwig launch root start ticks: `2308190`; PID-redacted identity SHA-256 `3f540119652a972b63c16b81929ea48652c557e40bbcb4ddca5e06edda7c1921`.
- Session 2 Bitwig launch root start ticks: `2321310`; PID-redacted identity SHA-256 `82f31d7d35e3738101d8aae510122a01e0a311132bcc4c84a37620ec97447b69`.
- The two launch identities were distinct.
- In each launch, `BitwigPluginHost-X64-AVX2` mapped the accepted module under a complete proven Unix ancestry to the corresponding Bitwig Flatpak launch root.
- The retained mapping device/inode and accepted filesystem device/inode agreed in both sessions.
- Bitwig-owned bounded index/log readback retained the exact class name, vendor, and processor CID, with no retained same-line applicable probe scan error.
- The operator's one-use nonce receipt SHA-256 is `cc7fabbb70a9c680eb34941fdeefc1cf46eafd2a1b08e8683505e325632d9ad1`.
- The operator attested to normal desktop launches, one insertion/removal in each session, and Session 2 discovery without another rescan.
- Both launch trees shut down cleanly, and no Bitwig, validator, Wine/Proton workload, UMU, or yabridge-host process remained afterward.

## Supporting parameter observation

- Gain in Bitwig's generic parameter view: `observed`.
- Bypass as a separately visible generic parameter: `not_observed`.
- Binding between the VST3 `kIsBypass` parameter and Bitwig's host-level device bypass: `unknown`.

These observations do not establish parameter operation, automation, bypass DSP behavior, or host-bypass binding.

## Accepted preservation and negative posture

- Bitwig app/runtime identities and user/system override bytes remained exact before and after.
- The HP0 publication, module, receipt, and build identity remained exact.
- Both accepted SR0 Serum regular-file hashes, sizes, and mtimes remained exact.
- Seventeen deterministic negative cases passed, covering fixture drift, process ancestry, stale/reused identities, forbidden-process contamination, nonce misuse, bounded state-readback failure, path escape, and preservation mismatch.
- Generated binaries, raw session state, nonce, durable PIDs, complete process maps, command lines/environments, projects, Bitwig account/license data, proprietary binaries, and Serum state remain untracked.

## Current unknowns

HP1 does not determine:

- audio or DSP correctness in Bitwig;
- gain or bypass parameter operation;
- sample-accurate automation;
- processor/controller state round-trip through a Bitwig project;
- project save, application restart, machine reboot, or project reopen behavior;
- host-level bypass binding;
- multi-instance behavior or Bitwig sandbox-mode interactions;
- editor behavior;
- whether the existing Serum proxy or Windows module is discoverable, authorized, or operational;
- any Windows runner, scanner, host, proxy, IPC, shared-memory, real-time, manager, broker, CLAP, another-DAW, or general-Linux capability.

## Nonclaims

HP1 is exact native discovery, class registration, operator-assisted instance admission, process ancestry, and native module-mapping evidence only. It does not claim that Bitwig processed audio through the probe, that parameter or project state works, that Serum works, that a Windows bridge exists, or that the result generalizes beyond the accepted fixture.

## Work selection

The technical lead must inspect the accepted native boundary and present one bounded next decision. Until the operator approves it, do not change Bitwig or Flatpak configuration, rebuild or move the native probe, touch Serum or its environment, select a Windows runner, add a Windows scanner/host, create a bridge proxy, or introduce IPC/shared-memory/manager/broker code.
