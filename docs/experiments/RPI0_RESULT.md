# RPI0 standalone ARM appliance result

## Disposition

- deterministic implementation: **PASSED**
- Box64/Wine physical Pi preflight: **NOT RUN**
- physical MIDI/audio/editor acceptance: **NOT RUN**
- overall RPI0: **PENDING PHYSICAL VALIDATION**

The Pi-ready deterministic implementation passed at the preserved executable
source and tree below. The implementation environment was an Apple ARM64 host,
not a Raspberry Pi, so the physical gates were not run. That absence is not an
implementation failure. No physical result is inferred from compilation or
simulation.

## Exact custody

- branch: `experiment/rpi0-standalone-arm64-appliance`
- required starting head: `de0c16eb4dbe7e3558bf4d5af422c024eb55dca5`
- required starting tree: `3e253dff1db4c9823cfaa517bdef7f8613780d7a`
- base: `a86f03e8a5d5302d9872f995a0a5ba376a0ab6d5`
- base tree: `11a9fefbbe8b184c52058f4b4610da3a5e3b06e4`
- executable-source head: `53fb9f2a334e2f5fbc3d3c1cf4469148fef30b45`
- executable-source tree: `fb4a662eba4ac1f8c733a41d63e5c98419832139`
- evidence head/tree: the later evidence-only draft PR head reported in the
  PR body

The pre-existing working checkout contained unrelated untracked `handoffs/`
material. It was left untouched. Work was performed in an isolated worktree at
the exact required branch head.

## Provisionally selected lane

The provisionally selected lane is Box64 0.4.4 commit
`2f130fab1d6e1a4ee8a71dc60cfdfcc839ad192a`, with a source-built x86-64 Wine
11.0 commit `db11d0fe6a169c457e23d007e20404643d067aa8`.
`rpi0/translation-lane.toml` fixes the build posture, x86-64 libraries, wrapped
ARM libraries, environment, and exact Box64 configuration. These pins were
verified against their upstream repositories; they were not built or executed
on a Pi in this result. Box64/Wine selection remains provisional until its four
physical preflight checks pass. FEX was not attempted because Box64 did not
produce a concrete physical-preflight failure.

## Intended first physical fixture

The first physical fixture is Raspberry Pi 5 with 8 GB RAM plus ShieldXL
CS4270/JACK hardware. It begins without active cooling and without overclock.
The ShieldXL integration contract has not yet been supplied; the branch
therefore waits at the SHIELDXL0 authority gate before physical preflight.
Raspberry Pi 4 Model B with 4 GB or 8 GB and Compute Module 5 with at least
8 GB remain accepted targets, but they are not this first fixture.

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

## Deterministic qualification

Hosted run
[`35376913147`](https://github.com/kasselvania/Linux-VST-bridge/actions/runs/35376913147)
on the executable-source head passed:

- native Linux AArch64 standalone tests: 11 passed, 0 failed;
- native Linux AArch64 JACK release build: passed;
- cross-architecture C++ layout assertions: passed;
- portable synth core: 7 passed, 0 failed;
- x86-64 Linux preflight probe: statically cross-compiled and verified as
  ELF64/AMD x86-64, but deliberately not executed on ARM without Box64;
- x86-64 Windows fixture/editor tests: 10 passed, 0 failed;
- x86-64 Windows preflight probe: passed natively on the Windows runner;
- unchanged Linux x86-64 native-audio-client: 11 passed, 0 failed;
- unchanged Linux x86-64 backend: 69 passed, 0 failed, 1 ignored.

AP8 Windows host regression run
[`35376913041`](https://github.com/kasselvania/Linux-VST-bridge/actions/runs/35376913041)
also passed on the same source head. Hosted compilation and native Windows
execution are deterministic qualification, not Pi or translation evidence.

Local implementation-host observations:

- Rust standalone unit tests: 9 passed, 0 failed.
- Portable synth-core tests: 7 passed, 0 failed.
- AArch64 Linux Rust compile check, including JACK and the full executable: passed.
- C++ cross-architecture layout assertions: passed on the local ARM64 compiler.
- Existing backend baseline on this sandbox: 52 passed, 16 failed, 1 ignored;
  every observed failure was a local sandbox socket `EPERM`. The same class was
  observed in the native-audio-client baseline (10 passed, 1 `EPERM` failure).

The hosted x86-64 Linux probe build and all cross-compilation remain distinct
from actual Box64 execution. None is a physical ARM claim.

## Pending physical resumption sequence

The physical sequence was not started.
Consequently there is no MIDI-controller identity, audio route, audible output,
mouse acceptance, state acceptance, service distribution, gap count, deadline
count, CPU/memory/thermal observation, throttling result, cleanup receipt, or
second-session restart receipt.

After SHIELDXL0 supplies the hardware contract, resume this same branch with:

1. native AArch64 standalone startup;
2. Box64 x86-64 Linux probe;
3. Wine Windows probe;
4. exact translated-host cleanup;
5. source-owned synth at 2048 bridge frames;
6. physical MIDI and ShieldXL audio;
7. editor and state acceptance;
8. clean stop and second-session restart.

Temperature, throttling status, and observed clock state must be recorded
throughout. A functionally successful but thermally throttled run may establish
basic compatibility; it cannot establish accepted performance, latency,
polyphony, or sustained stability.

## Preserved gaps and nonclaims

- The provisionally selected Box64/Wine lane has not run any of its four physical Pi
  preflight checks.
- No Pi hardware, OS image, kernel, firmware, page size, storage, cooling,
  temperature, governor, clock, JACK/PipeWire graph, editor display, CPU use, or
  memory peak has been observed.
- No 2048/1024/512 physical delay campaign has run.
- No DAW, Pigments, commercial plug-in, vendor installer, or authorization
  system was used.
- There is no universal ARM, Snapdragon, production hardware, or low-latency
  claim.

The deterministic implementation is closed unless physical execution identifies
a concrete defect. Resume the same branch only after the SHIELDXL0 hardware
integration contract is supplied.

Because overall RPI0 remains pending physical validation, there is no RPI1
Pigments recommendation.
