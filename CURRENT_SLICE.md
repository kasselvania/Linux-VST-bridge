# AP12 completion — repair stalled delivery and finish the musical chain

## Outcome, authority and merge decision

Continue [PR #79](https://github.com/kasselvania/Linux-VST-bridge/pull/79) and [issue #78](https://github.com/kasselvania/Linux-VST-bridge/issues/78) on `codex/ap12-arturia-everyday-use`. Preparation input is `01bc07ce490256cc160751806c01255b7d2aa38d`; main remains `1d5d693e2e2d12547dfd6b02e2b8492b27418592`. [Technical review 5148027460](https://github.com/kasselvania/Linux-VST-bridge/pull/79#pullrequestreview-5148027460) supplies the source-grounded findings. This replaces the now-completed state-repair instructions, not the AP12 product goal. No AP13, second implementation branch, or duplicate PR is selected.

**Do not merge yet:** the reported terminal LoFi delivery timeout is unresolved and the installed musical pair/restart outcome is unverified. Passing individual repair tests does not close those gaps. Complete the existing increment rather than rename an incomplete result.

The operator should open Bitwig normally, load **Arturia Pure LoFi** as an instrument and **Arturia Efx FRAGMENTS** as an effect after it, use both real editors and automation, save, restart, and reopen the same project without setup tooling. Leave the intended installation and project available for use.

Ordinary implementation, necessary builds, targeted fault injection/debugging, reversible bridge-software deployment and focused Deck verification are authorized under [AGENTS.md](AGENTS.md). Preserve dirty local work; do not change another running agent's checkout or experiment. Private implementation choices belong to the engineer. No receipt-writing loop, per-source approval, arbitrary run quota or duplicate diagnostic/acceptance campaign is required.

## Preserve the working baseline

The installed Rust manager, metadata-derived publication, automatic service, independent sessions and mailbox transport already exist. The previous readback/save-refusal/bootstrap/retirement repairs are implemented; preserve them and their regressions rather than reimplement them. Opaque vendor state is distinct from unavailable parameter mirrors; a completed read-only save refusal is not endpoint death; unsafe restore/protocol failures remain terminal. Editor focus must not fabricate parameter invalidation or state capture.

LoFi's edited-state recall and recorded/replayed automation have actual candidate-specific results. The current FRAGMENTS binding was reinspected and republished after the operator changed its module; load/editor/save/close now work, but its last check processed silence. Neither that result nor historical FRAGMENTS evidence proves the current musical pair. See [AP12 findings](docs/AP12.md) and [latest artifact/load/removal evidence](evidence/AP12/operator-replacement-load-removal.json).

Keep original installer results, operator-replacement results and authorization claims distinct. Validate the installed registrations and exact artifacts before use; do not weaken fingerprint checks, change vendor binaries, copy licensing state, reinstall working environments or silently revert to an earlier fixture. A missing authorization dialog is not proof of entitlement. Existing user projects are protected; use a separate project/copy for destructive removal checks.

## First repair: make an unfinished request diagnosable, then fix its cause

The last recorded first fault is `delivery response deadline` at stream position 823,296. Removal's GetState happened afterward. `backend/src/lib.rs::process_events` allows five seconds for this reply; it is not just a 512-frame presentation miss. The two later successful removal checks are not a repair and contain short gaps of their own.

There is a concrete diagnostic gap in `windows-factory-probe/source/delivery_trace.h`: retention happens in `complete()` and is gated by `armed`; `dump()` exports retained completed rows after thread shutdown, not a permanently stuck `current` request. Correct this before another blind live retry.

Extend the existing mechanism with small, fixed-size, safely published in-flight status that an independent owner can retain before containment, without waiting for the stuck Windows delivery/UI thread to finish. Record the exact session/generation/epoch/request identity, last completed stage and relevant outstanding UI/state/lifecycle activity. Distinguish request publication/consumption, vendor processing entry/return, response publication and native receipt. Cover fresh startup and stopped-transport processing before any note or non-silent audio; musical arming must not hide this failure.

Keep this minimal fault-stage visibility available in ordinary installed sessions. Detailed histories, thread sampling and intrusive profiling stay opt-in. Publication/reads must be race-free and bounded; no unsynchronized read of the mutable trace row, allocation or synchronous per-block logging on the delivery path, and no diagnostic work in Bitwig's audio callback. Keep clocks/domains explicit rather than subtracting unrelated timestamps. Retain bounded non-sensitive diagnostics outside disposable transport files; no audio/state payloads or credential dumps. Reuse existing ownership/retirement paths; failure to write a report must not block cleanup.

Use focused production-path tests that intentionally stop a peer at relevant stages and verify that the unfinished identity/stage is retained even when there is no normal completion or shutdown dump. Then run the actual editor/idle/chain workflow with the repaired visibility. Follow the first stalled request to its measured computation, wait, scheduling or lifecycle boundary and implement the demonstrated repair. A targeted thread/wait capture belongs at the implicated stage, not in a blanket permanent profiler.

Do not extend the five-second deadline, raise bridge delay, replay an unanswered request or substitute fake success. A stopped song transport does not imply the plug-in processing loop is stopped. Keep short presentation gaps, terminal reply loss, save refusal and removal as separate events. Preserve healthy siblings and the established state/ownership rules.

If the intermittent failure does not recur, continue the outstanding musical workflow rather than spend the turn on indefinite soak/retries. Retain the useful implementation and product results, state exactly what the new observations exclude, and leave the timeout unresolved for review. An instrumentation-only change or a later clean retry is not a causal fix or full AP12 acceptance.

## Finish the product outcome in the same turn

Use normally Applications-launched Bitwig through established SSH/Moonlight access. Build the ordinary DAW chain from its browser; Bitwig, not a private bridge connection, routes LoFi audio into FRAGMENTS.

Verify real note-driven sound through both Windows instances, with an input-dependent FRAGMENTS effect. Open/focus/close/reopen each actual editor without replacing DSP. Record and replay one vendor-exposed automatable parameter on each device, hands-off on replay, checking the host curve, vendor control and resulting sound. Reuse prior unaffected LoFi checks, but establish that the current pair and FRAGMENTS work together; silent processing is insufficient.

Save the pair project, quit Bitwig, and exercise automatic installed startup after an actual Deck reboot or full graphical-user-session restart. Preserve the project first and coordinate any disruptive desktop/restart action with the operator. A Bitwig-only restart is not the system/session result. Reopen the exact classes, edited sound, chain order and automation with setup tools closed and no checkout/manual preview owner dependency. Remove FRAGMENTS in a test copy while LoFi audibly continues; close/reopen independent instances and finish owned cleanup.

Retain **512 added frames per proxy**. Report per-device bridge and vendor latency, the serial total and actual gap/terminal-failure counters across the relevant actions. Two bridge delays alone are 1,024 frames / 21.33 ms at 48 kHz; distinguish reported/computed latency from a measured physical round trip. Diagnose a new reproducible regression rather than run a broad buffer matrix or advertise a new lower default.

## Deferred editor and runtime work

[#80](https://github.com/kasselvania/Linux-VST-bridge/issues/80) retains FRAGMENTS Advanced expansion and unsmooth parameter-drag redraw. They are not established causes of the timeout. Defer a general graphics overhaul while the required controls remain usable and audio/automation is unaffected. A necessary geometry/control repair is allowed here if it blocks this workflow or shares the demonstrated failure mechanism. Do not assume a fixed canvas: `VendorView::resizeView` already implements plug-in-requested resizing; inspect requested versus actual geometry, scale and `onSize` when relevant.

Retain the pinned Proton/runtime/environment first. The previously inspected `giang17/wine` `d2d1-dcomp-11.0` at `dbb8005a228d259f2b3d74f9225eafd832261e0a` is a source of possible runtime fixes, not a proven fix for our Arturia symptoms. A demonstrated matching runtime problem may justify one coherent, pinned, reversible comparison; no speculative Wine build, mixed graphics DLLs, global overrides or yabridge migration. Recheck actual loaded renderer and Wine X11 window-ID support on an alternate runner. Main-window presentation improvements and another DAW's 64-sample result are not evidence for our editor path or bridge latency.

## Handoff and completion

Keep this PR open until technical review supports both the repaired failure behavior and the actual installed pair/automation/restart/recall workflow. If a real external blocker remains, retain completed sub-results without inventing completion or starting a replacement plug-in hunt. Update the current status and results in this PR; keep all historical failures intact.

Rebuild/test the affected sides and reuse unaffected binaries/evidence. Leave both intended publications, the service, environment and saved project installed; remove temporary experiments and restore unrelated settings. Keep minimal fault reporting active, and turn off temporary heavy tracing/sampling. Report exact deployed sources and remaining limits, then leave #79 unmerged for review.

Full manager polish, universal routing, Serum qualification (#77), a new runtime baseline and a new low-latency default are not prerequisites. Relevant design: [dossier](docs/DESIGN_DOSSIER.md), [architecture](docs/ARCHITECTURE.md), [runtime/recovery](docs/design-dossier/03-activation-flatpak-runtime-and-recovery.md). The task is dependable musical use, not another information-only campaign.
