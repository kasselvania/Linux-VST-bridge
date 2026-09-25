# Product workstreams

Decision date: 2026-09-24. This is the work-allocation map, not a compatibility
certificate or live-machine status report. [SUPPORT_MATRIX.md](SUPPORT_MATRIX.md)
and [FAILURE_CLASSES.md](FAILURE_CLASSES.md) retain those separate roles.

## One product, separate execution lanes

| Workstream | Role | Next outcome | Boundaries |
|---|---|---|---|
| Native Linux DAW bridge | Primary release-driving product | Operator Desktop-mode soak, focused manager frontend, then GM0 with one known-good Serum instance | Existing proxy, Windows host, audio/state transport and support limits remain authoritative. Bitwig evidence is not REAPER evidence. |
| Managed Windows DAW | Active FL workspace lane | WD0's bounded trial-mode reinstall/launch result is complete. [WD1](WD1.md) installs one direct-hosted Serum 2 Windows VST3 and checks FL scan, music/export, clean retirement and fresh-session use. Licensed FLP recall remains deferred in trial mode. | [Windows DAW workspace architecture](WINDOWS_DAW_WORKSPACES.md). No Linux proxy in this audio path; no changes to the existing six plug-in environments. |
| ARM/manufacturer appliance | Separately owned parallel experiment | Continue its already-selected appliance work | No FL-on-ARM claim, no dependency on WD0, no import of unmerged appliance code into the DAW lane. |

FL Studio is selected first. Ableton Live follows a demonstrated FL workspace
and is a different application qualification, not a second simultaneous DAW
bring-up. Max for Live, new distributions and a larger plug-in set do not become
WD0 completion conditions.

The Windows lane starts now; it does not wait for universal touchscreen support,
a new finished manager frontend or elimination of every native-bridge underrun.
Mouse/trackpad is the first FL input posture. Conversely, WD0 must not consume
the native lane's running experiment or redefine the native product as unfinished
until FL works.

## Current allocation

### Operator / product-use lane

Use the current six products normally in Steam Deck Desktop Mode. Preserve only
useful failures and friction rather than turning the period into another test
matrix. A note should name the product, action, input method, whether audio
continued, what happened to the editor, and whether normal close cleaned up.

### FL Studio owner

One dedicated agent owns WD1's first Windows VST3 in the existing WD0 workspace.
While another custodian owns the physical Deck, the
FL agent may:

- reconcile the WD0 branch with canonical main;
- inspect and implement the typed workspace owner;
- run source tests and isolated unlicensed fixtures;
- read-only identify and fingerprint the installer in `Downloads`;
  - prepare the exact product installer and source-owned workspace projection.

It may not execute the installer, replace the shared manager/service or change
machine audio/Steam settings until it receives the physical Deck handoff.

### Manager frontend owner

A separate focused frontend slice first studies the current manager and presents
two or three concrete product-interface directions to the operator. It then
implements the selected direction as a projection of existing manager truth:
product readiness, available action, running sessions/capacity, incidents,
service health and recovery. It must not invent new lifecycle, installation,
publication or recovery authority merely to make the UI look complete.

Checkpoint that work at a coherent PR before moving the same owner onto GM0.

### GM0 owner

After the Desktop-mode soak, GM0 owns the one-device Gaming Mode transition:
Desktop clean state, switch to Gaming Mode, normal Bitwig launch under Gamescope,
one Serum candidate-D instance, editor/trackpad/touch/audio, clean quit and
return to a healthy Desktop session. FL Studio, six-device concurrency, suspend,
external displays and broad recovery are outside GM0.

## Branch and machine ownership

- Product implementation stays in `kasselvania/Linux-VST-bridge`. Do not create
  an independent FL-on-Wine product repository or a second compatibility
  database.
- Use one branch/worktree and one implementation PR per selected outcome.
  `CURRENT_SLICE.md` on an implementation branch names that branch's task; it
  is not an exclusive reservation of all project work.
- Integrate from reviewed canonical main. Reuse findings from an unmerged branch
  without silently consuming its implementation. Necessary code dependencies
  must be explicit and reviewed.
- Each lane owns separate mutable environment roots, launch/session ownership
  and evidence. Reuse immutable runners/artifacts only through verified
  references.
- The Deck remains the lead physical platform. Before a physical run, coordinate
  with the current custodian, require relevant projects saved/closed, and verify
  no conflicting experiment or uncertain owner.
- Builds and source work can proceed in parallel. GUI use, installation, service
  replacement, runner/environment transitions and device/audio-setting changes
  are serialized by the task custodians. Do not invent a scheduling daemon or a
  new permission-receipt framework.

## Installed and canonical state

Canonical main `a4e140a53cedb8b65b487a45d6fea0e44caea74d`, tree
`b3f3d264901267e2dcb723b9904c464baf6eefae`, includes the physically accepted
Serum candidate-D runtime support and `x11_touch_routing_v2` policy.

The Deck's current working generation, selected Serum candidate D and the other
five publications remain user-facing product state. WD0 workspace-only commands
must leave the shared native-bridge service and software catalogue untouched.
Any later common-package replacement must explicitly preserve every installed
runner policy and required host/source pair, including FC-MGMT-002.

Separate application state is an ownership boundary within one canonical
product, not permission to fork the management implementation.

## Progress and review rule

A selected implementation outcome includes ordinary code, builds, focused tests,
diagnosis and in-scope corrections. Preserve the first useful failure, learn
from it and continue the repair; do not return for a new tech-lead gate after
every successful substep. Ask only for a genuinely new security boundary,
spending decision, destructive action, secret-entry action or change of goal.

Review the complete implementation and observed result before merge. Do not
merge a launch-only or mock-only result as a working DAW. A vendor licensing
limit or unavailable device gets an explicit partial result, not a fabricated
pass and not another infrastructure campaign.

For shared fixes, update the relevant failure-class card and affected support
rows in the implementation PR. Do not add FL Studio or Ableton as supported
merely because these workstreams were selected.
