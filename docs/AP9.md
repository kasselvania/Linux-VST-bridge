# AP9 — Bridge latency and operating limits

AP9 is an incremental candidate on the reviewed AP8 basis `938791e42e6a6fde1f44ab07e9f1d4edc2357ded`. AP8's implementation and original evidence remain in history and in this branch; PR #67 is unchanged. This result belongs to issue #68 and remains subject to review, unmerged.

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

<!-- RESULTS_TABLE -->

## Measurement boundaries

The SDK host uses the real Windows AGain effect and an independently generated changing-gain reference. It compares every returned channel sample at the reported sample displacement. DAW compensation and audio devices are absent from this oracle. Serum uses its initialized patch, a one-note or 32-note chord every two seconds with a one-second hold/release, and real MainVol changes. The heavier workload changes MainVol twice per host block. An additional eight-note workload brackets the balanced setting. Serum's waveform and envelope are not used as a transport-delay oracle.

Linux timestamps cover admission, worker start, mapping preparation, send, reply, validation and publication. Windows times only `IAudioProcessor::process` using elapsed time on its own monotonic clock. The non-plug-in residual is calculated for each correlated request before aggregation. No unrelated absolute clocks or independently computed percentiles are subtracted. Histograms have eight buckets per octave: percentile fields are **bucket upper bounds**, while maxima and means retain directly measured durations.

The optional observer runs after publication on its own thread and has a bounded queue. Commercial timing begins at the first note-on and includes subsequent active and release blocks; it cannot be exhausted by pre-note desktop silence. Eight worst correlated requests and bounded gap traces are retained. `offered`, dropped jobs, unchecked samples and lost trace coverage stay visible. Returned-audio windows retain the first 32 active 100 ms bins; they do not claim whole-session coverage. The independent SDK reference oracle, rather than the optional state-seeded reference observer, provides the exact alignment comparison here.

Callback deadline misses, host scheduling misses, terminal faults, missing presentation frames and expired output are separate counts. A process may close cleanly after audible gaps; clean shutdown is not a latency pass. A last completed block discarded after stop may have a Windows duration but no publication duration, so stage sample counts can differ.

Native CPU is `getrusage` around the audio interval. Owner CPU is the systemd cgroup delta around the host's begin/end markers and includes the Python supervisor, Proton services, Wine and the Windows plug-in process. It excludes the native SDK host. Cgroup memory includes cached pages; native `maxrss` is a process high-water mark. A small `/proc` snapshot identifies CPU by owned process name; PIDs and private command lines are omitted from published evidence. No priority changes were used. The existing Moonlight session remained connected for control.

## Observed limits

The 48 kHz/128-frame reference run delivered exact alignment with zero gaps. Light Serum also passed at 128 added frames. The 32-note workload at the same delay missed 1,920 frames across six contiguous gaps, despite zero callback deadline misses and no terminal failure. The worst correlated chord request took 9.049 ms, of which 8.555 ms was inside Serum. Later requests queued behind that call. Changing bridge scheduling cannot turn that measured vendor computation into a 2.667 ms guarantee. The 512-frame headroom setting passed the same heavier workload.

At 64 added frames, the 48 kHz reference test missed one 64-frame block in eight seconds. It is an experimental boundary, not a recommended reliable mode. Larger host blocks also set a larger causal floor even though the Windows transport still uses 256-frame chunks.

CPU sampling at 48 kHz/128-frame blocks found substantial fixture costs outside DSP: about half a CPU core in the development Python owner, plus Wine server, device and Xalia services. These are included in the cgroup cost; they are not attributed to Serum's process duration or presented as production-manager efficiency. No supervisor protections or runner services were disabled to improve the benchmark.

The real AGain reports float32 and float64 capability; Serum 2.0.18 reports float32 support and rejects float64. The bridge implements float32 only and returns unsupported from its native float64 interface. For AGain this is a bridge implementation limit; for this Serum build the vendor also rejects it. Rates outside the explicit setup range and unsupported modes are refused. 88.2 and 192 kHz are admitted configuration values but are not qualified by the selected measurements unless a row below says otherwise.

## Bitwig and local audio

Desktop results are recorded separately from the SDK measurements. Bitwig is launched through the normal Applications list. Test projects are copies; original AP8 projects, settings and evidence are protected. The local audio route and actual requested host setup must agree with the retained records. Moonlight is never the audio-latency oracle.

<!-- DESKTOP_RESULT -->

Hardware converter round trip and controller-to-speaker latency are unmeasured. Software capture, if used, excludes converters. No claim of general Linux compatibility, vendor authorization, multi-commercial support, GUI embedding, reboot persistence, dynamic vendor-latency-change handling or consumer-ready deployment follows from these runs.

## Reproduction and provenance

Build with the existing `tools/ap8_build_native.py --performance` lane and the maintainer's private descriptor. `tools/ap9_benchmark.py` runs one bounded SDK comparison against already prepared AP9 reference/commercial owners. For example, on the declared Deck with the corresponding build receipt:

```sh
python3 tools/ap9_benchmark.py --name serum-fast --role serum --build BUILD_SHA256 --rate 48000 --block 128 --delay 128 --seconds 12 --voices 1
```

The helper does not install a plug-in, create a vendor environment or launch a DAW. Reuse the lawful installed module and exact pinned runner; private vendor files and state payloads are never committed. The prepared Serum environment is retained across tests, preserving its existing machine identity.

[Machine-readable measurements](../evidence/AP9/measurements.json) retain original case identities and counters. [Build and fixture identities](../evidence/AP9/provenance.json) identify the independently built Windows/native binaries. Initial compile, parser and state-routing failures remain recorded separately; no failed attempt is relabeled as a performance pass. Desktop preparation and cleanup receipts retain hashes rather than user paths or settings contents.
