# RPI1 Pigments census diagnosis — 2026-09-22 UTC

The exact Pi completed the Pigments census through the existing bounded
supervisor. The earlier exit 82 is reproduced by malformed Windows readiness
and gate paths in the manually constructed shell commands. No plug-in,
Windows-host, Proton, Box64 or graphical-policy repair was needed for this gate.

## Source and fixture

- Branch: `experiment/rpi1-pigments-arm64-appliance`.
- Executed source: `9bf4fa8d58a7eb543452c7a3dcdf75871e944819`.
- Source tree: `42dfd36d7cb17575e5820b82f2827f7c5a7ac2ca`.
- Pi 5 Model B Rev 1.1, AArch64, `6.18.50+rpt-rpi-v8`, 4 KiB pages.
- Exact inherited Proton 11.0-2c / SLR4 and Box64 pins, with the existing private
  Pi prefix, machine identity and X11 session preserved.
- Exact accepted AP18 census host: `d778f238…12307a`.
- Pigments module: `bdc91ebe…6a3c07`, version `7.0.1.6772`.

Full identities, source hashes, observations and nonclaims are in
[`pigments-census-diagnosis.json`](../../evidence/rpi1/pigments-census-diagnosis.json).
The complete runtime closure was verified in the earlier transfer evidence;
this run rechecked its entry points, adapter, Box64, host and module. It did
not repeat the full closure walk.

## Cause and controlled comparison

The retained failed launch commands used this shell token:

```sh
"C:\bridge\sessions\$session\$session.ready"
```

Within double quotes, the backslash before `$` prevents variable expansion.
The actual argument becomes:

```text
C:\bridge\sessions$session$session.ready
```

The gate path has the same defect. `parse_args` in the exact host requires
`C:\bridge\sessions\<session>\<session>.ready` and its matching gate.
It throws `handshake path mismatch`; the host's top-level exception handler
returns 82 before emitting `scanner_started`, publishing readiness, or loading
the module. Exit 82 alone was insufficient to identify a failed stdout write.

The Pi comparisons held the module-loading gate closed:

| Arguments | Host stdout | Readiness | Exit before cleanup |
| --- | --- | --- | --- |
| Correct structured arguments | Regular file | Passed | Host still waiting |
| Correct structured arguments | Pipe | Passed | Host still waiting |
| Original malformed paths | Pipe | Absent | 82 |

The malformed case produced zero stdout bytes. All three comparisons retired
their owned descendants and process groups. None loaded Pigments. These facts
reject file-connected stdout as the necessary cause and reproduce the actual
argument defect independently of the previous shell session's mutable variables.

## Working launch and result

The successful census called the unchanged
`bridge-manager/runtime/session.py` functions `command(spec)` and `run(spec)`.
A private adapter supplied the exact Pi registration and inherited ARM runtime
environment. Command construction remained a Python argument array, so Windows
backslashes never passed through shell interpolation. The existing supervisor
provided pipes, bounded records, exact readiness/gate matching, host/module
reverification, call deadlines and PID/start-identity cleanup.

The adapter preserved the previously established `PYTHONHOME=/usr` correction
for translated container Python. The old CPython `OSError: [Errno 22]` was
reconfirmed in the retained prior stderr; its absence in this successful launch
does not establish a general Box64/CPython fix.

One gate was released and one Pigments module census ran:

- Processor class: `41727475415649534B61743150726F63`.
- Second class: `4AFD4B6A35D7C240A5C31414FB7D15E6`, plug-in compatibility class.
- Two-channel auxiliary Sidechain input; two-channel main Stereo Out.
- MIDI input reports 16 channels; MIDI output reports zero channels, preserving
  the raw fact behind the inherited Pigments compatibility policy.
- 4,446 parameters; float32 accepted, float64 refused.
- Reported latency: 44 samples; reported tail: zero.
- State capture available: SDK result zero, 155,038 bytes. Opaque state was not
  exported; restoration was not tested.
- 230 protocol records, successful inspection close, module exit/unload and
  `scanner_completed`.
- Supervisor reported no error, confirmed cleanup and retired the session files.

The outer launcher was terminated by the supervisor's normal post-completion
cleanup and has raw exit `-15`. This is retained separately from the host's
successful lifecycle records; it is not a claim of natural launcher exit.
Final readback found zero private RPI1 runtime processes and no census session
directory. Temperature spot samples were 47.7–50.5°C, with `throttled=0x0`.

An initial adapter check incorrectly required the existing environment root to
have transport-root permissions. It refused before Windows launch. The adapter
was corrected without changing Pi filesystem permissions. The negative probe's
aggregate result filename collided with the comparison summary; both separate
owner stdout logs retained the original records used for the evidence export.

## Preserved limitations

The i386 setup-probe failure, SLR ldconfig helper signal 11 and fallback,
and missing native-wrapper warnings remain present. Earlier installer-time
power loss, undervoltage and filesystem orphan cleanup remain a real failure
from the operator's prior handoff. This census does not qualify USB power margin,
cooling, timing, sustained processing or physical operation.

The supplied Blackhole PR #135 review addresses a different boundary: a missing
mount source before Wine starts on the Deck. It was not imported or installed.
This Pi attempt used its existing graphical policy and needed no dead sockets.

## Solution forward

Use the existing supervisor's structured command constructor and bounded pipes;
do not resume the hand-built shell launch strings. Preserve the exact selected
Proton/SLR/Box64 closure, private Pi prefix, `PYTHONHOME=/usr`, operation-local
accessibility policy and observed Pigments class/module identity.

The next implementation job is the RPI1 standalone binding. The current RPI0
standalone supervisor launches its pinned control Wine directly, and its host
arguments use the source-owned instrument class. They do not yet implement the
selected Pigments Proton launch. Adapt those specific boundaries to reuse this
proven launch contract and the observed Pigments buses and inherited retirement,
editor, event-output and state policies. Keep RPI0 unchanged as its reference.

Then perform the selected 2,048-frame / 48 kHz physical sequence: MIDI and stereo
audio, real editor interaction and reopen, state restoration, clean retirement,
and a fresh second musical session. Those are still unproved. Overall RPI1
remains pending; the census blocker is resolved.
