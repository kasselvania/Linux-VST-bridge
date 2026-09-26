# PSL1 selected-state continuation on Pi

This continuation repairs the B-state readiness refusal without loosening the
state comparison. The input selection stays immutable. Preparation must return
the same complete state as ordinary restoration of that original selection.
All commercial state and audio recordings remain private.

## Cause and correction

Original PSL1 `8764432721274ce27fd2cb2c020e57c3f0e08563` completed B's eight
preparation blocks and then refused `final state readback differs`. Its first
restore was already `Accepted`, rather than byte-exact to the input. That
failure remains retained; it was not a preparation timeout or process fault.

On the Pi, a fresh ordinary restore of original B, a second ordinary restore
of original B, and a restore of the first returned snapshot were captured
privately. The first two produced identical complete envelopes. Their
controller-state bytes were identical to the input. The opaque component
bytes changed, and five parameter readbacks changed by one representable
double-precision step (maximum absolute change about 2.78e-17): Env 1 Attack,
Env 2 Attack/Release, Env 3 Attack/Release. Loading the returned snapshot
instead introduced another rounding step. No proprietary component layout was
decoded or modified, and no normalized derivative replaced B.

The executor now retains the first validated restore readback before
activation. After muted preparation, stop and deactivation, it restores the
**same original input**, validates the final envelope, and compares every byte
with that initial readback. It retains original and restored-state digests in
the receipt, which explicitly names `final_reference=initial_restored_state`.
A different component, controller, parameter, envelope length or identity still
refuses readiness/resumption. There is no tolerance or vendor-specific field
exclusion. This establishes no extra serialized-state effect from preparation,
not equivalence of unobservable vendor internals.

## Source and fixture

- Continuation source: `b0c0b113cfe610250db0aa4363152dd5dcf76ee8`, tree
  `4333a809dcffa3f26a5f7ade6791fe4c51142b99`, built on PSL1 PR #176. PR #176 and
  private-authoring PR #177 remain separate unmerged dependencies.
- ARM release executable SHA-256:
  `aad6563a6fa78d85878d63018331eb87e3816de045f433b3a200723638653646`.
  Built separately with `--release --locked --features jack-runtime` using
  Pi Cargo 1.98.1 and the same source dependency locks.
- Preparation source SHA-256:
  `aa3f36b1507ede3b65b05f1f506352328c375614d2a4ba937c364762fe174000`.
- Unchanged Windows host SHA-256:
  `b41eb3696e49f26ae715ac0656d9c0ced1383b3824008d998117f4dfbe404f24`.
- Unchanged Serum 2.1.5 module SHA-256:
  `501e7bb3dd9cafe416b3412df3d4e084c01b7468201e5690b9009d7ecd4e5283`;
  class `56534558667350736572756d20320000`.
- Unchanged v2 binding SHA-256:
  `1dc7e7251f2a6db860d4dbcb4c5885fe24f174600d43c1714886e7b50ba440a8`:
  channel 0, pitch 60, velocity 100, one hold block, six settle blocks.
- Raspberry Pi 5; GE Proton 11-7 AArch64/FEX, closed editor, 48 kHz/512 JACK,
  256-frame quantum, 512-frame reserve, protocol 12/map v1, unchanged
  governor, scheduling, runtime, recovery and installed selections.
  Kernel `6.18.50+rpt-rpi-v8`.

| State | Original envelope SHA-256 | Ordinary restored envelope SHA-256 |
|---|---|---|
| A | `41fa2e07500a7a8d46f9d833353f5a31cf4eb38117ea1fa5071ff607bf8a90ab` | same |
| B | `3d90d7cf3fbebb100693934f53f1409fb2ed8024e07f81c5d1fed9e88e7473fc` | `ca7f3a4b310af397c087efa6a3106356fbf6acddf3df73fb16bcfe024caf34d7` |
| C | `ba0815bb1480a8660e612c0f46f50b8804219c866f4ce7da0ebc869516533aa9` | same |

B and C are the prior agent's privately authored Deck fixtures using PR #177,
not newly substituted sounds. Their full prepared snapshots match independent
ordinary-restore snapshots on the Pi byte for byte.

## Source verification

All 36 standalone library tests pass locally and in the Pi ARM release build
with `jack-runtime`. New tests verify original-input reuse across both
restorations, changed-but-valid ordinary serialization, full-envelope final
comparison, and refusal without resumption when a valid final envelope differs
from the baseline (including a final envelope that equals the original input).
Existing identity, malformed-state, completion, timeout and lifecycle tests
remain intact. Touched-file formatting and diff checks pass.

## Measured runs

The original six/eight-second MIDI fixture alternates pitches 60 and 64 at
velocity 100. The native metrics count delivered, missing and expired frames;
an independent JACK capture also retained finite nonzero stereo samples in
later runs. These are digital-output observations, not a person hearing the
SHIELD output. No audio connection to user hardware was changed.

