# RPI2 live recovery re-arm candidate

Base: `6c6f5ac92cb56e2767bb82bfdbf6d6152a1b0ea3` (tree
`d5e23a58524ba22413e20b70d65ada557c8ab6bf`), draft PR #168.
Branch: `codex/rpi2-live-recovery-rearm`. The approved BR1 source and first
Serum failure record remain on PR #168; this is a separate stacked candidate.

Basis: `AGENTS.md` real-time, slice, identity and evidence laws;
`GOVERNANCE.md` “What evidence means”; `docs/ARCHITECTURE.md` host/audio
ownership; the retained BR1 source-owned modes and the first bounded Serum
gate in `evidence/rpi2/live-recovery-br1-serum-gate.md`.

## Primary claim and fixture

An explicit second-recovery appliance policy can re-arm only after 48,000
consecutive timely delivered frames following the first same-instance
recovery, and can never exceed two recoveries in one instance. A second
incident before re-arm and a third incident remain terminal. The existing
one-recovery policy and unselected ordinary path retain their behavior.

The exact source-owned mapped peer is the primary fixture. The physical Pi 5
fixture uses the retained ARM native toolchain and build cache, 48 kHz/JACK
512, 256-frame processing quantum, existing private source staging, and no
commercial plug-in. Its paced callback does not establish JACK callback or
vendor performance behavior.

## Scope and acceptance

Only `native-vst3-proxy/backend/src/live_recovery.rs`,
`native-vst3-proxy/backend/src/queued.rs`, their existing source-owned tests,
and `rpi1/standalone/src/main.rs` may change for the candidate. The
four-field `LVB_RPI2_LIVE_RECOVERY` selection carries the re-arm frame count;
the existing three-field one-recovery selection remains valid. No policy is
selected by default.

The source-owned peer must prove two separated finite stalls, exact epoch 1
to 2 to 3 transitions, note/control reconciliation, current nonzero audio,
bounded request occupancy, and normal retirement. It must also prove refusal
before healthy re-arm, a second-attempt worker deadline, and hard failure on
a third incident. Run the serial backend, event and appliance tests. Build a
private ARM native candidate and run one Pi source-owned paced smoke with
pre/post JACK graph, temperature and power checks. Record exact source and
binary identities and the relevant counters.

## Negative gates and nonclaims

Do not select this candidate for Serum or the normal runner in this slice.
No second commercial preset test, DSP speed claim, new buffer/latency setting,
FEX/Wine/UMU change, priority/affinity change, vendor binary edit, snapshot
replacement, or broad profiler belongs here. A worker that never returns
still times out; a repeated overload still reaches a terminal bound. This
candidate does not establish that the specific Serum preset can resume after
its later processing delay.

Preserve the previous native binary, Windows host, runner, authorization
state, canceled checkout and private logs. Retain only sanitized evidence in
the repository. Commit and push the focused change as a separate draft PR;
leave it unmerged for review before another owner-operated Serum test.
