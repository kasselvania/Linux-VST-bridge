# WR0 launch contract

```text
launch.py supervisor
  -> Runtime 4 _v2-entry-point --verb=run --
  -> Runtime 4 run / pressure-vessel-unruntime
  -> Proton 11.0 proton runinprefix
  -> runner wine
  -> runner x86_64-windows cmd.exe
  -> tracked wr0-probe.cmd
```

- Neutral application ID: `0`.
- Contract-source schema / SHA-256: `linux-vst-bridge-wr0-contract-source/v1` / `887b148b7862038fd8fc0af52146ecc7452a5fbf82c22c5af8eaec6fe47504b8`.
- Historical implementation commit / tree: `100713669295f67d47aa670cf33891c72bef0481` / `7d14b91bd838d60faee9a443b66ac2104fe72da9`.
- Tracked workload SHA-256: `4518ca37b8d7e005f01b441de5273447b70fa7c8c9c3d2b9c194a91782cfecac`.
- Persistent state roots: exact WR0 compatdata, runtime variable, cache, config, data, and temporary directories only.
- Explicit environment names: `HOME`, `USER`, `LOGNAME`, `PATH`, `LANG`, `XDG_RUNTIME_DIR`, `XDG_CACHE_HOME`, `XDG_CONFIG_HOME`, `XDG_DATA_HOME`, `TMPDIR`, `STEAM_COMPAT_DATA_PATH`, `STEAM_COMPAT_CLIENT_INSTALL_PATH`, `STEAM_COMPAT_APP_ID`, `SteamAppId`, `SteamGameId`, `PRESSURE_VESSEL_VARIABLE_DIR`, `STEAM_ZENITY`.
- Environment begins empty; ambient compatibility overrides, logging/tuning variables, library injection, and caller executable paths are absent.
- Run 1 / reuse deadlines: `180` / `90` seconds.
- Stream cap: `16384` bytes each; process cap: `256` identities; poll: `0.05` seconds; drain: `20.0` seconds.
- `windows_command` requires exactly one contiguous, case-exact `/d /q /c <full Z:\<REPO>\windows-fixtures\wr0-probe\wr0-probe.cmd> <nonce> <decimal-run> <source-digest> <workload-digest> [--exit-37]` vector; basename suffixes, partial tokens, wrong runs, reordered switches, duplicates, and misplaced exit flags are refused.
- Readiness uses the nonce/run-bound `LINUX_VST_BRIDGE_WR0_HANDSHAKE_V1` file gate. Immediately before gate commitment, a new bounded `/proc` census revalidates the Runtime root, the same selected Proton and Windows-command PID/start identities, both complete ancestry chains, the shared Runtime root, exact vector, ready file/stdout, and gate absence.
- Observation follows the complete bounded descendant tree of the exact runtime root PID/start identity; group/session membership is classified separately.
- Timeout cleanup targets the isolated group and only immediately revalidated observed PID/start identities that escaped it, calls the production empty-scope waiter after TERM and KILL, and raises `WR0_PROCESS_TOPOLOGY_BLOCKED` on any survivor. Cleanup deadline: `10.0` seconds.
- A separate actual held-command probe withheld the gate for `1.0` second after exact observation, terminated the real Runtime/Proton/Wine/cmd topology to zero, and left an unrelated same-comm sentinel alive.
- Detached Git worktrees beneath the canonical WR0 cache exercised evidence-only stability, changed workload/supervisor/wrapper refusal, unexpected governed-path refusal, historical-provenance non-authority, and dirty/staged-source refusal through the production source functions.
- Full command lines and environment values are not retained.

## Contract-source manifest

| Repository-relative path | Git mode | Git blob |
|---|---|---|
| `docs/WR0_RUNNER_LOCK.md` | `100644` | `d2076de6c90f6611ea8ab55426b57a6edf5c8a4c` |
| `tools/wr0-proton-bootstrap/README.md` | `100644` | `4e2b71a7ae524b3ab76e57db6f15e8595de746d1` |
| `tools/wr0-proton-bootstrap/common.sh` | `100755` | `2e382985402e19b09d71ceaf85078a77f52bab57` |
| `tools/wr0-proton-bootstrap/environment.sh` | `100755` | `b5389c36e0d75484b2cdad5f159f8a9cbd1d73e6` |
| `tools/wr0-proton-bootstrap/inspect-runner.sh` | `100755` | `e157340eff629127332d94650d4f369ad8688d94` |
| `tools/wr0-proton-bootstrap/launch.py` | `100755` | `2a5337be3c2d1a330a245289f694b5ca3a18ab66` |
| `tools/wr0-proton-bootstrap/negative-tests.sh` | `100755` | `b1882ddb3f903a441145866bfb24650ff046a20e` |
| `tools/wr0-proton-bootstrap/preflight.sh` | `100755` | `13bceffe6c75830ef6ed0828d32dcd7d7938068b` |
| `tools/wr0-proton-bootstrap/sanitize.sh` | `100755` | `44fd2d9ecf369030ed58b7898cdcfb87c2075cfb` |
| `windows-fixtures/wr0-probe/wr0-probe.cmd` | `100644` | `04c8707c01c92cfd007fe7f7a72a8adbd4de3588` |
