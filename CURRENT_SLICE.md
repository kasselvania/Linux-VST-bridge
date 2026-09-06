# Current work: AP9 — Lower bridge latency and measure its operating limits

## Outcome

Make the actual Serum bridge materially more responsive and explain what delay/CPU cost belongs to the bridge versus the plug-in and audio system. Deliver measured low-latency and higher-headroom operating choices, not merely a benchmark or a smaller constant. Issue #68; branch `codex/ap9-bridge-latency-performance`.

The operator requested this performance work. Implementation, necessary native/Windows builds, ordinary debugging and focused tests are included under AGENTS.md. This branch starts at AP8's technically reviewed `938791e42e6a6fde1f44ab07e9f1d4edc2357ded`; PR #67 remains unmerged at preparation. Do not change its branch or relabel its observations. Reconcile its eventual merge normally. AP9 must remain an incremental reviewable change, not another copy of AP8.

## Separate the measurements

Measure host-visible sample displacement through the real bridged reference effect against a direct/known-zero-delay reference, with DAW compensation excluded or explicitly accounted for. Measure Linux request admission through valid returned output and publication, and separately time the actual Windows plug-in process call. These answer different questions: service time consumes the presentation allowance; it is not automatically extra latency to add on top. Query vendor algorithmic latency in the SDK-valid setup phase, not before setupProcessing. Report bridge delay and vendor delay separately and their correctly declared total.

Use correlated requests and elapsed durations measured within each clock domain; do not subtract unrelated Windows/Linux timestamps or subtract independently computed percentiles. Retain typical/tail/observed-maximum service times, callback deadline misses, gaps and terminal failures. Arm bounded observations around active notes, not startup silence, and keep measurement work out of audio deadlines. Existing traces/helpers are the starting point, not a new telemetry system.

Real Serum measures instrument workload and responsiveness. Its attack envelope or randomized waveform is not an exact transport-delay oracle. Physical device round trip and controller-to-speaker latency require suitable actual measurements; a software loopback excludes converters. No available hardware loopback means that boundary is explicitly unmeasured, not a reason to stall bridge work. Moonlight is GUI control, never the latency reference.

## Implement useful configuration and improve the measured bottleneck

The current native setup accepts only 48 kHz, float32 and at most 256 frames; Windows setup also hard-codes 48 kHz, and queued.rs fixes added delay at 1024 frames. Replace these restrictions coherently where needed: host setup, protocol, Windows setup, buffer extents, event offsets, queue timeline and reported latency must agree. Change configuration through inactive/setup lifecycle; do not alter audible delay during playback without proper host notification/reconfiguration. Preserve old project identities/state and the reproducible 1024-frame baseline.

Begin at 48 kHz/float32 and lower added delay adaptively through 512/256/128/64 frames as the chosen host block size and pipeline permit. Aim toward 128 or fewer extra frames at 48 kHz, without promising it or concealing dropouts. A zero-delay setting is not implemented merely by writing zero into DELAY: explain any causal block floor and fix a demonstrated internal bottleneck. Wake-up, IPC, copies, packetization and scheduling changes are in scope when measured; unbounded callback waits, blind priority escalation and diagnostic-induced stalls are not acceptable shortcuts.

Suggested range to characterize: 44.1/48/88.2/96 kHz, and 32–1024-frame host blocks. Probe 192 kHz only where useful and supported. These are search ranges, not a compulsory Cartesian matrix. Distinguish actual DAW/driver rates and blocks from requested settings, hidden resampling and SDK-host-only results. Bracket a useful passing point and a failing/unsupported boundary, then investigate instead of repeatedly soaking.

Sample precision is float32/float64, not compressed bitrate or 16/24-bit recording depth. Query support at each real layer and mark unsupported versus unimplemented modes explicitly. Keep float32 as the primary latency path. A bounded float64 extension/comparison is permitted where supported but must not hold up the first useful latency improvement; no silent downconversion or compression. Larger block support must preserve parameter/note offsets and bounds rather than just enlarge a number.

## Finish with the product result

Use the existing reference effect for exact timing/alignment checks, then actual Serum with the same controlled light patch and a heavier note/polyphony/automation workload before and after changes. Record CPU/memory, test duration and load with missed-frame and failure counts; fast callbacks alone do not prove timely output. Keep one-commercial-instance scope; measure heavier work without claiming multi-commercial support. Preserve AP7 aligned gaps, ordered inputs/control, real DSP/state and owned cleanup.

Validate selected useful settings in normally Applications-launched Bitwig through local audio; label streamed/control-active load separately. Reuse the retained vendor environment and project copies. No installers, activation campaign, new broker, exhaustive compatibility matrix or duplicate diagnostic/acceptance runs.

Return one PR, unmerged, containing the improvement and a compact table of actual settings, bridge/vendor delay, service-time tails, CPU/load, gaps/faults, measured limitations and recommended fast/balanced/headroom choices for this hardware. Do not promote a brief quiet run into a universal customer guarantee. If a target cannot be reached, identify the measured limiting stage and best supported setting rather than fabricate a pass.

Relevant code: native-vst3-proxy/source/processor.cpp and processor.h; backend/src/queued.rs, lib.rs and observer.rs; windows-factory-probe/source/offline_processing.cpp and mapped_processing.cpp; existing SDK host and preview helpers. SDK references: [processing setup](https://steinbergmedia.github.io/vst3_doc/vstinterfaces/structSteinberg_1_1Vst_1_1ProcessSetup.html), [latency reporting](https://steinbergmedia.github.io/vst3_doc/vstinterfaces/classSteinberg_1_1Vst_1_1IAudioProcessor.html), [configuration lifecycle](https://steinbergmedia.github.io/vst3_dev_portal/pages/FAQ/Processing.html).