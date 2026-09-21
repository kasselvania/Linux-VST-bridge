# RPI0 standalone ARM appliance result

## Disposition

- deterministic implementation: **PASSED**
- Box64/Wine physical Pi preflight: **PASSED ON THE 4 KiB KERNEL**
- physical MIDI/audio/editor acceptance: **PASSED**
- overall RPI0: **FUNCTIONAL ARCHITECTURE PASSED; PERFORMANCE UNQUALIFIED**

The original deterministic implementation remains preserved below. Physical execution
found a concrete state-restore defect, so the branch was reopened only for that defect.
The repaired candidate then passed deterministic qualification, translated preflight,
the complete physical MIDI/audio/editor/state/cleanup/restart sequence, and the required
256-frame JACK graph. A later USB bus-power event set historical undervoltage/throttle
bits, and memory high-water telemetry was unavailable. Those facts do not erase the
functional pass, but they prohibit performance, low-latency, polyphony, or sustained-
stability claims.

## Exact custody

- branch: `experiment/rpi0-standalone-arm64-appliance`
- required starting head: `de0c16eb4dbe7e3558bf4d5af422c024eb55dca5`
- required starting tree: `3e253dff1db4c9823cfaa517bdef7f8613780d7a`
- base: `a86f03e8a5d5302d9872f995a0a5ba376a0ab6d5`
- base tree: `11a9fefbbe8b184c52058f4b4610da3a5e3b06e4`
- executable-source head: `53fb9f2a334e2f5fbc3d3c1cf4469148fef30b45`
- executable-source tree: `fb4a662eba4ac1f8c733a41d63e5c98419832139`
- repaired physical-candidate source head:
  `791088bb31fcd75212b2df4ac2c7a6efb95182ba`
- repaired physical-candidate source tree:
  `10f102fe04ce52a5a9436481331007146b85ebba`
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

## Physical fixture

The first physical fixture was Raspberry Pi 5 with 8 GB RAM plus ShieldXL
CS4270/JACK hardware, without active cooling or overclock.
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

