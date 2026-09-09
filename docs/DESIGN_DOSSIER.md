# Product Design Dossier

**Document identity:** `linux-audio-compatibility-bridge.soft-design-dossier.v1`  
**Purpose:** Product direction and current capability boundaries, not a release promise or ceremonial checklist.

## Product and architecture

A native Linux proxy represents an actual Windows plug-in class to the DAW. A supervised Windows host loads the real module under a selected compatible runner. Typed control/state and real-time audio/event paths connect them. A separate management plane owns installation, vendor-access handoff, scanning, publication, profiles, diagnostics and eventually updates/repair/rollback; its UI is not required for music playback.

Instruments and effects are equal product requirements. SDK factory/class metadata drives names and browser roles; negotiated buses determine I/O. Product names do not dispatch implementation. Native class IDs, ParamIDs and user projects survive friendly naming. An environment can be shared without sharing instance state, buffers, editors or cleanup identity.

The bridge is independent: yabridge is prior art, not the implementation. Reuse suitable maintained Wine/Proton work underneath the project-owned proxy, Windows host, transport, state and management.

The original [product](design-dossier/01-product-and-user-experience.md), [identity/presets](design-dossier/02-identity-presets-and-visual-system.md), [activation/runtime/recovery](design-dossier/03-activation-flatpak-runtime-and-recovery.md), [fixtures/pre-mortem](design-dossier/04-fixtures-proof-sequence-and-next-decision.md) and [architecture](ARCHITECTURE.md) retain the design reasoning. [AGENTS.md](../AGENTS.md) and [CURRENT_SLICE.md](../CURRENT_SLICE.md) govern active outcome-led work.

## Accepted development baseline

| Milestone | Reviewed source | Evidence |
| --- | --- | --- |
| AP8 / #67 | `938791e42e6a6fde1f44ab07e9f1d4edc2357ded` | Actual Serum note processing and Bitwig recall. [AP8](AP8_RESULT.md). |
| AP9 / #69 | `b45b76332a4fc06cd314f7463b7a86ac6601865c` | Configurable preview delay, setup, transport and supervisor improvements. [AP9](AP9.md). |
| AP10 / #71 | `fb0faed7cfc097ecb78653421d6e704bb9f25a00` | Mailbox, commercial effects, bidirectional results and payload lifetime. [AP10](AP10.md). |
| AP11 / #76 | `f24cf7b4de60821eebd02fde1df9c67d2798b40c` | Detached editor/focus, automation/recall and removal of false focus-triggered state capture. [AP11](AP11_REVIEW_FOLLOWUP.md). |
| AP12 / #79 | `66430ac40d5a398ea2a0943b330b1056dc7ecb03` | Installed Arturia chain, automatic startup, actual reboot/edited-state/automation recall, independent removal and a scoped editor-removal workaround. [AP12](AP12.md). |

