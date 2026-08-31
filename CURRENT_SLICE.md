# Current Slice: SR0 — Steam Deck Host and Serum 2 Pre-Installation Reconnaissance

## Status and basis

```text
status: active
basis commit: 7319ba8dea7cf2e81a2e4d32c39907bdc6279cf4
basis tree: b3dc324444f769e7d145bb85f5d56a7d587ea6f2
branch: codex/sr0-steam-deck-fixture-reconnaissance
claim level: fixture-observed reconnaissance
```

## Primary claim

Establish a reproducible, sanitized, exact baseline of the current Steam Deck + Bitwig Flatpak fixture and its existing Windows-audio compatibility state, including the observed presence or absence of Serum 2 artifacts, without mutating commercial software, compatibility environments, DAW configuration, or SteamOS.

## Exact host fixture

- Maintainer-owned Steam Deck running the currently installed SteamOS release.
- Repository checkout at `<HOME>/code/Linux-VST-bridge` in retained authority/evidence; pre-flight separately verifies the operator-declared absolute path.
- Installed or absent `com.bitwig.BitwigStudio` Flatpak and its matching Freedesktop runtime state.
- Existing user-space audio, development, Wine/Proton/UMU, runner, prefix, bridge, and plug-in state only.
- Lawfully owned Serum 2 material, if any, is observed only through safe metadata in declared bounded locations.

The captured packet replaces the real home, username, and hostname with placeholders. The Steam Deck fixture is exact; it is not a general Linux minimum or compatibility claim.

## Allowed tracked changed paths

- `CURRENT_SLICE.md`
- `tools/sr0-fixture-capture/**`
- `evidence/sr0-steam-deck-fixture-reconnaissance/**`

Temporary raw capture is permitted only under the ignored path `evidence/raw/sr0-steam-deck-fixture-reconnaissance/` or beneath `$XDG_CACHE_HOME`. It must never be staged or retained as review evidence.

## In-scope observations

- Exact repository commit, tree, branch, tool revision, timestamp, and Git version.
- SteamOS, kernel, CPU, memory, session/display presence, read-only posture, and bounded local-filesystem capacity.
- Bitwig Flatpak version, branch, origin, architecture, runtime, scope, permissions, declared plug-in paths, and matching Freedesktop/Linux Audio runtime state.
- Read-only PipeWire, WirePlumber, PulseAudio-compatibility, ALSA, audio-class/count, and privacy-safe default-route facts.
- Installed/absent development and Windows-compatibility tools with read-only versions where available.
- Immediate entries beneath declared runner and prefix roots, without recursive prefix inventory.
- Serum/Xfer-named artifacts only within declared installer, Linux plug-in, and already-discovered Windows VST locations.
- Bounded name-based process-presence checks without command lines or environments.
- Remaining operator-controlled installer, GUI, credential, authorization-channel, and next-slice inputs.

## Explicit non-goals and nonclaims

SR0 does not install, update, launch, authorize, scan, configure, or exercise Serum 2, Bitwig, Wine, Proton, UMU, winetricks, yabridge, Native Access, Splice, or any commercial installer. It does not create or mutate a Wine prefix, runner, compatibility profile, Flatpak override, Bitwig setting, audio route, service, firewall rule, SSH setting, SteamOS read-only setting, or package state.

SR0 adds no build system, application scaffold, VST3/CLAP implementation, native proxy, Windows host, IPC, shared memory, audio, editor, manager, or compatibility code. It does not inspect an entire home directory, dump registries or settings, retain proprietary content, or change another repository.

The evidence does not claim that Serum 2 exists outside the bounded locations, installs, authorizes, scans, opens, processes audio, appears in Bitwig, or interoperates with Proton or a bridge. A missing match is `not_found_in_bounded_locations`, never `does_not_exist`.

## Evidence requirements

Retain a machine-readable `fixture.json` and technical-lead-readable basis, host, Bitwig, toolchain/runtime, Windows-audio, Serum 2, findings, operator-handoff, sanitization, and hash reports under `evidence/sr0-steam-deck-fixture-reconnaissance/`.

Every relevant result uses one of these classes:

- `observed`
- `not_installed`
- `not_found_in_bounded_locations`
- `operator_input_required`
- `gui_session_required`
- `unknown`
- `explicitly_out_of_scope`

Exact candidate installer or module files retain only sanitized path, file type, size, modification time, and SHA-256. No candidate is copied, opened, or executed. Raw evidence, prefixes, proprietary binaries, settings, credentials, activation data, account identifiers, and paid content are prohibited from Git.

## Sanitization requirements

- Replace the real home path, username, and hostname.
- Redact local IP addresses, MAC addresses, UUID-like serial identifiers, emails, and account-like identifiers if encountered.
- Collect only allow-listed facts; do not collect environment variables wholesale.
- Do not retain display socket names, device serials, private Bluetooth names, complete process listings, full process command lines, or unrelated storage names.
- Bound command time, bytes, rows, recursion depth, and candidate count.
- Do not follow symlinks into unrelated storage.
- Verify raw evidence is ignored and unstaged before acceptance.

## Acceptance criteria

1. The capture scripts pass `bash -n` and run without root or package installation.
2. Two runs produce equivalent structured fixture facts apart from declared volatile fields such as timestamp, capacity, process state, and tool digest inputs.
3. The packet records the exact Deck, Bitwig Flatpak, runtime, audio, toolchain, runner/prefix, process, and bounded Serum/Xfer state without launching a prohibited program.
4. `fixture.json` parses, retained files are bounded UTF-8 text/JSON, and `hashes.sha256` verifies.
5. Redaction review finds no real username, home, hostname, IP, MAC, email, credential, token, cookie, serial, license material, or proprietary binary.
6. Git contains no raw evidence and the tracked changed-path envelope is exact.
7. Findings distinguish observation, absence of an installed tool, bounded search non-findings, unknowns, GUI/operator requirements, and out-of-scope claims.
8. The remaining lawful-installer location, license channel, graphical-control method, credential entry, and next-slice ownership are stated precisely.
9. The pull request claims reconnaissance only and remains open, non-draft, and unmerged for technical-lead review.

## Operator handoff

After every safe noninteractive observation, the operator supplies only unresolved private inputs: exact Serum 2 license channel, a local path for the lawful Windows installer, disclosure of any uninspected existing installation, the graphical remote-control method, and credentials directly to the vendor UI when a later authorized slice requires them. The recommended untracked input root is `~/.local/share/linux-vst-bridge-fixtures/serum2/`.

Installer launch and authorization belong to a separately selected slice. SR0 must not cross that boundary.

## Review standard

Review the exact PR head against this single observational claim, the pinned basis, the changed-path envelope, command and search bounds, non-mutation posture, classification accuracy, evidence/source correspondence, sanitization, proprietary-material exclusion, explicit unknowns/nonclaims, and operator handoff. A complete-looking inventory is not acceptable if it overclaims absence or crosses an authorization, installer, DAW, runner, prefix, or privacy boundary.
