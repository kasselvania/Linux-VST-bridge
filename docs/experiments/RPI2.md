# RPI2 — native ARM Wine/FEX on Pi 5

## Result

GE-Proton11-7 AArch64 with UMU 1.4.4 now runs the reference Windows VST
through the existing native bridge on the Pi: architecture handshake,
MIDI, stereo processing, and clean shutdown passed. The native supervisor
release build took 17.18 seconds; the existing Windows host and VST binaries
were reused.

The preceding account-free process launch proof needed no source build. Its host
published its environment readiness token, pumped its Windows message loop,
accepted its stop file, and exited successfully. All 14 processes sampled
in the owned test cohort were absent after shutdown.

The running host mapped `libarm64ecfex.so` and `libarm64ecfex.dll`. Its Wine
preloader, Wine server, Python launchers, and pressure-vessel processes were
native AArch64 ELF executables (machine 183). No Box64 process participated
in that observed cohort. This is direct evidence of the changed execution
topology, not a performance prediction:

```text
Native ARM UMU / Linux runtime / Wine Unix infrastructure
    └── FEX ARM64EC translation
          └── existing x86-64 Windows host
                └── reference VST → existing bridge transport → ARM JACK
```

This experiment is separate from RPI1 and the ordinary `CURRENT_SLICE.md`.
It does not declare RPI1 a failed architecture. It establishes another
runtime candidate on which the transferable bridge has now run.

## Reference bridge result

One session processed 9,152 Windows blocks over about 49 seconds, including
a five-second MIDI/audio capture. At 48 kHz, JACK used 512-frame callbacks,
Windows processed 256-frame blocks, and the bridge retained its existing
2,048-frame reserve (42.67 ms).

- Native and Windows architecture handshake bytes matched.
- The note-on, note-off, and CC123 messages were sent without errors.
- Both captured channels were identical and finite, with 73,766 nonzero
  samples each and peak amplitude 0.04535433. Initial and final silence
  were preserved; the active note measured approximately 262 Hz, consistent
  with MIDI note 60 at the measurement's 2 Hz resolution.
- All 4,576 callbacks completed without process failure, deadline miss,
  missing/expired frames, or a gap. The capture reported zero JACK xruns.
- The host containing the reference VST also mapped FEX; all 14 sampled
  cohort executables were native AArch64 ELF images.
- Close completed successfully, the session directory was removed, all
  sampled cohort PIDs disappeared, and the original JACK graph returned.

The implementation changes runner selection and supervision in the RPI0
standalone. A pinned native launcher and its pinned native leader replace
the Box64-specific fields when `runner=native` is selected. Existing Box64
configurations and their executable checks remain supported. Native cohorts
use leader identity and cgroup ownership, rather than requiring every child
to be Box64; this is process ownership, not a sandbox-security claim. Cleanup
checks cgroup population including descendants. A startup identity failure
also stops the launched unit before returning an error.

[`launch-host.sh`](../../rpi2/launch-host.sh) is a fixture adapter copied beside
the staged runner. It requires the RPI2 private prefix, checks the selected
Proton and runtime entry hashes, preserves Windows command paths, and execs
UMU under the existing supervisor. The native cohort is bounded to 90 seconds,
3 GiB memory, and 512 tasks. This does not define a distributed product runner.

The existing JACK fixture now accepts `LVB_QUALIFICATION_CLIENT` (default
`lvb-arm-pigments`) and the neutral `notes` alias. The run selected
`lvb-arm-standalone`; its note schedule, meters, and processing stayed intact.

Validation: 21 standalone Rust tests passed, including unchanged legacy
configuration parsing, native selection, mixed/unknown runner rejection,
and changed-file hash rejection. The live run verified the changed launch
path. Exact tested source hashes, configuration, native/Windows logs, audio
summary, handshakes, and cleanup are in
[`reference-bridge.json`](../../evidence/rpi2/reference-bridge.json).

The retained timing window covers 1,643 of 9,152 requests: mean Windows
processing was 11.174 microseconds and mean admission-to-publication was
249.611 microseconds. These are a partial-window description of this light
reference workload, not a complete trace or a comparison with RPI1. No DSP
or transport algorithm was changed. Pigments performance and behavior remain
separate questions.

## What ran

The fixture was Raspberry Pi 5, Debian 13.7, kernel
`6.18.50+rpt-rpi-v8`, AArch64, 4 KiB pages. The existing X11 session was
used. RPI1's audio service was inactive before and after the work; its
licensed environment was not used or changed.

