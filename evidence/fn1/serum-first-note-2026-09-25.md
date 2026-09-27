# FN1 — Pi Serum default-state first-note attribution

This is an instrumented Raspberry Pi 5 source-candidate result, not a musical
qualification or a repair. The named fixture is the captured Serum 2.1.5
**default state**, not the earlier unidentified preset. The normal installed
Pigments service and all earlier Pi candidates stayed unchanged. Each owned
test cohort retired; no test service or JACK client remained active.

## Exact source and environment

FN1 starts from merged PI-R `main` `25a6cfc3824164d4fdde79abda758ffdca636998`
(tree `ed16e8d971dda8c9b844a4235d94dd1a58e14f57`). The diagnostic source
tested here is `39a4cdddca21fe6f3416f79d98e3a82e7fc1899a` (tree
`69a6eb9fc647d164a2bc8958b7de12384865e7cb`). Its Pi ARM executable is
`d56daa0787b00c4f558a8347743c6810b33bc9ad1e6ae62e9b6bb828f3939e98`,
built with Rust/Cargo 1.98.1, `--release --locked --features jack-runtime` and
the existing thin-LTO, one-codegen-unit, panic-abort profile.

The production Windows host binary is
`b41eb3696e49f26ae715ac0656d9c0ced1383b3824008d998117f4dfbe404f24`.
It passed the AP8 Windows 2022/MSVC v143/SDK 10.0.19041 workflow at both PR
test-merge commits `8f9cd1f52817a10d3b4497eb1e98016bb14978eb` and
`c715bb7bb8b28dfba2f3022b3b741a6d5350b906`; the executable bytes were
identical. The first full FN1 capture declared the former source manifest,
`16e05395e34aa783aafe2b2db513c171f166d541cb85ece2a78500189309bd19`.
The JIT captures declared the latter,
`d2d4b1850b2f6c1fef090e689df8260fb846ecd5a195bc3aed1bfb0c583739be`.
The native-results and proof-policy checks also passed at the final source.

All MIDI runs restored the same private 44,280-byte state SHA-256
`41fa2e07500a7a8d46f9d833353f5a31cf4eb38117ea1fa5071ff607bf8a90ab`
against the same Serum 2.1.5 module SHA-256
`501e7bb3dd9cafe416b3412df3d4e084c01b7468201e5690b9009d7ecd4e5283`.
The returned state length matched. The four readbacks after restoration were
Main Vol 0.5, Filter 1 On 0, Filter 1 Freq 0.5, and Macro 1 0. The editor
stayed closed. The pinned runtime remained
`ge-proton11-7-aarch64-umu1.4.4-slr4.0.20260914.260627`, with JACK
48 kHz/512, protocol 12/map v1, a 256-frame quantum, 512-frame reserve,
ondemand governor, and normal scheduling and affinity. No vendor setting,
runtime binary, timeout, queue, reserve, recovery policy or installed selection
changed. `LVB_AP10_TRACE=1` enabled bounded diagnostic capture; the later
captures also enabled FEX **library** JIT naming. Block-level JIT naming was
not used. These diagnostic runs are not release-performance comparisons.

The existing source-owned two-note fixture began at frame 48,000 and ran for
six seconds: six note-ons, five note-offs at termination, zero MIDI-write
failures. The first request carried pitch 60/channel 0, and the later request
carried pitch 64/channel 0. The first full capture bound the slow call to epoch
1, sequence 201, source position 50,944, event offset 128 and note identity 1;
the later note was sequence 389, position 99,072, identity 2. The native
admission, worker start, prepare, send, reply, validation and publication
instants and the Windows process bracket were retained for both notes.

## Physical observations

| Owned session | Diagnostic posture | First-note Windows `process()` | Audio result | Retirement |
|---|---|---:|---|---|
| Existing verified PI-R host preflight | Native note identity and original total caller-CPU words | 32.62 ms | 1,280 first-note frames missing; 2,304 total missing | clean |
| FN1 host full capture | Separate caller/process user/kernel counters and Linux task samples | 32.89 ms | same first-note gap and 2,304 total missing | clean |
| FEX naming preflight | No MIDI; confirmed library map emitted under Wine process ID | not applicable | no audio claim | clean |
| FEX profiling, initial clock | Library map and `perf` samples; clock could not be aligned | 33.92 ms | same first-note gap and 2,304 total missing | clean |
| FEX profiling, aligned clock | `perf record -k mono` on exact Linux `lvb-audio` task | 33.83 ms | same first-note gap and 2,304 total missing | clean |
| FEX map-growth capture | 750 external map/task samples, median 1.20 ms spacing | 34.32 ms | same first-note gap and 2,304 total missing | clean |

