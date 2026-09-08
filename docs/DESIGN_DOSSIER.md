# Product Design Dossier

**Document identity:** `linux-audio-compatibility-bridge.soft-design-dossier.v1`  
**Purpose:** Product direction and current capability boundaries, not a release promise or an implementation checklist.

## Product

A native Linux proxy represents an actual Windows plug-in class to the DAW. A supervised Windows host loads the real module under a selected compatible runner. Control/state and real-time audio/event interfaces connect them. A separate management plane owns installation, vendor-authorized activation, scanning, selective publication, organization, updates, diagnostics, repair and rollback without being required for playback.

Users should not administer Wine prefixes, proxy synchronization, service startup or Flatpak paths to make music. Published devices identify the actual vendor product, retain stable class identity and respect user labels. Instruments and effects are both core coverage. Actual exported classes and negotiated buses determine I/O; product names do not. Factory/class subcategories determine browser role; an audio input does not by itself make a class an effect.

The project remains an independent bridge. Yabridge is prior art, not the implementation. Maintained Wine/Proton work may be reused as a runner beneath our native proxy, Windows host, transport, state, editor and management layers.

## Dossier set

- [Product and user experience](design-dossier/01-product-and-user-experience.md)
- [Identity, presets and visuals](design-dossier/02-identity-presets-and-visual-system.md)
- [Activation, Flatpak, runtime and recovery](design-dossier/03-activation-flatpak-runtime-and-recovery.md)
- [Fixtures, dependencies and pre-mortem](design-dossier/04-fixtures-proof-sequence-and-next-decision.md)

These retain original reasoning, not a mandatory ceremony or task sequence. Follow [AGENTS.md](../AGENTS.md), [development guidance](DEVELOPMENT_PROCESS.md) and [CURRENT_SLICE.md](../CURRENT_SLICE.md).

## Accepted development baseline

| Milestone | Reviewed source | Capability and evidence |
| --- | --- | --- |
| AP8 / #67 | `938791e42e6a6fde1f44ab07e9f1d4edc2357ded` | Windows Serum 2.0.18 note processing and normal Bitwig setting recall. [Results](AP8_RESULT.md). |
| AP9 / #69 | `b45b76332a4fc06cd314f7463b7a86ac6601865c` | Configurable delay, host processing setup, transport improvements and lower supervisor CPU. [Performance](AP9.md); [owner cost](AP9_OWNER_COST.md). |
| AP10 / #71 | `fb0faed7cfc097ecb78653421d6e704bb9f25a00` | Mailbox delivery repair, FRAGMENTS effect processing/control/recall, restored Serum, returned events/parameter feedback and corrected callback payload lifetime. [Findings](AP10.md). |
| AP11 / #76 | `f24cf7b4de60821eebd02fde1df9c67d2798b40c` | Same-instance editor, hidden/minimized focus, FRAGMENTS automation/stopped-edit recall and removal of a focus-triggered false state capture causing an audio stall. [Results](AP11.md); [follow-up](AP11_REVIEW_FOLLOWUP.md). |

Integrated main remains `1d5d693e2e2d12547dfd6b02e2b8492b27418592`. FRAGMENTS does not certify Serum's editor, every Arturia product, universal low latency or every SDK interface. Serum's lawful authorization/editor qualification remains #77.

AP4 [state recall](AP4_RESULT.md), AP5 [independent reference instances](AP5_RESULT.md), AP6 [last-confirmed-state recovery](AP6_RESULT.md), and AP7 [aligned transient gaps](AP7_PLAYBACK_REPAIR.md) remain the foundation. Historical attempts keep their exact scope and are not standing instructions.

## Current candidate: installed use exists; dependable pair use is unfinished

