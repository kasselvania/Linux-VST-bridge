# Current Slice: WR0 — Controlled Proton Runner and Isolated Windows Environment Bootstrap

## Status

```text
status: active_implementation_slice
slice: WR0
branch: codex/wr0-proton-isolated-bootstrap
basis commit: 3deb414a54174cd95432c84e117a642f30c482fe
basis tree: 7f29cc727ce128021a6a2d74d04ca9ad6e30cb13
target: main
```

WR0 owns one controlled Windows-process execution lane. It does not load a
Windows VST3 module, touch Serum, build a bridge, or choose the final
commercial-audio runner.

## Primary claim

On the exact accepted Steam Deck fixture, repository-owned supervision may use
only the already-installed Proton 11.0 deployment and its manifest-declared
Steam Linux Runtime to:

```text
repository-owned supervisor
  -> exact Steam Linux Runtime entrypoint
  -> exact Proton 11.0 script
  -> new staged project-owned compatdata environment
  -> runner-provided 64-bit Windows cmd.exe
  -> tracked wr0-probe.cmd
  -> exact stdout, internal receipt, architecture, and exit readback
  -> verified atomic environment promotion
  -> second launch through the same owned environment
  -> exact intentional exit-37 propagation
  -> clean owned-process-tree shutdown
```

Success proves controlled Windows command execution and isolated environment
bootstrap only.

## Exact runner candidate and implementability gate

- Candidate runner root:
  `<HOME>/.local/share/Steam/steamapps/common/Proton 11.0`.
- Candidate runtime roots are the installed `SteamLinuxRuntime` and
  `SteamLinuxRuntime_4` directories. Their names are not pairing authority.
- Before implementation, inspect the runner version/manifest/entrypoint,
  runner command executable, matching Steam app manifests, runtime metadata,
  runtime entrypoint, and pressure-vessel components.
- Establish the exact runtime app-ID pairing, supported Proton verb, minimum
  controlled environment, direct non-Steam-owned route, isolated layout, and
  expected mutation envelope.
- If those installed assets do not establish one exact contract, stop with
  `WR0_RUNNER_CONTRACT_BLOCKED`. Do not download, repair, update, or guess.

The content-bound implementation lock is `docs/WR0_RUNNER_LOCK.md`.

## Owned environment and transaction law

The only ordinary WR0 destination is:

```text
<HOME>/.local/share/linux-vst-bridge/environments/wr0-proton11
```

It must be canonical, absolute, beneath a symlink-free owned parent, and either
absent or an exactly verified WR0 environment. First creation occurs in a
temporary sibling. Run 1 executes there; verified prefix shape and the external
ownership/runner-binding marker are then promoted with an atomic rename. Run 2
and the expected exit-37 run use the promoted path. Failure restores prior
absence or the exact prior verified environment, and no stage/backup sibling
may remain.

The bounded WR0 repair may replace only the exact accepted predecessor
environment created by reviewed head
`9228217b2abf7314b9dfaecc5fc4323d5f3d7a89`. Before replacement, its complete
safe identity and receipts are verified. It is atomically moved to one
transaction-owned `.wr0-proton11.previous-*` sibling and remains untouched and
recoverable until repaired Run 1, promotion, Run 2, exit 37, live held-command
cleanup, preservation, and provisional evidence validation all pass. Any
pre-commit failure removes only new transaction-owned material and atomically
restores the exact predecessor. The replacement becomes authoritative only
after an exact transaction/runner/source/workload/run/protection-bound commit
record is atomically written, file-fsynced, directory-fsynced, and read back.
The in-memory state becomes post-commit before any predecessor content is
deleted. After that boundary the new final environment must never be removed
or overwritten by rollback: predecessor deletion, parent fsync, retired-record
readback, and final evidence are cleanup/finalization phases. An incomplete
retirement is `WR0_PREDECESSOR_RETIREMENT_BLOCKED`; an evidence failure after
retirement is `WR0_EVIDENCE_FINALIZATION_BLOCKED`. Both preserve the committed
new environment and its durable record.

The predecessor-backup operation guards the destination verification, exact
transaction-derived backup absence, rename, first parent-directory fsync, and
exact backup verification as one operation. Any failure after physical rename
must verify and restore the exact predecessor, fsync the parent, and prove the
canonical destination exact with the backup absent before rethrowing. Outer
pre-commit recovery classifies the actual destination/backup state and never
treats the `backup_created` event as filesystem authority. Unknown destination
or backup objects are preserved and refused. If neither the guarded helper nor
outer recovery can restore the exact predecessor, the result is
`WR0_PREDECESSOR_BACKUP_BLOCKED`.

