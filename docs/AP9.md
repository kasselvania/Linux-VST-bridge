# AP9 — Bridge latency and operating limits

AP9 is an incremental candidate on the reviewed AP8 basis `938791e42e6a6fde1f44ab07e9f1d4edc2357ded`. AP8's implementation and original evidence remain in history and in this branch; PR #67 is unchanged. This result belongs to issue #68 and remains subject to review, unmerged.

**Result:** the default added delay is halved, from 1024 to 512 frames (21.333 to 10.667 ms at 48 kHz). The real reference effect confirms exact sample displacement. A 128-frame setting (2.667 ms, 87.5% below baseline) passed the selected light SDK workload, but the normal Bitwig session exposed gaps at both 128 and 256. **Use 512 for the tested desktop fixture; treat 128/256 as workload-specific experimental choices.** The measured Windows IPC change reduces service cost independently of presentation delay.

## What changed

The native proxy negotiates the actual sample rate, float32 precision, process mode and transport maximum with the Windows SDK host while inactive. The Windows owner calls `setupProcessing`, then queries the plug-in's latency, tail and float32/float64 support. The proxy reports **bridge presentation delay + vendor algorithmic delay**. It does not add measured request service time to that sum. These follow the SDK's [setup contract](https://steinbergmedia.github.io/vst3_doc/vstinterfaces/structSteinberg_1_1Vst_1_1ProcessSetup.html) and [latency contract](https://steinbergmedia.github.io/vst3_doc/vstinterfaces/classSteinberg_1_1Vst_1_1IAudioProcessor.html).

Protocol minors 6 (reference) and 7 (commercial) add inactive Configure/Configured messages and a correlated Windows process-duration field to Done. Minors 1–5 and the original 256-frame shared-memory layout remain intact. Configure carries a bounded 24-byte payload; Configured returns 16 bytes. The process reply adds an unsigned nanosecond duration, never a Windows absolute timestamp.

Host blocks up to 1024 frames use ordered chunks of at most 256. Every event is validated against the full host block before admission, copied into exactly one chunk, and given a chunk-relative offset. Audio is copied before replacing in-place output. State/control barriers still follow admitted audio. Recovery retains the protocol, setup, identity and presentation delay. There is no callback wait for Windows, no silent resampling or float64-to-float32 conversion, and no silent change in delay during playback.

A demonstrated IPC cost was removed: the old Windows path called Winsock `select` before each read/write even when ready, and peeked before reading every command header. The candidate attempts bounded nonblocking I/O first, waits only after `WSAEWOULDBLOCK`, reads an available header directly, and enables TCP_NODELAY on both peers. Partial frames retain one deadline; healthy idle sessions, EOF and stalled replies retain their distinct outcomes. The five socket failure/idle tests run against both paths.

## Settings

The fixture currently reads a private text file, `$HOME/AP9-Performance/delay-frames`, during setup. Accepted values are 64, 128, 256, 512, 1024 and 2048. Explicit values shorter than one host block are refused. Change the setting with the instance inactive, then perform normal host reconfiguration or reopen it. The setting does not alter saved plug-in identity or opaque vendor state.

Without a file, the default is 512 frames, increased to the next power of two covering a larger requested host block. The 1024-frame setting remains available for reproduction. A file must be a regular, owner-private file of at most 16 bytes; symlinks, malformed values and unsupported settings are rejected. The native call result and setup records report the actual choice.

At 48 kHz, 128 frames are 2.667 ms, 256 are 5.333 ms, 512 are 10.667 ms and the old 1024-frame baseline is 21.333 ms. One asynchronous host block is a causal floor: a callback cannot consume output for the input it has just submitted without waiting for Windows. Reducing the callback's return time alone cannot remove that floor.

The selected fixture choices and measurements are below. These are short observed operating points, not a general reliability guarantee. `voices` in the test arguments means **simultaneously submitted notes**, not proof of the vendor's internal voice allocation or polyphony limit.

| Case | kHz / host block / bridge frames | Notes | Seconds | Service p99 bound / max ms | Windows max ms | Native / owner CPU % | Missing frames / gaps |
|---|---:|---:|---:|---:|---:|---:|---:|
| `balanced-serum-eight-256` | 48 / 128 / 256 | 8 | 10.00 | 1.152 / 3.251 | 2.670 | 6.9 / 119.2 | 0 / 0 |
| `baseline-reference-complete` | 48 / 256 / 1024 | effect | 12.00 | 0.960 / 2.367 | 0.043 | 6.0 / 111.9 | 0 / 0 |
| `baseline-serum-heavy-parserfixed` | 48 / 256 / 1024 | 32 | 12.00 | 1.920 / 10.019 | 9.178 | 5.7 / 114.0 | 0 / 0 |
| `baseline-serum-light-parserfixed` | 48 / 256 / 1024 | 1 | 12.00 | 1.280 / 1.300 | 0.452 | 5.9 / 108.8 | 0 / 0 |
| `boundary-reference-64` | 48 / 64 / 64 | effect | 8.00 | 0.640 / 1.155 | 0.017 | 8.2 / 124.3 | 64 / 1 |
| `fast-reference-128` | 48 / 128 / 128 | effect | 12.00 | 0.640 / 1.026 | 0.013 | 7.1 / 111.9 | 0 / 0 |
| `fast-serum-heavy-128` | 48 / 128 / 128 | 32 | 12.00 | 1.152 / 9.049 | 8.555 | 6.9 / 121.2 | 1920 / 6 |
| `fast-serum-light-128` | 48 / 128 / 128 | 1 | 12.00 | 0.768 / 1.456 | 0.373 | 6.9 / 112.1 | 0 / 0 |
| `headroom-serum-heavy-512` | 48 / 128 / 512 | 32 | 12.00 | 1.280 / 7.519 | 6.889 | 6.9 / 118.5 | 0 / 0 |
| `large-reference-1024` | 48 / 1024 / 1024 | effect | 8.00 | 1.536 / 2.239 | 0.010 | 5.5 / 108.6 | 0 / 0 |
| `large-serum-heavy-1024` | 48 / 1024 / 1024 | 32 | 8.00 | 6.656 / 11.209 | 8.103 | 5.3 / 109.8 | 0 / 0 |
| `optimized-reference-1024` | 48 / 256 / 1024 | effect | 12.00 | 0.768 / 1.147 | 0.135 | 6.1 / 109.2 | 0 / 0 |
| `optimized-serum-heavy-1024` | 48 / 256 / 1024 | 32 | 12.00 | 1.664 / 6.819 | 6.224 | 5.9 / 112.0 | 0 / 0 |
| `optimized-serum-light-1024` | 48 / 256 / 1024 | 1 | 12.00 | 0.896 / 1.508 | 0.402 | 6.2 / 110.6 | 0 / 0 |
| `rate-reference-44100` | 44.1 / 128 / 128 | effect | 8.00 | 0.704 / 0.805 | 0.019 | 6.8 / 111.2 | 0 / 0 |
| `rate-reference-96000` | 96 / 256 / 256 | effect | 8.00 | 0.640 / 1.946 | 0.011 | 7.4 / 116.6 | 0 / 0 |
| `rate-serum-heavy-96000` | 96 / 256 / 1024 | 32 | 8.00 | 1.664 / 8.592 | 7.792 | 6.9 / 123.8 | 0 / 0 |
| `reviewed-reference-128` | 48 / 128 / 128 | effect | 8.00 | 0.640 / 2.292 | 0.029 | 7.4 / 120.5 | 0 / 0 |

All completed SDK rows have zero terminal faults, callback rejections, callback deadline misses, host scheduling misses and callback audit effects. They close and round-trip actual component state. Gap rows remain performance failures. The 64-frame reference row has 120 unequal channel samples corresponding to its missing block; zero-valued expected samples do not count as unequal. Every gap-free reference row compares all output channel samples with zero error.

The matched 12-second comparisons at 48 kHz, 256-frame host blocks and **unchanged 1024-frame presentation delay** isolate the service improvement:

| Workload | Mean service before → after ms | Mean Windows process before → after ms | Mean correlated non-plug-in residual before → after ms |
|---|---:|---:|---:|
| Real AGain | 0.679 → 0.499 (26.4% lower) | 0.001675 → 0.001667 | 0.677 → 0.498 |
| Serum, one note | 0.794 → 0.604 (23.9% lower) | 0.120 → 0.122 | 0.674 → 0.482 |
| Serum, 32 notes + automation | 1.106 → 0.893 (19.3% lower) | 0.409 → 0.403 | 0.697 → 0.489 |

These are service improvements, not extra milliseconds to subtract from or add to the declared delay. The shorter configuration changes actual presentation displacement; the reference oracle proves that independently. The Windows process times remain nearly unchanged in the matched means. Light-Serum maximum service increased from 1.300 to 1.508 ms despite improved mean and p99, so this is not a claim that every tail improved.


## Measurement boundaries

The SDK host uses the real Windows AGain effect and an independently generated changing-gain reference. It compares every returned channel sample at the reported sample displacement. DAW compensation and audio devices are absent from this oracle. Serum uses its initialized patch, a one-note or 32-note chord every two seconds with a one-second hold/release, and real MainVol changes. The heavier workload changes MainVol twice per host block. An additional eight-note workload brackets the balanced setting. Serum's waveform and envelope are not used as a transport-delay oracle.

Linux timestamps cover admission, worker start, mapping preparation, send, reply, validation and publication. Windows times only `IAudioProcessor::process` using elapsed time on its own monotonic clock. The non-plug-in residual is calculated for each correlated request before aggregation. No unrelated absolute clocks or independently computed percentiles are subtracted. Histograms have eight buckets per octave: percentile fields are **bucket upper bounds**, while maxima and means retain directly measured durations.

The optional observer runs after publication on its own thread and has a bounded queue. Commercial timing begins at the first note-on and includes subsequent active and release blocks; it cannot be exhausted by pre-note desktop silence. Eight worst correlated requests and bounded gap traces are retained. `offered`, dropped jobs, unchecked samples and lost trace coverage stay visible. Returned-audio windows retain the first 32 active 100 ms bins; they do not claim whole-session coverage. The independent SDK reference oracle, rather than the optional state-seeded reference observer, provides the exact alignment comparison here.

Callback deadline misses, host scheduling misses, terminal faults, missing presentation frames and expired output are separate counts. A process may close cleanly after audible gaps; clean shutdown is not a latency pass. A last completed block discarded after stop may have a Windows duration but no publication duration, so stage sample counts can differ. Earlier measurement builds counted that missing publication in `negative_correlated_residuals`; final code separates `ap9_unpublished` from actual negative residuals. Original numerical records retain their original field values and the evidence describes this accounting repair.

Native CPU is `getrusage` around the audio interval. Owner CPU is the systemd cgroup delta around the host's begin/end markers and includes the Python supervisor, Proton services, Wine and the Windows plug-in process. It excludes the native SDK host. Cgroup memory includes cached pages; native `maxrss` is a process high-water mark. A small `/proc` snapshot identifies CPU by owned process name; PIDs and private command lines are omitted from published evidence. No priority changes were used. The existing Moonlight session remained connected for control.

## Observed limits

The 48 kHz/128-frame reference run delivered exact alignment with zero gaps. Light Serum also passed at 128 added frames. The 32-note workload at the same delay missed 1,920 frames across six contiguous gaps, despite zero callback deadline misses and no terminal failure. The worst correlated chord request took 9.049 ms, of which 8.555 ms was inside Serum. Later requests queued behind that call. Changing bridge scheduling cannot turn that measured vendor computation into a 2.667 ms guarantee. The 512-frame headroom setting passed the same heavier workload.

At 64 added frames, the 48 kHz reference test missed one 64-frame block in eight seconds. It is an experimental boundary, not a recommended reliable mode. Larger host blocks also set a larger causal floor even though the Windows transport still uses 256-frame chunks.

CPU sampling at 48 kHz/128-frame blocks found substantial fixture costs outside DSP: about half a CPU core in the development Python owner, plus Wine server, device and Xalia services. These are included in the cgroup cost; they are not attributed to Serum's process duration or presented as production-manager efficiency. No supervisor protections or runner services were disabled to improve the benchmark.

The real AGain reports float32 and float64 capability; Serum 2.0.18 reports float32 support and rejects float64. The bridge implements float32 only and returns unsupported from its native float64 interface. For AGain this is a bridge implementation limit; for this Serum build the vendor also rejects it. Rates outside the explicit setup range and unsupported modes are refused. 88.2 and 192 kHz are admitted configuration values but remain unmeasured. The 44.1/96 kHz and 1024-frame rows are SDK-host results; desktop checks stayed at 48 kHz.

## Bitwig and local audio

Desktop results are recorded separately from the SDK measurements. Bitwig is launched through the normal Applications list. Test projects are copies; original AP8 projects, settings and evidence are protected. The local audio route and actual requested host setup must agree with the retained records. Moonlight is never the audio-latency oracle.

Three copied-project runs used the retained Serum patch and repeating note clip at 110 BPM, with MainVol restored to 0.1150. Playback meters and numerical local speaker-monitor samples showed output; the project was saved and closed cleanly each time. The actual native/Windows setup and PipeWire graph agree:

| Added frames / ms | Actual host & graph block | Total processed seconds / visible playback seconds | Service p99 bound / max ms | Missing frames / gaps | Result |
|---|---:|---:|---:|---:|---|
| 512 / 10.667 | 256 | 76.30 / 44.78 | 1.280 / 6.371 | 0 / 0 | Recommended desktop setting for this fixture |
| 128 / 2.667 | 128 | 57.37 / 27.50 | 1.152 / 7.936 | 1152 / 7 | Too little allowance in this desktop session |
| 256 / 5.333 | 128 | 85.19 / 57.23 | 1.152 / 10.766 | 512 / 1 | Improved allowance; still a gap, not qualified gap-free |

Counts cover the complete component lifetime, including Bitwig's stopped audio callbacks; timing starts at the first note and includes subsequent silence/release. All three runs retain zero terminal faults, callback rejections and discontinuities, with positive owned-process containment and empty session directories after close. SDK callback deadline/audit instrumentation was not injected into normally launched Bitwig.

The slowest 128-frame desktop request spent only 0.139 ms of 7.936 ms in Serum. The 256-frame run's worst request spent 0.067 ms of 10.766 ms in Serum; 10.559 ms was in the send-complete-to-reply interval. This is a different limit from the heavy SDK chord's long vendor process call. The desktop stall lies in IPC, host work outside the measured process call, or scheduling; its precise underlying OS/runner cause remains unresolved. Several later callbacks arrived with less wall-clock separation than one nominal block, further reducing the available service time. The retained correlated gap traces show that distinction.

PipeWire graph ports used float32 at 48 kHz. The local built-in speaker sink reported S16LE at 48 kHz, ALSA period size 1024 and headroom 1024; those are device-path properties, not bridge sample precision or a measured round trip. Each monitor window retained 950,272 channel samples (about 9.90 seconds of stereo audio from a 12-second capture process including startup), with nonzero output. Monitor audio was reduced to numerical summaries and deleted privately.

At the selected snapshots, the speaker node error counter was 3 throughout; the Bitwig node counter was 0, 0 and 1 respectively. These are cumulative snapshots, not proof of a zero-error device path. The owner cohort used approximately 119%, 132% and 130% of one CPU core during the respective 12-second monitor windows; native DAW CPU was not separately instrumented.

To request 128-frame local operation, the existing PipeWire minimum quantum was temporarily changed from 256 to 128. After testing it was restored to 256, with rate 48000, normal quantum 512 and forced rate/quantum still zero. Bitwig preferences and scanner cache were restored from their backups; the temporary published proxy was moved into private AP9 results. AP9 owners are stopped, sessions empty, the retained Serum scanner/owner marker restored byte-for-byte, and the original AP8 project hash unchanged. Private AP9 projects and prepared test/build artifacts remain available for review. See [desktop records](../evidence/AP9/desktop.json) and [cleanup readback](../evidence/AP9/provenance.json).


Hardware converter round trip and controller-to-speaker latency are unmeasured. Software capture, if used, excludes converters. No claim of general Linux compatibility, vendor authorization, multi-commercial support, GUI embedding, reboot persistence, dynamic vendor-latency-change handling or consumer-ready deployment follows from these runs.

## Reproduction and provenance

Build with the existing `tools/ap8_build_native.py --performance` lane and the maintainer's private descriptor. `tools/ap9_benchmark.py` runs one bounded SDK comparison against already prepared AP9 reference/commercial owners. For example, on the declared Deck with the corresponding build receipt:

```sh
python3 tools/ap9_benchmark.py --name serum-fast --role serum --build BUILD_SHA256 --rate 48000 --block 128 --delay 128 --seconds 12 --voices 1
```

The helper does not install a plug-in, create a vendor environment or launch a DAW. Reuse the lawful installed module and exact pinned runner; private vendor files and state payloads are never committed. The prepared Serum environment is retained across tests, preserving its existing machine identity.

[Machine-readable measurements](../evidence/AP9/measurements.json) retain original case identities and counters. [Build and fixture identities](../evidence/AP9/provenance.json) identify the independently built Windows/native binaries. Initial compile, parser and state-routing failures remain recorded separately; no failed attempt is relabeled as a performance pass. Desktop preparation and cleanup receipts retain hashes rather than user paths or settings contents.

## Validation and review limits

The final backend passes 32 tests and strict all-target Clippy; the native client passes 9 tests. The pinned native SDK build and Windows MSVC build pass, including legacy/eager socket deadline, partial-frame, EOF and healthy-idle tests. Tests exercise larger-block event offsets, invalid admission, setup bounds/precision, real socket setup/state exchange and malformed replies. A regression first reproduced the missing retained setup, then passed after the fix. The final source preserves successful configuration for the existing recovery path; a new commercial crash/recovery acceptance run was not performed.

Provenance identifies every measured build. The matched light-Serum and 256-frame desktop checks ran the final native source. Earlier valid measurements remain attached to their original sources: final review repairs affected setup retention and malformed-reply/accounting behavior, not successful steady-state DSP, queue depth or IPC. This PR is stacked on AP8 while #67 remains unmerged.
