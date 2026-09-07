# Product Design Dossier

**Document identity:** `linux-audio-compatibility-bridge.soft-design-dossier.v1`  
**Purpose:** Human-facing product direction, not a release promise or implementation checklist.

## Product

A native Linux proxy represents a Windows plug-in class to the DAW. A supervised Windows host loads the actual module under a deliberately selected compatible runner. Explicit control/state and real-time audio/event interfaces connect them. The intended manager covers installation, vendor-authorized activation, scanning, publication, organization, updates, diagnostics and rollback.

The user should not need to administer Wine prefixes, Flatpak paths, synchronization commands or runner archaeology to make music. State and project recall are product behavior, not optional reporting polish. Instruments and audio effects are both core product coverage. Published devices should identify their actual product, with stable class identity and optional user labels; the prototype's generic name is not the intended experience.

## Dossier set

- [Product and user experience](design-dossier/01-product-and-user-experience.md): product declaration, user laws and installation-to-recall workflows.
- [Identity, presets and visuals](design-dossier/02-identity-presets-and-visual-system.md): persistent identity, DAW state, content and editor presentation.
- [Activation, Flatpak, runtime and recovery](design-dossier/03-activation-flatpak-runtime-and-recovery.md): vendor-owned authorization, publication and environment lifetime.
- [Fixtures, development dependencies and pre-mortem](design-dossier/04-fixtures-proof-sequence-and-next-decision.md): Serum/Kontakt pressure, technical dependencies, risks and scenarios.

These retain the original product reasoning. Historical status statements and suggested ordering in them are not current progress or an obligatory task sequence. Consult [current work](../CURRENT_SLICE.md) and reviewed results. Development follows [AGENTS.md](../AGENTS.md) and [the simplified process](DEVELOPMENT_PROCESS.md), not the retired selection/receipt/acceptance ceremony formerly reproduced here.

## Landed baseline and current candidates

AP7 is accepted through PR #65, merge `cdfd05be9af2576768d8f2ccc55d3064e9e72a36`, reviewed at `5232870e6ce024efe52c2d6be0af7f62b7f021c3`. Transient presentation gaps do not destroy healthy instances; optional numerical observation does not hold up audio delivery. [Results](AP7_PLAYBACK_REPAIR.md) retain source and coverage distinctions. [AP4](AP4_RESULT.md) complete-state recall, [AP5](AP5_RESULT.md) independent instances and [AP6](AP6_RESULT.md) explicit last-confirmed-state recovery remain the accepted reference-effect baseline. AGain already exercises real Windows audio input/output; Serum was not our first round-trip audio test.

