# Shared failure classes

This ledger is the current map from reusable failure mechanisms to fixes, artifacts, physical coverage, and remaining gaps. It is organized by shared boundary rather than by chronological campaign or plug-in name.

Historical AP/UIO/UIR/IF documents remain authoritative for what a particular experiment observed. This ledger is authoritative for the project's current cross-plug-in understanding.

## Status vocabulary

**Understanding**

- `reported` — operator or system report without a controlled reproduction;
- `reproduced` — controlled physical or deterministic reproduction;
- `bounded` — the responsible subsystem or boundary is narrowed;
- `causal` — evidence identifies the mechanism sufficiently to select a repair.

**Implementation**

- `none`;
- `instrumentation-only`;
- `proposed`;
- `source-fixed`;
- `candidate-built`;
- `deployed`;
- `accepted`;
- `rejected`;
- `superseded`.

**Physical coverage**

- `operator-report-only`;
- `observed`;
- `not-reproduced`;
- `verified-fixed`;
- `regressed`;
- `not-tested`.

**User posture**

- `supported`;
- `supported-with-workaround`;
- `unqualified`;
- `blocked`.

## Fix-chain rule

Every fix is tracked through distinct stages:

```text
source correction
→ built artifact
→ profile/candidate
→ installed generation
→ physical product result
```

Do not call an earlier stage a physical fix. Do not generalize one product's physical result to another product.

## Index