All new packages, dependencies, logs, and the account-free prefix were
staged in a separate private directory. The Debian launcher packages were
extracted there; nothing was installed system-wide. Runtime updates were
disabled after UMU's first resolved installation.

| Component | Exact selection |
| --- | --- |
| GE-Proton | [GE-Proton11-7 AArch64 release](https://github.com/GloriousEggroll/proton-ge-custom/releases/tag/GE-Proton11-7), published September 16, 2026 |
| UMU | [1.4.4 Debian 13 ARM64 package](https://github.com/Open-Wine-Components/umu-launcher/releases/tag/1.4.4) |
| Steam Linux Runtime | `steamrt4-arm64`, `4.0.20260914.260627` |
| pressure-vessel | `0.20260914.0` |
| Windows probe | Existing `rpi0-windows-probe.exe`, SHA256 `86161880adb171519bfa1b263ea136c6f40dd595097891fb5436d256cf727866` |
| Windows host | Existing `wf0-factory-probe.exe`, SHA256 `6f00cf377458037e4ae64c2425578b7c408a0a5ed6f027a0249e0d979f55fc2c` |

The GE archive and launcher package matched their GitHub release SHA256
digests. UMU reported a successful runtime archive SHA256 check and runtime
mtree verification. Package hashes, sanitized logs, live process topology,
launch settings, and results are retained in
[`native-arm-runtime-observations.json`](../../evidence/rpi2/native-arm-runtime-observations.json).

## Attempts and remaining startup faults

1. `ge-hello-01`: operator-agent launch error: the required `--self-test`
   argument was omitted. Exit 64, no pass marker. The 128.690-second service
   duration included downloading the runtime and creating the prefix.
   Optional Xalia game-accessibility startup also failed with “No displays
   available.” This attempt is preserved, not counted as a pass.
2. `ge-hello-02`: correct argument, Xalia disabled. Exit 0 in 7.592 seconds,
   but no marker in captured output. The Unix executable path selected
   GE-Proton's `umu.exe` launch route. Exit success alone was insufficient.
3. `ge-hello-03`: same probe copied inside the fresh prefix and launched as
   `C:\bridge\probes\rpi0-windows-probe.exe --self-test`. Exit 0 in 7.068
   seconds and exact marker
   `RPI0_X86_64_WINDOWS_PREFLIGHT_PASS 5250493057494e36` captured.
4. `ge-host-01`: the existing host's `--environment-owner` mode, with its
   actual executable hash, wrote the matching `environment.ready` token.
   A live process snapshot confirmed native ARM execution and FEX mappings.
   After writing `environment.stop`, the cohort exited 0 and retired.
   Its 21.376-second duration includes deliberate observation time; it is
   not a startup benchmark.

Successful launches still report locale generation, a missing destination
for optional `igdext64.dll` copying, and an empty game-drive-parent error.
UMU also warns that a Windows path is not a local Unix executable before
passing it to Proton successfully. These messages remain in the logs. No
claim of a fault-free runtime is made. `PROTON_USE_XALIA=0` disables the
unneeded game-accessibility helper; it does not disable the host's Windows
message loop.

Spot temperatures during setup reached 61.5°C; the final reading was 54.3°C.
The final throttle register was `0xe0000`: historical flags remained, with
current bits clear. This short launch work is not a thermal or DSP test.

## Continuing from the staged environment

The private directory retains a small `run-ge.sh` wrapper, downloaded
packages, extracted launcher/dependencies, pinned runner/runtime, and fresh
prefix. The wrapper uses an exclusive lock, a new log/unit name, a 90-second
systemd lifetime, a 3 GiB memory bound, and cleanup confined to its cgroup.
It is a fixture helper, not a product runner implementation.

For another account-free launch, use a new label and the existing Windows
path. For example, from that private directory:

```sh
./run-ge.sh ge-hello-new 'C:\bridge\probes\rpi0-windows-probe.exe' --self-test
```

That initial launch-only boundary has been extended by the reference bridge
run above. Its configuration replaces the legacy runner fields with
`runner=native`, `launcher_path` / `launcher_sha256`, and `leader_path` /
`leader_sha256`. The other RPI0 configuration fields remain unchanged. The
staged launcher is distinct from the earlier `run-ge.sh` process-only helper;
the Rust supervisor owns its systemd unit. Use a new evidence destination for
each subsequent session and hold the private `run.lock` exclusively.

Pigments, vendor authorization, demanding-workload audio deadlines, sustained throughput,
preset navigation, state recall, ShieldXL controls, and comparative thermal
behavior remain untested on this runtime. Hangover remains an alternative;
it was not installed or executed in this step.
