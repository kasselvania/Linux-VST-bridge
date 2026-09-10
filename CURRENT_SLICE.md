# AP15 — Direct detached vendor-editor lifecycle

## Outcome and authority

Make the ordinary bridged editor behave like an invisible compatibility layer:

```text
Bitwig opens the plug-in editor
-> the exact vendor editor appears or receives focus directly
-> closing the vendor window retires only that editor generation
-> DSP, automation, state and project identity remain alive
-> reopening presents exactly one fresh vendor editor for the same DSP instance
```

There must be no visible bridge-owned **Open / focus vendor editor** / **Close vendor editor** intermediary and no second user action.

This slice activates [issue #84](https://github.com/kasselvania/Linux-VST-bridge/issues/84) from integrated AP14 `main` at `84abd6a9755ab10b015c3744549b1e46ab1b85cb` on branch `codex/ap15-direct-editor-lifecycle`. Continue in one implementation PR against `main`, leave it unmerged for independent technical review, and preserve the installed exact Arturia fixture.

AP14 is accepted and integrated. Its exact Pure LoFi and Efx FRAGMENTS profile revision-3 claims remain `verified_exact_fixture`; their native-panel editor posture, artifact bindings and retained rollback history are immutable. **512 added frames per proxy remains supported and recommended. 256 remains available but unqualified.**

Implementation, focused deterministic tests, necessary builds, reversible qualification deployment and proportionate Steam Deck evidence are authorized under [AGENTS.md](AGENTS.md). Preserve vendor files, authorization, saved projects, prior publications, useful failures and unrelated dirty work.

## Current implementation boundary

`native-vst3-proxy/source/vendor_panel.h` currently supplies the Linux VST3 `IPlugView` Bitwig attaches. It creates a mapped 560×150 X11 child, draws the two bridge buttons, invokes `PanelOwner::panelOpen` / `panelClose`, forwards focus through the owner, and closes the vendor editor from `removed()`.

`native-vst3-proxy/source/commercial_controller.h` creates that view and owns the native editor request/status bridge. `native-vst3-proxy/include/ap11_gui.h` defines the fixed UI ABI (`Open`, `Close`, `Focus`, `EditorStatus`, activation context and view epoch), while `native-vst3-proxy/backend/src/gui.rs` and `queued.rs` own the mapped native session/channel. On Windows, `windows-factory-probe/source/gui_channel.h`, `editor_session.h` and `vendor_view.h` own the mapped channel, generation-aware service loop and detached Win32 vendor view. AP14’s persistent hidden `environment_desktop.h` owner must remain intact.

Deleting the panel or returning no view is not an accepted design. Bitwig still owns a VST3 view lifecycle, and the bridge needs a host-correct adapter. A zero-sized, unmapped, transparent or otherwise minimal native delegate is a hypothesis to test, not an assumption.

## Product laws

### Direct presentation

- One successful DAW attach/open produces one open-or-focus request for the exact processing instance.
- Repeated attach, focus or show activity must focus the existing vendor editor rather than create a duplicate.
- The bridge-owned native delegate must not appear as an ordinary visible editor surface.
- Editor availability must never become a prerequisite for DSP admission or audio processing.

### Bidirectional close

- User close of the vendor window must produce an exact editor-closed result for the current generation.
- DAW view removal/close must request retirement exactly once.
- Plug-in/device removal retires the editor before instance and transport retirement under the existing ownership law.
- Closing an editor must not remove the device, terminate healthy DSP, discard opaque state, invalidate automation or change stable processor/controller identity.

### Reopen and generation safety

- Reopen after either close route creates exactly one fresh editor generation for the same DSP instance.
- Late resize, focus, status or close messages from a retired generation are refused or ignored without affecting the current generation.
- A stale native view or vendor handle cannot close or focus a newer editor.
- Repeated open/focus/close cycles must not leak X11 windows, Win32 windows, timers, callbacks, controller references, processes, leases or transport ownership.

### Failure isolation and truth

- Vendor editor refusal, window loss or editor-path crash remains distinct from DSP/process failure.
- Healthy processing continues when the established architecture permits it.
- Native and Windows status must distinguish opening, open, focused, closing, closed, refused and failed outcomes without fabricating success.
- No editor request, wait, logging, allocation, X11/Win32 operation or filesystem work enters the audio callback.

## Host and platform correctness

Respect:

- VST3 `IPlugView` attach/remove, frame, sizing and thread requirements;
- Bitwig/X11 run-loop registration and removal;
- Win32 owner-thread and message-pump requirements;
- activation and focus without fake parameter invalidation or state capture;
- plug-in-requested resize, scale/DPI, child windows, popups and menus;
- process-scoped Windows accessibility selection;
- AP14 environment-desktop lifetime and service ownership;
- exact instance, controller and editor-generation identity.

The independent detached-editor architecture remains selected. This slice does not authorize embedding or reparenting the Windows editor into Bitwig, switching runners, adopting yabridge, or redesigning transport/state.

The standalone `vendor-editor INSPECTION.json` route remains a separate editor-only/no-DAW-audio/no-project-recall surface. Do not change it implicitly. Any shared mechanical improvement must preserve its distinct admission and cleanup contract.

## Profile and publication transition

AP14 revision 3 is immutable and accurately means:

```text
editor = detached_owner_thread_with_native_panel
```

A native/editor behavior change requires a new exact artifact identity and a new profile revision/capability. Add the smallest closed editor capability needed for direct detached lifecycle. Do not mutate or relabel revision 3.

A new `review_candidate` must not become ordinary activation authority before independent review. Qualification on the Deck may use an existing bounded development surface or a narrowly scoped reversible qualification route, but it must:

- preserve the revision-3 publication as the exact known-good rollback target;
- remain explicit and non-default;
- avoid weakening `SelectionPurpose::Activation` or `Manager::managed_publish` claim checks;
- avoid creating a general arbitrary-profile execution bypass;
- leave exact physical/provenance/rollback evidence.

The final verified profile revision is selected only after implementation evidence and independent review. Retained candidate and verified revisions remain immutable.

## Deterministic verification

Add or extend deterministic fixtures to cover at least:

1. DAW attach requests one editor generation without a user button action.
2. Repeated attach/focus requests do not duplicate the editor.
3. Vendor-window close produces an exact closed notification and leaves DSP ownership alive.
4. DAW view removal closes the current editor exactly once.
5. Reopen after either route creates one new generation for the same instance.
6. Late messages from a retired generation cannot affect the replacement.
7. Editor refusal/failure is isolated from processing and reported truthfully.
8. Device removal orders editor retirement before instance/transport retirement.
9. Repeated cycles leave no native/Windows handles, timers, callbacks, references or leases.
10. Existing resize, popup, parameter gesture, state and automation behavior remains intact where touched.
11. The native delegate satisfies Bitwig/VST3 lifecycle expectations without presenting the bridge controls.
12. Existing AP11–AP14 editor, failure, state, ownership and callback tests remain meaningful.

Use production helpers rather than a parallel editor implementation. Preserve exact failure cases; do not turn a failed close into a successful status merely to simplify the state machine.

## Exact Steam Deck validation

Use the installed SteamOS 3.8.16 / Bitwig 6.1 / pinned Proton-SLR Arturia fixture and protected project copies.

Before mutation:

- record integrated source, installed software/profile/publication identities and current physical links;
- ensure no active DSP lease before publication/native changes;
- preserve revision-3 targets, vendor state, authorization, original projects and environment keeper;
- keep 512 selected.

Then demonstrate for both Pure LoFi and Efx FRAGMENTS:

1. One ordinary Bitwig editor action directly presents the real vendor window with no visible bridge panel or second click.
2. Repeated open/focus activity keeps exactly one vendor editor.
3. Closing the vendor window leaves audio/DSP and the same project instance alive.
4. Reopening creates one fresh editor generation for that same instance.
5. Closing Bitwig’s editor view closes the vendor editor exactly once.
6. Multiple cycles retire cleanly without stale windows, processes, leases or callbacks.
7. Editor failure/refusal remains isolated and visible without damaging the project.
8. A bounded save/reopen and parameter/automation check confirms no touched behavior regressed.
9. A healthy sibling remains independent during editor close/reopen or failure.

Use local on-device observation for window behavior; remote-control transport is not frame-rate or physical-latency evidence. A service restart and ordinary Bitwig relaunch are proportionate. A full reboot is required only if startup/persistence code changes or readback leaves a real uncertainty.

## Explicit non-goals

AP15 does not include:

- FRAGMENTS Advanced expansion/redraw issue #80 unless the same demonstrated lifecycle mechanism owns the defect;
- residual delivery/performance issue #72 or promotion of 256;
- Serum authorization/qualification issue #77;
- a Wine/Proton/runtime migration;
- embedded Windows editors, Wayland hosting or a GUI toolkit;
- general profile distribution, consumer installer or remote marketplace;
- sidechains, arbitrary buses, float64, broad MIDI/MPE, CLAP or another DAW;
- a transport, state or publication rewrite.

## Completion report

Update the single implementation PR with:

- exact final head, parent and tree;
- the selected native `IPlugView`/editor lifecycle mechanism and rejected alternatives;
- changed source ownership and generation/state laws;
- profile candidate/revision and exact native/host artifact identities;
- deterministic tests and all relevant CI results;
- exact Deck open/focus/vendor-close/DAW-close/reopen/failure evidence;
- physical publication/rollback and installed state left behind;
- preserved projects, vendor state and authorization;
- known limitations and nonclaims;
- explicit confirmation that 512 remains supported/recommended and 256 unqualified.

Leave the PR open and unmerged for independent technical review.
## Implementation checkpoint (not acceptance)

### Current qualification authorization

Technical review `5161191208` and the current operator work order supersede the historical capacity restriction below. All four hosted lanes at `0d82c75` passed. Windows artifact run `34418620336` records synthetic merge commit `46c7b85fce2370644cccec6197a6f1a06c188be1` with the identical reviewed tree. The finite exact ReviewCandidate roster may now be staged and exercised through `qualify-editor`, preserving ordinary verified-only activation and exact revision-3 rollback. Restore revision 3 after the bounded actual Bitwig qualification. Keep PR #86 draft until that campaign completes; never relabel a candidate verified in place.

### Historical source-only checkpoints

Local AP15 lifecycle/GUI-ownership changes and a sealed engineering qualification route are in progress on the existing PR #86. The exact Windows artifact is unavailable because GitHub jobs cannot start under the account's current billing/spending allowance. The operator authorized continuing locally and deferring Windows issues. The candidate roster remains closed with `qualification_artifacts_pending`; no candidate profile is verified or ordinarily activatable. The unmapped noninteractive delegate has SDK/Xvfb fixture evidence but still needs actual Bitwig acceptance. Keep this PR draft. See `docs/AP15.md` and `evidence/ap15/` for performed checks and untouched installed revision-3 state; the acceptance criteria above remain outstanding where no exact evidence exists.

The current repair pass is limited to source ownership hardening and available local validation, as requested against `dd596b3e`. Windows capacity remains exhausted. Do not populate the qualification roster, deploy native AP15 builds, change revision-3 publications, or mark PR #86 ready. UI ABI 4 / mapped UI 5 adds explicit callback origins; the repair and available results are recorded separately in `docs/AP15.md` and `evidence/ap15/source-repair/`. Required Windows builds/tests and actual Bitwig qualification remain outstanding.

Focused R2 from technical review `5160946354` separates XCB unmap, host-close send and flush outcomes. A rejected/exhausted unmap must still request the live host parent's retirement. The exact failing-before/passing-after Linux fixture and refreshed validation are recorded under `evidence/ap15/r2/`. This remains a source-only checkpoint: `AP15_SOURCE_CLEAR_WINDOWS_BLOCKED`, `BITWIG_QUALIFICATION_BLOCKED`, `AP14_BASELINE_PRESERVED`. Keep PR #86 draft and unmerged; no AP15 installation/publication or qualification roster is authorized in this pass.

### Current qualification result (independent review pending)

Revision-4 attachment and revision-5 focus failures were reproduced and narrowly repaired on Linux. Revision-6 sealed candidates were exercised through the engineering route in normally Applications-launched Bitwig: both direct editors, same-session vendor/DAW close and reopen, gestures, existing automation, explicit save/relaunch recall and independent FRAGMENTS removal. Both exact verified revision-3 targets were then physically restored. Candidate claims remain `review_candidate`; no ordinary activation eligibility changed. See `evidence/ap15/qualification/` and `docs/AP15.md` for exact artifacts, sessions, gaps, rollback and limitations. This current result supersedes the historical blocked checkpoints above, without rewriting them as passes. Independent review and merge remain outstanding.