The marker is outside registry content and binds the environment schema,
creation transaction/time, neutral application ID, exact runner/runtime
identity digest, and relative layout. It contains no credential, hostname,
username, account identifier, Windows machine identifier, or private path.

WR0 must not use or mutate `.wine`, Steam `steamapps/compatdata`, Bottles,
yabridge prefixes, Serum, or another Windows environment.

## Allowed tracked paths

Only these tracked paths may change:

```text
CURRENT_SLICE.md
docs/WR0_RUNNER_LOCK.md
windows-fixtures/wr0-probe/**
tools/wr0-proton-bootstrap/**
evidence/wr0-proton-bootstrap/**
```

No Rust, CMake, native-probe, binary, Windows VST3 source, scanner, bridge,
proxy, IPC, shared memory, manager, compatibility profile, runner/runtime
payload, generated prefix material, or proprietary content is authorized.

## Launch, supervision, and Steam-ancestry law

- Pin every executable and verify the declared launch-critical manifest before
  and after execution.
- Use Runtime `_v2-entry-point --verb=run --`, the pinned Proton script's
  `runinprefix` verb, and runner-owned x86_64 Windows `cmd.exe` only.
- Use neutral fixture app ID `0`; never impersonate a real Steam application.
- Start from an allow-listed environment and omit ambient compatibility
  overrides. Redirect persistent runtime/cache/config/data state beneath WR0.
- Normal mode accepts no caller-selected executable or environment root.
- Bind every governed WR0 tool, wrapper, workload, and runner-lock document to
  a clean-commit sorted Git path/mode/blob contract-source manifest; retain the
  `.cmd` SHA-256 independently and verify the historical live identity at the
  final evidence-only head.
- Use a nonce/run-bound ready/gate file handshake. Release the gate only after
  exact Proton-script and Windows-command identities and ancestry are live and
  proved; stdout alone is insufficient.
- Classify `windows_command` only when the Wine-hosted `cmd.exe` identity has
  one exact unique ordered argument vector: fixed `/d /q /c` switches, the
  complete Windows form of the tracked probe path, exact current nonce, exact
  decimal run number, contract-source digest, workload SHA-256, and
  `--exit-37` exactly when and only when exit 37 is expected. A matching
  basename, partial token set, duplicate subsequence, alternate path, or wrong
  switch/run/exit form is insufficient.
- Immediately before gate publication, take a fresh bounded `/proc` snapshot;
  revalidate the Runtime-root, Proton, and Windows-command PID/start identities,
  both exact ancestry chains, the full command vector, ready bytes/stdout, and
  continued gate absence. If an identity disappeared or changed, withhold the
  gate.
- Bound stdout, stderr, process count, polling interval, and wall time; create a
  new process group and census the complete descendant tree of the exact root
  PID/start identity, retaining safe identities/parent and group/session facts.
- On timeout, signal the isolated group and only exact observed PID/start
  identities that escaped it. Never use `killall`, global `pkill`, or unscoped
  wineserver termination. Revalidate each exact identity immediately before
  signalling. After TERM and any final KILL, run the production bounded
  owned-empty verifier; success requires the original group and every exact
  observed identity to be absent or zombie before deadline. Any survivor is
  `WR0_PROCESS_TOPOLOGY_BLOCKED`.
- Separately exercise that cleanup against the actual Runtime/Proton/Wine/cmd
  topology with the gate deliberately withheld. Prove every owned identity is
  gone, the unrelated same-family sentinel outside the ancestry survives, all
  handshake files and the disposable stage are removed, and the accepted final
  environment remains exact.
- Do not retain durable PIDs, command lines, environments, maps, or unrelated
  processes.
- Ordinary Steam/web helpers may remain open. Prove the repository supervisor
  owns the Unix launch root and that no Steam game-launch process is an
  ancestor. Do not create a Steam entry or modify Steam configuration.

The accepted wording is “project-owned process launch using exact existing
Steam-installed runner assets,” not Steam-independent distribution.

## Preservation and evidence law