AP12 is accepted by [review 5149097123](https://github.com/kasselvania/Linux-VST-bridge/pull/79#pullrequestreview-5149097123) as an installed engineering preview. Status reconciliation is documentation only. Original failed attempts retain their facts; this does not certify a glitch-free release, arbitrary Arturia installations or vendor entitlement.

AP4 [state](AP4_RESULT.md), AP5 [independent instances](AP5_RESULT.md), AP6 [recovery](AP6_RESULT.md) and AP7 [aligned gaps](AP7_PLAYBACK_REPAIR.md) remain foundations, not instructions to repeat their entire campaigns.

## What now exists

The installed Rust register/publish/status/unpublish service and packaged supervision replace manual preview startup. Both exact Arturia devices appear normally in Bitwig. The [musical record](../evidence/AP12/musical-pair-reboot-removal.json) covers note-driven LoFi → FRAGMENTS audio, both same-instance editors, recorded/replayed controls, saved edited sound, a real Deck reboot, the service already active before Bitwig, normal application launch and independent removal. The environment, publications and operator project remain installed.

Normal installation fixed the old copied-module LoFi initialization barrier but did not establish full vendor access. Later operator-supplied module results are distinct from original installer and authorization evidence. Keep module fingerprints and stable class IDs separate: a shared class ID does not guarantee cross-build state compatibility.

## Retained semantic decisions

Opaque vendor state, supplemental parameter readback, persistence availability, editor readiness and process health are separate. Unavailable readback does not discard good opaque state or become a fabricated normalized value. An ordinary completed save refusal does not kill healthy processing or save an older snapshot as current. Unsafe/partial restore, malformed protocol and lost endpoints remain explicit failures. A prior snapshot is only prior state.

Fresh vendor-editor access can precede save availability without bypassing a vendor restriction. Opening/focusing must not fabricate parameter invalidation or state capture. Respect SDK thread affinity, accepted-edit ordering, same-instance views, sample/event timing and bounded payload ownership.

Diagnostics do not own cleanup. Rich-report failure cannot skip physical cleanup or peer retirement; unproved ownership remains visible. Minimal [fault status](AP12-FAULT-STATUS.md) is independently retainable before containment, with atomic single-writer lanes and explicit clock domains. Detailed profiling is optional and outside native audio callbacks.

## Repaired crash and remaining limits

The [measured editor-removal fault](../evidence/AP12/delivery-cause-and-repair.json) is a null dereference in the pinned runner's `uiautomationcore.dll` during LoFi `IPlugView::removed`. The Windows process vanishes and leaves the next audio request unconsumed. The exact-registration `disable_windows_accessibility` option prevents that reproduced crash; FRAGMENTS uses it too. This disables Windows accessibility integration for selected processes, not musical VST automation. It is not an upstream UIA fix. The older untraced timeout remains unassigned.

At 48 kHz, 512 bridge + 48 vendor frames for LoFi and 512 + 192 for FRAGMENTS give 1,264 frames / 26.333 ms, matching the reported chain. Bridge delay alone is 21.333 ms. Device latency is extra. Short gaps remain: the post-reboot enclosing sessions record 23,808 and 6,656 underrun frames (27/7 gaps), not a zero-dropout result or a sum of unique audible chain loss. A 106.603-ms completed service with 1.450 ms inside vendor processing remains incompletely classified.

Float32 and bounded stereo/main-bus paths exist; arbitrary sidechains, all formats/MIDI/MPE, every SDK interface, native Wayland views and broad hardware qualification do not. Historical resource warnings retain their artifact scope. [#80](https://github.com/kasselvania/Linux-VST-bridge/issues/80) tracks Advanced-panel access and redraw separately.

## Next direction: AP13 delivery consistency and lower-delay operation

The installed workflow no longer needs another publication milestone. Next improve the real chain's delivery deadlines and validate a lower-delay option. [#72](https://github.com/kasselvania/Linux-VST-bridge/issues/72) is the performance basis. A new implementation branch starts from integrated main; its CURRENT_SLICE selects the bounded outcome.

Distinguish actual host callback cadence/max block, internal request chunk, bridge presentation delay and vendor delay. The registered baseline hard-codes 512 frames while the preview supports configured delays only when they cover the host maximum. An asynchronous nonwaiting callback cannot promise output from work it has not yet completed. Lowering a number without respecting that constraint is not optimization.

Trace and repair a demonstrated source of avoidable waiting, scheduling, queueing or copying. Compare matched musical/editor conditions and report gaps, latency and CPU. Preserve state/automation, independent instances and failure semantics. Do not trade away correctness, hide late audio with drift/replay, permanently spin cores, or increase hidden buffering to improve a benchmark. A lower option is not promoted until its actual host/setup boundary passes; useful verified consistency repairs may land without a claimed new default.

Keep the exact Proton runtime and scoped workaround first. The previously inspected `giang17/wine` branch at `dbb8005a228d259f2b3d74f9225eafd832261e0a` is reusable runtime work, not an established Arturia/performance fix. Compare coherently and reversibly only for a matching identified cause; do not mix graphics DLL families or treat another DAW's 64-sample result as our latency.

## Follow-through and working standard

[#72](https://github.com/kasselvania/Linux-VST-bridge/issues/72): delivery/latency; [#74](https://github.com/kasselvania/Linux-VST-bridge/issues/74): prompt known-process-loss notification; #73: historical native-close crash; #77: Serum vendor/editor qualification; #80: geometry/redraw. These are ordinary issues, not approval systems.

One selected outcome, ordinary implementation/debugging, focused tests and one reviewed PR. Preserve user projects, installed environments, historical evidence, credentials and dirty work. Leave the product installed. Reuse unaffected proof; do not replace progress with repeated information-only campaigns.
