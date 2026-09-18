# RPI0 standalone ARM appliance result

## Disposition

`PHYSICAL_FIXTURE_UNAVAILABLE`

RPI0 is **not passed**. This branch contains a Pi-ready deterministic
implementation, but the implementation environment was an Apple ARM64 host,
not a Raspberry Pi, and no authorized Pi endpoint, physical MIDI controller,
stereo sink, or X11/XWayland Pi session was available. No physical result is
inferred from compilation or simulation.

## Exact custody

- branch: `experiment/rpi0-standalone-arm64-appliance`
- required executable-source head: `de0c16eb4dbe7e3558bf4d5af422c024eb55dca5`
- required executable-source tree: `3e253dff1db4c9823cfaa517bdef7f8613780d7a`
- base: `a86f03e8a5d5302d9872f995a0a5ba376a0ab6d5`
- base tree: `11a9fefbbe8b184c52058f4b4610da3a5e3b06e4`
- evidence head/tree: the draft PR head reported in the PR body

The pre-existing working checkout contained unrelated untracked `handoffs/`
material. It was left untouched. Work was performed in an isolated worktree at
the exact required branch head.

## Selected lane

The one selected lane is Box64 0.4.4 commit
`2f130fab1d6e1a4ee8a71dc60cfdfcc839ad192a`, with a source-built x86-64 Wine
11.0 commit `db11d0fe6a169c457e23d007e20404643d067aa8`.
`rpi0/translation-lane.toml` fixes the build posture, x86-64 libraries, wrapped
ARM libraries, environment, and exact Box64 configuration. These pins were
verified against their upstream repositories; they were not built or executed
on a Pi in this result. FEX was not attempted because Box64 did not fail a
physical preflight; the physical preflight was unavailable.

## Implemented architecture

`lvb-arm-standalone` is a Rust Linux AArch64 executable. It owns one JACK MIDI
input and two JACK audio outputs. The JACK callback accepts only note-on,
note-off, velocity, CC64 sustain, and CC123 all-notes-off, preserves the MIDI
channel, keeps sample offsets, and counts malformed, unsupported, or overflowing
messages. It invokes the existing queued AP10/AP11 backend with its asynchronous
presentation delay; it contains no substitute synth or new Windows transport.

The callback uses fixed arrays and a fixed SPSC control queue. It performs no
heap allocation, mutex operation, filesystem access, process operation,
logging, JSON work, network work, editor operation, or synchronous Windows
wait. The supported JACK blocks are 64, 128, 256, 512, and 1024 frames at 48
kHz. The first delay is 2048 frames (42.667 ms), followed by 1024 (21.333 ms)
and conditionally 512 (10.667 ms).

Before the Windows gate opens, both processes exchange a fixed 64-byte
architecture record covering endian order, pointer width, all shared extents,
32/64-bit atomic availability, page-size observations, and a layout digest. A
mismatch refuses the session. The normal x86-64 architecture guard remains
unchanged; `RPI0_ARM_BUILD_ONLY` is the only AArch64 CMake entrance.

The translated cohort runs in `lvb-rpi0-<session>.service` with
`KillMode=control-group`. Identity binds the unit, cgroup, main PID, Linux start
ticks, Box64 executable, session, module hash, class ID, and protocol session.
There is no `pkill`, process-name cleanup, or global Wine termination.

## Source-owned instrument

`LVB ARM Appliance Synth` is an original x86-64 Windows VST3 instrument with:

- processor class `5250493041524d41505053594e540001`;
- controller class `5250493041524d41505053594e540002`;
- zero audio inputs, one stereo output, and one 16-channel event input;
- fixed 16-voice, table-driven synthesis with velocity and deterministic voice
  retirement;
- per-channel sustain and all-notes-off;
- a gain parameter plus hidden bridge-control parameters;
- a 16-byte versioned opaque state;
- a real resizable Win32 child editor with a mouse-operated gain control;
- editor remove and fresh reopen without processor replacement;
- no installer, network, service, account, content, or preset dependency.

No third-party plug-in binary is committed.

## Deterministic qualification observed locally

- Rust standalone unit tests: 9 passed, 0 failed.
- Portable synth-core tests: 7 passed, 0 failed.
- AArch64 Linux Rust compile check, including JACK and the full executable: passed.
- C++ cross-architecture layout assertions: passed on the local ARM64 compiler.
- Linux probe logic compiled and self-tested on the local ARM64 macOS host;
  the required x86-64 Linux artifact and translated execution remain CI/physical work.
- Existing backend baseline on this sandbox: 52 passed, 16 failed, 1 ignored;
  every observed failure was a local sandbox socket `EPERM`. The same class was
  observed in the native-audio-client baseline (10 passed, 1 `EPERM` failure).

The draft PR runs native AArch64 and x86-64 Windows jobs. Their results must be
reported separately from these local observations. Cross-compilation is not a
physical ARM claim.

## Physical sequence

The required 23-step sequence from the implementation request was not started.
Consequently there is no MIDI-controller identity, audio route, audible output,
mouse acceptance, state acceptance, service distribution, gap count, deadline
count, CPU/memory/thermal observation, throttling result, cleanup receipt, or
second-session restart receipt.

The exact pending sequence is: start the host; establish Box64/Wine and the
fixture; connect physical MIDI and stereo audio; play two pitches at distinct
velocities; prove audible and measured stereo output; hold/release; exercise
sustain and release; exercise all-notes-off; open the real editor; change gain
with the mouse and prove output change; close while playing; reopen on the same
DSP instance; save; change; restore and verify; stop; verify cohort/cgroup/maps/
editor absent; repeat a fresh playable session; stop and verify cleanup again.

## Preserved gaps and nonclaims

- The selected Box64/Wine lane has not passed any of its four physical Pi
  preflight checks.
- The Windows fixture and Windows host require the draft PR Windows job before
  their build/test result can be called observed.
- No Pi hardware, OS image, kernel, firmware, page size, storage, cooling,
  temperature, governor, clock, JACK/PipeWire graph, editor display, CPU use, or
  memory peak has been observed.
- No 2048/1024/512 physical delay campaign has run.
- No DAW, Pigments, commercial plug-in, vendor installer, or authorization
  system was used.
- There is no universal ARM, Snapdragon, production hardware, or low-latency
  claim.

Because RPI0 has not passed, there is no RPI1 Pigments recommendation.
