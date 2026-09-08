# Product Design Dossier

**Document identity:** `linux-audio-compatibility-bridge.soft-design-dossier.v1`  
**Purpose:** Product direction and current capability boundaries, not a release promise or an implementation checklist.

## Product

A native Linux proxy represents an actual Windows plug-in class to the DAW. A supervised Windows host loads the real module under a selected compatible runner. Explicit control/state and real-time audio/event interfaces connect them. A separate management plane owns installation, vendor-authorized activation, scanning, selective publication, organization, updates, diagnostics, repair, and rollback without being required for playback.

Users should not administer Wine prefixes, proxy synchronization, service startup, or Flatpak paths to make music. Published devices identify the actual vendor product, retain stable class identity, and respect user labels. Instruments and effects are both core coverage. Actual exported classes and negotiated buses determine I/O; product names do not. Factory/class subcategories determine browser role; the existence of an audio input does not by itself make a class an effect.

The project remains an independent bridge. Yabridge is prior art, not the implementation. Maintained Wine/Proton work may be reused as a runner beneath our native proxy, Windows host, transport, state, editor, and management layers.

## Dossier set

- [Product and user experience](design-dossier/01-product-and-user-experience.md)
- [Identity, presets and visuals](design-dossier/02-identity-presets-and-visual-system.md)
- [Activation, Flatpak, runtime and recovery](design-dossier/03-activation-flatpak-runtime-and-recovery.md)
- [Fixtures, dependencies and pre-mortem](design-dossier/04-fixtures-proof-sequence-and-next-decision.md)

These retain the original reasoning, not a mandatory ceremony or task sequence. Follow [AGENTS.md](../AGENTS.md), the [simplified process](DEVELOPMENT_PROCESS.md), and [CURRENT_SLICE.md](../CURRENT_SLICE.md).

## Accepted development baseline

| Milestone | Reviewed source | Capability and evidence |
| --- | --- | --- |
| AP8 / #67 | `938791e42e6a6fde1f44ab07e9f1d4edc2357ded` | Actual Windows Serum 2.0.18 note processing and normal Bitwig setting recall. [Results](AP8_RESULT.md). |
| AP9 / #69 | `b45b76332a4fc06cd314f7463b7a86ac6601865c` | Configurable delay, actual host processing setup, transport improvements, and lower supervisor CPU. [Performance](AP9.md); [owner cost](AP9_OWNER_COST.md). |
| AP10 / #71 | `fb0faed7cfc097ecb78653421d6e704bb9f25a00` | Mailbox delivery repair, real FRAGMENTS effect processing/control/recall, restored Serum, bounded returned events/parameter feedback, and corrected callback payload lifetime. [Findings](AP10.md). |
| AP11 / #76 | `f24cf7b4de60821eebd02fde1df9c67d2798b40c` | Same-instance detached vendor editor, proper hidden/minimized focus, real FRAGMENTS automation and stopped-edit recall, plus removal of a focus-triggered false state capture that caused a measured audio stall. [Results](AP11.md); [follow-up](AP11_REVIEW_FOLLOWUP.md). |

The integrated main basis after AP11 is `1d5d693e2e2d12547dfd6b02e2b8492b27418592`. The acceptance is bounded: FRAGMENTS does not certify Serum’s editor, every Arturia product, universal low latency, or every VST3 interface. Serum-specific lawful authorization/editor qualification remains #77.

AP4 [state recall](AP4_RESULT.md), AP5 [independent reference instances](AP5_RESULT.md), AP6 [last-confirmed-state recovery](AP6_RESULT.md), and AP7 [aligned transient gaps and independent observation](AP7_PLAYBACK_REPAIR.md) remain the foundation. Historical unsuccessful attempts and their exact provenance remain evidence rather than live instructions.

## Current product gap

The bridge’s musical and editor paths now work, but normal commercial use still depends on prepared artifacts, fixture-named directories, a manually available private owner, and environment assumptions created for development. The current commercial preview has a fixed endpoint and a capacity-one/session-directory shape. A musician cannot yet install/register a supported plug-in once, restart the machine, launch Bitwig, and load multiple published devices without reconstructing the experiment.

The next work therefore moves management and publication just far enough to make the accepted bridge usable by the operator. This is a product vertical, not a request for the complete future manager.

## Performance and compatibility boundaries

**512 added frames per proxy at 48 kHz (10.67 ms)** remains the retained desktop recommendation. Serial bridge devices accumulate that presentation delay; vendor and hardware latency are additional. AP11’s final focused FRAGMENTS session completed 47,705 callbacks / 12,212,480 frames without missed or expired frames, but does not establish a lower default or universal long-session reliability.

The first AP11 GUI-related stall was traced to a false parameter refresh on focus: Bitwig captured unchanged state on the serialized control path, delaying the following audio request. Removing that trigger was a causal repair. Genuine saves and vendor state work remain real operations whose latency may require later targeted refinement. Historical reply/preparation outliers remain unclassified.

