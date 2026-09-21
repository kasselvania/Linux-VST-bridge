# RPI0 standalone ARM appliance result

## Disposition

- deterministic implementation: **PASSED**
- Box64/Wine physical Pi preflight: **PASSED ON THE 4 KiB KERNEL**
- physical MIDI/audio/editor acceptance: **NOT RUN**
- overall RPI0: **PENDING PHYSICAL VALIDATION**

The deterministic implementation remains passed at the preserved executable
source and tree below. Physical execution subsequently passed the bounded
Box64/Wine/Windows-host preflight on a Raspberry Pi 5 using the distribution's
4 KiB kernel. It did not run the physical MIDI/audio/editor sequence, so overall
RPI0 remains pending rather than passed.

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

## Selected translation lane

The selected lane is Box64 0.4.4 commit
`2f130fab1d6e1a4ee8a71dc60cfdfcc839ad192a`, with a source-built x86-64 Wine
11.0 commit `db11d0fe6a169c457e23d007e20404643d067aa8`.
`rpi0/translation-lane.toml` fixes the build posture, x86-64 libraries, wrapped
ARM libraries, environment, and exact Box64 configuration. The pinned artifacts
were built and executed on the Pi. Box64 ran the Linux probe, Wine ran the Windows
probe, the source-owned Windows host opened and closed, and the translated cohort
retired exactly. FEX was not attempted because Box64 passed the bounded preflight
after selecting the installed 4 KiB kernel.

## Intended first physical fixture

The first physical fixture is Raspberry Pi 5 with 8 GB RAM plus ShieldXL
CS4270/JACK hardware. It begins without active cooling and without overclock.
The ShieldXL integration contract is supplied. Integration exposed a kernel
contract collision: Wine under Box64 fails on the contract's 16 KiB kernel, while
the original CS4270 module was not built for the required 4 KiB kernel. SHIELDXL0
has now extended its exact contract: the pinned module builds, loads, binds, and
survives reboot on `6.18.50+rpt-rpi-v8`, and the SHIELDXL ALSA/JACK identities return.
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

## Physical preflight observation

The observed fixture was Raspberry Pi 5 Model B revision 1.1 with 8 GB RAM,
Debian 13.7 trixie, firmware `ab8a9dde`, microSD/ext4 storage, no active
cooling, and no overclock.

On `6.18.50+rpt-rpi-2712` with 16 KiB pages:

- the x86-64 Linux probe passed through Box64;
- the exact Wine binary reported `wine-11.0`;
- the Windows probe exited 139 in `ntdll.so/signal_init_process` while accessing
  `0x7ffe1000`;
- the translated process cohort was absent after the failure.

The same distribution image already supplied `6.18.50+rpt-rpi-v8` with 4 KiB
pages. After a reversible boot selection, the same Box64/Wine and Windows
artifacts passed. The repository-owned full preflight passed in sessions
`94a328565d43c78bb7aea52a46ffae8d` and
`9933b83197accc4cb027ee3ade072db2`. Each pass established the source-owned
Windows host's exact unit, cgroup, PID/start/executable identity, natural close,
unit retirement, session cleanup, and restart after the first clean result.

The generated appliance config initially failed because `rpi0/package.py`
copied only Wine's launcher ELF without its adjacent runtime tree. The successful
physical preflight used a separately retained config pointing to the same hashed
ELF at its installed runtime path. No executable was rebuilt or substituted.

Provisioning reached an observed high of 80.7 degrees C. The firmware throttle
register remained `0x0`, and 2.4 GHz was observed under load. Those values are
provisioning observations, not performance or sustained-thermal qualification.

## 2048-frame translated source-synth diagnostic

After the SHIELDXL0 extension, the frozen executable source started the source-owned
Windows instrument at 2048 bridge frames on the live 4 KiB/SHIELDXL JACK graph. No
RPI0 executable was rebuilt. One no-MIDI session saved and restored 680 bytes of state,
ran 12,552 callbacks with zero process failures, deadline misses, missing/expired
frames, or gaps, and retired its exact unit/session cleanly.

A second explicitly non-acceptance diagnostic injected one synthetic sequencer note and
captured the standalone host's two JACK output ports. The capture contained 240,000
stereo frames at 48 kHz, 194,996 nonzero samples, normalized peak 0.0472412, and equal
left/right RMS 0.0172765. That session ran 9,936 callbacks with zero process failures,
deadline misses, missing/expired frames, gaps, malformed/unsupported MIDI, or unpublished
requests, then retired cleanly. Windows `process()` p99 was at most 24 microseconds;
admission-to-publication p99 was at most 512 microseconds. These figures are diagnostic
distributions, not worst-case or bridge-only physical-latency guarantees.

The translated cohort had nine processes and a point-in-time aggregate RSS of 611,964
KiB. A memory high-water mark and full native/translated CPU distributions were not
available and are not inferred. Temperature was observed at 53.2 degrees C, throttling
was `0x0`, and sampled clocks were 1.5 to 1.7 GHz during the diagnostic. Retained detail
is in `source-synth-2048-diagnostic.json`.

The source-synth architecture diagnostic passed, but the complete physical acceptance
sequence is still pending. No USB MIDI controller was enumerated, the SSH session had
no X11/Wayland display, and the actual JACK graph used 512-frame periods. Synthetic MIDI
and direct JACK capture do not substitute for physical MIDI, an audible ShieldXL route,
or real mouse interaction.

## Pending physical resumption sequence

Continue this same frozen executable source with:

1. connect and identify one physical USB MIDI controller;
2. set the accepted 128- or 256-frame JACK graph and start the source-owned synth at
   2048 bridge frames;
3. prove physical MIDI and audible ShieldXL stereo audio;
4. exercise the real editor and state change/restore;
5. cleanly stop and complete a second fresh accepted session.

There is still no physical MIDI-controller identity, accepted audible ShieldXL route,
mouse/editor result, changed-parameter state restore, full CPU/memory distribution, or
complete physical-session restart receipt. The diagnostic gap and deadline counts above
must not be relabeled as the missing physical campaign.

Temperature, throttling status, and observed clock state must be recorded
throughout. A functionally successful but thermally throttled run may establish
basic compatibility; it cannot establish accepted performance, latency,
polyphony, or sustained stability.

## Preserved gaps and nonclaims

- Box64/Wine and the source-owned synth passed a 4 KiB translated diagnostic, but not
  a physical MIDI/audible/editor acceptance session.
- The 16 KiB Box64/Wine fault and incomplete Wine runtime staging remain preserved.
  The former missing 4 KiB ShieldXL module is a resolved, retained blocker.
- No accepted 128/256-frame physical graph, physical MIDI, editor display, complete CPU
  distribution, or memory peak has been observed for RPI0.
- No 2048/1024/512 physical delay campaign has run.
- No DAW, Pigments, commercial plug-in, vendor installer, or authorization
  system was used.
- There is no universal ARM, Snapdragon, production hardware, or low-latency
  claim.

Physical execution identified concrete packaging and kernel-integration defects.
The kernel integration defect is now closed in SHIELDXL0; the packaging defect remains.
The frozen executable source remains preserved and should not be reopened unless the
remaining physical execution identifies a concrete defect.

Because overall RPI0 remains pending physical validation, there is no RPI1
Pigments recommendation.
