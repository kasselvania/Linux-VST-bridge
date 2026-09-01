# Current Slice: None Selected

## Status

```text
status: no_active_implementation_slice
implementation: forbidden
last accepted slice: WR0 — Controlled Proton Runner and Isolated Windows Environment Bootstrap
```

No Windows VST3 scanner, native proxy/factory crossing, bridge IPC, shared-memory transport, Serum operation, Bitwig operation, installer/authorization session, manager, broker, editor, CLAP work, cleanup program, design amendment, or compatibility claim is selected or implied.

## Last accepted slice

```text
slice: WR0
implementation PR: #7
reviewed head: 9228217b2abf7314b9dfaecc5fc4323d5f3d7a89
reviewed tree: 8da6817eba1f259d3565e377fcb50098ab8f3cf2
basis commit: 3deb414a54174cd95432c84e117a642f30c482fe
basis tree: 7f29cc727ce128021a6a2d74d04ca9ad6e30cb13
implementation merge: 8237b96ce7c885edcf4e7a0923f2ac78d05a928d
```

## Accepted claim

WR0 establishes one exact controlled Windows-process execution and environment-ownership lane on the accepted Steam Deck fixture:

```text
repository-owned supervisor
  -> exact Steam Linux Runtime 4 entrypoint
  -> exact installed Proton 11.0 runner
  -> runner Wine
  -> exact runner-provided x86_64 cmd.exe
  -> exact tracked nonce/run/source/workload-bound command fixture
  -> isolated project-owned environment
  -> verified Run 1
  -> atomic promotion
  -> verified Run 2 through the same environment
  -> exact live exit-37 propagation
  -> verified scoped cleanup of the real Runtime/Proton/Wine topology
  -> durable environment replacement commit
  -> predecessor retirement
  -> finalized retained evidence
```

The retained packet is under:

```text
evidence/wr0-proton-bootstrap/
```

The reusable exact runner, environment, process-supervision, transaction, and evidence tooling is under:

```text
tools/wr0-proton-bootstrap/
```

The tracked Windows command fixture is:

```text
windows-fixtures/wr0-probe/wr0-probe.cmd
```

## Accepted runner and environment identities

- Runner: `1787334450 proton-11.0-2-x86_64`.
- Runtime: Steam Linux Runtime 4 `4.0.20260805.254769`.
- pressure-vessel: `0.20260805.0`.
- Launch-critical identity: `linux-vst-bridge-wr0-launch-critical/v1` / `2d64df1d36786ca2d0e955c553005423dc2b5bdd714bd0a17872622e33912547`.
- Contract-source identity: `linux-vst-bridge-wr0-contract-source/v1` / `887b148b7862038fd8fc0af52146ecc7452a5fbf82c22c5af8eaec6fe47504b8`.
- Workload SHA-256: `4518ca37b8d7e005f01b441de5273447b70fa7c8c9c3d2b9c194a91782cfecac`.
- Owned environment: `<HOME>/.local/share/linux-vst-bridge/environments/wr0-proton11`.
- Final environment transaction: `wr0-20260901T045337Z-caf9f4eaf52d2d34`.
- Final environment identity: `447e6d4dfccfbebe7be44cc521e8ed192bfad4ab1fe1f16673d391161db73c24`.
- Durable replacement-record schema: `linux-vst-bridge-wr0-replacement-commit/v1`.
- Final retired replacement-record SHA-256: `aba3913537590b6c1302cfd473cb29e201b68342b58e8d92cdc6d09fe61ec6f1`.

## Accepted operating laws

WR0 proved several laws that later stateful/process slices must consume rather than rediscover casually:

- The installed runner/runtime pairing is content-bound and may not be selected from directory names alone.
- A Windows workload is not admitted from stdout alone; its exact process identity, command vector, run identity, and ancestry are causally proved before release.
- Process ownership follows the exact Runtime-root descendant tree across process-group and session boundaries.
- Cleanup signals only exact PID/start identities and must prove the owned scope empty.
- An environment replacement remains reversible before durable commit.
- After durable commit, the new environment is authoritative and cannot be destroyed to restore an old predecessor.
- Destructive predecessor retirement begins only after exact durable commit and readback.
- Filesystem physical state and exact identity—not an in-memory boolean alone—govern recovery.
- Evidence finalization is a separate post-commit phase and cannot rewrite implementation authority.

## Claim ceiling

WR0 does not prove that Proton can load, scan, instantiate, authorize, display, or process audio through a Windows VST3 module. It did not inspect or launch Serum, launch Bitwig, modify `.wine` or Steam compatdata, implement a native proxy, cross a VST3 factory boundary, implement bridge IPC or shared memory, establish real-time safety, select a final distributable product runner, or generalize beyond the exact fixture.

## Work selection

The technical lead must inspect the accepted native-host and Windows-runner boundaries and present one bounded next decision. The operator has directed that the repository first formalize a human-facing implementation-design gate, adversarial design review, and successor-selection protocol before another high-risk implementation slice begins.

That direction does not itself authorize repository changes beyond a separately bounded governance slice, and it does not authorize Windows VST3, Serum, bridge, IPC, audio, or manager implementation.
