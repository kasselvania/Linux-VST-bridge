# Current work selection

Canonical source base: main merge commit
`8bb191b28eedee8aabf058b06706860c9fc64933`, tree
`39ec5f35f706ea285707ac6ebb5b26b23e505fa7`. The WD0 branch
`codex/wd0-fl-studio-workspace` integrates that main by a normal merge in
PR [#163](https://github.com/kasselvania/Linux-VST-bridge/pull/163). The
native Linux DAW bridge remains the primary release-driving product.

## Completed native manager slice — UI0

UI0 [#160](https://github.com/kasselvania/Linux-VST-bridge/issues/160) is
merged and physically accepted on the Steam Deck. The installed immutable
manager/frontend generation is
`29e52e7537c2e150a68db7f66799621a94d39b7aac7b20e22aded0d03a28facf`;
its sanitized [Deck result](evidence/ui0/deck-acceptance-2026-09-24.md) records
ordinary and narrow navigation, one Serum 2 session, manager close/reopen,
continued Bitwig audio and clean retirement. The previous manager/frontend
generation remains a rollback target. Home, Plug-ins, Workspaces, Activity,
Setup and Diagnostics, including `ActivityCertainty` and exact session routing,
are accepted source and installed behavior. WD0 must preserve them.

The six selected native publications, Serum candidate D,
`x11_touch_routing_v2`, required exact Windows host/source pairs and runner
policies remain current product state. WD0 workspace work does not alter the
native bridge service, publication, proxy, capacity or audio path.

## Active Deck-mutating slice — WD0

WD0 [#158](https://github.com/kasselvania/Linux-VST-bridge/issues/158)
implements a managed FL Studio workspace under
[docs/WD0.md](docs/WD0.md),
[docs/WINDOWS_DAW_WORKSPACES.md](docs/WINDOWS_DAW_WORKSPACES.md),
[docs/WORKSTREAMS.md](docs/WORKSTREAMS.md), `AGENTS.md` and
`docs/ARCHITECTURE.md`. Its connected outcome remains a normal managed FL
install, launch, stock-project playback/export, clean relaunch and licensed
recall where the vendor permits. Source and installed results stay separate.

The active WD0 correction is a lifecycle bug: a durable workspace currently
retains its first installation operation after clean uninstall and then
refuses every later install. **Workspace lifetime is not installation
lifetime.** The primary claim of this correction is that the manager can
select an exact admitted FL installer and install again in the same workspace
after clean uninstall, retaining every earlier install/uninstall operation and
its evidence. Same-version reinstall and controlled version change must both
work. The current installed application, selected installer, active operations
and append-only history must remain distinct facts.

The exact physical fixture is the existing manager-owned FL workspace on the
Steam Deck: schema-1 state, cleanly `Uninstalled`, prior install and uninstall
evidence retained, with its mutable prefix, projects, preferences, exports,
account state and selected runtime preserved. This workspace is not reset or
recreated. Before source rereview and explicit Deck custody confirmation, only
source work, tests and read-only inventory are permitted; do not execute an FL
installer or mutate the live workspace.

In scope: `bridge-manager` workspace state/CLI, deterministic fail-closed
schema-1 migration, closed installer selection from canonical custody,
repeatable install/uninstall history, truthful status, and only the minimal
canonical manager-UI Workspaces projection needed to show manager-offered
actions. Focused regression tests and WD0 documentation are in scope. No
arbitrary path, executable, PID, process name, environment, arguments or
external-install adoption is admitted. No in-place auto-upgrade or destructive
fresh-prefix reset is added.

Source acceptance requires same-version reinstall, installer-B version change,
active-operation and cleanup refusal, retired-failure retry, exact schema-1
migration, no history erasure, no unmanaged executable adoption and native
catalogue/publication isolation. Run the full manager suite, binary and runtime
tests, strict manager Clippy, relevant frontend tests/Clippy, AP12, PX2, package
tests and `git diff --check`. Negative acceptance: no duplicate or overwritten
operation; no uncertain old attempt treated as clean; no workspace, prefix or
user-root replacement; no native bridge change. Do not repeat Serum touch,
AP17, Ubuntu, Game Mode, plug-in scan or FL audio qualification for this
state-model source change.

After source rereview and custody confirmation, physical acceptance uses that
existing clean-uninstalled workspace: preserve prestate, choose the exact new
admitted installer/release, install through the manager, prove the new operation
and executable/version while retaining the same workspace and all old history,
launch through the normal route, then quit cleanly. A basic stock audio check
belongs only to the existing WD0 workflow when it is ready. The lifecycle
correction does not by itself qualify FL Studio in `docs/SUPPORT_MATRIX.md`.