AP8 [PR #67](https://github.com/kasselvania/Linux-VST-bridge/pull/67), reviewed at `938791e42e6a6fde1f44ab07e9f1d4edc2357ded`, adds installed Serum 2.0.18 note processing and normal Bitwig setting recall. [AP8 results](https://github.com/kasselvania/Linux-VST-bridge/blob/938791e42e6a6fde1f44ab07e9f1d4edc2357ded/docs/AP8_RESULT.md) distinguish SDK audio measurements from desktop observations and do not establish a license channel, full editor support or general commercial compatibility.

AP9 [PR #69](https://github.com/kasselvania/Linux-VST-bridge/pull/69) adds configurable delay, host setup propagation and measured IPC improvements. Its follow-up at `b45b76332a4fc06cd314f7463b7a86ac6601865c` reports 57% less Python CPU and 22% less whole-owner CPU on a matched workload, not a whole-owner utilization of 22%. The matched pair does not demonstrate reduced service latency. 512 added frames at 48 kHz remains the observed desktop recommendation; the newer 256-frame run missed 2,688 frames in two gaps. A Bitwig close-time engine crash is separately unresolved. [Owner-cost report](https://github.com/kasselvania/Linux-VST-bridge/blob/b45b76332a4fc06cd314f7463b7a86ac6601865c/docs/AP9_OWNER_COST.md) preserves the facts and limitations.

At this direction update, both PRs remain unmerged; this document does not merge or newly approve their implementation. Preserve the distinction between the landed baseline, reviewed candidate improvements and unresolved failures.

## Immediate follow-through: explain and repair, not just benchmark

Keep the existing workload and trace the first stalled request through queueing, IPC, Windows host work, the plug-in call and output publication. Separate thread CPU execution from elapsed time, scheduling delay and blocking. The latest long request includes time queued behind earlier work: do not mistake that follower's queue delay for the original cause. Test a concrete hypothesis and repair the demonstrated bottleneck; another buffer-size table alone is not closure. Retain useful improvements without claiming they explain historical failures or establish a universal minimum latency.

Triage the retained Bitwig engine crash independently using existing application/engine logs and available stack/core records, plus the native teardown path. Normal Windows exit and positive owned cleanup do not establish correct native host teardown or explain an engine crash. Use a targeted reproduction or fault test where the evidence warrants one, preserve failures, and state any missing diagnostic evidence. Do not invent a cause or repeat an entire playback campaign to get a quiet close. This is follow-through on the current reliability work, not a restart of selection or a new permission gate.

## Next product coverage: a real commercial audio effect

After that focused follow-through, prioritize **audio track -> actual Windows Efx FRAGMENTS -> processed audio in normally launched Bitwig**. A recorded clip provides repeatable input without requiring new hardware; physical input/output round-trip remains a separately measured device claim. Reuse the working transport and installed candidates rather than start another bridge. Effects are a different compatibility and signal-integrity challenge, not inherently more CPU-intensive than an instrument.

AP8 [inspection evidence](https://github.com/kasselvania/Linux-VST-bridge/blob/938791e42e6a6fde1f44ab07e9f1d4edc2357ded/evidence/ap8-commercial-instrument/installed-module-inspections.json) records FRAGMENTS initializing with stereo main input/output and an auxiliary sidechain; processing was not tested. Its current runtime/content/authorization must be checked without disturbing the user's installation. It is the preferred effect fixture, not an assumed success. The commercial descriptor currently requires zero audio inputs and a 16-channel note input; generalize those actual restrictions instead of carrying instrument assumptions into the effect path.

The useful result is changing a real effect control, hearing/measuring input-dependent output, stopping/starting safely, and saving/reopening the effect state. Required coverage should follow the admitted modes:

- Preserve input samples and channel/bus identity through in-place buffers and block splitting. Support the main stereo route first; explicitly deactivate unsupported auxiliary buses or implement and test sidechain routing before claiming it.
- Separate transport delay from intentional effect timing. Check dry/bypass and parallel-path alignment under the plug-in's documented behavior, including its reported latency. Use AGain or another known deterministic route for exact displacement, not a bit-identical oracle for randomized granular processing.
- Preserve effect tails or retained/frozen audio when input becomes silent. Carry actual tempo/transport context for any admitted host-synchronized mode; automation and context must remain aligned across chunks. Do not substitute hard-coded tempo or equate silent input with silent output.
- Measure selected real effect workloads, gaps and chain-added latency, then investigate failures. Keep CPU/latency, input integrity, intended wet processing and clean host teardown separate. A successful instrument run does not certify these effect behaviors.

A practical detached vendor editor may be included where needed to operate the selected effect; full editor embedding, a preset-manager UI and an exhaustive compatibility matrix are not prerequisites. The next effects task is not activated by this roadmap addition, and ongoing AP9 code/experiments are not changed. References: [Arturia FRAGMENTS](https://www.arturia.com/store/software-effects/efx-fragments), [VST3 processing and buffers](https://steinbergmedia.github.io/vst3_dev_portal/pages/FAQ/Processing.html), [processing context](https://steinbergmedia.github.io/vst3_dev_portal/pages/Technical%2BDocumentation/API%2BDocumentation/Index.html), [latency and tail interfaces](https://steinbergmedia.github.io/vst3_doc/vstinterfaces/classSteinberg_1_1Vst_1_1IAudioProcessor.html).

## Using this material

Leads use the dossier and current code to choose a coherent result. Implementers receive that result, relevant boundaries and a few targeted references, with room to choose private implementation details. Additional design is for consequential unresolved behavior; ordinary debugging and test-helper repairs stay within the implementation task. A successful, relevant observation is reviewed on its merits without a compulsory second run under another label.
