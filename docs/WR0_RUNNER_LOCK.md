# WR0 installed runner and runtime lock

## Selection result

The installed candidate is implementable without Steam game-launch ancestry.
The runner's own manifest—not either directory name—selects the runtime:

| Fact | Locked value |
|---|---|
| Runner root | `<HOME>/.local/share/Steam/steamapps/common/Proton 11.0` |
| Runner `version` | `1787334450 proton-11.0-2-x86_64` |
| Runner Steam app/build | `4628710` / `24867889` |
| Runner installed depot manifest | `3114679013132291065` |
| Runner-required tool app | `4183110` |
| Runtime root | `<HOME>/.local/share/Steam/steamapps/common/SteamLinuxRuntime_4` |
| Runtime Steam app/build | `4183110` / `24599767` |
| Runtime installed depot manifest | `78117001432799844` |
| Runtime depot version | `4.0.20260805.254769` |
| pressure-vessel version | `0.20260805.0` |
| Neutral WR0 application ID | `0` |

`SteamLinuxRuntime` is the separately installed Scout runtime, app `1070560`.
It is not the selected pairing and is refused by WR0.

## Exact launch contract

The only ordinary route is:

```text
repository launch.py supervisor
  -> <RUNTIME>/_v2-entry-point --verb=run --
  -> <RUNTIME>/run
  -> <RUNTIME>/pressure-vessel/bin/pressure-vessel-unruntime
  -> <RUNNER>/proton runinprefix
  -> <RUNNER>/files/bin/wine
  -> <RUNNER>/files/lib/wine/x86_64-windows/cmd.exe
  -> <REPO>/windows-fixtures/wr0-probe/wr0-probe.cmd
```

The runner script requires `STEAM_COMPAT_DATA_PATH` and reads
`STEAM_COMPAT_CLIENT_INSTALL_PATH` while constructing the prefix. Runtime state
is redirected with `PRESSURE_VESSEL_VARIABLE_DIR`. WR0 starts from an empty
explicit environment mapping and declares only identity/locale basics,
`XDG_RUNTIME_DIR`, the three neutral
app-ID variables, exact compat paths, WR0-owned XDG cache/config/data paths, and
disabled GUI integration. Ambient compatibility overrides, Proton logging or
tuning, library injection, UMU, and yabridge state are absent.

The inspected native-only contract probe returned exact magic
`WR0_RUNTIME_NATIVE=OK` and exit `0`. It used an SSH-owned invocation and an
owned disposable pressure-vessel variable directory; no Steam game launch was
called.

## Command fixture

The executable fixture is the runner-provided PE32+ console command processor:

```text
safe path: runner/files/lib/wine/x86_64-windows/cmd.exe
type: PE32+ executable for WINE (console), x86-64
size: 1210529
sha256: 74a8fece1a1affc3ad06c82726f069ff7ca9ce0e6a703fe088d89aa38a6aaa4b
```

The tracked `.cmd` is project workload input, not a Windows binary.

The workload publishes a nonce/run-bound ready file under its owned prefix and
waits in a finite `cmd.exe` built-in loop. The supervisor verifies the exact
ready bytes and stdout, observes the exact Proton script and `cmd.exe` process
identities in the complete runtime-root descendant tree, proves their ancestry,
and only then atomically commits the matching gate. `srt-bwrap` and other
Runtime wrappers cannot satisfy the Proton role merely because their argument
vectors contain downstream paths.

The Windows-command role requires the Wine-hosted `cmd.exe` identity and one
exact unique ordered argument subsequence: `/d`, `/q`, `/c`, the complete
Windows form of the fixed tracked workload path, the current 32-hex nonce, the
exact decimal run number (`1`, `2`, or `37`), contract-source digest, workload
SHA-256, and `--exit-37` exactly when and only when exit 37 is expected. A
same-basename alternate path, wrong run, missing/reordered switches, partial
tokens, or duplicate matching vector is refused.

Immediately before publishing the gate, the supervisor takes a fresh bounded
`/proc` snapshot and revalidates the exact Runtime-root, Proton, and command
PID/start identities, both ancestry chains to the same Runtime root, the full
argument vector, exact ready file/stdout, and continued gate absence. Gate
publication is refused if an identity disappeared or changed.

