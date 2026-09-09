# Product Design Dossier

**Document identity:** `linux-audio-compatibility-bridge.soft-design-dossier.v1`  
**Purpose:** Product direction and current capability boundaries, not a release promise or ceremonial checklist.

## Product and architecture

A native Linux proxy represents an actual Windows plug-in class to the DAW. A supervised Windows host loads the real module under a selected compatible runner. Typed control/state and real-time audio/event paths connect them. A separate management plane owns installation, vendor-access handoff, scanning, publication, profiles, diagnostics and eventually updates/repair/rollback; its UI is not required for music playback.

Instruments and effects are equal product requirements. SDK factory/class metadata drives names and browser roles; negotiated buses determine I/O. Product names do not dispatch implementation. Native class IDs, ParamIDs and user projects survive friendly naming and publication revisions. An environment can be shared without sharing instance state, buffers, editors or cleanup identity.

The bridge is independent: yabridge is prior art, not the implementation. Suitable maintained Wine/Proton work may be reused beneath the project-owned proxy, Windows host, transport, state and management.

The original [product](design-dossier/01-product-and-user-experience.md), [identity/presets](design-dossier/02-identity-presets-and-visual-system.md), [activation/runtime/recovery](design-dossier/03-activation-flatpak-runtime-and-recovery.md), [fixtures/pre-mortem](design-dossier/04-fixtures-proof-sequence-and-next-decision.md) and [architecture](ARCHITECTURE.md) retain the design reasoning. [AGENTS.md](../AGENTS.md) and [CURRENT_SLICE.md](../CURRENT_SLICE.md) govern active outcome-led work.

## Accepted development baseline

| Milestone | Reviewed source | Evidence |
| --- | --- | --- |
| AP8 / #67 | `938791e42e6a6fde1f44ab07e9f1d4edc2357ded` | Actual Serum note processing and Bitwig recall. [AP8](AP8_RESULT.md). |
| AP9 / #69 | `b45b76332a4fc06cd314f7463b7a86ac6601865c` | Configurable preview delay, setup, transport and supervisor improvements. [AP9](AP9.md). |
| AP10 / #71 | `fb0faed7cfc097ecb78653421d6e704bb9f25a00` | Mailbox, commercial effects, bidirectional results and payload lifetime. [AP10](AP10.md). |
| AP11 / #76 | `f24cf7b4de60821eebd02fde1df9c67d2798b40c` | Detached editor/focus, automation/recall and removal of false focus-triggered state capture. [AP11](AP11_REVIEW_FOLLOWUP.md). |
| AP12 / #79 | `66430ac40d5a398ea2a0943b330b1056dc7ecb03` | Installed Arturia chain, automatic startup, actual reboot/edited-state/automation recall, independent removal and scoped editor-removal workaround. [AP12](AP12.md). |
| AP13 / #82 | `5620c56660651c0624deb3935c91c91eb0653aa6` | Active state capture no longer serializes audio delivery; installed delay selection, prompt known-peer loss, prefaulted callback storage, supervisor CPU reduction and bounded 256 qualification. [AP13](AP13.md). |

AP13 is integrated by merge `ca0e2f7d5c85515c8ec22b67d434c6288ca611f7`. The accepted claim remains an installed engineering preview for the exact fixtures, not a glitch-free release, arbitrary Arturia installation, every VST3 or universal Linux compatibility. Original failed attempts retain their facts.

AP4 [state](AP4_RESULT.md), AP5 [independent instances](AP5_RESULT.md), AP6 [recovery](AP6_RESULT.md) and AP7 [aligned gaps](AP7_PLAYBACK_REPAIR.md) remain foundations, not instructions to repeat their entire campaigns.

## What now exists

The installed Rust register/publish/status/unpublish service and packaged supervision replace manual preview startup during playback. Both exact Arturia devices appear normally in Bitwig. The AP12 [musical record](../evidence/AP12/musical-pair-reboot-removal.json) covers note-driven LoFi → FRAGMENTS audio, both same-instance editors, recorded/replayed controls, saved edited sound, a real Deck reboot, automatic service startup, normal application launch and independent effect removal.

AP13 adds protocol-12 read-only state capture that can progress on the Windows UI owner while ordered mailbox audio continues. Restore/lifecycle operations remain exclusive. Complete validated state, ordinary save refusal, prior snapshot, malformed protocol and dead endpoint remain distinct. The deterministic SDK regression requires later real process calls to complete before `getState` can finish; the native full-callback regression preserves exact delayed output while a large state response remains unfinished.

The installed manager now exposes an inactive-only 256/512 bridge-delay preference separately from class identity and vendor state. Setup still enforces delay at least the actual negotiated host maximum. Existing registrations default to 512, and 512 is restored/recommended.

Normal installation fixed the old copied-module LoFi initialization barrier but did not establish full vendor access. Later operator-supplied module results remain distinct from original installer and authorization evidence. Keep module fingerprints and stable class IDs separate: a shared class ID does not guarantee cross-build state compatibility.

## Retained semantic decisions

Opaque vendor state, supplemental parameter readback, persistence availability, editor readiness and process health are separate. Unavailable readback does not discard good opaque state or become a fabricated normalized value. An ordinary completed save refusal does not kill healthy processing or save an older snapshot as current. Unsafe/partial restore, malformed protocol and lost endpoints remain explicit failures. A prior snapshot is only prior state.