Physical execution later identified a concrete JACK/state/editor defect. Repaired-source
run [`35639347278`](https://github.com/kasselvania/Linux-VST-bridge/actions/runs/35639347278)
passed the current Windows fixture/editor tests (11), native AArch64 build/tests (13),
unchanged x86-64 Rust plane, and the frozen original Windows-artifact job. On the Pi,
the exact repaired AArch64 source also passed 13 tests and a JACK release build. The
original `53fb9f2` source and artifacts remain retained; the accepted repaired physical
candidate is `791088b` / tree `10f102f`.

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

That diagnostic preceded the physical campaign and remains retained as such. The later
accepted sequence used physical MIDI, audible ShieldXL output, real mouse interaction,
and an accepted 256-frame JACK graph.

## Physical acceptance result

Physical execution first exercised the preserved `53fb9f2` source. Note input, pitch,
sustain, all-notes-off, audible stereo output, real editor interaction, editor close and
reopen, and clean restart worked. State restore exposed a concrete defect: JACK was
deactivated, the three external routes disappeared, 12,288 frames were missing and
expired with seven gaps, and the already-open editor did not redraw. This was an
implementation defect, not a fixture failure.

The bounded repair produced candidate `791088b` / tree `10f102f`. It pauses callback
admission without deactivating JACK, waits for exact backend epoch acknowledgement,
corrects setup reporting, and invalidates the live Win32 editor after state changes. The
candidate's deterministic workflow and Pi-native build/tests passed before deployment.
Its exact plugin SHA-256 is `a7718bfb...0675`; its exact AArch64 standalone SHA-256 is
`e4e34d19...7c1f`.

On the final strict session (`94ba4b80261680a5ca36a7cf3c8d49ae`):

- JACK ran at 48 kHz, 256 frames, three periods; bridge presentation delay was 2,048
  frames (42.667 ms);
- the Monolit USB controller produced physical note-ons `90 30 3a` (pitch 48, velocity
  58) and `90 32 5d` (pitch 50, velocity 93), plus note-offs; sustain hold/release and
  all-notes-off passed;
- ShieldXL carried audible stereo output, and the retained returned-audio audit recorded
  274,482 nonzero samples across 30 of 32 windows, maximum RMS 0.025526 and peak
  0.043937;
- the real Win32 editor was visible over the bounded X11/VNC path; physical mouse gain
  changes affected audio, close left DSP running, and a fresh editor generation reopened
  on the same DSP instance;
- a 680-byte state was saved, changed, and restored; the already-open editor visibly
  returned to the saved value, audio returned at that value, and all MIDI/stereo JACK
  routes remained connected;
- 86,859 callbacks delivered 22,230,528 frames with zero process failures, deadline
  misses, missing frames, expired frames, or gaps. The callback maximum was 34.740
  microseconds; restore pause was explicitly counted as 1,280 frames;
- the Windows `process()` distribution covered 76,174 requests: mean 19.343 us, p50 at
  most 20 us, p95 at most 22 us, p99 at most 24 us, maximum 2.078 ms. Admission through
  publication was mean 221 us, p99 at most 352 us, maximum 2.350 ms. The correlated
  non-plugin residual was mean 201.657 us, p99 at most 320 us, maximum 609.450 us;
- editor/module/mapping cleanup passed, and the exact cohort, unit, cgroup, session
  directory, mappings, and processes were absent after stop.

The required fresh restart (`e674a1e98fc49f168f36929a0a07beaa`) again produced audible
physical-MIDI audio and stopped cleanly: 10,609 callbacks, zero failures/deadlines/
missing/expired/gaps, 37.796 us callback maximum, and exact cleanup. ShieldXL was then
returned to its normal 512-frame JACK setting with Auto-Mute off.

The bridge-delay descent also ran at the hardware contract's normal 512-frame JACK
period. The 2,048-, 1,024-, and 512-frame bridge settings all had zero missing/expired
frames, gaps, deadline misses, or process failures. Their configured bridge-only delays
were 42.667, 21.333, and 10.667 ms. The operator perceived lower delay at the smaller
settings, but that is subjective and is not an end-to-end or worst-case latency claim.

Native host point samples were approximately 4.0% of one core and 162,332 KiB RSS; the
Windows host point sample was 8.1% and 108,568 KiB RSS. The final translated cohort
contained nine processes. Cgroup memory high-water telemetry was unavailable and is not
inferred. The accepted session sampled 52.7 degrees C and 1.6 GHz; the campaign maximum
was 53.8 degrees C.

The firmware register began at `0x0`. A Monolit button woke its USB-bus-powered display,
coincident with a power jump; the register thereafter read `0x50000` (historical
undervoltage and throttling) while all current bits remained clear. Because the
historical bits are sticky, the campaign retains the operator correlation but does not
claim proof of causation or erase the event. Functional architecture passes; performance,
latency, polyphony, and sustained stability remain unqualified.

## Preserved gaps and nonclaims

- The 16 KiB Box64/Wine fault and incomplete Wine runtime staging remain preserved.
  The former missing 4 KiB ShieldXL module is a resolved, retained blocker.
- The `0x50000` historical power/throttle register and unavailable memory high-water
  prevent accepted performance, latency, polyphony, or sustained-stability claims.
- Point CPU/RSS samples and bounded timing distributions are not full resource
  distributions or worst-case guarantees.
- No DAW, Pigments, commercial plug-in, vendor installer, or authorization
  system was used.
- There is no universal ARM, Snapdragon, production hardware, or low-latency
  claim.

Physical execution identified concrete packaging, kernel-integration, and state-restore
defects. The kernel and state-restore defects are closed on the exact retained fixture;
the package staging defect remains. Detailed sanitized evidence is in
`physical-acceptance.json` and the retained native JSONL files.

## RPI1 recommendation

Proceed only as a private, user-owned follow-on: exact Pigments installation -> the
same repaired ARM64 standalone host -> the same pinned Box64/Wine lane -> physical MIDI
-> ShieldXL stereo audio -> the real mouse-operable Pigments editor -> preset/state
restore -> clean close and restart. Keep paid binaries, content, presets, account data,
credentials, activation state, and licensing payloads private and uncommitted. RPI1 must
not inherit a performance or low-latency claim from RPI0.
