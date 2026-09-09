# AP13 — Consistent delivery and a validated lower-delay option

## Outcome and authority

Make the installed **Pure LoFi → Efx FRAGMENTS** chain more dependable during musical playback, automation and editor interaction, then validate a lower bridge-added-delay option. The first target is 256 frames per device on a compatible host setup, not an unconditional new default or zero-latency claim.

[Issue #81](https://github.com/kasselvania/Linux-VST-bridge/issues/81), branch `codex/ap13-delivery-consistency`, starts from integrated AP12 main. AP12's reviewed implementation is `66430ac40d5a398ea2a0943b330b1056dc7ecb03`, accepted in [review 5149097123](https://github.com/kasselvania/Linux-VST-bridge/pull/79#pullrequestreview-5149097123). Its installed workflow and evidence stand. [#72](https://github.com/kasselvania/Linux-VST-bridge/issues/72) is the performance backlog; do not reopen AP12 or recreate its setup.

Implementation, necessary builds, related mechanical repairs, focused regression tests and reversible bridge-only deployment are authorized under [AGENTS.md](AGENTS.md). Private organization and algorithms belong to the engineer. Preserve dirty work and coordinate control of the operator's open desktop/project. One implementation PR against main, left unmerged. No permission receipts, fixed retry quotas or duplicate diagnostic/acceptance campaign.

## Why this is next

[AP12's timing and workflow results](docs/AP12.md#latency-gaps-and-remaining-operating-limits) show both products working, but short gaps persist. The recorded 48 kHz chain has 512+48 and 512+192 frames: 1,264 / 26.333 ms reported, of which 1,024 / 21.333 ms is bridge-added. A completed request took 106.603 ms with 1.450 ms inside vendor process. That difference is unclassified elapsed time, not proof of bridge CPU consumption. An early pre-note request had 1.078 ms before consumption after a callback burst but needed 2.113 ms to finish. The exact causes of scheduling/bursts and other long service intervals remain open.

Source checks at the accepted head locate the work: `backend/src/performance.rs::selected_delay` fixes registered use at 512; preview delay validation requires delay >= negotiated host maximum; `wire` caps Windows requests at 256. `queued.rs` splits/queues work and presents results without waiting in the DAW callback. `backend/src/lib.rs`, `mailbox.rs`, Windows `delivery_mailbox.h`, `mapped_processing.cpp` and `offline_processing.cpp` contain the delivery, polling and owner-service paths. Measure their actual cost/interaction rather than replace them speculatively.

## First make the deadline failure actionable, then change the responsible mechanism

Reuse the existing short-gap records, completed traces and AP12 independent fault status. Add only the bounded correlation missing for short/pre-note requests: actual native callback entry and completion deadline, request/chunk identity, queue residence, publication/consumption, vendor entry/return, reply publication/receipt, and overlapping UI/state/lifecycle work. Minimal status already exists; do not build a second observability system. Detailed sampling is temporary, outside Bitwig's audio callback, and must not require a hung thread to finish. Match requests by identity; clocks remain separate. Distinguish CPU execution, runnable delay, blocking and elapsed time where evidence permits.

Use a focused production-path regression to exercise the measured callback burst or wait, including the actual parent host-block grouping, not only evenly spaced 256-frame requests. Identify and implement a causal repair in queueing, wake/notification, scheduling configuration, validation/copying, or control/UI contention. An appropriately scoped batching/wake change is allowed; a blanket rewrite is not selected. Test prompt known-endpoint-loss handling (#74) when that path is touched: a dead peer differs from a late live result. Do not assign the older untraced timeout to the known UIA removal crash.

The delivered work must include a demonstrated mechanism improvement, not only instrumentation, settings or more averages. If one suspected cause is disproved, use that result to choose the next implicated boundary; do not thrash across runtimes, plug-ins and buffer matrices. No requirement to reconstruct every historical outlier.

## Close the actual timing contract

Keep four quantities distinct: host maximum/actual callback frames and cadence; internal request chunk; fixed bridge presentation delay; vendor-reported delay. A 512-frame host callback containing two 256-frame requests does not give the worker a fresh 5.33-ms wait between them. Sample-index delay does not guarantee equally spaced wall-clock callbacks.

The selected implementation remains asynchronous and nonwaiting in the native real-time callback. Retain the current delay >= negotiated host-maximum constraint unless a separately reasoned and tested mechanism establishes correctness without waiting for same-callback work; simply deleting the guard is prohibited. Begin the 256-frame target with an actual negotiated host maximum <=256, and record both it and the audio-device setting. A smaller host block alone is not evidence that bridge code became faster. No synchronous IPC, sleeps, unbounded spin, allocation, logging or process operations in Bitwig's audio callback. No global priority/power changes, permanently busy CPU cores or extra unreported buffering to obtain a pass; evaluate the CPU tradeoff of any bounded worker wake strategy.

Read the pinned SDK [ProcessModes and latency contract](https://github.com/steinbergmedia/vst3_pluginterfaces/blob/4f547e8e102b47de4a8b8aaf343c73b700786372/vst/ivstaudioprocessor.h). Observe the actual process mode; a burst alone does not prove kPrefetch or a host bug. Current support rejects prefetch. Do not relabel calls or silently pace offline/prefetch output as realtime. Broader mode support is not required unless it is the demonstrated boundary preventing the selected workflow. A stopped song still permits audio callbacks.

## Make the installed performance setting real

Add the smallest explicit, validated, inactive-only selection of bridge delay on the registered path. Existing registrations retain 512 by default; 256 is an opt-in qualification target with easy return to 512. Use the installed management/binding route, not the old `AP9-Performance/delay-frames` fixture file. Separate performance preference from vendor state and class identity. Unsupported combinations fail clearly before activation; never silently change delay midstream.

Keep latency reporting and host compensation synchronized through the SDK setup/reactivation contract. Every emitted sample, returned event and parameter-feedback offset must reflect the selected delay once, not twice or only in the displayed number. Do not drop late note-offs, shift the stream after a gap, replay requests, fabricate audio/state, drop automation for smoother graphics or weaken payload/lifetime validation. Preserve same-instance editors, stopped-state saving, ordinary save refusal, independent instances and reporting-independent cleanup. Protocol/schema additions must remain explicitly versioned and existing saved projects readable.

## Verification and completion

Compare the accepted and repaired path at the same 48 kHz/float32, 512-delay host setup first. Then compare repaired 512 versus 256 at the same compatible smaller host setup. Use the existing musical project copy and short action-labelled intervals: idle/start, musical playback, recorded automation, manual editor changes, and required saves/close. Include a supported live-note/input interaction rather than assuming prerecorded playback proves playing latency. Retain existing fault checks and independent sibling behavior where affected. Recheck installation persistence without another full reboot campaign unless startup changes.

Use deterministic SDK/transport consumers for exact sample ordering, impulse/ramp delay through one and two proxies, event/automation offsets at chunk/host boundaries, zero-frame updates and burst/stall handling. Commercial results supply actual sound/editor/control behavior, not an invented waveform oracle. Verify Bitwig's reported delay against the actual configured pipeline. A physical loopback measurement is useful only if existing equipment/routing supports one; do not call sink-monitor capture physical latency.

Report missed/expired/priming/rejected frames and terminal faults per device and action interval, duration and CPU with heavy tracing off. Do not add per-device counts as unique audible chain loss. A bounded zero-gap musical result is the goal for promoting a lower profile, not proof of universal reliability. If 256 remains unsuitable, retain 512 as the recommendation and deliver the causal consistency repair and measured remaining constraint. Do not call the lower target passed or hold useful repairs hostage to indefinite retries. Instrumentation/configuration alone does not satisfy this slice.

At unchanged vendor delays, two 256-frame bridges would total 752 frames / 15.667 ms at 48 kHz; 512 frames / 10.667 ms of that is bridge-added. These are target arithmetic, not a promised or measured result. Restore the supported setting at handoff unless the qualified lower option is explicitly selected. Leave the operator's installation and project usable; preserve earlier evidence.

## Runtime and scope

Keep the exact AP12 modules, Proton/runtime and process-scoped accessibility selections. `giang17/wine` at previously inspected `dbb8005a228d259f2b3d74f9225eafd832261e0a` is a runtime reference beneath the independent bridge, not a yabridge pivot or a cure for unclassified waits. A matching measured runtime defect can justify one coherent, reversible isolated comparison; account for actual graphics modules, host-executable defaults, synchronization support and Wine X11 identity. Do not mix DLL families or assume its maintainer's 64-sample DAW result applies to our path.

Graphics #80, full manager/update UI, new commercial fixtures, licensing, universal mode/precision/routing support and an arbitrary smaller default are outside this task unless directly necessary to its demonstrated mechanism. Keep the old untraced timeout, historical native-close issue and unresolved gaps honest. Relevant design: [architecture §§6–7](docs/ARCHITECTURE.md), [dossier](docs/DESIGN_DOSSIER.md), and [runtime/recovery](docs/design-dossier/03-activation-flatpak-runtime-and-recovery.md).