| Run | Preparation / state | First user-note Windows bracket | Missing frames | Result |
|---|---|---:|---:|---|
| Prior B, unprepared | ordinary restore | 44.8 ms | 4,608 | retained prior cold-note failure |
| B, isolated prepared, 8 s | exact ordinary-state equivalence | 4.263 ms | 0 | nonzero stereo, no xrun/fault, clean retirement |
| C, unprepared, 6 s | ordinary restore | 47.496 ms | 5,632 total, 5,120 after the initial status | cold-note and later gaps, clean retirement |
| C, prepared, 6 s | input and ordinary-state exact | 4.061 ms | 0 | finite nonzero captured stereo, no xrun/fault, clean retirement |
| A then B in one process | both exact ordinary-state equivalence | A 2.622 ms; B not separately retained by the bounded note trace | A 0; B 256 during the later 16 s segment | state transition succeeded; later isolated gap remains a failed gap-free run |

B moved a 58.469 ms Windows bracket into 355.433 ms of total preparation;
C moved a 60.751 ms bracket into 516.282 ms. A's five-second prepared idle
interval delivered 239,616 silent frames with no nonzero samples or missing
frames. The A-to-B preparation took 288.380 ms; its output was discarded while
playback was paused. The saved B snapshot after that transition matches the
independent ordinary B snapshot.

The later A-to-B gap was at B epoch-4 position 499,456, not its first note.
The missing request waited 7.761 ms behind a preceding request and took
11.966 ms in total. The preceding Windows processing bracket was 3.216 ms;
the following bracket before output completion was 4.326 ms and includes
enabled CPU diagnostic queries. This identifies a delay outside the vendor
processing bracket, not its exact cause. A separate run disables the existing
optional native/Windows diagnostics with the same binaries and state; results
are recorded below. Diagnostic-off results do not erase the traced failure.

With diagnostics disabled, one fresh process completed A startup, five
seconds of silent readiness, six seconds of A notes, prepared A-to-B restore,
60 seconds of B notes, prepared B-to-C restore and 30 seconds of C notes.
All three preparation receipts matched their complete initial restored state.
The saved B and C snapshots after in-session changes match their independent
ordinary-restore snapshots. Across the session, 5,056,512 delivered frames at
the final status had **zero missing/expired frames, zero gaps, zero JACK xruns
and no processing fault**. Independent recordings retained finite nonzero
stereo output for all three sounds. The B recording contains 59 note-ons and
59 note-offs; C contains 30 and 29. The original fixture ends on a wall-clock
boundary and can stop before its last note-off; normal cohort retirement
followed. This is bounded two-pitch coverage, not an arbitrary MIDI workload.

That run used a separate, digest-pinned private launcher whose sole functional
change was `LVB_AP10_TRACE=0`; native tracing was also disabled. No binary,
vendor setting, scheduling policy, quantum, reserve or installed selection
changed. CPU-query instrumentation remains available for diagnostics but is
not the normal playback posture. The successful untraced run is not by itself
proof that diagnostics caused every earlier gap.
The diagnostic-off launcher SHA-256 is
`4a1f142432922453c509eacf176b3d0d113aab724946f886ae5b9ad4330e53d9`.

A final fresh-process B comparison used the same executable and diagnostics
disabled in both arms. The unprepared six-second run again lost 4,608 frames.
The prepared twelve-second run lost zero frames, produced finite nonzero
captured stereo, matched the complete ordinary-restored B snapshot and retired
cleanly. Its first-user-note repair therefore does not depend on enabling the
diagnostics or on the earlier A preparation in the same process.

An earlier B candidate run accidentally overlapped an ARM test build and lost
137,216 frames. It is retained as a confounded failed run, excluded from the
isolated playback comparison. Its complete prepared-state snapshot still
matched ordinary restoration. Builds completed before all subsequent runs.

## Environment and limitations

The initial connection found a running JACK server whose socket was absent.
With no instrument client active, the same system service was restarted and
client reachability, 48 kHz and 512 frames were verified. The private config
was rebound to the current working X authority digest. Its first stale-digest
refusal launched no Windows session. Intermittent filesystem I/O stalls also
delayed startup; they are not classified as Serum preparation or audio
processing failures. No storage policy, driver or kernel was changed.
Final readback found no candidate cohort or candidate JACK ports, and the
existing JACK server accepted clients at the original 48 kHz/512 settings.
The remote-development connection remains open temporarily; independence from
the user login lifecycle is not established by that cleanup readback.

This patch does not install a normal Pi generation or make arbitrary editor
preset changes prepared automatically. `--prewarm` and `restore prepared`
remain explicit. It establishes bounded selected-state behavior; general
preset/polyphony coverage, ordinary preset-selection UX, physical listening,
power-cycle recall and instrument qualification require further product work.

The allow-listed [machine-readable results](selected-state-results.json)
retain all nine completed continuation runs, failed/confounded results,
preparation receipts, metric samples, audio hashes, and seven exact complete
snapshot comparisons. Raw state, recordings and operational paths are excluded.