Float32 and bounded main-stereo paths are implemented. Returned events/parameter feedback, state, automation, and detached editor lifecycle exist within current limits. Arbitrary multichannel/dynamic topology, full sidechain qualification, complete MIDI/MPE, every precision/mode, native Wayland editor integration, and broad customer hardware remain outside the accepted baseline.

FRAGMENTS retains a resource-integrity warning and a product-specific Windows-accessibility workaround on the current fixture. These are visible compatibility facts, not reasons to hide working behavior or claim unrestricted support.

## Selected: AP12 — Everyday Pure LoFi → Efx FRAGMENTS use in Bitwig

[Issue #78](https://github.com/kasselvania/Linux-VST-bridge/issues/78) is authorized on `codex/ap12-arturia-everyday-use` from main `1d5d693e2e2d12547dfd6b02e2b8492b27418592`. [CURRENT_SLICE.md](../CURRENT_SLICE.md) is the implementation work order.

The user outcome is a normal Bitwig device chain:

```text
MIDI / notes
    → Arturia Pure LoFi (instrument)
    → Arturia Efx FRAGMENTS (audio effect)
    → track output
```

After one setup transaction and restart, both devices appear under the correct vendor/product/browser roles, start their supervised Windows hosts automatically, open the actual same-instance editors, automate real controls, and save/reopen without a developer checkout, SSH session, manually started preview owner, or manager window remaining active.

### Normal installation, not copied-module ambiguity

Use the existing user-owned Pure LoFi and FRAGMENTS installers in a persistent bridge-owned Arturia environment, preserving current environments for rollback. The prior Pure LoFi `initialize()` failure came from a copied-module test environment and is not a final incompatibility finding. Re-test the normally installed module and diagnose the first actual failure. Lawful vendor/operator authorization remains vendor-owned; no secret collection or bypass.

A vendor-family environment is the default when the actual installers and modules support it. Environment sharing never means processor/controller/session sharing. Normal installation may also resolve or better explain FRAGMENTS’ resource warning; do not manufacture guessed resources.

### Exact identity and publication

Registration binds the exact Windows module digest and class ID to an environment revision, runner revision, compatibility settings, proxy build, and Bitwig publication. Friendly names and paths are projections. Stable native class and parameter identity survive product naming and restarts.

Carry actual factory vendor, class name, version where available, and VST3 subcategories. Browser role must not be inferred from bus shape. Publish Pure LoFi as an instrument and Efx FRAGMENTS as an audio effect without generic duplicate bridge entries or hard-coded product behavior. Publication is atomic and idempotent.

### Persistent owner and independent instances

Implement a small Rust-owned register/publish/status/unpublish path and automatic bounded user-session startup. A CLI is sufficient; the full graphical manager is deferred. Existing supervision may be reused, but installed playback cannot depend on `.git`, a worktree, fixture-named directories, or manually launched scripts.

At least one Pure LoFi and one FRAGMENTS instance must coexist with independent audio/event buffers, state, editor, automation, failure, and cleanup. Closing/removing one leaves the other healthy. Management activity and service discovery remain outside the callback and may not recreate AP11’s false refresh/state-capture dependency.

### Runner posture

Retain the exact currently verified Proton runner first and store it as registered compatibility state. Product-specific overrides remain explicit and scoped.

`giang17/wine` `d2d1-dcomp-11.0`, reviewed at `0077f1c63098d65a4d2554cd31d07903773cf992`, is a candidate runner source for matching DirectComposition/Direct2D/DirectWrite, font, or window defects. It is not a bridge replacement, yabridge dependency, or mandatory AP12 build. A justified comparison uses a coherent pinned runner and reversible environment; never mix graphics DLL families or apply global Wine-detection workarounds blindly. Its reported 64-sample Windows-DAW result is not our bridge latency.

### Proof

The decisive evidence is the operator’s ordinary workflow after setup exits and the system restarts: find both products in Bitwig, create the chain, hear LoFi through FRAGMENTS, use both editors, record/replay one parameter per plug-in, save/quit/restart/reopen, and remove FRAGMENTS while LoFi continues. Keep 512 frames per proxy, report the total serial delay and gap counters, and investigate a demonstrated regression rather than running a broad matrix.

Leave the intended environment, service, publications, and project installed. Restore temporary diagnostics and unrelated settings. Publish one unmerged implementation PR for review.

## Tracked follow-through

- [#72](https://github.com/kasselvania/Linux-VST-bridge/issues/72): residual delivery stalls and lower-latency suitability.
- [#73](https://github.com/kasselvania/Linux-VST-bridge/issues/73): historical native Bitwig engine-close crash.
- [#74](https://github.com/kasselvania/Linux-VST-bridge/issues/74): prompt notification of known endpoint failure.
- [#77](https://github.com/kasselvania/Linux-VST-bridge/issues/77): lawful Serum authorization and Serum-specific editor/preset qualification.

These are normal backlog items, not automatic prerequisites to AP12.

## Working standard

One useful outcome, ordinary implementation/debugging, focused tests, and one reviewed PR. Private types, service organization, storage choice, and algorithms belong to the engineer. Preserve user projects, vendor environments, installers, credentials, old evidence, and dirty local work. Diagnose repeated failures rather than rerunning them without new information.