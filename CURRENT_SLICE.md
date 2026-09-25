# Current work selection

## Parallel shared-audio source slice — AS1

The operator selected a focused, shared audio-execution storage repair while
WD0 remains the separate managed-Windows-DAW lane below. AS1 starts from
canonical `main` commit `b76c9e2270c63c78be2a51adf6562a3f838ffa7f`, tree
`74e13f5b1e10bd4a53d8b626657c403320fb62d2`. Its basis is the operator's
2026-09-25 instruction, AGENTS real-time and slice laws, GOVERNANCE “Decisions”
and “What evidence means”, ARCHITECTURE §§5.2, 5.7–5.9, 6.3–6.6, 7, 13 and 15,
PR #171's `docs/ARM_PORTABILITY_REASSESSMENT.md`, FC-AUDIO-001/002 and the
SUPPORT_MATRIX exact processing conditions.

**One source claim:** after session setup, the selected shared processing
request/reply path uses reusable, bounded per-instance bridge storage without
recurring allocation, reallocation or deallocation. The covered path is the
native transport worker's audio mapping, event/context request encoding,
mailbox or socket frame exchange and result decode, plus the Windows host's
request decode and result publication. Protocol minors 5 and 7–13, float32,
negotiated blocks 0–256 where admitted, up to 256 input events, 64 returned
events, 128 parameter points and 4096 returned payload bytes are the bounded
source contract. Protocol, quantum, reserve, queue, state, recovery policy,
checks and callback ownership are unchanged. SDK/vendor/runtime allocations,
setup/state/lifecycle/error paths and historical protocol minors 1–4 are not
part of this claim.

Changed implementation scope: `native-audio-client` mapping/frame/event and
endpoint helpers; `native-vst3-proxy/backend` session/context/mailbox storage;
Windows `ap1_protocol.h`, `delivery_mailbox.h` and `mapped_processing.cpp`;
focused tests and this slice entry. No runner, FEX, Wine, graphics, governor,
priority, JACK, capacity, timeout, policy, vendor setting, plug-in, frontend
or installed generation change is admitted. Acceptance requires exact wire
equivalence, first-block and repeated zero bridge allocation counts, maximum
valid traffic, zero/partial blocks, multi-output checks, malformed responses,
independent-instance ownership and a Windows production build. Failure must
remain explicit and must not release a mailbox slot before its reply is copied.

The exact Pi comparison fixture is a Raspberry Pi 5 at JACK 48 kHz/512 using
the existing supervised standalone path and Serum 2.1.5 module digest
`501e7bb3dd9cafe416b3412df3d4e084c01b7468201e5690b9009d7ecd4e5283`,
Windows host digest `4ab203ab8aa25555eebce6782cd52a11643a1b935abe8a2ce0cb04baeb453161`,
protocol 12, 256-frame processing quantum and 512-frame presentation reserve.
The BR1 gate's unnamed preset is not a reproducible fixture identity. No AS1
physical comparison may be attributed to this patch until a runnable
current-main/Pi source pair has been reconciled in a separate prerequisite,
built with matching options, and baselined before applying the AS1 change.
The Pi fork's map-v2/512-capacity specialization and current main's
map-v1/256-capacity plus protocol-13 multi-output support must be reconciled
without importing BR1/BR1R recovery policy. Keep all installed generations,
private state and experimental candidates in place. A source pass is not a
musical or performance qualification; record the physical result or exact
blocker in the implementation PR.

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

The exact physical fixture was the existing manager-owned FL workspace on the
Steam Deck: schema-1 state, cleanly `Uninstalled`, prior install and uninstall
evidence retained. The controlled Deck handoff installed official FL Studio
`26.1.5.5618` into that same workspace after deterministic schema-2 migration.
The old install/uninstall receipts remain, a new install operation and exact
executable are recorded, and a normal managed FL launch has a completed,
cleanup-confirmed receipt. Current status is `ready`. The installer itself
returned a nonzero outer status, retained as historical `needs_user_action`;
this is not relabeled clean installer completion. Exact evidence and trial
limits are in [the Deck result](evidence/wd0/deck-trial-2026-09-25.md).

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

The repeatable-install lifecycle gate has a physical result on the original
workspace. WD0's trial-mode musical result and its exact audio/backend/export
claims remain separate from that lifecycle evidence. Licensed saved-project
recall is deferred while the operator uses FL in trial mode. WD1 begins only
after PR #163 records and merges its honest bounded WD0 result; WD1 will install
one Windows VST3 into FL's existing workspace without changing FL's application
installation slot or the six native publications.
