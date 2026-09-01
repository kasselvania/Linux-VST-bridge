# WR0 controlled Proton bootstrap tooling

These tools prove one exact Windows command-process lane through the already
installed Proton 11.0 and its manifest-declared Steam Linux Runtime 4.0. They
do not load a plug-in, launch Bitwig, touch Serum, use `.wine` or Steam
compatdata, install software, or select a product runner.

## Fixed ordinary paths

All ordinary paths are derived from the passwd-owned home and repository root;
none is caller-selectable:

```text
runner:       <HOME>/.local/share/Steam/steamapps/common/Proton 11.0
runtime:      <HOME>/.local/share/Steam/steamapps/common/SteamLinuxRuntime_4
Steam root:   <HOME>/.local/share/Steam
environment:  <HOME>/.local/share/linux-vst-bridge/environments/wr0-proton11
workload:     <REPO>/windows-fixtures/wr0-probe/wr0-probe.cmd
raw sessions: <HOME>/.cache/linux-vst-bridge/wr0/sessions/
tests:        <HOME>/.cache/linux-vst-bridge/wr0/tests/
```

Every path is absolute, canonical, contained beneath its declared root, and
checked for symlinked ancestors. Synthetic tests alone may use dynamic paths,
and only beneath the one canonical WR0 test root.

## Commands

From the exact Deck checkout:

```bash
tools/wr0-proton-bootstrap/inspect-runner.sh
tools/wr0-proton-bootstrap/preflight.sh --implementation-worktree
tools/wr0-proton-bootstrap/negative-tests.sh
python3 tools/wr0-proton-bootstrap/launch.py inspect-contract-source
tools/wr0-proton-bootstrap/environment.sh
tools/wr0-proton-bootstrap/sanitize.sh
python3 tools/wr0-proton-bootstrap/launch.py verify-retained-source
```

`inspect-runner.sh` verifies every declared file and prints the content-bound
manifest digest. `preflight.sh` checks pinned ancestry/path envelope, SteamOS
read-only posture, forbidden process families, accepted fixtures, and runner
lock. The contract-source inspection requires clean governed source files and
binds their sorted Git modes/blobs plus the independent `.cmd` SHA-256.
`negative-tests.sh` preserves the original 30 cases and adds strict
command-vector, handshake, verified-cleanup, and Git-backed production
contract-source cases beneath the test root. It also exercises the production
replacement phase, durable-record, retirement, and evidence-finalization
helpers plus the guarded predecessor-backup and physical-state recovery helpers
with 24 bounded failure-injection cases. The clean-source suite contains
75 non-live cases. `--implementation-worktree` runs the 68 deterministic cases
that are valid before the intermediate implementation commit; it skips only
the clean-source detached-worktree matrix.

`environment.sh` re-runs preflight and the clean-source suite, captures the
protected fixture, and verifies the accepted predecessor environment. It moves
that predecessor to a transaction-owned recoverable sibling through one guarded
rename/first-fsync/exact-verification operation. Its outer pre-commit recovery
uses exact physical state rather than the caller's event flag and refuses
unknown destination or backup objects. It then creates
a new stage, executes and promotes repaired Run 1, executes Run 2 and exit 37,
then runs actual wrong-nonce/wrong-run gate probes and a separate live
gate-withheld cleanup exercise. It compares protected state, re-verifies the
runner lock, provisionally validates the complete sanitized evidence packet,
and only then atomically writes, file-fsyncs, directory-fsyncs, and reads back
the exact replacement commit record. That readback makes the new environment
authoritative before predecessor deletion begins. A pre-commit failure removes
only new transaction-owned material and restores the exact predecessor. A
post-commit retirement or evidence failure preserves the new environment and
never enters the rollback path. Final evidence is staged and validated without
a retirement claim, then published as finalized only after predecessor absence,
parent fsync, exact final-environment readback, and retired-record readback.
Run 1 has a 180-second deadline; reused and exit runs have 90-second deadlines;
the held-command cleanup uses its shorter declared bound. Each stream is capped
at 16 KiB, sampling is bounded to 256 process identities at 50 ms intervals,
and post-exit drain is bounded.

The supervisor starts a new Unix session/process group, then follows the
complete bounded descendant tree of the exact root PID/start identity. The
Windows command publishes a nonce-bound ready file and cannot pass its finite
built-in wait loop until the supervisor has observed the exact Proton and
`cmd.exe` identities, proven their ancestry, and atomically written the exact
gate. The command role requires the exact fixed switches, full workload path,
nonce, run number, contract digest, workload digest, and exit-37 flag posture.
A fresh snapshot immediately before gate publication revalidates all three
process identities, both ancestry chains, the exact vector, ready bytes/stdout,
and gate absence. A timeout sends TERM/KILL to the isolated group and exact
observed PID/start identities that escaped it, then the production owned-empty
verifier must reach zero or report `WR0_PROCESS_TOPOLOGY_BLOCKED`. It never
invokes global process termination or wineserver shutdown. It retains safe
start identities, parent relations, and group/session classifications instead
of durable PIDs or command lines. The live held-command exercise additionally
proves an unrelated sentinel outside the Runtime ancestry remains alive.

`sanitize.sh` verifies the evidence roster, JSON, hash manifest, UTF-8/NUL/size
bounds, placeholders, and redaction patterns. Raw session JSON remains in the
user cache and is never a Git input except through the allow-listed renderer.
It also verifies that the historical live-session contract-source digest still
matches the current PR head after an evidence-only amendment.

## Failure posture

Unknown destinations, fixture drift, runner/runtime mismatch, output mismatch,
unexpected exit, missing receipt, ancestry contamination, timeout, orphan,
promotion/rollback failure, evidence overflow, and redaction failure are hard
errors. Exact command-role observation failures are
`WR0_PROCESS_OBSERVABILITY_BLOCKED`; surviving exact identities after final
scoped KILL are `WR0_PROCESS_TOPOLOGY_BLOCKED`. Pre-commit replacement failure
is `WR0_REPLACEMENT_PRECOMMIT_BLOCKED` after exact restoration. Incomplete
post-commit predecessor cleanup is `WR0_PREDECESSOR_RETIREMENT_BLOCKED`, and
post-retirement evidence failure is `WR0_EVIDENCE_FINALIZATION_BLOCKED`; both
preserve the authoritative new environment and durable record. No command
repairs or downloads anything.

Success proves only deterministic Windows command execution and owned
environment bootstrap on this exact fixture. It does not show that Proton can
host a Windows VST3 or support audio, GUI, state, licensing, or bridge behavior.
