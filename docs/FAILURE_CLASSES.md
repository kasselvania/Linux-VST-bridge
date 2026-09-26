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

**Product coverage**

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
| [FC-UI-002](#fc-ui-002--x11-raw-touch-release-retains-contact-on-pointer-up) | X11 raw-touch release retains contact on pointer-up | Proton/Wine `winex11.drv` | causal | deployed | Serum / Deck: flag-only C failed; combined D passed | supported-with-workaround | Preserve corrected End flags; no flag-only fix claim |
| [FC-UI-003](#fc-ui-003--touch-opened-serum-popup-stalls-the-editor-message-loop) | Touch-opened Serum popup stalls editor message loop | Shared Wine X11 event-window routing and popup capture | causal | accepted | Serum 2 / Deck: exact popup verified-fixed | supported | Check other exact Wine lineages independently |
| [FC-UI-004](#fc-ui-004--windows-touch-release-processing-continues-long-after-x11-release) | Long Windows touch-release tail | X11→Wine→User32 admission/retrieval | bounded | instrumentation-only | Pigments / Steam Deck observed | unqualified | Controlled single-contact attribution |
| [FC-UI-005](#fc-ui-005--transient-popup-lacks-an-ordinary-win32-owner-chain) | Ownerless transient popup targeting | Diagnostic surface identity | causal | instrumentation-only | Pigments / Steam Deck observed | unqualified | Bind exact popup identity for the selected action |
| [FC-UI-006](#fc-ui-006--touch-triggered-editor-loss-on-non-arturia-products) | Touch-triggered editor disappearance/loss | Unknown editor/window/runtime boundary | reported | none | Blackhole, Kontakt, Serum reports | unqualified | One exact product/process reproduction |
| [FC-UI-007](#fc-ui-007--editor-or-host-failure-is-not-always-visible-in-manager-state) | Missing visible incident attribution | Manager terminal/readback projection | bounded | deployed | Multiple product incidents | supported-with-workaround | One consistent minimal incident surface |
| [FC-GFX-001](#fc-gfx-001--directcomposition-presentation-capability) | DirectComposition presentation capability | Proton/Wine graphics runner | causal | accepted | Blackhole / Steam Deck | supported | Preserve exact runner; close stale issue #132 disposition separately |
| [FC-LIFE-001](#fc-life-001--graphical-session-and-keeper-authority) | Graphical-session/keeper authority | Manager/supervisor lifecycle | causal | accepted | Blackhole, Kontakt / Steam Deck; FRAGMENTS / Ubuntu | supported | Gaming Mode transition coverage |
| [FC-LIFE-002](#fc-life-002--failed-launch-cleanup-and-truthful-recovery-state) | Failed launch cleanup and truthful recovery | Manager ownership/leases/results | causal | deployed | Steam Deck and Ubuntu fixtures | supported-with-workaround | Manager recovery UX |
| [FC-AUDIO-001](#fc-audio-001--residual-audio-deadline-misses) | Residual deadline misses | Native queue/Windows processing/scheduler | bounded | instrumentation-only | Arturia Deck and FRAGMENTS Ubuntu observations | supported-with-workaround | Pi FN1 attribution and PW1 prewarm source candidate are separate unqualified fixture results |
| [FC-AUDIO-002](#fc-audio-002--host-block-exceeds-the-selected-bridge-presentation-envelope) | Host block exceeds selected bridge presentation envelope | Proxy setup, selected delay, DAW audio settings | causal | accepted | FRAGMENTS / Ubuntu at Bitwig 512/48 kHz | supported-with-workaround | Actionable requested-versus-supported block message |
| [FC-CAP-001](#fc-cap-001--capacity-enumeration-versus-lease-retirement-race) | Capacity scan versus lease retirement | Manager capacity ownership | causal | none | AP17 exact fixture | supported-with-workaround | Repair issue #93 |
| [FC-MGMT-001](#fc-mgmt-001--managed-inventory-refresh-authority) | Managed inventory freshness and refresh | Manager catalogue/registry/onboarding | causal | accepted | Blackhole, Kontakt / Deck; FRAGMENTS / Ubuntu | supported | Preserve one canonical refresh route |
| [FC-MGMT-002](#fc-mgmt-002--exact-verified-hostsource-omitted-across-software-generations) | Exact verified host/source omitted across generations | Software catalogue, profile/candidate and publication | causal | accepted | Pure LoFi, FRAGMENTS, Serum / Deck | supported | Preserve required exact pairs in every new generation |
| [FC-PLAT-001](#fc-plat-001--nativewindows-transport-requires-shared-private-loopback) | Native/Windows transport needs shared loopback | Platform namespace adapter | causal | accepted | FRAGMENTS / Ubuntu | supported | Regression gate for new adapters |
| [FC-BOOT-001](#fc-boot-001--volatile-runtime-and-publication-restoration-after-boot) | Runtime/publication restoration after boot | Platform service adapter | causal | accepted | FRAGMENTS / Ubuntu | supported | Preserve in packaging ports |

---

## FC-UI-001 — Generic editor input starvation behind posted work

### Shared boundary

Windows host `VendorView::pump` message retrieval

### Understanding

causal

The generated workload selected the generic pump boundary.

### Implementation

accepted

### User posture

supported

### Symptom

Hardware input can wait behind sustained finite posted-message traffic while the UI thread continues heartbeats.

### Mechanism

The prior bounded pump did not guarantee hardware input a turn while ordinary posted work remained continuously available.

### Fix chain

- **Source correction:** bounded `PM_QS_INPUT` fairness in the generic production pump.
- **Built artifact:** UIR1 Windows host SHA-256 `50c09be65eb2d2930f744afc16212f953c0b948f47132fa6cac1c90bbb91773d`.
- **Profile/candidate:** Pigments candidate 12 fingerprint `b8d6289d5a50a941deec82b0c3ac644b4c905da684fce8a861f4e847019d11c6`; accepted ordinary revision 13 fingerprint `a74dd61397dcdc9bfcf4a1f39de74eb00f1ca48e3a634b3c03eddc1a416dbfc7`.
- **Installed generation:** UIR1 candidate-12 publication `886e48242434864def572a4f5620a9f9` was physically tested; ordinary revision 13 subsequently accepted. Current Pigments revision 18 retains that host-pump behavior.
- **Physical result:** Pigments macro drag and page click completed with the repaired host.

### Product coverage

| Product | Platform | Coverage | Evidence |
|---|---|---|---|
| Pigments | Steam Deck | verified-fixed for the selected actions | [UIR1](UIR1.md) |
| Serum 2 | Steam Deck | not a claim for touch-popup behavior | [FC-UI-003](#fc-ui-003--touch-opened-serum-popup-stalls-the-editor-message-loop) |
| Other products | — | not-tested for this exact mechanism | — |

### Claim limit

The generated mechanism does not prove that every historical Pigments delay, every touch defect, or every vendor popup stall had the same cause. The closed [FRAGMENTS expansion/redraw issue #80](https://github.com/kasselvania/Linux-VST-bridge/issues/80) is not assigned to this pump mechanism.

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

### Shared boundary

pinned Proton/Wine `dlls/winex11.drv` raw-touch translation

### Understanding

causal

The exact source flag defect is established; causality for the observed popup stall is unproven.

### Implementation

deployed

Draft [PR #151](https://github.com/kasselvania/Linux-VST-bridge/pull/151) contains the source; the immutable runner and candidate C were installed, but the physical touch result failed. Candidate D retains this corrected End mapping and passed its separate per-window routing gate; the release-flag change alone is not claimed as the fix.

### User posture

supported-with-workaround

### Symptom

A physical touch that opens Serum's waveform dropdown leaves the editor and popup visible while the editor thread stops progressing. Mouse on the same dropdown works.

### Mechanism

The exact pinned Wine source maps `XI_RawTouchEnd` to `WM_POINTERUP` while its common flag assignment still includes `POINTER_MESSAGE_FLAG_INCONTACT`.

### Fix chain

- **Source correction:** clear `INCONTACT` for `XI_RawTouchEnd`; retain it for Begin/Update.
- **Built artifact:** immutable runner `proton-11.0-2c-x11-touch-release-v1`, complete-tree SHA-256 `d095f1f052ecebb67c66d685dbd88e373b633b0c3000343a7607c4f1bc4d1920`.
- **Profile/candidate:** Serum candidate C `91b699291eb7b7d1ff6e725d5e6fd1abed88ea721dbd81a621dd2fb39d38d207`, revision `807828d4b94fccd69e57c352a60dbcd1`.
- **Installed generation:** `30d144c0b65437e7963692d434f930fe45faf2ec73527d5864580fc08eb913ac`; only Serum changed publication.
- **Physical result:** mouse menu and ordinary touch passed, but the first finger-opened menu stalled. Audio continued; focus cycling restored visible response. The touch repair **did not pass**. [Retained result in PR #151](https://github.com/kasselvania/Linux-VST-bridge/blob/4ae414b33e37f771c422d8a70a9fa78a20231118/evidence/serum-x11-touch-release/physical-attempt-002.json).

### Product coverage

| Product | Platform | Coverage | Evidence |
|---|---|---|---|
| Serum 2 2.1.5 | Steam Deck | candidate C failed with the corrected release flag alone; candidate D passed the combined per-window route | [candidate-C result](https://github.com/kasselvania/Linux-VST-bridge/blob/4ae414b33e37f771c422d8a70a9fa78a20231118/evidence/serum-x11-touch-release/physical-attempt-002.json), [candidate-D result](../evidence/serum-x11-touch-routing/candidate-d-physical.json) |
| Pigments | Steam Deck | different delayed-release observation; cause not assigned | [UIO3](UIO3.md) |
| Blackhole | Steam Deck | operator report only; cause not assigned | [FC-UI-006](#fc-ui-006--touch-triggered-editor-loss-on-non-arturia-products) |
| Kontakt | Steam Deck | operator report only; cause not assigned | [FC-UI-006](#fc-ui-006--touch-triggered-editor-loss-on-non-arturia-products) |

### Claim limit

The observed Serum touch failure does not establish whether corrected `WM_POINTERUP` reached the menu. It does not establish that Blackhole, Kontakt, or the older Pigments release tail shared the source defect.

### Related failure classes

FC-UI-003, FC-UI-004, FC-UI-006.

### Remaining gate

Candidate C's physical popup gate failed. Candidate D retained the corrected
End mapping and passed the exact Serum menu gate after switching to per-window
touch delivery. Other runner lineages still need their own source and physical
checks; the standalone release-flag correction is not promoted to a universal fix.

### Evidence and historical sources

[PLUGIN_RELIABILITY_FOLLOWUP](PLUGIN_RELIABILITY_FOLLOWUP.md), [UIO3](UIO3.md), [draft PR #151 and its physical receipt](https://github.com/kasselvania/Linux-VST-bridge/pull/151).

### Tracking issue

No dedicated shared issue yet.

### Last reviewed

2026-09-24.

---

## FC-UI-003 — Touch-opened Serum popup stalls the editor message loop

### Shared boundary

Wine X11 touch event-window targeting and popup message delivery

### Understanding

causal

The candidate-C to candidate-D intervention is physically verified for the
exact Serum popup. The claim is not universal to Wine or other vendors.

### Implementation

accepted

The FC-UI-002 candidate did not resolve this stall. The shared per-window route
passed both the source-owned popup fixture and one physical Serum session.

### User posture

supported

### Symptom

Under candidate C, mouse/trackpad opened Serum's waveform dropdown and an
ordinary touch control worked, but finger-opened popup selection/dismissal
stalled while audio and the Windows host continued. Focus cycling restored
visible response in an earlier attempt. The action-bound capture showed a
real popup owner, retained capture, and 16,379 repeated in-contact
`WM_POINTERUPDATE` messages to the old editor child with no new physical
input; the observer ring overflowed by 79 records. Under candidate D, the
same popup interaction selected and dismissed by finger and the editor
continued responding.

### Mechanism

Candidate C's root-raw touch path had no per-window XI delivery target. The
action capture showed the old editor child receiving repeated in-contact
pointer updates after popup creation, although its overflowed observer did not
retain a complete XI Begin/End pair. Candidate D replaced that route with
per-window `XI_TouchBegin/Update/End` delivered through the actual X event
window to Wine's HWND and kept the corrected release flag. With no Serum,
proxy, host, buffer, or capacity change, finger selection and dismissal now
complete without focus cycling. This physical before/after supports the
routing correction for this exact fixture; it does not prove which individual
Windows menu API the vendor consumed. The observer's `GetPointerInfo` failures
were its own reads of a Wine stub, not evidence that Serum called that API.

### Fix chain

- **Source correction:** per-window `XI_TouchBegin/Update/End` selection and exact X event-window to HWND routing, preserving FC-UI-002's End flags in draft [PR #151](https://github.com/kasselvania/Linux-VST-bridge/pull/151).
- **Built artifact:** immutable `proton-11.0-2c-x11-touch-routing-v2`, complete-tree SHA-256 `de6c55c2a8c82abf2b1b0b47a97ee97a00657fa04582b2112327fa3d0095a698`; the [source-owned popup fixture](../evidence/serum-x11-touch-routing/popup-fixture-physical.json) passed physical finger and trackpad paths.
- **Profile/candidate:** Serum candidate D `b43421069dca3872cf7c28440616d1086d192f827ac8ccd80bf16102dc2681ab`, publication `d39392e65959e4fba15d769d9fea9b1a`; candidate C is the retained predecessor.
- **Installed generation:** `d2e90f7b3a38fe1263171d33b9cfc640abf59bc46f1f470f87cb6c8750ba6c4d`.
- **Physical result:** [one Deck Serum session](../evidence/serum-x11-touch-routing/candidate-d-physical.json) passed mouse menu, ordinary touch, finger-open/finger-select, finger-open/finger-dismiss, responsive editor and following audible note. Windows cleanup and transport retirement were confirmed; zero DSP and no cleanup uncertainty remained. The session reported zero underrun gaps, not a general gap-free guarantee.

### Product coverage

Serum 2 2.1.5 on Steam Deck: verified-fixed for the exact waveform popup
interaction under candidate D. Candidate C's failure remains retained.

### Claim limit

This entry does not establish a universal popup defect or explain the older broad editor-disappearance reports. The action-bound trace overflow prevents claiming a complete Windows message sequence.

### Related failure classes

FC-UI-001, FC-UI-002, FC-UI-005, FC-UI-006.

### Remaining gate

The [exact lineage check](../evidence/serum-x11-touch-routing/cross-plugin-lineage.json)
shows Blackhole's DComp Wine already has the per-window route; do not apply
Serum's routing patch to it. Kontakt's NI Wine still has the root-raw route
and accepts both source patches, but its managed runner/candidate transition
needs a separately closed NI authority before a physical product check. Do
not assign this Serum result to either product or Pigments' older release tail.

### Evidence and historical sources

[PLUGIN_RELIABILITY_FOLLOWUP](PLUGIN_RELIABILITY_FOLLOWUP.md), [candidate-C action capture](../evidence/serum-x11-touch-routing/candidate-c-action-capture.json), [candidate-D physical result](../evidence/serum-x11-touch-routing/candidate-d-physical.json), [PR #151](https://github.com/kasselvania/Linux-VST-bridge/pull/151).

### Tracking issue

No dedicated shared issue yet.

### Last reviewed

2026-09-24.

---

## FC-UI-004 — Windows touch-release processing continues long after X11 release

### Shared boundary

X11/XInput → Wine admission → User32 retrieval/procedure path

### Understanding

bounded

The exact delaying stage is not causal yet.

### Implementation

instrumentation-only

### User posture

unqualified

### Symptom

The retained Pigments UIO3 session recorded all X11 releases, then continued to observe Windows release procedure activity for at least 16.514 seconds.

### Mechanism

Unresolved. Wine translation/admission, User32 queueing/retrieval, vendor servicing, and observer effects remain possible. UIR2 did not obtain the controlled physical contact needed to select between them.

### Fix chain

- **Source correction:** none selected; UIR2 added bounded observation only.
- **Built artifact:** no repair artifact.
- **Profile/candidate:** unchanged Pigments profile; no selected touch-tail candidate.
- **Installed generation:** no touch-tail repair generation.
- **Physical result:** UIO3 observed the tail; UIR2 did not close attribution.

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

### Shared boundary

diagnostic identity and input authorization for vendor popups

### Understanding

causal

This is an observation/authority boundary, not an established product failure cause.

### Implementation

instrumentation-only

The diagnostic targeting/refusal behavior was exercised, but no product repair is claimed.

### User posture

unqualified

This is a diagnostic input-targeting boundary, not a product support claim.

### Symptom

Pigments created several visible top-level popup surfaces on the same process/thread, all with no usable `GW_OWNER`/root-owner chain to the editor.

### Mechanism

Ordinary owner-chain identity is insufficient for those transient surfaces. UIO2 correctly refuses to guess from title, appearance, or location.

### Fix chain

- **Source correction:** UIO2 action-bound transient-surface groups and strict refusal where ownership cannot be proven.
- **Built artifact:** UIO2 diagnostic tools, not a Wine or host repair.
- **Profile/candidate:** no plug-in candidate change.
- **Installed generation:** no product repair generation.
- **Physical result:** Pigments popup identity was observed; no popup failure cause was proven.

### Product coverage

Pigments / Steam Deck diagnostic work.

### Claim limit

Ownerless popup identity does not prove a renderer, touch, resize, or vendor
failure. The later Serum waveform popup had an exact Win32 owner; FC-UI-005
does not explain that stall. See the [action-bound capture](../evidence/serum-x11-touch-routing/candidate-c-action-capture.json).

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

### Shared boundary

unknown editor/window/runtime boundary

### Understanding

reported

### Implementation

none

### User posture

unqualified

### Symptom

The operator reported physical touch causing Blackhole, Kontakt, and Serum editors to disappear or become unusable while Bitwig still considered the instance active.

### Mechanism

Not established as one shared cause. The controlled Serum popup stall is narrower and does not retroactively explain every report.

### Fix chain

- **Source correction:** none shared; Serum-specific hypothesis is tracked separately in FC-UI-002/003.
- **Built artifact:** no shared repair artifact.
- **Profile/candidate:** no cross-product candidate.
- **Installed generation:** none for these broad reports.
- **Physical result:** operator reports only for disappearance/loss; the controlled Serum popup stall is narrower.

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

### Shared boundary

manager incident retention and operator projection

### Understanding

bounded

### Implementation

deployed

Several terminal summaries exist, but the everyday manager incident projection remains incomplete.

### User posture

supported-with-workaround

### Symptom

A failed or disappeared editor can leave Bitwig's device present while the manager does not show a useful product-level incident.

### Mechanism

Detailed capture eligibility and minimal terminal status have historically been coupled too closely. Missing optional detailed capture must not erase a known terminal/editor incident.

### Fix chain

- **Source correction:** existing terminal/cleanup retention; no complete everyday incident projection yet.
- **Built artifact:** retained manager generations, not a new incident-surface artifact.
- **Profile/candidate:** no candidate change.
- **Installed generation:** current Deck and Ubuntu managers have partial terminal readback.
- **Physical result:** incidents could still leave the operator without a useful product-level explanation.

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

### Shared boundary

pinned Wine graphics implementation and runner selection

### Understanding

causal

This understanding is specific to the Blackhole blank editor.

### Implementation

accepted

### User posture

supported

### Symptom

The predecessor runner produced an all-white Blackhole editor.

### Mechanism

Its `CreateSwapChainForComposition` returned `E_NOTIMPL`; the exact selected reference Wine graphics stack supplied the missing capability.

### Fix chain

- **Source correction:** coherent reference Wine graphics build `c27f058814b402a5709e073adccd42baa66810b9`, retained in [BLACKHOLE_EDITOR](BLACKHOLE_EDITOR.md).
- **Built artifact:** immutable `proton-11.0-2c-dcomp-c27f058-reference` runner.
- **Profile/candidate:** managed Blackhole candidate `80e06590b7dc3f4e15d2903237541bdb466c012c8dfb647f6d0843daefb68b7d`, as in the last fleet readback.
- **Installed generation:** exact runner/candidate selected in the Deck managed environment; the graphics receipt is bound to that candidate, not a generic Wine install.
- **Physical result:** Blackhole editor rendered, mouse Bypass worked, and the operator confirmed audio. [BLACKHOLE_EDITOR](BLACKHOLE_EDITOR.md#delivered-bitwig-result-and-stopping-point).

### Product coverage

Blackhole Immersive 1.4.4 / Steam Deck: verified-fixed for the exact candidate.

### Claim limit

No universal graphics support or applicability to other runners/products.

### Related failure classes

FC-UI-006, FC-LIFE-001.

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

### Shared boundary

manager/supervisor graphical context, keeper startup, session replacement

### Understanding

causal

### Implementation

accepted

### User posture

supported

### Symptom

Prelaunch keeper admission could fail after graphical-session replacement or when a required private graphical mount source was absent.

### Mechanism

Host-private Xauthority/Wayland/D-Bus paths, missing denial mount sources, and overlong denial paths broke exact identity and mounting. The accepted boundary uses authenticated aliases where identity matches and existing private non-listening denial sockets otherwise.

### Fix chain

- **Source correction:** manager/supervisor keeper and graphical-authority path in the accepted canonical product, documented by [PLUGIN_RELIABILITY_FOLLOWUP](PLUGIN_RELIABILITY_FOLLOWUP.md).
- **Built artifact:** canonical product at Deck main `32fa422581b29318dd2c4653120ab1b63203192d`; Ubuntu's admitted canonical product `b0ba8164da0e568309387ee152f3c4ba37910767`.
- **Profile/candidate:** exact Blackhole and Kontakt managed candidates on Deck; FRAGMENTS review candidate revision 12 on Ubuntu.
- **Installed generation:** Deck's retained managed generations; Ubuntu `095b21aa05d991f0e6ca2c5d471da8bd1e9ff8dab75c5a758fe583e5aed4509f`.
- **Physical result:** Blackhole/Kontakt Desktop opens and retirement; Ubuntu FRAGMENTS editor/audio and post-login namespace proof.

### Product coverage

Blackhole and Kontakt on Steam Deck; FRAGMENTS on Ubuntu.

### Claim limit

Gaming Mode transitions remain a separate physical surface.

### Related failure classes

FC-LIFE-002, FC-BOOT-001, FC-PLAT-001.

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

### Shared boundary

manager leases, supervisor results, service stop, recovery projection

### Understanding

causal

### Implementation

deployed

Exact cleanup and truthful stop checks exist; everyday recovery UI remains incomplete.

### User posture

supported-with-workaround

### Symptom

Prelaunch or service failures could leave blocked admission, stale control records, or a falsely successful stop receipt.

### Mechanism

Process exit alone was mistaken for clean retirement; abrupt or timed-out termination can leave service authority behind. Exact owner and control-directory checks are required.

### Fix chain

- **Source correction:** retained first failure, positive retirement, exact dead-control reconciliation and truthful stop result in the canonical manager and [Ubuntu-lab PR #5](https://github.com/kasselvania/Linux-VST-bridge-ubuntu-lab/pull/5).
- **Built artifact:** canonical product `b0ba8164da0e568309387ee152f3c4ba37910767` and merged Ubuntu adapter `473ac0a926e6d95d470c002244b851857b83f41c` for the Ubuntu boundary.
- **Profile/candidate:** no profile change; exact selected FRAGMENTS review candidate remains revision 12.
- **Installed generation:** Ubuntu `095b21aa05d991f0e6ca2c5d471da8bd1e9ff8dab75c5a758fe583e5aed4509f`.
- **Physical result:** exact stale predecessor control directory retired and subsequent clean FRAGMENTS sessions; abrupt active-owner recovery not tested as an automatic path.

### Product coverage

Steam Deck managed products and Ubuntu FRAGMENTS bring-up.

### Claim limit

Abrupt power-loss recovery with active owners is not universally automatic.

### Related failure classes

FC-LIFE-001, FC-BOOT-001, FC-UI-007.

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

### Shared boundary

callback admission, native worker, Windows processing, scheduler and reply path

### Understanding

bounded

### Implementation

instrumentation-only

This status is for the residual deadline-miss classes. AP16 separately accepted a disk-backed
hot-mapping repair. [AS1 PR #172](https://github.com/kasselvania/Linux-VST-bridge/pull/172)
removed recurring bridge-owned allocation from its covered shared audio
request/reply path as a source correction only. No reduction in deadline misses,
CPU time, dropouts or instrument failures has been established.
The staged Pi comparison in [PI-R PR #173](https://github.com/kasselvania/Linux-VST-bridge/pull/173)
measured lower mean completed-request service time, but the same first-note
delivery gap and overlapping CPU ranges. It is not an installed or accepted
residual deadline-miss fix.
The [FN1 Pi result](../evidence/fn1/serum-first-note-2026-09-25.md) attributes
the exact default-state first-note span to caller-executed FEX ARM64EC cold
translation/JIT work, with new Serum guest-code map entries during the call.
FN1 changes diagnostic source only. It does not repair the gap or establish a
cause for older unidentified presets, Deck, Ubuntu or other residual misses.
The staged [PW1 Pi result](../evidence/pw1/serum-default-prewarm-2026-09-25.md)
shows that an opt-in, state-restoring startup prewarm moved the cold call before
JACK readiness and removed the first-user-note gap in three runs of the exact
default-state source candidate. It is not installed or accepted as a general
residual deadline-miss fix; the broad implementation and user postures remain
unchanged.
PSL1 moves that opt-in sequence into a strict v2 product binding and shared
standalone lifecycle executor at source stage. Its three-state physical result
is pending operator-supplied private B/C states. This does not change the
residual deadline-miss implementation or support posture.

### User posture

supported-with-workaround

### Symptom

Retained sessions contain startup, queue/reply, editor/removal, lifecycle, and continuing deadline misses.

### Mechanism

AP16 established and fixed one disk-backed hot-mapping stall. For the older
*residual* Deck and Ubuntu records, the retained elapsed-time data do not
distinguish vendor CPU work from runnable wait, blocking/faults, native
ordering, or cgroup throttling.
The Pi default-state first note is now narrower: Linux `lvb-audio` ran through
the call with little runnable delay, FEX-boundary samples and Serum guest-code
map growth. The particular FEX routine and per-millisecond fault cost remain
unresolved.

### Fix chain

- **Source correction:** AP16's private-tmpfs hot-transport mapping addressed the demonstrated backing-store class. AS1 [PR #172](https://github.com/kasselvania/Linux-VST-bridge/pull/172) removes recurring bridge-owned allocation from the covered request/reply path at the source stage only; no residual deadline-miss mechanism or fix has been established.
- **Built artifact:** AP16 corrected software revision `f6a19c78100fce548ae380ac043489d85d1543497ea20806e029526ad8fb8f0b`; no residual repair artifact.
- **Profile/candidate:** AP16 retained ordinary revision-7 LoFi/FRAGMENTS profiles; no residual candidate.
- **Installed generation:** AP16 corrected transport revision was installed on the Deck; Ubuntu FRAGMENTS remained on its accepted revision 12.
- **Physical result:** AP16 matched result for the backing-store class; residual gaps persisted in later Deck and Ubuntu sessions. The staged Pi A/B in [the exact Serum default-state result](../evidence/pi-reconciliation/serum-default-2026-09-25.md) retained a 1,280-frame first-note gap in all eight runs despite lower mean request service in the AS1 arm. No installed-generation or support claim follows.
- **Attribution:** FN1 [Pi default-state evidence](../evidence/fn1/serum-first-note-2026-09-25.md) records caller CPU/run time, FEX-boundary samples, minor faults and 116 new Serum-named guest JIT regions during the slow first-note call. This is diagnostic source and physical attribution only, not a repair or support change.
- **Pi source candidate:** PW1 [exact default-state evidence](../evidence/pw1/serum-default-prewarm-2026-09-25.md) records a fresh FN1 baseline with the 1,280-frame first-note gap and three final candidate runs with zero missing frames, exact second state readback and clean retirement. The cold 43 ms call moved into bounded startup. No installed-generation or general support claim follows.
- **Product-owned preparation source:** PSL1 adds one strict binding-v2 recipe, explicit operator authorization, a reused prepared-state executor, and fail-closed state readback. Its ARM build and source tests do not establish a new physical or support result while private B/C inputs are pending.

### Product coverage

Arturia Deck sessions and FRAGMENTS Ubuntu sessions contain retained gap counters. Functional use is accepted; dropout-free operation is not claimed.
The separate Pi standalone Serum source candidate is unqualified for musical
use. FN1 retains the first-note gap; PW1 removes it only in the named
default-state fixture under an opt-in startup sequence.

### Claim limit

Counters are not automatically audible-dropout evidence. Elapsed Windows
`process()` time is not automatically thread CPU time. AS1's allocation result
does not establish a CPU, deadline, dropout or instrument-failure improvement.

### Related failure classes

FC-AUDIO-002, FC-CAP-001, FC-LIFE-002.

### Remaining gate

The separate FN1/PW1 source reviews do not authorize installation or a
general support claim. PSL1 is the selected product-owned, state-aware source
slice; its three-state physical classification remains pending. Other residual misses still need their own exact thread/queue
capture; the Pi result does not assign them a shared cause. Later resilience
work must reconcile bounded recovery on the shared core without importing
callback-side policy from the old fork.

### Evidence and historical sources

[AP16](AP16.md), issue #90, [Pi matched result](../evidence/pi-reconciliation/serum-default-2026-09-25.md), [FN1 Pi result](../evidence/fn1/serum-first-note-2026-09-25.md), [PW1 Pi result](../evidence/pw1/serum-default-prewarm-2026-09-25.md).

### Tracking issue

[#90](https://github.com/kasselvania/Linux-VST-bridge/issues/90).

### Last reviewed

2026-09-25.

---

## FC-AUDIO-002 — Host block exceeds the selected bridge presentation envelope

### Shared boundary

Native proxy `setupProcessing`, selected bridge presentation delay, and DAW audio configuration.

### Understanding

causal

The Ubuntu load refusal was an exact 1024-frame host request against a 512-frame publication, not a processing deadline miss.

### Implementation

accepted

The fail-closed product behavior is intentional; the accepted operating configuration is explicit Bitwig 512 samples at 48 kHz.

### User posture

supported-with-workaround

### Symptom

Bitwig requested a 1024-frame maximum; the 512-frame FRAGMENTS publication returned `kResultFalse` from `setupProcessing` and did not load.

### Mechanism

The selected bridge delay must cover the host's declared maximum block. The proxy refuses an unsupported processing shape rather than claiming it can present that block. `PIPEWIRE_QUANTUM=512/48000` alone left Bitwig's internal maximum at 1024; changing Bitwig's Audio settings to 512 samples and 48 kHz was necessary.

### Fix chain

- **Source correction:** none; the proxy's fail-closed refusal is the intended product law.
- **Built artifact:** exact registered revision-12 FRAGMENTS native proxy SHA-256 `f29e4cf0d3157308a78097b25f10a05264277291203c77a62db6cc1a2cfa4c1a`.
- **Profile/candidate:** Ubuntu FRAGMENTS review candidate revision 12, selected 512 added bridge frames, publication `d1273fdb5a50d9f73009bc6473cd3f36`.
- **Installed generation:** Ubuntu `095b21aa05d991f0e6ca2c5d471da8bd1e9ff8dab75c5a758fe583e5aed4509f`.
- **Physical result:** explicit Bitwig 512-sample/48-kHz Audio settings together with Bitwig-only `PIPEWIRE_QUANTUM=512/48000` preceded accepted load, editor, audible processing, parameter use, removal, save/reopen, and clean retirement.

### Product coverage

FRAGMENTS 1.3.1.6566 / Ubuntu: the 1024-frame refusal was observed; the exact 512/48-kHz configuration was physically accepted. No other product/platform configuration inherits this result.

### Claim limit

This does not qualify 1024-frame host blocks, 256 bridge frames, other sample rates, or arbitrary DAW configurations. It is separate from residual in-session deadline misses in FC-AUDIO-001.

### Related failure classes

FC-AUDIO-001, FC-LIFE-002.

### Remaining gate

Present an actionable manager/frontend message naming the requested host maximum and the supported selected maximum; retain the refusal until a separately qualified configuration exists.

### Evidence and historical sources

[Ubuntu-lab PR #5](https://github.com/kasselvania/Linux-VST-bridge-ubuntu-lab/pull/5), [manual functional checkpoint](https://github.com/kasselvania/Linux-VST-bridge-ubuntu-lab/blob/473ac0a926e6d95d470c002244b851857b83f41c/evidence/UA1/20260923T0623Z-frg1-manual-closeout/result.json), [SUPPORT_MATRIX](SUPPORT_MATRIX.md).

### Tracking issue

None; actionable configuration messaging is not yet selected work.

### Last reviewed

2026-09-24.

---

## FC-CAP-001 — Capacity enumeration versus lease-retirement race

### Shared boundary

manager capacity ownership

### Understanding

causal

### Implementation

none

The fail-closed race remains a documented issue.

### User posture

supported-with-workaround

### Symptom

An otherwise available class slot can be refused during concurrent lease retirement.

### Mechanism

Capacity enumeration can observe a durable lease path just as retirement removes it. The scanner refuses the changed custody rather than risking over-admission.

### Fix chain

- **Source correction:** none for this race.
- **Built artifact:** no race-repair artifact.
- **Profile/candidate:** no capacity-policy change.
- **Installed generation:** no race repair installed.
- **Physical result:** AP17 established the six-instance envelope while retaining this bounded refusal race.

### Product coverage

AP17 Steam Deck fixture. It does not invalidate the accepted six-instance envelope or permit over-admission.

### Claim limit

This race can create a temporary unnecessary refusal; it does not over-admit, double-refund capacity, or invalidate the accepted six-instance envelope.

### Related failure classes

FC-LIFE-002, FC-AUDIO-001.

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

### Shared boundary

manager catalogue, retained onboarding, registry ownership, scanner lock

### Understanding

causal

### Implementation

accepted

### User posture

supported

### Symptom

Managed experimental environments could have stale scanner evidence yet no lawful refresh action because they were absent from the ordinary installed catalogue.

### Mechanism

The earlier action route equated catalogue presence with all managed authority. Canonical rescan/retry now derives authority from current catalogue plus exact retained managed ownership and revalidates under the final scanner lock.

### Fix chain

- **Source correction:** merged managed-environment refresh and exact quarantine retry authority in canonical product [PR #143](https://github.com/kasselvania/Linux-VST-bridge/pull/143) and predecessor manager work.
- **Built artifact:** canonical CPI2 merge `407a37679cbe9ccc3bed86d1b33c9ed49728c7da`.
- **Profile/candidate:** retained FRAGMENTS revision-11 qualification candidate during Ubuntu retry; Deck Blackhole/Kontakt managed candidates retained.
- **Installed generation:** Ubuntu CPI2 generation `467cbbc5fbb8cddb0d65bd421b73b2f5f5a2a2a6b907679fc12e1f86fcd6fadc` for exact retry.
- **Physical result:** one exact quarantined FRAGMENTS retry produced a healthy inventory; the prior failed inventory remained in history.

### Product coverage

Blackhole and Kontakt on Steam Deck; FRAGMENTS catalogue-free retry on Ubuntu.

### Claim limit

This does not reopen installation or grant arbitrary rescans.

### Related failure classes

FC-MGMT-002, FC-LIFE-002, FC-BOOT-001.

### Remaining gate

Preserve the single canonical refresh/retry authority in future manager work.

### Evidence and historical sources

[PLUGIN_RELIABILITY_FOLLOWUP](PLUGIN_RELIABILITY_FOLLOWUP.md).

### Tracking issue

None; accepted behavior.

### Last reviewed

2026-09-23.

---

## FC-MGMT-002 — Exact verified host/source omitted across software generations

### Shared boundary

Immutable software catalogue, retained profile/candidate authority, and publication current-host verification.

### Understanding

causal

The exact registered Windows host and source manifest must remain available to a retained activation-permitted profile or selected candidate after an immutable software-generation change.

### Implementation

accepted

PR #149 carries required exact host/source pairs for activation-permitted profiles. Serum candidate C additionally required an exact candidate-package host selection before publication.

### User posture

supported

### Symptom

Pure LoFi and Deck FRAGMENTS remained physically published with intact module, native proxy, link and retained host bytes, yet manager readback reported `installed_host_mismatch` and `needs_attention`. The first Serum candidate-C package similarly disabled its publication action as historical candidate inputs before any Serum launch.

### Mechanism

A new software generation retained only its default Windows host/source pair, omitting a different exact pair still required by active verified profiles. The first candidate-C package selected a default pair different from the candidate's retained preparation pair. Neither mismatch was a stale inventory or missing proxy problem.

### Fix chain

- **Source correction:** [PR #149](https://github.com/kasselvania/Linux-VST-bridge/pull/149) carries forward only exact registered host/source pairs required by current activation-permitted profiles; the Serum transition used exact candidate-package host selection, not a new generic admission rule.
- **Built artifact:** canonical merge `32fa422581b29318dd2c4653120ab1b63203192d`, tree `4e90a002d8bbca01d9d46283c17947af4c30a62d`.
- **Profile/candidate:** current Pure LoFi and Deck FRAGMENTS ordinary profiles require the retained host/source pair; Serum candidate C `91b699291eb7b7d1ff6e725d5e6fd1abed88ea721dbd81a621dd2fb39d38d207` requires candidate B's preparation pair.
- **Installed generation:** corrected Serum candidate-C package `30d144c0b65437e7963692d434f930fe45faf2ec73527d5864580fc08eb913ac` selected that exact pair. The initial wrong-host generation `d4a08aa5cf9a29ca448b8c4a5c591e44d14e288928e8c0604e936261e5f011ee` remains a separate failed attempt.
- **Physical result:** after the host-retention repair, the operator confirmed Pure LoFi and FRAGMENTS audio/editor use and clean retirement; corrected candidate-C package enabled and completed exact Serum publication without a rescan or proxy rebuild. This does not imply the separate Serum touch-menu defect passed.

### Product coverage

Pure LoFi and Efx FRAGMENTS / Steam Deck: the prior mismatch and later product use are observed. Serum 2 / Steam Deck: first package refusal and corrected candidate-C publication are retained. Ubuntu FRAGMENTS uses separate platform authority and is not inferred from these Deck results.

### Claim limit

Retain only immutable host/source pairs already required by exact verified profile or candidate authority. This is not permission to admit arbitrary historical hosts, rescan to mask the mismatch, or substitute a different plug-in build.

### Related failure classes

FC-MGMT-001, FC-LIFE-002.

### Remaining gate

Every future product package/software generation must prove that its required exact host/source pairs survive current-host verification without rescan, proxy rebuild, or publication replacement.

### Evidence and historical sources

[Six-product Deck readback](../evidence/fl1-deck-fleet-readonly/README.md), [PR #149](https://github.com/kasselvania/Linux-VST-bridge/pull/149), [Serum candidate-C physical attempts](https://github.com/kasselvania/Linux-VST-bridge/pull/151).

### Tracking issue

None; accepted management law.

### Last reviewed

2026-09-24.

---

## FC-PLAT-001 — Native/Windows transport requires shared private loopback

### Shared boundary

platform network namespace adapter

### Understanding

causal

### Implementation

accepted

### User posture

supported

### Symptom

FRAGMENTS was discovered and published on Ubuntu but its native proxy could not connect to the Windows host's AP1 listener.

### Mechanism

The proxy and Windows host used `127.0.0.1` in separate network namespaces; a port recorded in shared files was not reachable across those two loopback stacks.

### Fix chain

- **Source correction:** merged Ubuntu adapter joins native proxy and Windows host to one private loopback namespace while preserving separate mounts and one-way PID authority.
- **Built artifact:** [Ubuntu-lab PR #5](https://github.com/kasselvania/Linux-VST-bridge-ubuntu-lab/pull/5), merge `473ac0a926e6d95d470c002244b851857b83f41c`.
- **Profile/candidate:** exact FRAGMENTS review candidate revision 12; no profile workaround for missing loopback.
- **Installed generation:** Ubuntu `095b21aa05d991f0e6ca2c5d471da8bd1e9ff8dab75c5a758fe583e5aed4509f`.
- **Physical result:** FRAGMENTS editor/audio and saved-setting recall completed after shared-loopback repair.

### Product coverage

FRAGMENTS / Ubuntu: verified-fixed.

### Claim limit

This does not authorize host networking.

### Related failure classes

FC-LIFE-001, FC-LIFE-002, FC-BOOT-001.

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

### Shared boundary

platform service startup and retained user selection

### Understanding

causal

### Implementation

accepted

### User posture

supported

### Symptom

The Ubuntu user service originally depended on earlier setup to populate volatile `/run` state; canonical startup also intentionally removed engineering publications.

### Mechanism

The service needed a generation-owned pre-start runtime preparer, while exact user selection intent needed to be separate from canonical publication reconciliation.

### Fix chain

- **Source correction:** packaged `prepare-frg1-runtime` helper, graphical-session activation, retained-selection record and canonical same-revision rollback in [Ubuntu-lab PR #5](https://github.com/kasselvania/Linux-VST-bridge-ubuntu-lab/pull/5).
- **Built artifact:** Ubuntu-lab merge `473ac0a926e6d95d470c002244b851857b83f41c`.
- **Profile/candidate:** exact FRAGMENTS review candidate revision 12, publication `d1273fdb5a50d9f73009bc6473cd3f36`.
- **Installed generation:** Ubuntu `095b21aa05d991f0e6ca2c5d471da8bd1e9ff8dab75c5a758fe583e5aed4509f`.
- **Physical result:** actual boot ID changed; runtime marker recreated; manager started without repair; same revision reselected and second readiness was a no-op.

### Product coverage

FRAGMENTS / Ubuntu: controlled restart and machine reboot verified.

### Claim limit

This is normal clean boot behavior, not universal automatic recovery from an interrupted active owner.

### Related failure classes

FC-LIFE-001, FC-LIFE-002, FC-PLAT-001.

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
