# Current work selection

## Current Pi source reconciliation — PI-R

The operator selected the minimal current-main Pi standalone reconciliation after
the shared-storage repair merged as PR #172. PI-R starts at canonical `main`
`f7668b6ffcb2ae1261d121514aaaed0c9b8986c7`, tree
`00495de1d8653a26e25b331fef1f93348d14606b`. Its basis is the operator's
Pi repair direction, the real-time and slice laws in `AGENTS.md`, GOVERNANCE
“Decisions” and “What evidence means”, ARCHITECTURE §§5.2, 5.7–5.9, 6.3–6.6,
7, 13 and 15, draft PR #171 at `6fe296bd1139b071c9ae6ffb7da628b519309bd2`,
and FC-AUDIO-001/002. PR #171 remains unmerged and is cited as evidence, not
repository-local authority.

**One claim:** the standalone ARM JACK/MIDI frontend and explicit Pi execution
adapter can run against the canonical shared bridge core and Windows host,
using protocol 12, map v1, a 256-frame processing quantum and the existing
512-frame Serum presentation reserve. PI-R may add the minimum Windows host
architecture check needed to preserve the cross-process mapping contract.
Source scope is `rpi0/standalone`, `rpi1/standalone`, source-owned `rpi2` binding
metadata, the bounded `rpi0` backend adapter, and that host handshake. The
commercial fixture is the existing private Serum 2.1.5 module on the Pi 5,
with JACK 48 kHz/512 and its pinned GE Proton/FEX runtime. Build artifacts,
private configuration and state stay outside the repository.

Acceptance requires ARM release build and focused source tests, exact
architecture/identity validation, normal MIDI/audio and state operation,
supervised lifecycle and clean retirement on the Pi. An incorrect layout,
module, runtime, state, or response must refuse explicitly. Retain the actual
physical observations, including any incomplete gate. The comparison arms are
the pre-AS1 `b76c9e2` base plus identical reconciliation R and the post-AS1
base plus R. Establish the runnable pre-AS1 arm before attributing any measured
effect to the storage repair.

This slice does not import BR1/BR1R recovery, change callback policy, runtime,
JACK configuration, governor, graphics, quantum, reserve, queue, timeout,
vendor settings, state identity or installed selections. It does not qualify
musical usability or a performance gain from a successful source build.

The staged [PI-R Serum default-state result](evidence/pi-reconciliation/serum-default-2026-09-25.md)
establishes a runnable current-main Pi source candidate and a matched
three-pair pre/post-AS1 comparison. Mean completed-request service fell by
17–20 µs per pair, while the same 1,280-frame first-note gap remained in all
six runs and CPU ranges overlapped. All sessions retired cleanly. This is
candidate evidence for [PR #173](https://github.com/kasselvania/Linux-VST-bridge/pull/173),
not approval of the old recovery policy, an installed change, or Pi musical
qualification. The next repair must be selected from the repeatable first-note
Windows processing span rather than folded into PI-R.

## Completed shared-audio source slice — AS1

The operator selected a focused, shared audio-execution storage repair. AS1
starts from canonical `main` commit
`b76c9e2270c63c78be2a51adf6562a3f838ffa7f`, tree
`74e13f5b1e10bd4a53d8b626657c403320fb62d2`. Its basis is the operator's
2026-09-25 instruction, AGENTS real-time and slice laws, GOVERNANCE “Decisions”
and “What evidence means”, ARCHITECTURE §§5.2, 5.7–5.9, 6.3–6.6, 7, 13 and 15,
[draft PR #171](https://github.com/kasselvania/Linux-VST-bridge/pull/171) at
`6fe296bd1139b071c9ae6ffb7da628b519309bd2` (its
`docs/ARM_PORTABILITY_REASSESSMENT.md`), FC-AUDIO-001/002 and the
SUPPORT_MATRIX exact processing conditions. PR #171 is not merged into this
source base.

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

The native Linux DAW bridge remains the primary release-driving product.
WD0 merged into this canonical base through
[PR #163](https://github.com/kasselvania/Linux-VST-bridge/pull/163).
[WD1 PR #169](https://github.com/kasselvania/Linux-VST-bridge/pull/169) is
the active, separate managed-Windows-DAW lane; it is not an AS1 dependency or
part of this source change. No Deck-mutating work is selected by AS1.

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
are accepted source and installed behavior. WD0 preserved that native manager
boundary.

The six selected native publications, Serum candidate D,
`x11_touch_routing_v2`, required exact Windows host/source pairs and runner
policies remain current product state. WD0 workspace work did not alter the
native bridge service, publication, proxy, capacity or audio path.

## Completed bounded source slice — WD0

WD0 [#158](https://github.com/kasselvania/Linux-VST-bridge/issues/158)
merged as [PR #163](https://github.com/kasselvania/Linux-VST-bridge/pull/163)
at the AS1 canonical base above. Its bounded result is a managed FL Studio
workspace with repeatable same-workspace installation after clean uninstall,
append-only operation history, a normal managed trial-mode launch, saved-file
presence, and confirmed cleanup. Stock-audio/export behavior and licensed
saved-project recall remain unqualified as recorded in
[the Deck result](evidence/wd0/deck-trial-2026-09-25.md). The original
workspace migrated from schema 1 to schema 2 and installed official FL Studio
`26.1.5.5618`; the earlier install/uninstall receipts remain. The installer
returned a nonzero outer status, retained as historical `needs_user_action`,
while the later managed launch completed with confirmed cleanup. Licensed
saved-project recall remains unqualified in trial mode. The broader issue
#158 remains open; its merged WD0 source slice is complete.
