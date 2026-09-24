# Current work selection

UI0 base: canonical main commit `9f37338140a61e838e83d757628b6cb914b83bb5`, tree
`3bb22f1d9971c97f01dc65a8227f1a7d7f727f54`. This branch does not supersede
another task's branch-local slice or grant ownership of its checkout or live
experiment.

## Native-Linux product: current sequence

The native-Linux DAW bridge remains the first release-driving product. Serum 2
candidate D and the shared per-window X11 touch route are merged and physically
accepted for the exact Deck waveform-popup interaction. Current support and
remaining shared failures are maintained in
[docs/SUPPORT_MATRIX.md](docs/SUPPORT_MATRIX.md) and
[docs/FAILURE_CLASSES.md](docs/FAILURE_CLASSES.md).

The selected near-term sequence is:

1. the operator uses the current six-product Deck fleet in Desktop Mode and
   records only useful real-use failures or friction;
2. UI0 improves manager product readiness, live-session, incident and recovery
   presentation without inventing new backend authority;
3. after that Desktop-mode soak, GM0 validates one known-good Serum instance
   through the Desktop-to-Gaming-Mode graphical/audio/session transition.

These are product-development steps, not instructions to replay the historical
qualification campaigns.

## Active native manager slice — UI0

Issue: [#160](https://github.com/kasselvania/Linux-VST-bridge/issues/160).
Branch: `codex/ui0-manager-first-lift`.
Selected direction: a music-first Home for the native plug-in bridge, with a
strict separate Workspaces area for managed DAWs. The Workspaces view may explain
its empty state, but must not show an FL Studio card, status or action until the
canonical manager projects a real workspace.

Primary claim: at 960×720 and a narrow touch-friendly width, the existing
Rust/egui frontend makes canonical bridge readiness, capacity, published
products, live activity, attention and setup work understandable while
preserving every manager-offered action, refusal, request feedback, exact
reconciliation, history and technical detail.

Basis: `AGENTS.md` (management authority and workstream rules),
`docs/ARCHITECTURE.md` (management plane and canonical readback),
`docs/MF1.md` (operator snapshot and actions), `docs/MF2.md` and
`docs/MF2-R2.md` (installer custody and uncertain acknowledgment),
`docs/SUPPORT_MATRIX.md` and `docs/FAILURE_CLASSES.md` (bounded product claims
and incident gaps), plus merged `docs/WORKSTREAMS.md`,
`docs/WINDOWS_DAW_WORKSPACES.md` and `docs/WD0.md` (the separate DAW lane).

In scope: `manager-ui` projection, navigation, source-owned previews and focused
tests; a read-only canonical session identity in the operator snapshot to
distinguish exact Activity rows. No install, publication, recovery, runner,
service-admission, DAW-workspace, Game Mode, plug-in or audio behavior change. No
arbitrary execution path.

Fixture: the repository's synthetic `manager-ui/examples/library-preview.json`
at 960×720 and 560×720 in light and dark system visuals on the source machine.
It does not represent an installed Deck readback. Only `manager-ui`, the
read-only `bridge-manager` session projection, this slice document and UI0
evidence may change; no dependency or schema version change is permitted.

Acceptance: show healthy, busy, unavailable and cleanup-uncertain system states;
ready, running, unpublished, experimental and attention products; active and
recent terminal sessions; attention links to exact affected records; visible
request feedback and touch-readable refusal reasons; all former setup, vendor,
incident, diagnostic and technical surfaces reachable. Verify one click sends
one request and reconciliation never resubmits. Render and inspect ordinary and
narrow source-owned captures. Run manager-ui tests and strict Clippy, plus
manager tests for the read-only projection. Commit and push one draft PR with
actual evidence, exact source head/tree, remaining limits and the next slice.
Negative acceptance: no duplicate request after uncertain acknowledgment, no
false active-DSP claim from unavailable capacity or cleanup uncertainty, no
invented FL workspace, and no executable escape hatch. Retain source-owned
captures under `evidence/ui0/`. Rollback is a source revert; no live machine
state is touched by this slice.

## Parallel Windows-DAW outcome

The selected Windows-DAW implementation is
[WD0 — managed FL Studio stock-project workflow](docs/WD0.md), governed by
[docs/WINDOWS_DAW_WORKSPACES.md](docs/WINDOWS_DAW_WORKSPACES.md). It proceeds in
parallel with the native bridge and the separately owned ARM appliance.
Coordination and integration rules are in
[docs/WORKSTREAMS.md](docs/WORKSTREAMS.md).

WD0 must deliver a real managed install, normal launch, stock-project playback,
export, clean relaunch and licensed project recall in a new FL workspace. It is
not a Linux VST proxy path and not a generic Windows-program launcher.

The operator reports that the official Windows installer is present in the
Steam Deck user's `Downloads` directory. That statement admits no filename,
version, path alias, digest or signature yet. The WD0 owner must identify and
fingerprint the exact file read-only before execution, preserve the original
bytes, and then import it through the managed installer boundary. The installer,
credentials, license material and private projects never enter Git.

## Shared Deck ownership

Source work and isolated builds may proceed concurrently. Only one task custodian
may mutate or physically exercise the Deck at a time. Installing software,
replacing the manager/service, changing a runner or environment, changing audio
or Steam launch settings, and running a physical acceptance session require an
explicit handoff from the current custodian.

The WD0 owner may complete source, tests, installer admission and private
workspace preparation while the operator uses Bitwig. It must not replace the
installed manager generation, stop another task's session or execute the FL
installer until it receives the physical-work window. UI0 likewise must not
replace the installed frontend or begin physical acceptance before its Deck
handoff.

Every common product package must preserve the now-canonical
`x11_touch_routing_v2` policy, Serum candidate-D authority, all required exact
Windows host/source pairs and the other five selected publications.

## Retained completed slice

The completed Serum touchscreen slice remains available at the exact pre-WD0
main pointer and in
[evidence/serum-x11-touch-routing](evidence/serum-x11-touch-routing/). Its
accepted result is current product authority; it is not an instruction to rerun
Serum before WD0 or GM0.
