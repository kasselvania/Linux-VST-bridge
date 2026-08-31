# Current Slice: None Selected

## Status

```text
status: no_active_implementation_slice
implementation: forbidden
last completed slice: SR0 — Steam Deck Host and Serum 2 Pre-Installation Reconnaissance
```

No successor, host probe, installer/authorization session, runner experiment, Flatpak override change, bridge implementation, VST3/CLAP implementation, build-system scaffold, or compatibility claim is selected or implied.

## Last accepted slice

```text
slice: SR0
implementation PR: #1
reviewed amended head: 7eee151da1bd94a0faf0af5c481ce4c8ea013408
reviewed tree: ac8e440222df71ae735bff21b94c519dea47f2c6
basis commit: 7319ba8dea7cf2e81a2e4d32c39907bdc6279cf4
basis tree: b3dc324444f769e7d145bb85f5d56a7d587ea6f2
merge commit: 0c41a43ce6f5b1e32eb3da5e169c66aacaa007fe
merge tree: ac8e440222df71ae735bff21b94c519dea47f2c6
```

## Accepted claim

SR0 establishes a reproducible, sanitized, exact baseline of the maintainer's Steam Deck + Bitwig Flatpak fixture and its existing Windows-audio compatibility state, including bounded Serum 2 artifact observations, without mutating commercial software, compatibility environments, DAW configuration, or SteamOS.

The retained packet is under:

```text
evidence/sr0-steam-deck-fixture-reconnaissance/
```

The reusable capture is under:

```text
tools/sr0-fixture-capture/
```

## Accepted fixture facts

- Steam Deck model `Galileo`, SteamOS `3.8.16`, x86_64, KDE/Wayland, read-only mode enabled.
- Bitwig Flatpak `6.0.11`, system installation, Flathub stable, Freedesktop `25.08`.
- Bitwig package metadata declares VST, VST3, and CLAP Linux Audio extension paths.
- Current user overrides leave effective `VST3_PATH` and `CLAP_PATH` empty and produce effective VST `/app/extensions/Plugins/vst;=/usr/lib/`; SR0 did not change them.
- No installed `org.freedesktop.LinuxAudio*` extension matched exact branch `25.08`.
- PipeWire `1.6.4`, WirePlumber `0.5.14`, PulseAudio compatibility, and ALSA were observed.
- Proton `11.0`, Steam Linux Runtime entries, and UMU `steamrt3` data were observed as existing runner/data directories.
- One direct `.wine` environment and 33 immediate Steam compatdata prefixes were retained through a complete bounded prefix census.
- A yabridge-style native Serum 2 proxy bundle and corresponding Windows VST3 bundle are present in bounded locations.
- Native proxy regular-file SHA-256: `317d70f95a3c7559ff3d43b014c3e7b792fd03362d5e999d550a5566cf25b184`.
- Windows module regular-file SHA-256: `838bc7ab42d5d039156768680ffc3e0175d6e6bed9f99802989a01d695e13175`.
- No lawful Serum 2 installer or other Serum/Xfer content was found in the fully completed declared bounded locations.

## Accepted capture posture

- Collection requires a trusted GNU `timeout`; no unbounded fallback is permitted.
- Completeness-sensitive observations fail closed on timeout, command failure, row/output truncation, candidate-cap exhaustion, rejected roots, or incomplete prefix coverage.
- Dynamically accepted roots require lexical and canonical containment beneath the real home and reject symlinked ancestors.
- Linux Audio extensions match Bitwig only on the exact Freedesktop runtime branch.
- Seven deterministic self-tests cover timeout propagation, artifact-cap exhaustion, directory-row truncation, root self-match exclusion, runtime-branch mismatch, symlinked-ancestor rejection, and normal complete non-finding.
- Raw data remains ignored/transient; retained evidence is sanitized, bounded text/JSON with verified hashes.

## Current unknowns

SR0 does not determine:

- the exact Serum 2 build/version;
- the lawful Serum 2 license channel;
- current authorization state;
- whether Bitwig currently discovers the existing proxy;
- whether the existing proxy and Windows module can instantiate;
- whether audio, MIDI/events, automation, state, project recall, editor behavior, or crash containment work;
- which Wine/Proton/UMU runtime created or can operate the existing environment;
- whether the current Bitwig Flatpak overrides are intentional, stale, or causally related to plug-in visibility.

## Nonclaims

SR0 does not claim that Serum 2 installs, authorizes, scans, opens, processes audio, appears in Bitwig, or interoperates with Proton, Wine, yabridge, or a project bridge. It does not generalize this fixture to Linux support and does not authorize credentials, installer execution, environment mutation, Flatpak override changes, or bridge implementation.

## Work selection

The technical lead must inspect this accepted evidence and present one bounded next decision. Until the operator approves that decision, do not add `Cargo.toml`, CMake projects, SDK sources, native proxies, Windows hosts, runners, compatibility profiles, or experimental host-path changes.