[AP12/#79](https://github.com/kasselvania/Linux-VST-bridge/pull/79) at preparation input `01bc07ce490256cc160751806c01255b7d2aa38d` has implemented persistent Arturia installation support, SDK-derived browser identities, a Rust register/publish/status/unpublish core, installed automatic startup and distinct Windows sessions. Do not describe these as wholly absent or repeat the initial installation work order.

LoFi has actual edited-state recall and automation results on its recorded candidate. FRAGMENTS' operator-replaced module was reinspected and its proxy fingerprint updated, restoring load/editor/save/close on the installed path. That FRAGMENTS check was silent processing. The complete musical pair, FRAGMENTS automation on this artifact, full user-session/Deck restart and audible-sibling removal are still unverified. LoFi has an intermittent terminal delivery timeout as well as short presentation gaps. See [AP12 results](AP12.md) and [latest evidence](../evidence/AP12/operator-replacement-load-removal.json).

The original installer results and operator-replacement results are separate. Normal installation fixed LoFi initialization but did not establish full vendor access. A new module digest or absent login window is not an authorization certificate. Do not change vendor/licensing files, silently relax artifact checks or reuse an earlier artifact's result for a replacement.

**Merge disposition:** keep #79 draft/unmerged and #78 open. The next turn completes AP12; it is not a new AP13 or another stacked implementation PR. Individual passing repairs do not certify the daily-use milestone.

## Architectural lessons already implemented in the candidate

Opaque vendor component/controller state, the supplemental parameter mirror, persistence availability, editor readiness and process health are distinct facts. An unavailable scalar readback must not discard a successful opaque capture or become a fabricated normalized value. Ordinary completed save refusal returns failure without inventing a saved sound or automatically killing healthy processing. Unsafe restore, malformed protocol and lost endpoints remain explicit failures. A prior snapshot is not a successful fresh save.

Fresh vendor-editor access may precede persistence availability. This enables normal user/vendor interaction without pretending a demo can save or restoring an existing project to defaults. Native mirror bootstrap must not recreate a hidden save prerequisite.

Rich reporting is not ownership authority. Diagnostic-write failure must not skip physical cleanup, native release or transport retirement; genuinely unconfirmed cleanup remains visible and blocked. Preserve the fixed-size/real-time boundaries, SDK thread affinity, stable IDs and these production regressions.

## Selected next execution: delivery repair plus the actual musical chain

The latest LoFi timeout preceded the removal/save request. The native mailbox waits five seconds for that reply, distinct from the 512-frame presentation horizon. Do not label it a small-buffer miss or claim that the earlier save repair explains it.

The current Windows trace retains completed, armed requests and dumps after thread shutdown. It can therefore lose the exact unfinished request or omit pre-note idle failures. Extend that existing mechanism with bounded, race-free in-flight identity/stage publication readable and retainable by an independent owner before containment. Minimal installed fault visibility should not require an agent to have enabled a full profiler first; detailed histories and thread sampling remain opt-in. No synchronous logging or diagnostic locks on the audio callback, no cross-clock arithmetic without an established relationship, and no proprietary payload/credential dumps.

Use the last observed stage to distinguish request consumption, vendor processing, response publication and native receipt, then repair the demonstrated wait/lifecycle/runtime cause. Test the failure path deliberately; do not rely on the stuck thread returning. Continue the necessary musical workflow even when an intermittent fault does not recur, preserving that uncertainty instead of an indefinite soak. A clean retry is not a causal fix, and instrumentation alone is not full AP12 completion.

The product outcome remains:

```text
MIDI / notes
    → Arturia Pure LoFi (instrument)
    → Arturia Efx FRAGMENTS (audio effect)
    → track output
```

Bitwig owns the serial chain. Establish input-dependent musical effect processing, same-instance editors, one recorded/replayed automatable parameter per device, edited-sound/pair recall after an actual Deck or full graphical-user-session restart, and removal of FRAGMENTS while LoFi audibly continues. Preserve user projects before disruptive steps. Installed startup must work with setup tools closed and no checkout/manual preview owner dependency. Leave the intended environment, publications, service and project available.

## Identity, management and deployment boundaries

Registration binds an exact module digest/class to an environment, runner, compatibility settings and proxy artifact. SDK metadata drives names and roles. Friendly naming and paths do not regenerate native class/parameter identity. Stable class IDs do not alone establish saved-state compatibility across changed module bytes.

Each DAW instance has its own transport, buffers, DSP/controller state, editor, automation and cleanup identity. Sharing a vendor environment does not merge instances. Management/status work stays outside processing and must not generate AP11's false refresh/state-capture trigger. The existing installed core is the implementation basis; full manager polish and universal update/installer support remain future work.

## Performance and editor/runtime scope

Retain **512 added frames per proxy at 48 kHz (10.67 ms)**. Two serial bridges contribute 1,024 frames / 21.33 ms before vendor and device latency. Report actual configured vendor delays, computed serial totals and any measured physical path separately. Existing candidate gaps/timeouts mean this is a retained setting, not a new gap-free guarantee. Do not increase buffering or lengthen a terminal deadline to hide the fault.

Float32 and documented bounded stereo/bus/event paths exist. Arbitrary multichannel/dynamic topology, full sidechains/MIDI/MPE, native Wayland, every precision and broad hardware reliability are not claimed. Historical FRAGMENTS resource-integrity warnings retain their original scope; its current explicit Windows-accessibility workaround remains a compatibility setting, not a global default.

[#80](https://github.com/kasselvania/Linux-VST-bridge/issues/80) records the operator's FRAGMENTS Advanced-panel expansion and unsmooth-redraw observations. Resize negotiation, clipping, display scaling and renderer behavior must be distinguished; there is already a plug-in-requested resize implementation. General graphics work is deferred unless it blocks required controls, loses musical updates or shares the demonstrated timeout cause.

The previously reviewed `giang17/wine` `d2d1-dcomp-11.0` at `dbb8005a228d259f2b3d74f9225eafd832261e0a` is a potential runner source below this independent bridge. Retain the current pinned Proton configuration first. A matching measured runtime problem can justify one coherent, isolated, reversible comparison; no speculative Wine build or yabridge migration. Actual renderer, executable-specific defaults, coherent graphics DLL families and AP11's Wine X11 window-ID capability matter. Top-level presentation optimizations need not benefit child/popup editors, and another Windows DAW's 64-sample result is not our bridge latency.

## Tracked follow-through

- [#72](https://github.com/kasselvania/Linux-VST-bridge/issues/72): residual stalls/lower-latency suitability; the current AP12 terminal reply loss is immediate work in #79.
- [#73](https://github.com/kasselvania/Linux-VST-bridge/issues/73): historical native Bitwig engine-close crash.
- [#74](https://github.com/kasselvania/Linux-VST-bridge/issues/74): prompt notification of known endpoint failure.
- [#77](https://github.com/kasselvania/Linux-VST-bridge/issues/77): lawful Serum authorization/editor/preset qualification.
- [#80](https://github.com/kasselvania/Linux-VST-bridge/issues/80): vendor-editor geometry and redraw follow-through.

These remain normal engineering issues, not new approval systems. Related defects are repairable in the active task; unrelated historical unknowns do not demand repeated campaigns.

## Working standard

One useful outcome, ordinary implementation/debugging, focused tests and one reviewed PR. Preserve projects, vendor environments, credentials, original evidence and dirty local work. Use real plug-ins, not substitute DSP or fabricated state. Keep exact sources and limits visible. Diagnose repeated failures instead of rerunning them without learning; complete product work rather than substituting an information-only report.