| ID | Failure class | Shared boundary | Understanding | Implementation | Verified products/platforms | User posture | Next gate |
|---|---|---|---|---|---|---|---|
| [FC-UI-001](#fc-ui-001--generic-editor-input-starvation-behind-posted-work) | Generic editor input starvation behind posted work | Windows host editor pump | causal | accepted | Pigments / Steam Deck | supported | Preserve in future host builds |
| [FC-UI-002](#fc-ui-002--x11-raw-touch-release-retains-contact-on-pointer-up) | X11 raw-touch release retains contact on pointer-up | Proton/Wine `winex11.drv` | causal in exact source | candidate-built | none yet | supported-with-workaround | One patched Serum physical session |
| [FC-UI-003](#fc-ui-003--touch-opened-serum-popup-stalls-the-editor-message-loop) | Touch-opened Serum popup stalls editor message loop | Serum popup + Win32 pointer state | reproduced / bounded | proposed through FC-UI-002 | Serum / Steam Deck observed | supported-with-workaround | Candidate-C physical result |
| [FC-UI-004](#fc-ui-004--windows-touch-release-processing-continues-long-after-x11-release) | Long Windows touch-release tail | X11→Wine→User32 admission/retrieval | bounded | instrumentation-only | Pigments / Steam Deck observed | unqualified | Controlled single-contact attribution |
| [FC-UI-005](#fc-ui-005--transient-popup-lacks-an-ordinary-win32-owner-chain) | Ownerless transient popup targeting | Diagnostic surface identity | causal as tooling boundary | accepted instrumentation behavior | Pigments / Steam Deck | supported-with-workaround | Use surface-group authority where needed |
| [FC-UI-006](#fc-ui-006--touch-triggered-editor-loss-on-non-arturia-products) | Touch-triggered editor disappearance/loss | Unknown editor/window/runtime boundary | reported | none | Blackhole, Kontakt, Serum reports | unqualified | One exact product/process reproduction |
| [FC-UI-007](#fc-ui-007--editor-or-host-failure-is-not-always-visible-in-manager-state) | Missing visible incident attribution | Manager terminal/readback projection | bounded | partial | Multiple product incidents | supported-with-workaround | One consistent minimal incident surface |
| [FC-GFX-001](#fc-gfx-001--directcomposition-presentation-capability) | DirectComposition presentation capability | Proton/Wine graphics runner | causal for Blackhole blank editor | deployed / physically accepted | Blackhole / Steam Deck | supported | Preserve exact runner; no universal claim |
| [FC-LIFE-001](#fc-life-001--graphical-session-and-keeper-authority) | Graphical-session/keeper authority | Manager/supervisor lifecycle | causal | accepted | Blackhole, Kontakt / Steam Deck; FRAGMENTS / Ubuntu | supported | Gaming Mode transition coverage |
| [FC-LIFE-002](#fc-life-002--failed-launch-cleanup-and-truthful-recovery-state) | Failed launch cleanup and truthful recovery | Manager ownership/leases/results | causal | accepted with residual UX work | Steam Deck and Ubuntu fixtures | supported-with-workaround | Manager recovery UX |
| [FC-AUDIO-001](#fc-audio-001--residual-audio-deadline-misses) | Residual deadline misses | Native queue/Windows processing/scheduler | bounded | partial | Arturia Deck and FRAGMENTS Ubuntu observations | supported-with-workaround | One causal scheduler/thread capture |
| [FC-CAP-001](#fc-cap-001--capacity-enumeration-versus-lease-retirement-race) | Capacity scan versus lease retirement | Manager capacity ownership | causal | open | AP17 exact fixture | supported-with-workaround | Repair issue #93 |
| [FC-MGMT-001](#fc-mgmt-001--managed-inventory-refresh-authority) | Managed inventory freshness and refresh | Manager catalogue/registry/onboarding | causal | accepted | Blackhole, Kontakt / Deck; FRAGMENTS / Ubuntu | supported | Preserve one canonical refresh route |
| [FC-PLAT-001](#fc-plat-001--nativewindows-transport-requires-shared-private-loopback) | Native/Windows transport needs shared loopback | Platform namespace adapter | causal | accepted | FRAGMENTS / Ubuntu | supported | Regression gate for new adapters |
| [FC-BOOT-001](#fc-boot-001--volatile-runtime-and-publication-restoration-after-boot) | Runtime/publication restoration after boot | Platform service adapter | causal | accepted | FRAGMENTS / Ubuntu | supported | Preserve in packaging ports |

---

## FC-UI-001 — Generic editor input starvation behind posted work

**Shared boundary:** Windows host `VendorView::pump` message retrieval  
**Understanding:** causal for the generated workload  
**Implementation:** accepted  
**User posture:** supported

### Symptom

Hardware input can wait behind sustained finite posted-message traffic while the UI thread continues heartbeats.

### Mechanism

The prior bounded pump did not guarantee hardware input a turn while ordinary posted work remained continuously available.

### Fix chain

- **Source correction:** bounded `PM_QS_INPUT` fairness in the generic production pump.
- **Built artifact:** accepted Windows host generation recorded by UIR1.
- **Profile/candidate:** Pigments UIR1 engineering candidate and accepted ordinary successor.
- **Installed generation:** accepted Pigments ordinary generation.
- **Physical result:** Pigments macro drag and page click completed with the repaired host.

### Product coverage

| Product | Platform | Coverage | Evidence |
|---|---|---|---|
| Pigments | Steam Deck | verified-fixed for the selected actions | [UIR1](UIR1.md) |
| Serum 2 | Steam Deck | not a claim for touch-popup behavior | [FC-UI-003](#fc-ui-003--touch-opened-serum-popup-stalls-the-editor-message-loop) |
| Other products | — | not-tested for this exact mechanism | — |

### Claim limit

The generated mechanism does not prove that every historical Pigments delay, every touch defect, or every vendor popup stall had the same cause.

### Related failure classes

FC-UI-002, FC-UI-003, FC-UI-004.

### Remaining gate

Preserve the fairness behavior in every future Windows-host build.

### Evidence and historical sources

[UIR1](UIR1.md), `evidence/uir1/repair/`.

### Tracking issue

None; accepted behavior.

### Last reviewed

2026-09-23.

---

## FC-UI-002 — X11 raw-touch release retains contact on pointer-up

**Shared boundary:** pinned Proton/Wine `dlls/winex11.drv` raw-touch translation  
**Understanding:** causal in exact pinned source; physical relationship supported but not yet closed  
**Implementation:** candidate-built in draft PR #151  
**User posture:** supported-with-workaround

### Symptom

A physical touch that opens Serum's waveform dropdown leaves the editor and popup visible while the editor thread stops progressing. Mouse on the same dropdown works.

### Mechanism

The exact pinned Wine source maps `XI_RawTouchEnd` to `WM_POINTERUP` while its common flag assignment still includes `POINTER_MESSAGE_FLAG_INCONTACT`.

### Fix chain

- **Source correction:** clear `INCONTACT` for `XI_RawTouchEnd`; retain it for Begin/Update.
- **Built artifact:** immutable runner `proton-11.0-2c-x11-touch-release-v1`, complete-tree SHA-256 `d095f1f052ecebb67c66d685dbd88e373b633b0c3000343a7607c4f1bc4d1920`.
- **Profile/candidate:** prospective Serum candidate C `91b699291eb7b7d1ff6e725d5e6fd1abed88ea721dbd81a621dd2fb39d38d207`.
- **Installed generation:** pending.
- **Physical result:** pending one reviewed Serum session.

### Product coverage

| Product | Platform | Coverage | Evidence |
|---|---|---|---|
| Serum 2 2.1.5 | Steam Deck | reproduced on predecessor; patched candidate pending | [PLUGIN_RELIABILITY_FOLLOWUP](PLUGIN_RELIABILITY_FOLLOWUP.md), PR #151 |
| Pigments | Steam Deck | different delayed-release observation; cause not assigned | [UIO3](UIO3.md) |
| Blackhole | Steam Deck | operator report only; cause not assigned | [FC-UI-006](#fc-ui-006--touch-triggered-editor-loss-on-non-arturia-products) |
| Kontakt | Steam Deck | operator report only; cause not assigned | [FC-UI-006](#fc-ui-006--touch-triggered-editor-loss-on-non-arturia-products) |

### Claim limit

A Serum pass does not establish that Blackhole, Kontakt, or the older Pigments release tail shared this defect.

### Related failure classes

FC-UI-003, FC-UI-004, FC-UI-006.

### Remaining gate

Install the exact repaired generation, transition Serum to candidate C, and complete one mouse/ordinary-touch/waveform-touch/audio/retirement session.

### Evidence and historical sources

[PLUGIN_RELIABILITY_FOLLOWUP](PLUGIN_RELIABILITY_FOLLOWUP.md), [UIO3](UIO3.md), draft PR #151.

### Tracking issue

No dedicated shared issue yet.

### Last reviewed

2026-09-24.

---

## FC-UI-003 — Touch-opened Serum popup stalls the editor message loop

**Shared boundary:** Serum popup/menu tracking under Win32 pointer input  
**Understanding:** reproduced and bounded  
**Implementation:** proposed through FC-UI-002  
**User posture:** supported-with-workaround

### Symptom

Mouse/trackpad opens Serum's waveform dropdown successfully. Physical touch on an ordinary control succeeds. Physical touch on that dropdown leaves the editor and dropdown visible while Serum's editor thread stops progressing inside its message pump. Bitwig and the Windows host remain alive.

### Mechanism

The strongest current causal lead is FC-UI-002: the release message says `WM_POINTERUP` while retaining contact. The relationship remains pending the patched physical result.

### Fix chain

See FC-UI-002. No Serum-specific hook or generic editor-pump change is accepted.

### Product coverage

Serum 2 2.1.5 on Steam Deck: reproduced. Mouse/trackpad is the current workaround.

### Claim limit

This entry does not establish a universal popup defect or explain the older broad editor-disappearance reports.

### Related failure classes

FC-UI-001, FC-UI-002, FC-UI-005, FC-UI-006.

### Remaining gate

One candidate-C physical session.

### Evidence and historical sources

[PLUGIN_RELIABILITY_FOLLOWUP](PLUGIN_RELIABILITY_FOLLOWUP.md), draft PR #151.

### Tracking issue

No dedicated shared issue yet.

### Last reviewed

2026-09-24.

---

## FC-UI-004 — Windows touch-release processing continues long after X11 release

**Shared boundary:** X11/XInput → Wine admission → User32 retrieval/procedure path  
**Understanding:** bounded, not causal  
**Implementation:** instrumentation-only  
**User posture:** unqualified

### Symptom

The retained Pigments UIO3 session recorded all X11 releases, then continued to observe Windows release procedure activity for at least 16.514 seconds.

### Mechanism

Unresolved. Wine translation/admission, User32 queueing/retrieval, vendor servicing, and observer effects remain possible. UIR2 did not obtain the controlled physical contact needed to select between them.

### Fix chain

No source fix selected.

### Product coverage

Pigments / Steam Deck: observed. No other product coverage.

### Claim limit

Do not reclassify this observation as FC-UI-002 without an exact causal bridge.

### Related failure classes

FC-UI-001, FC-UI-002.

### Remaining gate

One controlled single-finger observation with action-bound Linux and Windows release evidence.

### Evidence and historical sources

[UIO3](UIO3.md), [UIR2](UIR2.md).

### Tracking issue

No dedicated shared issue yet.

### Last reviewed

2026-09-23.

---

## FC-UI-005 — Transient popup lacks an ordinary Win32 owner chain

**Shared boundary:** diagnostic identity and input authorization for vendor popups  
**Understanding:** causal as an observation/authority boundary  
**Implementation:** accepted instrumentation behavior  
**User posture:** supported-with-workaround

### Symptom

Pigments created several visible top-level popup surfaces on the same process/thread, all with no usable `GW_OWNER`/root-owner chain to the editor.

### Mechanism

Ordinary owner-chain identity is insufficient for those transient surfaces. UIO2 correctly refuses to guess from title, appearance, or location.

### Fix chain

The diagnostic route gained action-bound transient surface groups and stricter refusal behavior. This is tooling authority, not a plug-in compatibility repair.

### Product coverage

Pigments / Steam Deck diagnostic work.

### Claim limit

Ownerless popup identity does not prove a renderer, touch, resize, or vendor failure.

### Related failure classes

FC-UI-003.

### Remaining gate

Use the accepted transient-surface authority only when a selected physical task needs popup input/capture.

### Evidence and historical sources

[UIO2](UIO2.md).

### Tracking issue

None; retained tooling boundary.

### Last reviewed

2026-09-23.

---

## FC-UI-006 — Touch-triggered editor loss on non-Arturia products

**Shared boundary:** unknown editor/window/runtime boundary  
**Understanding:** reported  
**Implementation:** none  
**User posture:** unqualified

### Symptom

The operator reported physical touch causing Blackhole, Kontakt, and Serum editors to disappear or become unusable while Bitwig still considered the instance active.

### Mechanism

Not established as one shared cause. The controlled Serum popup stall is narrower and does not retroactively explain every report.

### Fix chain

None shared. Serum is being tested through FC-UI-002/003.

### Product coverage

| Product | Coverage |
|---|---|
| Blackhole | operator-report-only |
| Kontakt | operator-report-only |
| Serum 2 | controlled popup stall reproduced; broader disappearance not separately established |

### Claim limit

Do not create vendor-specific touch workarounds or apply the Serum runner to other environments without an exact reproduction and runner/source match.

### Related failure classes

FC-UI-002, FC-UI-003, FC-UI-007.

### Remaining gate

Select one exact remaining product, compare mouse and physical touch on the same control, and retain window/process/message-pump state.

### Evidence and historical sources

[PLUGIN_RELIABILITY_FOLLOWUP](PLUGIN_RELIABILITY_FOLLOWUP.md).

### Tracking issue

No dedicated shared issue yet.

### Last reviewed

2026-09-23.

---

## FC-UI-007 — Editor or host failure is not always visible in manager state

**Shared boundary:** manager incident retention and operator projection  
**Understanding:** bounded  
**Implementation:** partial  
**User posture:** supported-with-workaround

### Symptom

A failed or disappeared editor can leave Bitwig's device present while the manager does not show a useful product-level incident.

### Mechanism

Detailed capture eligibility and minimal terminal status have historically been coupled too closely. Missing optional detailed capture must not erase a known terminal/editor incident.

### Fix chain

Several slices retain terminal summaries and exact cleanup state. A single everyday manager incident surface is still incomplete.

### Product coverage

Multiple editor/host incidents across Pigments, Blackhole, Kontakt, and Serum work.

### Claim limit

This is a reporting defect, not a claim that all incidents share one runtime cause.

### Related failure classes

FC-LIFE-002, FC-UI-006.

### Remaining gate

Expose one stable minimal incident status per affected instance, with optional private detailed capture and truthful recovery state.

### Evidence and historical sources

[PLUGIN_RELIABILITY_FOLLOWUP](PLUGIN_RELIABILITY_FOLLOWUP.md), [IF1](IF1.md), [IF2](IF2.md).

### Tracking issue

No dedicated shared issue yet.

### Last reviewed

2026-09-23.

---

## FC-GFX-001 — DirectComposition presentation capability

**Shared boundary:** pinned Wine graphics implementation and runner selection  
**Understanding:** causal for the Blackhole blank editor  
**Implementation:** deployed and physically accepted  
**User posture:** supported

### Symptom and mechanism

The predecessor runner returned `E_NOTIMPL` from `CreateSwapChainForComposition`, producing an all-white Blackhole editor. An immutable DirectComposition-capable reference runner was built and selected.

### Fix chain

- source/build: exact reference Wine graphics build;
- artifact: immutable DComp runner;
- candidate: Blackhole reviewed candidate;
- installed generation: retained;
- physical result: Blackhole editor rendered and interacted successfully.

### Product coverage

Blackhole Immersive 1.4.4 / Steam Deck: verified-fixed for the exact candidate.

### Claim limit

No universal graphics support or applicability to other runners/products.

### Remaining gate

Preserve exact runner identity and regression smoke after relevant graphics changes.

### Evidence and historical sources

[BLACKHOLE_EDITOR](BLACKHOLE_EDITOR.md), issue #132.

### Tracking issue

[#132](https://github.com/kasselvania/Linux-VST-bridge/issues/132).

### Last reviewed

2026-09-23.

---

## FC-LIFE-001 — Graphical-session and keeper authority

**Shared boundary:** manager/supervisor graphical context, keeper startup, session replacement  
**Understanding:** causal  
**Implementation:** accepted  
**User posture:** supported

### Symptom and mechanism

Host-private Xauthority/Wayland/D-Bus paths, absent denial mount sources, overlong denial paths, and graphical-session replacement caused prelaunch keeper failures. The accepted design uses exact authenticated aliases where identity matches and private existing non-listening denial sockets otherwise.

### Product coverage

Blackhole and Kontakt on Steam Deck; FRAGMENTS on Ubuntu.

### Claim limit

Gaming Mode transitions remain a separate physical surface.

### Remaining gate

Complete the current Gaming Mode product checks without weakening the exact graphical authority.

### Evidence and historical sources

[PLUGIN_RELIABILITY_FOLLOWUP](PLUGIN_RELIABILITY_FOLLOWUP.md), [BLACKHOLE_EDITOR](BLACKHOLE_EDITOR.md).

### Tracking issue

No dedicated shared issue yet.

### Last reviewed

2026-09-23.

---

## FC-LIFE-002 — Failed launch cleanup and truthful recovery state

**Shared boundary:** manager leases, supervisor results, service stop, recovery projection  
**Understanding:** causal  
**Implementation:** accepted with residual UX work  
**User posture:** supported-with-workaround

### Symptom and mechanism

Prelaunch or service failures could leave blocked admissions, stale control records, or falsely successful stop receipts. Repairs retain first failure, require positive retirement, reconcile only exact dead authority, and surface cleanup uncertainty.

### Product coverage

Steam Deck managed products and Ubuntu FRAGMENTS bring-up.

### Claim limit

Abrupt power-loss recovery with active owners is not universally automatic.

### Remaining gate

Provide an everyday manager recovery/panic workflow that preserves evidence and refuses uncertain cleanup.

### Evidence and historical sources

[PLUGIN_RELIABILITY_FOLLOWUP](PLUGIN_RELIABILITY_FOLLOWUP.md), Ubuntu-lab PR #5 history.

### Tracking issue

No dedicated shared issue yet.

### Last reviewed

2026-09-23.

---

## FC-AUDIO-001 — Residual audio deadline misses

**Shared boundary:** callback admission, native worker, Windows processing, scheduler and reply path  
**Understanding:** bounded  
**Implementation:** partial  
**User posture:** supported-with-workaround

### Symptom and mechanism

Retained sessions contain startup, queue/reply, editor/removal, lifecycle, and continuing deadline misses. AP16 fixed one measured disk-backed mapping stall by moving hot mappings to private tmpfs. Other classes remain causally unresolved.

### Product coverage

Arturia Deck sessions and FRAGMENTS Ubuntu sessions contain retained gap counters. Functional use is accepted; dropout-free operation is not claimed.

### Claim limit

Counters are not automatically audible-dropout evidence. Elapsed Windows `process()` time is not automatically thread CPU time.

### Remaining gate

One bounded exact-thread capture distinguishing CPU execution, runnable wait, blocking/faults, and cgroup throttling for one continuing miss.

### Evidence and historical sources

[AP16](AP16.md), issue #90.

### Tracking issue

[#90](https://github.com/kasselvania/Linux-VST-bridge/issues/90).

### Last reviewed

2026-09-23.

---

## FC-CAP-001 — Capacity enumeration versus lease-retirement race

**Shared boundary:** manager capacity ownership  
**Understanding:** causal  
**Implementation:** open  
**User posture:** supported-with-workaround

### Symptom and mechanism

Capacity enumeration can observe a durable lease path that is removed concurrently by retirement, causing a fail-closed but unnecessary refusal.

### Product coverage

AP17 Steam Deck fixture. It does not invalidate the accepted six-instance envelope or permit over-admission.

### Claim limit

This race can create a temporary unnecessary refusal; it does not over-admit, double-refund capacity, or invalidate the accepted six-instance envelope.

### Remaining gate

Serialize scan/refund ownership or make the scan restartable while preserving fail-closed behavior.

### Evidence and historical sources

[AP17](AP17.md), issue #93.

### Tracking issue

[#93](https://github.com/kasselvania/Linux-VST-bridge/issues/93).

### Last reviewed

2026-09-23.

---

## FC-MGMT-001 — Managed inventory refresh authority

**Shared boundary:** manager catalogue, retained onboarding, registry ownership, scanner lock  
**Understanding:** causal  
**Implementation:** accepted  
**User posture:** supported

### Symptom and mechanism

Managed experimental environments could have stale scanner evidence but no lawful refresh action because they were absent from the ordinary installed catalogue. The manager now derives one managed roster from current catalogue and retained managed ownership, projects exactly one refresh, and revalidates authority under the final scanner lock.

### Product coverage

Blackhole and Kontakt on Steam Deck; FRAGMENTS catalogue-free retry on Ubuntu.

### Claim limit

This does not reopen installation or grant arbitrary rescans.

### Remaining gate

Preserve the single canonical refresh/retry authority in future manager work.

### Evidence and historical sources

[PLUGIN_RELIABILITY_FOLLOWUP](PLUGIN_RELIABILITY_FOLLOWUP.md).

### Tracking issue

None; accepted behavior.

### Last reviewed

2026-09-23.

---

## FC-PLAT-001 — Native/Windows transport requires shared private loopback

**Shared boundary:** platform network namespace adapter  
**Understanding:** causal  
**Implementation:** accepted  
**User posture:** supported

### Symptom and mechanism

The native proxy listens on `127.0.0.1`; the Windows host connects to that loopback address. Separate network namespaces made the port visible in shared files but unreachable.

### Fix chain

Ubuntu manager/Windows host and Bitwig/native proxy now join one private loopback-only network namespace while preserving separate mounts and one-way PID visibility.

### Product coverage

FRAGMENTS / Ubuntu: verified-fixed.

### Claim limit

This does not authorize host networking.

### Remaining gate

Every new platform adapter must prove a real native-to-Windows loopback connection.

### Evidence and historical sources

Ubuntu-lab PR #5 and retained CPI2 results.

### Tracking issue

None; accepted platform law.

### Last reviewed

2026-09-23.

---

## FC-BOOT-001 — Volatile runtime and publication restoration after boot

**Shared boundary:** platform service startup and retained user selection  
**Understanding:** causal  
**Implementation:** accepted  
**User posture:** supported

### Symptom and mechanism

The user service originally assumed volatile `/run` state had already been prepared, and canonical startup intentionally removed engineering publications. Ubuntu now has a generation-owned runtime preparer plus exact retained-selection intent that reselects the same reviewed revision through canonical rollback.

### Product coverage

FRAGMENTS / Ubuntu: controlled restart and machine reboot verified.

### Claim limit

This is normal clean boot behavior, not universal automatic recovery from an interrupted active owner.

### Remaining gate

Preserve this behavior in Debian/package adapters.

### Evidence and historical sources

Ubuntu-lab PR #5 reboot receipt.

### Tracking issue

None; accepted platform law.

### Last reviewed

2026-09-23.

---

## Maintenance rules

When a PR changes one of these classes:

1. update the index row;
2. update the detailed card;
3. update affected rows in [SUPPORT_MATRIX.md](SUPPORT_MATRIX.md);
4. preserve historical documents rather than rewriting old observations;
5. state the new fix-chain stage and claim limit in the PR body.

A new class should represent a reusable mechanism, shared architectural boundary, current user limitation, or accepted fix that future work must preserve. Do not add one entry for every failed command or harness mistake.
