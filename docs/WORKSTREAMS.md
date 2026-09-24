# Product workstreams

Decision date: 2026-09-24. This is the work-allocation map, not a compatibility certificate or live-machine status report. [SUPPORT_MATRIX.md](SUPPORT_MATRIX.md) and [FAILURE_CLASSES.md](FAILURE_CLASSES.md) retain those separate roles.

## One product, separate execution lanes

| Workstream | Role | Next outcome | Boundaries |
|---|---|---|---|
| Native Linux DAW bridge | Primary release-driving product | Continue the usable Deck fleet, shared touch repair, mixed-product usage and manager usability; retain Ubuntu as the portability reference | Existing proxy, Windows host, audio/state transport and support limits remain authoritative. REAPER needs its own host coverage; Bitwig evidence is not REAPER evidence. |
| Managed Windows DAW | Active parallel implementation; FL Studio first | [WD0](WD0.md): a managed FL Studio workspace that launches, plays a stock project, exports, closes and relaunches; licensed project recall completes the full gate | [Windows DAW workspace architecture](WINDOWS_DAW_WORKSPACES.md). No Linux proxy in this audio path; no changes to the existing six plug-in environments. |
| ARM/manufacturer appliance | Separately owned parallel experiment | Continue its already-selected appliance work | No FL-on-ARM claim, no dependency on WD0, no import of unmerged appliance code into the DAW lane. |

FL Studio is selected first. Ableton Live follows a demonstrated FL workspace and is a different application qualification, not a second simultaneous DAW bring-up. Max for Live, hardware-control integrations, new distributions and a larger plug-in set do not become WD0 completion conditions.

The Windows lane starts now; it does not wait for universal touchscreen support, a new manager frontend, or elimination of every native-bridge underrun. Mouse/trackpad is the first FL input posture. Conversely, WD0 must not consume the native lane's running experiment or redefine the native product as unfinished until FL works.

## Branch and machine ownership

- Product implementation stays in `kasselvania/Linux-VST-bridge`. Do not create an independent FL-on-Wine product repository or a second compatibility database.
- Use one branch/worktree and one implementation PR per selected outcome. `CURRENT_SLICE.md` on an implementation branch names that branch's task; it is not an exclusive reservation of all project work.
- Integrate from reviewed canonical main. Reuse findings from an unmerged branch without silently consuming its implementation. Any necessary code dependency must be explicit and reviewed.
- Each lane owns separate mutable environment roots, launch/session ownership and evidence. Reuse immutable runners/artifacts only through verified references.
- The Deck remains the lead physical platform. Before a physical run, coordinate with the current custodian, require the relevant projects saved/closed, and verify no conflicting experiment or uncertain owner. Do not concurrently benchmark, install or trace unrelated workloads on the same machine.
- Builds and source work can proceed in parallel. GUI use, service replacement and device/audio-setting changes must be serialized by the task custodians, without inventing a new scheduling daemon or permission-receipt system.

## Installed state is not the same as main

The documentation baseline is canonical main `05cb957e0174c3437aac8934b89294e57b53308f`. At this decision, Serum PR #151 remains unmerged while candidate C is installed on the Deck. Its runner policy is not thereby present in canonical main.

WD0 must not replace the working shared bridge service with a plain-main build that cannot admit the installed candidate. Use the canonical product's new workspace-scoped command/launcher and private WD0 application state, without rewriting the native-bridge software catalogue or service selection. A later common-package replacement requires explicit reconciliation of all installed policies and retained host/source pairs, including FC-MGMT-002. Separate application state is an ownership boundary within one product, not permission to fork the management implementation.

## Progress and review rule

A selected implementation outcome includes ordinary code, builds, focused tests, diagnosis and in-scope corrections. Preserve the first failure, learn from it and continue the repair; do not return for a new tech-lead gate after every successful substep. Ask only for a genuinely new security boundary, spending decision, destructive action, secret-entry action, or change of goal.

Review the complete implementation and observed result before merge. Do not merge a launch-only or mock-only result as a working DAW. A vendor licensing limit or unavailable device gets an explicit partial result, not a fabricated pass and not another infrastructure campaign.

For shared fixes, update the relevant failure-class card and affected support rows in the implementation PR. Do not add FL Studio or Ableton as supported merely because these workstreams were selected.