## Repository-owned contract-source identity

Schema: `linux-vst-bridge-wr0-contract-source/v1`.

A stable sorted manifest binds repository-relative path, Git file mode, and Git
blob identity for every tracked file beneath `tools/wr0-proton-bootstrap/` and
`windows-fixtures/wr0-probe/`, plus this lock document. Generation requires a
clean implementation commit and refuses missing, untracked, dirty, staged,
unmerged, duplicate, symlinked, non-regular, or unexpected governed paths. The
tracked `.cmd` SHA-256 is retained independently. The source digest is bound
into the raw session, ownership marker, run receipts, evidence, and
`fixture.json`.

Live evidence records the clean implementation commit/tree as provenance. A
final evidence-only amendment may change the PR head, but the retained manifest
must still verify from the final head; any governed source change invalidates
it and requires new live execution.

## Launch-critical identity

Schema: `linux-vst-bridge-wr0-launch-critical/v1`.

The manifest is generated from the safe-relative roster embedded in
`launch.py`, including runner version/manifest/script and launch-time Python
inputs, Wine/wineserver/preloader/command processor, selected default-prefix
registries, relevant licenses/notices, both Steam app manifests, Runtime
versions/manifest/entrypoint/platform metadata, and selected pressure-vessel
wrappers/helpers. Every entry binds type, size, and SHA-256. Verification
refuses a missing, symlinked, changed, or undeclared pairing before launch and
again after execution.

The canonical manifest digest is
`2d64df1d36786ca2d0e955c553005423dc2b5bdd714bd0a17872622e33912547`
and must agree with retained live evidence and the environment ownership
marker. The installed deployments are
never updated, validated through Steam, repaired, copied, or recursively hashed
as a substitute for this declared lock.

## Owned layout and mutation envelope

```text
<HOME>/.local/share/linux-vst-bridge/environments/
  .wr0-proton11.stage-<transaction>/
    wr0-environment.json
    compatdata/pfx/
    runtime-var/
    host-cache/
    host-config/
    host-data/
    receipts/
```

After Run 1 verification, the stage directory is atomically renamed to
`wr0-proton11`; Run 2 and exit propagation use that same inode-backed root and
transaction marker. Persistent writes are confined to this root. Runtime may
use bounded transient `/tmp`, `/dev/shm`, and `XDG_RUNTIME_DIR` objects while
its exact root-descendant tree lives; completion requires the group, every
observed PID/start identity (including descendants outside that group), and
handshake artifacts to be gone. TERM and any final KILL are followed by the
production owned-empty verifier; a surviving exact identity is
`WR0_PROCESS_TOPOLOGY_BLOCKED`. A separate live gate-withheld exercise uses the
actual installed topology, proves scoped cleanup reaches zero while an
unrelated same-family sentinel survives, and removes only its disposable
transaction stage. No installed runner file, Runtime file, Steam compatdata
prefix, `.wine` file, or protected fixture is a mutable target.

The repaired governed source replaces only the exact accepted WR0 predecessor.
That environment is first verified in full and atomically retained as a
transaction-owned `.wr0-proton11.previous-*` sibling. It is restored exactly on
any pre-commit failure. After the complete replacement launch, live cleanup,
preservation, and commit-ready evidence validation succeed, the new environment
is made authoritative by a `linux-vst-bridge-wr0-replacement-commit/v1` record
that is atomically written, file-fsynced, directory-fsynced, and read back. The
record binds both environment identities, transaction identities, runner lock,
contract source, workload, three run receipts, held-cleanup result, and
protected-fixture digest. Only after that durable readback may predecessor
retirement begin. Post-commit deletion/fsync failure preserves the new final
environment and remaining transaction-owned predecessor material; it never
enters the pre-commit rollback path. Final evidence may claim retirement only
after predecessor absence, parent fsync, exact new-environment readback, and a
retired commit-record update are all verified.

## Claim ceiling

This lock supports only the WR0 deterministic Windows command workload. It is
not selection of a distributed product runner and is no evidence that Proton
can host a Windows VST3, audio process, GUI, authorization flow, or bridge.
