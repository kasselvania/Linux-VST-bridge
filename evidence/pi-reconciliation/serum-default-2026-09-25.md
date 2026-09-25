# Pi current-main reconciliation and matched storage comparison

This is a staged Raspberry Pi 5 standalone result, not a support qualification or
an exact replay of the earlier unnamed Serum preset. The normal installed
`lvb-rpi1-audio-gate.service` was inactive and remained unchanged. Its selected
Pigments executable SHA-256 is `4d69b1c8167f93882be222df26750ebe772eb9fbe872b1775e29c2f215e05a80`;
its config SHA-256 is `c9824beddc7e58f09290b7ad52e743e0ed061d401096fadcb8cf17f8c09d1a92`.
The separately staged Serum configuration and earlier host
(`4ab203ab8aa25555eebce6782cd52a11643a1b935abe8a2ce0cb04baeb453161`)
were also preserved. No service, normal binary, selected config, vendor module,
authorization, or state was replaced.
The normal Pigments selection is protocol 12 at 48 kHz with a 2,048-frame
bridge reserve, Pigments 7.0.1.6772 module SHA-256
`bdc91ebef8e5b486c8f998f1eef6a99626dd5a0b46d986263eeb1980b96a3c07`,
Windows host SHA-256 `64d629e84a0fdf8833e97b9393cd41b9a5a214f82fe418d1792281622f0007f2`,
source manifest SHA-256 `b96f536c6b1164a5a4e308f1d196c27fb5dcad77e194e84546f13a6b36293ce2`,
and pinned Box64/Proton runtime `proton-11.0-2c-25118279-slr4-4.0.20260805.254769`
(Box64 source `2f130fab1d6e1a4ee8a71dc60cfdfcc839ad192a`). Its configuration
does not declare a map version or source commit, so neither is inferred.
The pre-existing staged Serum configuration instead selected the native
GE Proton/FEX path, protocol 12, 48 kHz, 512-frame reserve, and the earlier
host above. Neither pre-existing selection was an A/B arm.

## Exact comparison identities

