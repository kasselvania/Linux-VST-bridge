# Product Design Dossier

**Document identity:** `linux-audio-compatibility-bridge.soft-design-dossier.v1`  
**Purpose:** Product direction and current capability boundaries, not a release promise or an implementation checklist.

## Product

A native Linux proxy represents an actual Windows plug-in class to the DAW. A supervised Windows host loads the real module under a selected compatible runner. Explicit control/state and real-time audio/event interfaces connect them. The separate manager is intended to handle installation, vendor-authorized activation, scanning, selective publication, organization, updates and recovery without needing its window open for playback.

Users should not administer Wine prefixes or Flatpak paths to make music. Published devices should identify the actual product, retain stable class identity, and respect user labels. Instruments and effects are both core coverage. Actual exported classes and negotiated buses determine I/O; product names do not. Audio and notes may coexist when the class declares and the bridge supports them.

## Dossier set

- [Product and user experience](design-dossier/01-product-and-user-experience.md).
- [Identity, presets and visuals](design-dossier/02-identity-presets-and-visual-system.md).
- [Activation, Flatpak, runtime and recovery](design-dossier/03-activation-flatpak-runtime-and-recovery.md).
- [Fixtures, dependencies and pre-mortem](design-dossier/04-fixtures-proof-sequence-and-next-decision.md).

These retain the original reasoning, not a mandatory task sequence or current status. Follow [AGENTS.md](../AGENTS.md), the [simplified process](DEVELOPMENT_PROCESS.md) and [current position](../CURRENT_SLICE.md). The integration preserves the commercial-effects and causal-investigation direction recorded on main at `1adde984cb33cc29a7c9aab79ec4508c977887e1`, updated with the subsequently reviewed implementation.

## Accepted development baseline

| Milestone | Reviewed source | Capability and evidence |
| --- | --- | --- |
| AP8 / #67 | `938791e42e6a6fde1f44ab07e9f1d4edc2357ded` | Actual Windows Serum 2.0.18 notes and normal Bitwig setting recall. [Results](AP8_RESULT.md) distinguish SDK audio measurements from desktop observations. |
| AP9 / #69 | `b45b76332a4fc06cd314f7463b7a86ac6601865c` | Configurable delay, actual host processing setup, measured socket and supervisor improvements. [Performance](AP9.md); [owner cost](AP9_OWNER_COST.md). |
| AP10 / #71 | `fb0faed7cfc097ecb78653421d6e704bb9f25a00` | Mailbox delivery repair, real FRAGMENTS effect processing/control/recall, restored Serum, and bounded returned events/parameter feedback including native callback payload lifetime. [Findings](AP10.md); [R1 evidence](../evidence/AP10/r1-payload-lifetime.json). |

The integration accepts these bounded outcomes, not every original stretch target or universal compatibility. AP4 [state recall](AP4_RESULT.md), AP5 [independent reference instances](AP5_RESULT.md), AP6 [last-confirmed-state recovery](AP6_RESULT.md), and AP7 [aligned transient gaps and independent observation](AP7_PLAYBACK_REPAIR.md) remain the foundation. AGain already exercised actual Windows audio input/output; FRAGMENTS expands commercial effect coverage rather than inventing a separate bridge.

Source/build distinctions matter. Earlier commercial observations are reused for the reviewed R1 native storage-only repair; they are not renamed as fresh executions. The production SDK fixture, not commercial audio that emitted no events, establishes the returned-event semantics. Historical unsuccessful attempts, ledgers and handoffs retain their original labels and are not active instructions.

## Performance and compatibility boundaries

**512 added bridge frames at 48 kHz (10.67 ms)** remains the observed desktop recommendation. Short mailbox runs at 256 passed, but lower latency is not yet a general reliability guarantee. The 599.836 to 241.326 microsecond comparison measures observed mean non-plug-in service, not fixed presentation delay, converter latency or a worst case. AP9's 57% Python / 22% owner-cohort CPU reductions are separate findings and did not prove that those changes removed desktop stalls.

The causal Wine-server dependency and mailbox improvement are real; the precise monopolizing server operation and all historical outliers are not explained. Vendor-reported delay is separate: retained FRAGMENTS setup reports 512 bridge plus 192 vendor frames. Its zero tail report despite observed frozen output remains a compatibility limitation, not a reason to discard continuing audio on silent input.

Current operation still needs prepared artifacts, a private owner and retained vendor environment. Float32 and bounded main-stereo paths are implemented. Auxiliary sidechain activation, arbitrary dynamic/multichannel topology, complete MIDI/MPE input, all format/precision modes, simultaneous commercial instances, vendor-editor integration, reboot persistence, authorization channels and hardware round trip are not broadly established. Named reference success cannot silently stand in for a different vendor mode.

## Tracked follow-through

[Latency #72](https://github.com/kasselvania/Linux-VST-bridge/issues/72): identify the first stalled request, separating execution, scheduling, blocking, queueing and host callback bursts. A queued follower is not the initiating cause. Fix measured mechanisms and compare like-for-like; another unexplained table of buffer failures is not the desired deliverable. Keep 512 as a baseline while evaluating lower settings under real instrument/effect and eventual editor load.

[Engine close #73](https://github.com/kasselvania/Linux-VST-bridge/issues/73): the historical Bitwig engine crash has no useful retained stack. Inspect native callback/worker/controller teardown and capture useful evidence on relevant closes. Successful Windows cleanup or a later clean host exit does not explain the old failure. Do not demand indefinite reproduction before independent progress.

[Failure notification #74](https://github.com/kasselvania/Linux-VST-bridge/issues/74): known endpoint failure can currently be learned through the five-second reply deadline. Improve prompt terminal notification without making transient lateness fatal, replaying work or adding callback waits.

These are normal backlog items, not new execution gates or an instruction to keep AP10 open.

## Next user-facing candidate: the actual vendor editor

Prefer a practical editor attached to the **same processing instance**: open it from the DAW, change real controls/presets, synchronize host parameters, save/reopen the result and close the window without stopping audio. Product-derived naming fits this work. Confirm the SDK/UI-thread and current controller architecture before selecting the concrete implementation; do not assume a second standalone plug-in window controls the playing instance.

Use the existing permitted desktop/SSH workflow and keep GUI operations outside the audio deadline path. Include representative playing/editing and gap checks rather than a complete historical benchmark replay. A detached editor may be a useful first increment; full embedding, a manager UI, arbitrary routing and customer hardware qualification are not automatic prerequisites. No new editor slice is activated by this candidate description.

## Working standard

One useful outcome, ordinary implementation/debugging, focused tests and one reviewed PR. Decisions about private types, instrumentation and algorithms belong to the engineer. Consequential changes to public realtime/security behavior require a concrete decision, not speculative permission machinery. Preserve projects, vendor environments, credentials, old evidence and dirty local work. New work branches from current main; completed slice branches and historical status reports are not live work orders.