The initial profiling data remains private, but its samples were **not** used
for time-window attribution because its clock was ambiguous. The aligned
capture is the one used below. Every MIDI run delivered nonzero output,
reported zero JACK xruns and zero bridge faults, and retired its exact Windows
cohort. The 2,304 missing frames comprise a 1,024-frame startup gap and five
256-frame chunks at the first note, positions 50,944–51,968 in the full and
map-growth captures. The first-note loss is 1,280 frames, or 26.67 ms at
48 kHz. A 512-frame reserve covers 10.67 ms.

In the full capture the first request's native admission-to-publication time
was 33.83 ms. Its stage durations were 0.445 ms queue wait, 0.002 ms
preparation, less than 0.001 ms send, 33.364 ms send-to-reply, 0.008 ms
validation and 0.007 ms publication. The Windows call took 32.89 ms. The
later note-bearing request's Windows call was about 1.73 ms. The Windows
`GetThreadTimes` reading for the first call was 20 ms user plus 10 ms kernel,
and the host-process reading was 30 ms user plus 10 ms kernel. Those Wine API
counters have coarse increments; `QueryThreadCycleTime` returned unavailable.
They are supporting evidence, not a serial CPU sum.

The Linux task samples in the same full first-note bracket showed about
34.1 ms of `lvb-audio` run time, 0.23 ms runnable wait, 155 minor faults and
zero major faults. In the map-growth bracket, the caller accumulated 35.423 ms
run time and 0.075 ms runnable wait over a 36.299 ms sampled interval, with
199 minor and zero major faults. The caller was observed running through the
long span; no worker dependency or large scheduling delay is needed to account
for it. The sampled cgroup reported about 30.3 ms usage during the full-run
bracket. Its `cpu.stat` exposed no throttling fields, so throttling was not
quantified. Scheduler tracepoints were inaccessible to the ordinary SSH user;
the retained `/proc` task state and `schedstat` samples are the Linux timing
authority here.

The installed ARM64EC FEX build emitted a library-named perf map, but named it
for its Wine process ID rather than the Linux host PID. A private copy was
temporarily associated with the exact Linux PID for offline `perf script`
symbolization; that temporary copy was removed afterward. In the aligned
first-note native send-to-reply window (34.514 ms), all **13** captured
user-space cycle samples landed in `libarm64ecfex.dll`. None landed in a named
Serum guest-code region. The DLL lacks usable symbols in this installation,
so the samples identify the ARM64EC FEX boundary, not an internal function.
One later note window contained one FEX-boundary sample; that count is too
small to compare distributions.

The decisive map observation is separate from those samples. In the
map-growth run the first request's native send-to-reply bracket was 34.802 ms,
and Windows reported 34.322 ms inside `processor.process()`. The JIT map grew
**12,005 bytes in three steps**, 10.337, 18.542 and 27.661 ms after native
send. The new complete map lines named **116 Serum guest regions** and **7
bridge-host regions**. Because the non-process part of the entire reply
bracket was under 0.5 ms, all three growth steps fell within the Windows
process call regardless of the unknown cross-clock offset. The full private
map SHA-256 is
`084702b11f18ea4e595bce6739a16c32b086a74caff77bf88f13eda162564795`;
the sampled growth record SHA-256 is
`dba1c1222055b8f65fa741d1b94d7a41d8e239ea1e335e19c574f7b4073762b8`.
The aligned `perf.data` SHA-256 is
`466606d04347e7d27277a6b84ac5ae4fffd860346fcaf45edd2be3b0a6015fbd`.
All raw maps, task rows, perf data, proprietary paths, state and binaries remain
private on the Pi.

## Classification and limit

The repeatable first-note overrun is a **caller-executed FEX ARM64EC cold
translation/JIT path on this exact Serum state**. The caller was running,
library-level samples sat in the FEX boundary, and Serum guest-code map entries
appeared during the same call. The map growth and faults make code generation
the supported mechanism; they do not prove how much of the 34 ms was spent in
each FEX routine, which kernel operations caused the faults, or that the same
cause explains older unidentified presets or other plug-ins. The sampling and
map instrumentation may perturb time, so their timings are not performance
claims. The audible effect was not separately assessed; the delivered-frame
gap remains.

The next engineering action is a **separate state-safe prewarm experiment**:
restore the exact state, activate with muted output, send one bounded note-on
and note-off, stop and deactivate, restore the same state again, verify its
bytes and control readbacks, then activate for the first user note. Compare
that note against this named fixture for gap, state identity, subsequent audio,
stuck notes and clean retirement. Do not fold that lifecycle change into FN1
or treat BR1 restart policy as a cold-path solution.