Before and after live execution compare exact accepted identities for Bitwig
app/runtime and override bytes; HP0 publication/module/receipt/build source;
HP1 evidence; both SR0 Serum files including hash/size/mtime; the known `.wine`
module and selected registry hashes; the immediate Steam compatdata roster; and
the launch-critical Proton/Runtime/app-manifest lock. Steam background activity
is not globally frozen. Any protected drift invalidates WR0.

Run 1 must initialize a previously absent staged environment with run number
`1`, exact output/receipt/architecture/exit `0`, then promote atomically. Run 2
must reuse the same transaction/environment identity with run number `2`, a
fresh nonce, distinct process-tree identity, exact readback, and exit `0`. The
test path must return `37` through the supervisor exactly and leave the
environment valid.

Retain the bounded packet under `evidence/wr0-proton-bootstrap/`, including the
runner lock/digest, launch contract, environment transaction, three run
results, causal handshake, complete-root ancestry/guards, preservation, the
complete negative/production ledger, sanitization, `fixture.json`, and
`hashes.sha256`. Retain no private path, hostname, raw
launch directory, PID, full command line/environment/map, registry content,
machine identifier, Steam account data, prefix file, proprietary content, or
secret.

The 30 accepted negative cases remain required. The repaired ledger also runs
production helpers for strict command/run/path identity, readiness and bad-gate
handling, verified cleanup failure, exact escaped-descendant cleanup, Git-backed
contract-source identity across detached temporary worktrees, and the separate
live held-command exercise. It additionally injects failures before backup,
after backup, after promotion, during commit write/readback, after durable
commit, during predecessor deletion/fsync, and during evidence
rendering/sanitization; every case proves the phase-appropriate rollback or
post-commit preservation law. The production backup helper is additionally
exercised across first-fsync failure, backup-verification failure, physical
rename before event recording, outer physical-state recovery, exact-backup
false-event recovery, forged/unknown object refusal, both-absent refusal, and
successful receipt-bound rollback. Temporary worktrees remain beneath the canonical
WR0 cache test root and are removed and pruned.

## Blocked-result law, cleanup, and review

`WR0_RUNNER_LAUNCH_BLOCKED` remains reserved for an actual pairing or direct
launch failure. If the exact run/path-bound Windows-command identity cannot be
observed, use `WR0_PROCESS_OBSERVABILITY_BLOCKED`. If the actual held-command
topology cannot be safely terminated and verified empty, use
`WR0_PROCESS_TOPOLOGY_BLOCKED`. A failure before durable replacement commit is
`WR0_REPLACEMENT_PRECOMMIT_BLOCKED` after the exact predecessor is restored. A
failure that cannot restore the exact predecessor during or after guarded
backup is `WR0_PREDECESSOR_BACKUP_BLOCKED`. A
post-commit predecessor-cleanup failure is
`WR0_PREDECESSOR_RETIREMENT_BLOCKED`; a post-retirement evidence failure is
`WR0_EVIDENCE_FINALIZATION_BLOCKED`. Neither post-commit path may invoke the
pre-commit rollback helper or remove the authoritative replacement. Retain
bounded failure ownership, reverify preservation, and do not install UMU or
select another Wine/Proton build.

Cleanup is transaction- and process-group-scoped. Failed first creation
restores absence; failed synthetic replacement restores its byte-identical
owned predecessor; success leaves only the verified final environment for a
separately authorized future slice. Review the exact PR head against this one
claim, content lock, ownership transaction, output/exit, ancestry, clean
shutdown, preservation, negative ledger, changed-path envelope, sanitized
evidence, and claim ceiling. Leave the PR ordinary, open, unmerged, and target
`main`.

## Explicit non-goals and nonclaims

WR0 does not load, inspect, launch, install, authorize, or claim usability of
Serum or any VST3 module. It does not launch Bitwig, alter Flatpak overrides,
use `.wine` or Steam compatdata, install/build/update a runner, build a Windows
binary, scan plug-ins, process audio, implement IPC/shared memory/proxy/bridge
or manager behavior, add Rust, prove real-time safety, choose a product runner,
claim Steam-independent distribution, or generalize beyond this exact fixture.

## Accepted predecessor: HP1

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

## WR0 selection receipt

The technical lead selected only WR0 against the exact pinned basis after the
read-only pre-flight and runner/runtime contract scan. The operator explicitly
approved replacing the no-active-slice card with this bounded authority. No
later scanner, VST3, Serum, Bitwig, bridge, IPC, shared-memory, manager, broker,
Rust, audio, or successor-slice work is selected or implied.