| | A: before shared storage | B: after shared storage |
|---|---|---|
| Canonical source base | `b76c9e2270c63c78be2a51adf6562a3f838ffa7f` | `f7668b6ffcb2ae1261d121514aaaed0c9b8986c7` (merged #172) |
| Base plus identical Pi reconciliation R | `11682c0068345a00850ebb4d047a4fcc2364255b` | `17b9584e444dc5fdac307585397a04d9fd63450d` (PR #173) |
| ARM release executable SHA-256 | `65e4f7a3f54aa33c6c6223e41aed5d3e7b27bf209a0add28ea9e704609ca0834` | `438fb199f3437e752ae857a659623b53abcb76065c5ae9e744d46be5ef53583e` |
| Windows CI source commit | `11682c0068345a00850ebb4d047a4fcc2364255b` | PR test merge `97ef7be564bd23fe99eb62804cd2850cf0664359` |
| Windows host SHA-256 | `da121152e59febd3d60410b05d1bb4dce37031b29ccf7fa18a00c28b6e7bb25f` | `fa9a1ebc830c3e8a0e283cff1a4c530d8ed0f4ee85b951e49826f7ae203c899f` |
| Windows source manifest SHA-256 | `f8c1826c6e881ebb56555d1a5be82a3a61a0fd6d1e7498fba31a19538321f6a6` | `8ae164ec06d900ad762c17656bc2532c1cc3a5c6fe97f116882ce7d806c9969b` |

The same R patch was applied to both bases; the imported standalone/adapter
source files are byte-identical between A and B. R adds the ARM JACK/MIDI
frontend, private-session adapter, exact binding and Windows architecture
handshake. It imports no BR1/BR1R recovery policy. Both Windows executables
passed the same AP8 Windows 2022/MSVC v143/SDK 10.0.19041 workflow (runs
`36183264443` and `36183154611`). Both ARM builds used Rust/Cargo 1.98.1,
`--release --locked --features jack-runtime`, including the same thin LTO,
one codegen unit, panic-abort crate profile. The staged source never replaced
the normal installed selection.

The machine was Linux `6.18.50+rpt-rpi-v8` AArch64, governor `ondemand`,
JACK 48 kHz/512. Both arms used protocol 12/map v1, a 256-frame processing
quantum, 512-frame presentation reserve, one private Serum 2.1.5 module SHA-256
`501e7bb3dd9cafe416b3412df3d4e084c01b7468201e5690b9009d7ecd4e5283`,
and the same pinned `ge-proton11-7-aarch64-umu1.4.4-slr4.0.20260914.260627`
runtime/launcher (`a9e1e0b711bc05a65a731de42a1a936b62307cb05764547902191d9161e1bc14`).
The launcher allowlisted only the two exact new host filenames. The editor
remained closed; no JACK, FEX/Wine/Proton, graphics, priority, affinity,
governor, timeout, queue, vendor-quality, polyphony, multicore, or recovery
setting was changed for the comparison.

## State and workload

The retained named fixture is **Serum 2.1.5 default state**, captured on the
runnable A baseline, 44,280 bytes, SHA-256
`41fa2e07500a7a8d46f9d833353f5a31cf4eb38117ea1fa5071ff607bf8a90ab`.
It is not the old unnamed failed preset. Both arms restored that exact file
before activation. A separate restore/capture on each arm reproduced its bytes
and digest exactly. The four exposed control readbacks after restoration were
Main Vol 0.5, Filter 1 On 0, Filter 1 Freq 0.5, and Macro 1 0. The serialized
state and these controls are verified; opaque runtime-only vendor settings
were not separately exposed.

The source-owned `tools/pi/jack_note_fixture.c` (SHA-256
`9df4d76f8a5af442949f814a857dde041eed16623f00612289f09171e204f163`)
connected directly to the standalone JACK MIDI port. Each matched run used
30 seconds of a 48-kHz, two-note, two-second loop; the first note started at
fixture frame 48,000. Each produced 30 note-ons, 29 note-offs at termination,
and zero MIDI-write failures. At 15 seconds the native control path queued
Macro 1 = 0.5. There was no preset-browser transition because this controlled
fixture does not expose one. The one-time A setup run, the initial B handshake
failure before R was complete, and all six matched sessions are retained as
separate outcomes; no favorable warm interval was selected in place of the
attack or startup frames. The initial B attempt announced Windows readiness
then correctly refused because the current-main host lacked the Pi layout
handshake. That missing handshake was repaired in R before either matched arm.

## Matched release-path result

Every row completed 5,445 measured requests, 2,820 JACK callbacks, 5,640
256-frame processing chunks, 1,443,840 processed frames, and 379,078 nonzero
output samples per channel. `service` is native admission-to-publication
elapsed time; `process` is the Windows-reported processing span, not vendor
thread CPU. The CPU columns are independent 12-second wall windows: native
process user+system CPU and the supervised Windows cohort's cgroup CPU.
They must not be summed as serial elapsed time.

| Arm/run | Service mean / p99 bucket / max | Queue mean / max | Prepare mean | Windows process mean / max | Delivered / missing frames | Native / Windows CPU seconds |
|---|---:|---:|---:|---:|---:|---:|
| A1 | 561.84 µs / ≤1,152 µs / 34.65 ms | 233.95 µs / 24.41 ms | 3.44 µs | 179.49 µs / 34.16 ms | 1,441,024 / 2,304 | 0.55 / 1.788 |
| B1 | 544.99 µs / ≤1,152 µs / 34.55 ms | 227.33 µs / 24.31 ms | 2.08 µs | 177.84 µs / 34.07 ms | 1,441,536 / 1,792 | 0.56 / 1.791 |
| A2 | 565.69 µs / ≤1,152 µs / 34.37 ms | 234.66 µs / 24.09 ms | 3.65 µs | 180.43 µs / 33.84 ms | 1,441,536 / 1,792 | 0.56 / 1.778 |
| B2 | 548.03 µs / ≤1,152 µs / 34.50 ms | 228.51 µs / 24.26 ms | 2.08 µs | 178.49 µs / 34.01 ms | 1,441,024 / 2,304 | 0.56 / 1.804 |
| A3 | 568.71 µs / ≤1,152 µs / 34.43 ms | 237.92 µs / 24.20 ms | 3.63 µs | 179.93 µs / 33.93 ms | 1,441,536 / 1,792 | 0.57 / 1.818 |
| B3 | 549.08 µs / ≤1,152 µs / 34.83 ms | 229.56 µs / 24.58 ms | 2.08 µs | 178.96 µs / 34.34 ms | 1,441,024 / 2,304 | 0.56 / 1.794 |

The candidate's mean request service was 17–20 µs lower in each matched pair
(three-run means: A 565.41 µs; B 547.37 µs). Mean preparation fell from
3.44–3.65 µs to 2.08 µs. These are measured release-path elapsed-time
differences for this fixture. The p99 bucket, 34–35 ms maximum service span,
Windows processing tail, and CPU ranges overlap. There is **no established CPU
gain or improved audio delivery**.

Each run had two delivery gaps: a variable startup gap of 512 or 1,024 frames
(10.7 or 21.3 ms), then the same first-note gap of 1,280 frames (26.7 ms) at
processing positions 50,432–51,456. The first-note Windows processing span
was about 34 ms in every run. Total missing frames equaled expired frames:
1,792 or 2,304 (37.3 or 48 ms), varying in both arms. No run recorded a JACK
xrun, bridge fault, unpublished completed request, or MIDI
write failure. All six sessions produced nonzero samples, retired their exact
Windows cohort, printed `PI_CLEAN_SHUTDOWN`, and left no running test unit or
JACK client. This is not an audible or usable-instrument qualification.

The retained timing histograms describe completed requests. No separate live
outstanding-age series was retained for these runs; clean retirement and zero
unpublished requests show no terminal in-flight work, but they cannot explain
a future call that never returns. The existing last-known worker stage/identity
remains the appropriate failure record. A 34 ms Windows processing span still
does not distinguish vendor execution from runnable wait, translated code,
or a vendor worker dependency.

The AS1 allocation tests in [PR #172](https://github.com/kasselvania/Linux-VST-bridge/pull/172)
establish zero bridge-owned alloc/realloc/free on the covered normal native
request path and zero `new`/`delete` in Windows result publication after setup.
The same production native allocation tests passed with the Pi feature enabled
in this branch. The release A/B did not insert an allocator tracer into the
vendor/runtime and makes no claim about their allocations or state operations.

**Next action:** isolate the repeatable first-note 34 ms Windows processing
span with a bounded stage/thread capture on this exact state and MIDI fixture,
then choose a repair from that evidence. The storage repair should not be
expanded to guess at the Serum stall.
