# Product Design Dossier

**Document identity:** `linux-audio-compatibility-bridge.soft-design-dossier.v1`  
**Purpose:** Human-facing product direction, not a release promise or implementation checklist.

## Product

A native Linux proxy represents a Windows plug-in class to the DAW. A supervised Windows host loads the actual module under a selected compatible runner. Explicit state/control and real-time audio/event interfaces connect them. The intended separate manager handles installation, vendor-authorized activation, scanning, selective publication, organization, updates, diagnostics and rollback. Its window is not a playback dependency.

Users should not administer Wine prefixes, Flatpak paths or runtimes to make music. Published devices should identify the actual product, with stable class identity and optional user labels; the prototype's generic name is temporary. Instruments and audio effects are both core coverage. Product names and categories do not define I/O: actual exported classes and negotiated buses do. An instrument may also consume audio, and one product family can export distinct instrument and effect classes.

## Dossier set

- [Product and user experience](design-dossier/01-product-and-user-experience.md).
- [Identity, presets and visuals](design-dossier/02-identity-presets-and-visual-system.md).
- [Activation, Flatpak, runtime and recovery](design-dossier/03-activation-flatpak-runtime-and-recovery.md).
- [Fixtures, dependencies and pre-mortem](design-dossier/04-fixtures-proof-sequence-and-next-decision.md).

Those retain original reasoning, not an obligatory task sequence or current progress. Follow [AGENTS.md](../AGENTS.md), the [simplified process](DEVELOPMENT_PROCESS.md) and the branch's [current work](../CURRENT_SLICE.md). This update carries forward main's direction amendment at `1adde984cb33cc29a7c9aab79ec4508c977887e1`; it does not reinstate historical receipt/acceptance ceremonies.

## Landed baseline and inherited candidates

AP7 is accepted through PR #65, merge `cdfd05be9af2576768d8f2ccc55d3064e9e72a36`. Transient presentation gaps do not destroy healthy instances; optional observation does not hold up audio. [AP4](AP4_RESULT.md) state recall, [AP5](AP5_RESULT.md) independent instances and [AP6](AP6_RESULT.md) explicit last-confirmed-state recovery remain the reference-effect baseline. [AP7 results](AP7_PLAYBACK_REPAIR.md) preserve source and coverage distinctions. AGain already proves real Windows audio input/output; a commercial effect is broader compatibility, not our first round trip.

AP8 #67 at `938791e42e6a6fde1f44ab07e9f1d4edc2357ded` demonstrates installed Serum 2.0.18 notes and normal Bitwig setting recall. [Results](AP8_RESULT.md) distinguish SDK numerical audio from desktop observations. Full vendor-editor, license-channel, reboot and general compatibility claims do not follow.

AP9 #69 at `b45b76332a4fc06cd314f7463b7a86ac6601865c` supplies configurable delay, real host setup and measured IPC/owner improvements. [AP9](AP9.md) and [owner-cost results](AP9_OWNER_COST.md) retain the limits: 512 added frames is the observed desktop recommendation; 256 still had gaps; 57% less Python CPU and 22% less whole-owner CPU did not demonstrate lower latency. A Bitwig close-time engine crash remains unresolved. Clean Windows cleanup is not proof of correct native teardown. At AP10 preparation both PRs remain unmerged; this branch inherits their code without newly accepting those unresolved behaviors.

## Selected now: AP10 — Delivery repair and real commercial effects

[Issue #70](https://github.com/kasselvania/Linux-VST-bridge/issues/70) authorizes one connected implementation task on `codex/ap10-deadline-repair-and-effects`: improve real low-buffer delivery through causal diagnosis and code repair, preserve Serum's instrument path, and deliver a commercial main-stereo audio effect through the same core. [CURRENT_SLICE.md](../CURRENT_SLICE.md) owns the work order, not this roadmap.

Trace the first slow request and distinguish computation, scheduling and blocking before blaming Serum, Proton or Bitwig. Preserve the supervisor CPU saving without calling it a latency fix. Triage the engine crash using retained logs, available crash artifacts and native lifetime code. Repeated buffer counts or a later quiet close cannot establish a cause. Fix demonstrated bottlenecks and compare matched conditions; 256 then 128 added frames at 48 kHz are targets, not guarantees. Keep software service time, presentation delay, vendor latency and hardware round trip separate.

The preferred effect is installed Efx FRAGMENTS. Its [retained inspection](../evidence/ap8-commercial-instrument/installed-module-inspections.json) found a stereo main input/output and auxiliary sidechain, but did not prove processing. [Xfer documents Serum 2 FX separately](https://xferrecords.com/forums/general/serum-effects-as-separate-plugins); inspect whether that exact FX module/class is already available as a bounded fallback. Do not assume the tested synth class accepts input or install/activate repeatedly while hunting for a fixture.

Use bounded SDK-derived buses, arrangements and activation instead of separate hard-coded synth/effect plumbing. Main stereo is the first supported effect route. Inactive auxiliaries keep their correct indices; sidechain routing is supported only when implemented and tested. Supported I/O/latency changes go through inactive host renegotiation, not buffer changes under callbacks or acknowledged no-op host notifications. Preserve input integrity, in-place processing, chunk-relative controls, actual tempo/transport context, tails and opaque state. A granular effect's intended timing is not bridge delay, and silent input need not produce silence.

Commercial fixtures can run sequentially in this task. A recorded audio clip is sufficient; new loopback hardware and simultaneous-commercial support are not prerequisites. Verify actual input-dependent wet processing, relevant dry/bypass alignment, state recall and host closure. Use deterministic reference input for exact transport checks rather than invent an oracle for randomized effects.

## GUI and later refinement

A small detached vendor editor for the same processing instance is allowed in AP10 where needed to operate the effect; opening a second disconnected instance proves nothing about the active sound. Full editor integration remains the next user-facing priority: open/close without interrupting audio, correct host parameter notifications, preset/control edits, focus/resize and recall. Product-derived naming fits that work. The manager UI, embedded/streamed windows, broad routing/precision coverage and customer hardware qualification are separate improvements, not prerequisites to every audio repair.

## Working standard

One outcome, implementation and focused tests, one reviewable PR. Reuse fixtures/binaries and preserve failed observations. Local design decisions belong to the engineer; consequential changes to realtime/security architecture require a concrete evidence-backed decision. A blocked leg is reported honestly without erasing useful work or declaring the whole task complete. No compulsory duplicate campaign, source-permission loop or arbitrary retry quota.

Technical references: [VST3 dynamic I/O](https://steinbergmedia.github.io/vst3_dev_portal/pages/Technical%2BDocumentation/Change%2BHistory/3.0.0/Multiple%2BDynamic%2BIO.html), [processing context](https://steinbergmedia.github.io/vst3_doc/vstinterfaces/structSteinberg_1_1Vst_1_1ProcessContext.html), [latency/tails](https://steinbergmedia.github.io/vst3_doc/vstinterfaces/classSteinberg_1_1Vst_1_1IAudioProcessor.html), [Arturia FRAGMENTS](https://www.arturia.com/store/software-effects/efx-fragments).
