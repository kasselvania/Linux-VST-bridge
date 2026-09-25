# RPI2 bounded second-recovery candidate

Base is BR1 evidence head `6c6f5ac92cb56e2767bb82bfdbf6d6152a1b0ea3`
(tree `d5e23a58524ba22413e20b70d65ada557c8ab6bf`). This branch does not
change PR #168 or its first Serum result. The retained private performance
trace showed one epoch-1 to epoch-2 recovery before the owner changed a Serum
preset, followed by an epoch-2 processing delay and terminal
`recovery_exhausted`. The final completed Windows processing span was 38.801
ms. That commercial result motivated a separately selected, bounded
second-recovery candidate; it does not prove Serum's later call would recover.

## Policy and callback ownership

Ordinary and unselected behavior is unchanged. The existing three-field
`LVB_RPI2_LIVE_RECOVERY=1024:1500:1` policy still permits exactly one
automatic recovery. A new four-field form, for example
`1024:1500:2:48000`, permits a second same-instance recovery only after
48,000 consecutive frames were delivered on time after the first recovery.
At 48 kHz that is one second of delivered frames. Any missing or expired
delivery resets the consecutive count. Intentional silence can still be
timely delivery; signal amplitude is not used to arm the policy. Two is the
hard per-instance recovery cap. A second incident before re-arm, a third
incident, a missed worker deadline and a lifecycle error remain terminal.

The callback adds fixed-size frame and re-arm state, four atomic stores at
each recovery request to clear the previous attempt's return/ack/resume
observations, and constant-time arithmetic after normal delivery. It adds no
allocation, lock, logging, worker wait, I/O or lifecycle call to the audio
path. The same existing completion-confirmed intent ledger, old-request
discard, STOP/START, reconciliation and re-prime path runs on either attempt.
`request_high` remains a historical high-water mark.

## Source-owned results

The mapped peer proved two separated finite stalls and epoch 1 to 2 to 3
progress. After the second acknowledgement, a confirmed old note was
released, the currently held note and latest parameter were applied, finite
nonzero stereo audio resumed, and STOP/DEACTIVATE/CLOSE completed. A second
incident before the healthy interval produced `recovery_exhausted` without a
second restart. A third incident after two recoveries remained terminal. A
second attempt held beyond its deadline produced `recovery_timeout`, with
admission closed and no request overflow. The original finite, timeout,
one-recovery exhaustion, capacity, stale-result and intent-custody tests
continue to pass.

Mac serial suites: backend 88 passed / one fixture-dependent ignore;
backend with `rpi0` 91 passed / one ignore; native event client 12 passed;
standalone appliance library 20 passed. The same updated `rpi0` source-owned
tests on the Pi passed 17/17.

## Pi 5 paced smoke and staged candidate

The Pi source files were copied to a separate private `rearm-source` tree.
Local and remote SHA-256 matched for the implementation files:

- `live_recovery.rs`: `e5bd962ac60f37c19e060b2884d2e8e117ef696db1d7de43c3d3650855d0d681`
- `queued.rs`: `cb6b3048c28b909141295ef410835b3c72a55d032f56725f633a6a9f3106ffc6`
- `rpi1/standalone/src/main.rs`: `97b460aba3e51a133bb25b7e2e4e38d6fb1335b66c0f8c60ac699f6517438950`

The exact focused test source after adding the result line was SHA-256
`3c09d1c6933b0d195f23cb02509f93aefbfc09159a7a19b037d4ed5076a767bd`
on the Pi. One paced Pi run of that test passed: second acknowledgement at
epoch 3, two recoveries, historical request high-water 9/2,048, 18 old
requests discarded cumulatively, zero queued old results discarded, 11.843
ms from the second request to worker return and 49.969 ms from worker return
to resumed audio in this paced fixture. It verified current note/control
state and clean source-owned retirement.

JACK was 48 kHz/512 frames with system-only ports before and after the test.
No device or audio setting changed. Temperature was 45.5 C before and 47.2 C
after, with `get_throttled=0x0` both times. The native ARM release binary was
built offline with the retained `jack-runtime,rpi2-quantum` feature pair and
staged privately as `lvb-arm-plugin-standalone-rearm-candidate`, SHA-256
`cab317f402b80f81af39c05e15a00404e83fe7625cce7ad8a597ec9a4b0c43ba`.
The previous staged native candidate remained SHA-256
`4b7259edc8ed5290aaba96b7c31c2c0a70d6432c399f235ea54cc5f1ffcf1403`.
The normal runner was not changed.

This paced mapped-peer run is a source-owned ARM correctness result, not a
JACK callback, analog latency, DSP-capacity or Serum preset result. No
commercial plug-in was launched with this candidate. The hard cap still
means a later repeated overload stops the session. The first Serum stall's
cause and the later 38.801 ms Windows processing span remain unexplained.
