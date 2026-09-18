# RPI0 appliance build and physical run

RPI0 is an isolated Linux AArch64 experiment. It does not launch a DAW and it
does not consume a commercial plug-in.

## Build planes

- Windows x86-64: configure the repository with `WF0_BUILD_ONLY=ON`, the pinned
  VST3 SDK, MSVC v143 and Windows SDK 10.0.19041.0. Build
  `wf0-factory-probe`, `rpi0-arm-appliance-synth`, `rpi0-windows-tests`, and
  `rpi0-windows-probe`.
- Linux AArch64: install JACK development headers, then configure with
  `RPI0_ARM_BUILD_ONLY=ON`. The build produces `lvb-arm-standalone`,
  `rpi0-layout-assertions`, `rpi0-linux-probe`, and `rpi0-core-tests`.
- Existing x86-64 targets retain their existing architecture guards. RPI0 does
  not widen those targets.

`rpi0/package.py` stages only explicitly supplied local artifacts and emits a
hash-complete `appliance.conf`; it performs no download. The configuration can
also be prepared from `appliance.conf.example`. The parser rejects extra or missing keys, symbolic
links, hash mismatches, an unpinned Box64/Wine identity, a sample rate other
than 48 kHz, or a bridge delay outside 512/1024/2048 frames.

## Translation preflight

Build Box64 0.4.4 at commit
`2f130fab1d6e1a4ee8a71dc60cfdfcc839ad192a` and Wine 11.0 at commit
`db11d0fe6a169c457e23d007e20404643d067aa8`. Record all resulting artifact
hashes in the configuration. Then run:

```text
lvb-arm-standalone preflight /absolute/path/appliance.conf
```

That bounded command executes the x86-64 Linux probe, the x86-64 Windows probe,
and a start/identity/natural-close cycle of the existing Windows VST host. Each
process is placed in an exact systemd user unit. Failure of any program or
failure to retire its cgroup is fatal.

## Physical session

Run `rpi0/capture_fixture.sh` before and after the campaign, retain its output,
and start with `bridge_frames=2048`:

```text
lvb-arm-standalone run /absolute/path/appliance.conf
```

The host announces the exact JACK MIDI and stereo audio ports. Route those
ports manually. Type `open`, `close`, `save /absolute/path`,
`restore /absolute/path`, `status`, or `quit` on the non-real-time control
thread. A mouse interaction with the actual Win32 gain control produces AP11
begin/value/end traffic; the value reaches audio through a fixed SPSC queue.

Repeat a complete clean session at 2048, then 1024, and only then 512 bridge
frames. Use new `evidence_path` and `windows_evidence_path` values for every
run. A successful close copies the bounded native timing report and exact
Windows unit journal outside the session directory, observes natural cgroup
retirement, and then removes every session mapping. If bridge close or cgroup
retirement fails, the session files are retained and the command fails.

No first-run overclock is permitted. Do not use Pigments, Arturia Software
Center, Native Access, another commercial plug-in, or a DAW in RPI0.