Fresh vendor-editor access can precede save availability without bypassing a vendor restriction. Opening/focusing must not fabricate parameter invalidation or state capture. Respect SDK thread affinity, accepted-edit ordering, same-instance views, sample/event timing and bounded payload ownership.

Diagnostics do not own cleanup. Rich-report failure cannot skip physical cleanup or peer retirement; unproved ownership remains visible. Minimal [fault status](AP12-FAULT-STATUS.md) is independently retainable before containment, with atomic single-writer lanes and explicit clock domains. Detailed profiling is optional and outside native audio callbacks.

Compatibility profiles are closed declarative policy. They may select separately implemented reviewed capabilities; they may not contain arbitrary code, secrets, proprietary binaries, authorization bypasses or destructive command hooks. Local paths and environment IDs are local bindings, not portable profile identity. Exact module/class/runner/profile revisions bound support claims.

## AP13 repair and remaining limits

The first traced baseline LoFi gap had spent 105.569 ms queued behind a state capture and only 3.325 ms after dispatch, including 3.081 ms inside vendor processing. The repair removes that demonstrated control barrier without adding another socket, synchronous callback wait, hidden buffer or fabricated state. It does not explain every historical outlier.

Routine owner-process tracking now performs an initial census and follows validated PID/start-time descendants. In matched heavy-tracing-off 45-second host512/bridge512 intervals, the environment/LoFi/FRAGMENTS owners fell from a combined 63.136% to 12.014% of one core, about 81%. Both comparison intervals already had zero gaps, so this is not an 81% gap-rate result or total plug-in CPU reduction.

At 48 kHz, 512 bridge + 48 vendor frames for LoFi and 512 + 192 for FRAGMENTS give 1,264 frames / 26.333 ms, matching the reported chain. Bridge delay alone is 21.333 ms; device latency is extra. The opt-in 256 setting at an actual 256-frame host reports 752 serial frames / 15.667 ms and passed bounded musical/live-note intervals, but startup gaps and a later 1,536-frame FRAGMENTS gap prevent qualification.

Short gaps and 24–31 ms native preparation/publication intervals remain. Those elapsed intervals are not yet classified as CPU execution, allocator blocking or scheduler delay. The older untraced timeout remains unassigned. Float32 and bounded stereo/main-bus paths exist; arbitrary sidechains, all formats/MIDI/MPE, every SDK interface, native Wayland views and broad hardware qualification do not.

The measured LoFi editor-removal null dereference remains addressed by the exact process-scoped `disable_windows_accessibility` selection. This disables Windows UI Automation/screen-reader integration for selected hosts, not VST automation. It is not an upstream runtime fix.

## Next direction: AP14 profile-driven managed publication

The next product cut is [AP14 / #83](https://github.com/kasselvania/Linux-VST-bridge/issues/83). It converts the exact Arturia registration from engineer-assembled inputs into one coherent managed workflow backed by the first versioned declarative profiles.

The manager must derive local registration from exact supervised discovery/inspection plus reviewed profile policy. The operator should not type class IDs, module hashes, compatibility booleans, generated native paths or Wine commands. Zero/ambiguous matches, changed digests, missing classes, role/runner/environment mismatches and unsupported capabilities fail before publication mutation.

AP14 also introduces an explicit immutable publication revision and update/reconcile/rollback transaction. The prior known-good target remains active until a candidate is complete and verified. Durable records and physical pointer/target readback govern recovery. A failed candidate cannot erase or reconstruct the prior publication from current mutable files; rollback selects an exact retained revision. Active device leases block unsafe changes, foreign entries are preserved, and vendor modules/state/projects remain outside the transaction.

The first schema stays narrow. It covers only facts and capabilities needed for the exact Pure LoFi and Efx FRAGMENTS fixtures, including the current accessibility posture, detached editor, supported 512 performance profile and relevant protocol/state capability. It is not a universal plug-in database, remote profile marketplace, general vendor updater or new compatibility claim.

Completion requires deterministic parser/matcher/transaction fault tests plus one bounded Deck workflow: profile-driven republish without hand-authored identities, correct Bitwig roles and stable existing project identity/state, deliberate incompatible-candidate refusal, exact reconcile/rollback and a focused 512 playback/editor/save/recall/removal smoke check. Reuse AP13 for unchanged audio/performance claims.

## Editor and other follow-through

[#84](https://github.com/kasselvania/Linux-VST-bridge/issues/84) records the operator's later editor-lifecycle requirement: opening the DAW plug-in editor should directly show/focus the detached vendor window without the visible native **Open / focus vendor editor** and **Close vendor editor** panel. Closing the vendor window should retire only the editor session while healthy DSP and project state continue. This remains separate from AP14.

[#80](https://github.com/kasselvania/Linux-VST-bridge/issues/80) separately tracks FRAGMENTS Advanced-panel geometry and redraw smoothness. [#72](https://github.com/kasselvania/Linux-VST-bridge/issues/72) retains residual delivery/performance classification; #73 retains the historical native-close issue; #77 retains Serum vendor/editor qualification.

## Working standard

One selected outcome, ordinary implementation/debugging, focused tests and one reviewed PR. Preserve user projects, installed environments, historical evidence, credentials and dirty work. Leave the product installed and usable. Reuse unaffected proof; do not replace progress with repeated information-only campaigns. Current operator instruction and [AGENTS.md](../AGENTS.md) govern over retired procedural ceremony.