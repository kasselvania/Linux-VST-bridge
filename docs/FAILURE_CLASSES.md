# Shared failure classes

This ledger is the current map from reusable failure mechanisms to fixes, artifacts, physical coverage, and remaining gaps. It is organized by shared boundary rather than by chronological campaign or plug-in name.

Historical AP/UIO/UIR/IF documents remain authoritative for what a particular experiment observed. This ledger is authoritative for the project's current cross-plug-in understanding.

The 2026-10-02 [graphics/runtime source investigation](GRAPHICS_RUNTIME_REVIEW.md)
retains FC-GFX-001's exact rendering claim. BEAM's reported editor-open crackling
has no assigned cause: its historical V4 rendering/input result is not an audio
or acceleration qualification. Source inspection identifies controller-update
failure propagation to audio and an owner-exception/thread-retirement hazard.
Subsequent Windows SDK fixtures reproduce those boundaries; FC-UI-009 below
records the owner repair and explicit controller failure custody. Neither is
assigned to a commercial incident. The shared explicit graphics assessment now
adds module dependency hints, editor-time library observations and independent
Windows context probes through the existing inspector. The editor's own device,
child renderers and rendering-cost measurements remain gaps. No physical support
posture is widened.

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
| [FC-MIDI-001](#fc-midi-001--recognized-expression-rejected-an-entire-native-input-callback) | Recognized expression rejected an entire native input callback | Native VST3 proxy input admission | causal in source; physical attribution open | source-fixed | Pinned SDK fixture; Push / Deck operator report only | unqualified for Push expression | Build and publish exact proxy successor; physical Push/Bitwig release check |
| [FC-MIDI-002](#fc-midi-002--late-note-off-permanently-fails-processing) | Late note-off permanently fails processing | Native SDK signed timestamp conversion | causal in source and matched comparison | deployed | Pinned SDK regression; recovery1 Pure LoFi physical Deck comparison | qualified for this late-release recovery only | Residual timing and broader host/event qualification remain open |
| [FC-AUDIO-001](#fc-audio-001--residual-audio-deadline-misses) | Residual deadline misses | Native queue/Windows processing/scheduler | preemption and callback-burst loss attributed; older startup/output silence remain | recovery6 bounded completion repair passes physical SDK A/B and two short Bitwig runs; longer acceptance open | Pure LoFi Deck burst repair; no general audio qualification | blocked for dependable musical use | Frozen-artifact interaction/soak and separate output-silence attribution; no gap-free fallback established |
| [FC-AUDIO-002](#fc-audio-002--host-block-exceeds-the-selected-bridge-presentation-envelope) | Host block exceeds selected bridge presentation envelope | Proxy setup, selected delay, DAW audio settings | causal | accepted | FRAGMENTS / Ubuntu at Bitwig 512/48 kHz | supported-with-workaround | Actionable requested-versus-supported block message |
| [FC-AUTO-001](#fc-auto-001--automation-refusal-collides-with-terminal-silence) | Automation refusal collides with terminal silence | Native curve admission / SDK result interpretation | causal collision; sparse-curve capability still incomplete | deployed collision correction; whole-block successor source-only | Ubuntu reference effect explicitly refuses 0x107; state still fails | blocked for the failed saved-automation journey | Deliver paired protocol-14 whole DAW blocks and repeat recall; audio gaps remain separate |
| [FC-CAP-001](#fc-cap-001--capacity-enumeration-versus-lease-retirement-race) | Capacity scan versus lease retirement | Manager capacity ownership | causal | none | AP17 exact fixture | supported-with-workaround | Repair issue #93 |
| [FC-MGMT-001](#fc-mgmt-001--managed-inventory-refresh-authority) | Managed inventory freshness and refresh | Manager catalogue/registry/onboarding | causal | accepted | Blackhole, Kontakt / Deck; FRAGMENTS / Ubuntu | supported | Preserve one canonical refresh route |
| [FC-MGMT-002](#fc-mgmt-002--exact-verified-hostsource-omitted-across-software-generations) | Exact verified host/source omitted across generations | Software catalogue, profile/candidate and publication | causal | accepted | Pure LoFi, FRAGMENTS, Serum / Deck | supported | Preserve required exact pairs in every new generation |
| [FC-PLAT-001](#fc-plat-001--nativewindows-transport-requires-shared-private-loopback) | Native/Windows transport needs shared loopback | Platform namespace adapter | causal | accepted | FRAGMENTS / Ubuntu | supported | Regression gate for new adapters |
| [FC-MGMT-004](#fc-mgmt-004--managed-publication-is-mistaken-for-a-static-catalogue-fixture) | Managed publication is mistaken for a static catalogue fixture | Catalogue/publication ownership | causal | installed | Ubuntu internal22 normal status and product controls passed | resolved at publication readback | DAW use remains untested |
| [FC-MGMT-005](#fc-mgmt-005--partial-installation-retry-omitted-from-setup) | Partial installation retry omitted from Setup | Existing installer offers to Setup projection | causal | installed | Ubuntu internal56: two partial-stop and isolated-retry repetitions | resolved for the recorded partial retry | Remaining recovery cases and platform qualification |
| [FC-MGMT-006](#fc-mgmt-006--exact-prebuilt-catalogue-blocks-unfamiliar-plug-ins) | Exact prebuilt catalogue blocks unfamiliar plug-ins | Preparation/descriptor/publication | causal in source; Nibbi user report | installed Ubuntu unfamiliar Windows processing, state migration and restoration | No repaired Deck/Nibbi result | Deck blocked; reference candidate unqualified | Actual DAW workflow, dependency and runtime trials |
| [FC-STATE-001](#fc-state-001--saved-state-rejects-an-explicitly-selected-module-update) | Updated module rejects earlier saved state | Native state envelope / selected execution identity | causal for reference fixtures | installed reference repair passed | Ubuntu SDK instrument/effect; normal manager rollback | reference regression resolved; real DAW/commercial unqualified | Physical DAW update/recall and declared interaction/soak |
| [FC-PLAT-002](#fc-plat-002--delivered-runtime-lifetime-lock-cannot-be-opened) | Delivered runtime permissions conflict with upstream | Runtime extraction/pressure-vessel | causal | installed | Ubuntu -r3 acquired, installed and discovered trial; native publication completed | resolved at delivered runtime use | DAW usability remains open |
| [FC-MGMT-003](#fc-mgmt-003--whole-runtime-hashing-blocks-bounded-setup-admission) | Whole-runtime hashing blocks setup admission | Runtime integrity/status projection | causal | deployed | Ubuntu internal26 idle-service cold load/editor/audio captured; 72.440-second startup | unqualified | Shorten startup without weakening verification; inspect-to-DAW keeper transition |
| [FC-UI-008](#fc-ui-008--vendor-editor-removal-crashes-the-windows-host) | Vendor editor removal crashes the Windows host | Windows IPlugView removal / Wine UI Automation | causal null-provider defect; vendor caller unproved | exact process accessibility policy installed; isolated DLL guard is reference-only | Official FRAGMENTS 1.0.0 trial / Ubuntu internal30 close/reopen and retirement passed | review candidate; Windows screen-reader integration unavailable | Preserve bounded policy and verify persistence/usability separately |
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

Internal51 exposed a separate native-desktop gap in the same boundary:
Openbox/X11 supplied the private home-directory authentication file, but the
supervisor searched only the user runtime directory for an exact copy. Its
keeper refused before readiness, with `host Xauthority exact alias unavailable`;
no DSP was admitted. [The installed failure](../evidence/self-service-delivery/internal51-sdk-lifecycle-failure.json)
is retained. The source successor accepts the authenticated peer's exact
host-visible private file only when file identity and complete bytes match,
retaining the checked alias route for a private namespace. Internal52 passed
contended restoration and both actual SDK reference roles on the populated
Openbox/X11 fixture, with positive state, processing and retirement. That
installed development result does not qualify GNOME, commercial products or
DAW project recall; previous bounded physical results stay as recorded.

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

### Live installer recovery follow-up

The integrated delivery run also found an unstarted standalone inspection whose
exclusive environment lock refused while a retained keeper held its shared
lock. Acquisition occurred before the prelaunch finalizer, leaving a lease with
no result or retirement receipt. The ordinary UI had no recovery offer for this
abandoned raw CLI inspection and package transition remained blocked. The
source successor brings lock acquisition inside the exact prelaunch finalizer;
the inspection caller then applies canonical positive-receipt reconciliation
under the registry lock before reporting the retained failure. A successful
process exit with a wrong receipt still refuses. Fourteen Linux ownership/census
tests, 71 supervisor tests under a reaping harness, nine Rust inspection tests
including a real Python lock refusal, and strict all-target Clippy passed.
The selected internal38 required a controlled maintainer retirement using its
existing finalizer and exact positive receipt; that run remains failed. This
correction is not in internal39 and requires successor delivery. See the
[inspection failure and source checks](../evidence/self-service-delivery/inspection-lock-refusal-2026-10-01.json).

An internal39 Ubuntu project reopen also failed before host launch because the
desktop's authenticated session-bus address included the standard optional
server GUID. The supervisor rejected every comma, including this supported
D-Bus syntax. The source successor parses one filesystem socket address and its
optional GUID, preserves that server identity, and retains exact peer-generation
and device/inode checks. Unknown transports, ambiguous fields and malformed
escaping still refuse; an unmappable peer socket never falls back to the host
bus. Twelve Linux tests include a real `/proc` peer and socket connection.
This correction is not installed in internal39; the reopen remains failed.
See the [startup and input receipt](../evidence/self-service-delivery/native-input-and-graphical-startup-2026-10-01.json).

The new clean Ubuntu account on internal27 reached the official FRAGMENTS
trial's Finish screen, but its exact installer cohort remained live with 15
processes. Overview failed in 7.614 seconds with
`operator_current_artifact_changed_refresh`, hiding the normal Focus/Stop
controls. The supervisor rewrites even identical progress JSON every half
second, changing both the report inode and its directory timestamps.

Current readback now permits progress-report replacement for an exact live
installer. The operation's record and containing directory identity remain
bound, and both external probes require the same operation to remain live.
These progress bytes do not authorize installation, scanning or retirement;
the live row offers exact Focus/Stop. Retired results retain the full stamp
watch. Seven focused current-readback tests and all-target Clippy passed.
Internal28 was built, installed and selected through normal product controls.
Live installer recovery verification is pending.

The integrated successor separates exact Installer Focus/Stop admission from
runtime verification and global DSP capacity. It retains current operation,
environment and live-unit checks. The frontend no longer invalidates a fresh
Overview for every unchanged unavailable-service pulse or disables controls for
a silent refresh. Source tests cover damaged runtime, stale/dead/foreign targets
and stable inactive pulses. No installed GUI Stop/Retry pass follows from these
source results; the delivered frontend retest remains required.

Internal31 was installed and selected on the populated Ubuntu account through
normal package controls, retaining FRAGMENTS registration. Its headless hold
reached the live Setup card during unavailable capacity readback. Focus refused
because that fixture has no window; Stop was not successfully exercised before
natural retirement. Selected actions were below the viewport. The next source
correction prioritizes that card and explicitly disables expired offers in the
fast Setup view; a rendered click test permits fresh Stop without capacity and
refuses stale Stop. All 81 frontend tests pass. A first-party windowed installer
with a deliberate hold/partial-install hold is now source instrumentation;
installed Stop/Retry/discovery remains required.

Internal32 delivers the windowed fixture and both stateful reference modules.
The populated Stop → Select → Start package route retained the exact FRAGMENTS
registration. Normal Add Windows installer then refused the root-owned packaged
Recovery installer with “Choose an owned regular installer file.” No installer
ran and no Stop pass follows. The shared source-file eligibility correction
permits explicitly selected root-owned files without group/other write access;
read-only descriptor checks, private custody, hashing and source-change refusal
remain mandatory. The fixed delivered GUI journey is still required.

The old installed UI required an exact supervisor stop outside the GUI. It
completed in 2.804 seconds: cancellation, outer exit -15, zero owned live
processes, confirmed cleanup and durable files classified installed. This is
an engineering recovery, not a self-service recovery pass. The vendor hang's
cause remains unproved and its earlier result is retained.

### State-update failure follow-up

The general4 reference restore-refusal test exposed a distinct transport race:
the native owner consumed an outstanding `F` failure notice where it expected
the final `R` retirement acknowledgement, then closed before the supervisor
could reply. Windows process cleanup succeeded, transport cleanup remained
unconfirmed, and the native consumer was still alive at that decision. That
failed result and its offline VM checkpoint remain retained. A subsequent VM
restart is not counted as successful recovery of the case.

The general5 correction at `9da4dc66` accepts at most one pending `F` while still
requiring `R` within the original deadline. Separately, the supervisor can prove
the authenticated native process generation ended and retire its exact owned
resources without requiring acknowledgement from a dead process. Unknown
identity, a live peer with a failed handshake or incomplete cleanup still
refuses; neither positive path hides the original processing/restore failure.

Installed malformed-state, vendor-refusal and partial-restore cases now complete
SDK teardown and unload. A deliberately abrupt SDK consumer exit independently
passes exact process/transport retirement, with the authenticated generation-death
basis recorded. A healthy sibling keeps producing correct samples throughout
each failed-instance retirement. No passing general5 case uses reboot or manual
record deletion. The normal manager then restores exact retained predecessors.
See [FC-STATE-001](#fc-state-001--saved-state-rejects-an-explicitly-selected-module-update)
and the [installed comparison](../evidence/preparation/2026-10-03-state-update-installed.json).
This covers the declared Ubuntu first-party fixture; commercial and physical
DAW failure recovery remain unqualified.

### Claim limit

Abrupt power-loss recovery with active owners is not universally automatic.

### Related failure classes

FC-LIFE-001, FC-BOOT-001, FC-UI-007.

### Remaining gate

Provide an everyday manager recovery/panic workflow that preserves evidence and refuses uncertain cleanup.

### Evidence and historical sources

[PLUGIN_RELIABILITY_FOLLOWUP](PLUGIN_RELIABILITY_FOLLOWUP.md), Ubuntu-lab PR #5 history,
and the installed state-update comparison above.

### Tracking issue

No dedicated shared issue yet.

### Last reviewed

2026-10-03.

---

## FC-MIDI-001 — Recognized expression rejected an entire native input callback

### Shared boundary

Native VST3 proxy `Processor::process()` input-event admission, before the
bounded AP8 note transport.

### Understanding

The source mechanism is causal: before MIDI0, poly pressure or note-expression
value/text took the unsupported-event branch and rejected the entire callback.
A later note-off in that callback could be lost. The operator reports held
notes from Push 3 pads through bridged plug-ins, while a raw controller capture
showed matching note-on/off and recent bridge reports showed callback
rejections. The exact VST3 event type in those rejected physical callbacks is
not known, so physical attribution remains open.

### Implementation and coverage

`source-fixed` for valid, admitted callbacks with at most 256 events on bus
zero: the three recognized expression types are skipped, ordinary notes retain
order and identity, and the lifecycle report counts one affected callback.
Pinned SDK tests cover all three branches and unknown-event refusal. This does
not transport expression or establish MPE, and the currently installed native
proxy has not been replaced. Invalid callback inputs still refuse.

### User posture and remaining gate

Push 3 note release through the bridge is unqualified. Build an exact native
proxy successor, publish it through managed publication authority together
with any paired manager generation, then test note release and clean retirement
in Bitwig. A manager package by itself does not install the native fix. Keep
all existing product qualifications scoped to their original controller and
session evidence.

### Evidence

[MIDI0 source contract](MIDI0_PUSH_NOTE_RELEASE.md) and PR #189. No new
physical product result is claimed.

---

## FC-AUDIO-001 — Residual audio deadline misses

The [2026-10-03 general5 Deck interaction](../evidence/audio-recovery/2026-10-03-general5-deck-installation.json)
is a separate terminal-instance failure, not another established queued-dropout
reproduction. Pure LoFi's status mapping disappeared during a combined transport
and editor/resize-menu test. The private observer called that delivery failure;
initial narration incorrectly called it missing audio. Final native records show
zero underruns and rejections for both plug-ins, an explicit terminal failure for
Pure LoFi, and confirmed retirement for both. Bitwig refused temporary state saving
for the failed instrument before normal exit. Editor failure was sampled, but the
trigger and responsible shared boundary are not attributed. The 30-minute window
stopped after 497.630 seconds; its planned engineering soak never began. Do not
assign this incident to the historical deadline mechanism or count it as a pass.
The operator selected a platform architecture/method reassessment, not another
automatically selected vendor workaround.

### Shared boundary

callback admission, native worker, Windows processing, scheduler and reply path

### Understanding

Physical Deck render-thread preemption and callback-burst loss attributed;
older startup and editor-interval output silence remain unresolved.

### Implementation

installed scheduling capability and focused burst-delivery repair demonstrated;
dependable musical continuity remains unqualified

On 2026-10-02, recovery1 in Bitwig 6.1 reproduced five missing 512-frame
Pure LoFi blocks. The retained [request and scheduler witness](../evidence/audio-recovery/2026-10-02-render-preemption-witness.json)
places a continuous 6.000040 ms kernel preemption inside request 9072's SDK call
and across its presentation deadline. The SDK wall interval is not DSP CPU time.
A controlled RealtimeKit request changed only that render thread from SCHED_OTHER
0 to SCHED_RR 5 with RESET_ON_FORK and a 200 ms RTTIME ceiling. The matched copied
chain completed 19,480 Pure LoFi blocks and 19,104 FRAGMENTS blocks with zero
missing frames or rejections, effective end readback and confirmed retirement.
This short, traced helper intervention is not an installed product repair or
dependable-musical-use qualification. FRAGMENTS stayed under SCHED_OTHER 0.
Both whole-session recordings are retained privately; effect tails mean the
chain recording's lack of exact silence cannot exonerate an upstream gap.

The source candidate makes the bounded request from the owning supervisor via
Rust, outside the callback, once per render-thread start. Existing PID/start
custody and the mapped session status inode select the target. Readback or
unavailability is retained; another policy is preserved. This is post-start
capability acquisition, not a new readiness gate. It does not change buffers,
runtime, affinity, the Windows host or DSP.

The [installed recovery3 result](../evidence/audio-recovery/2026-10-02-owned-render-scheduling.json)
at `d5e710a2` verifies that both actual render threads received RR 5 with
RESET_ON_FORK automatically. Both native publications and the entire registry
were unchanged; the selected application's exact matching host/source pair
selected the new supervisor for both. With tracing disabled, Pure LoFi lost
512 frames in one gap over 40,076 blocks; FRAGMENTS lost none over 39,687.
The final clean live read precedes stop/save/close, so this narrows the gap to
that interval without attributing it to any particular action. A separate
512-frame exact silent span exists in the captured output during playback
while later bridge counters were clean. Its origin is unestablished.

A traced reopen and focused stop/save/restart comparison completed 27,828 Pure
LoFi and 27,435 FRAGMENTS blocks with zero gaps and confirmed retirement.
Neither that repetition nor the earlier manual intervention turns the failed
untraced lifetime into a pass. Normal Setup and Updates restored the exact
recovery1 application after testing; the failed candidate and all whole-session
captures remain retained. Stage 2 stays open: capture the native worker, render
thread and callback together across the residual gap before another repair.
The monitor silence also needs a separate native-reference comparison. The
30-minute interaction and longer soak gates are unperformed.

The next [bounded comparison](../evidence/audio-recovery/2026-10-02-residual-gap-observation.json)
on unchanged recovery3 completed 27,947 / 27,556 traced and 45,502 / 45,120
untraced Pure LoFi / FRAGMENTS blocks with zero missing frames and clean
retirement. Both had external scheduler capture and whole output recording.
This does not clear the failed prior lifetime or establish an observer effect.
The new source observation retains the first 16 missing presentation spans
without sample tracing, with close-only export and explicit omission counts.
Independent progress reads are not a causal or atomic snapshot. Its old-source
regression fails; 22 queued tests pass on macOS and 89 backend tests pass on Linux
(one existing Windows-fixture test ignored). Installed recovery4 preserves the
host/runtime/module identities and changes both native publications. A controlled
40 ms owned-process suspension with sample tracing off produced four retained
spans matching exactly 2,048 silent captured frames, followed by resumed audio and
clean retirement. That establishes observation coverage, not the cause of the
earlier natural gap. A 120-second native Bitwig 1 kHz capture passed the continuous
signal check after startup; the earlier isolated monitor silence remains
unattributed. Audio continuity remains blocked.

Recovery4 subsequently reproduced a two-block FRAGMENTS gap while both missing
requests were still waiting for native-worker consumption. A 23.273160 ms native
worker preemption spans both deadlines; this collector lacks the namespace
mapping needed to prove the exact numeric per-session TID link. In a later
exactly mapped intervention, both native workers received RR 5 and accumulated
no additional bridge gaps for about 630 seconds. Pure LoFi had already lost one
startup block; additional monitor silence remains unexplained. Full lifetimes
remain failed. The next source correction requests native scheduling during
worker preparation through the existing authenticated supervisor, preserving
DAW limits and recording independent effective readback. See the
[native preparation contract](../native-vst3-proxy/host/NATIVE_SCHEDULING.md).

That correction was built and installed as recovery5 from `8ade8723`.
[Both complete comparisons](../evidence/audio-recovery/2026-10-02-native-worker-scheduling.json)
verify automatic native-worker preparation and later Windows-render RR 5
readback. The first has no bridge misses over 52,935 / 52,544 blocks, but has an
unexplained 512-frame monitor silence during editor interaction. The second has
one 512-frame startup loss in each plug-in over 23,638 / 23,249 blocks. Native
scheduling was effective before those losses. The missing positions were still
in the worker's processing operation; the non-atomic observations do not locate
Windows execution versus transport wait. First-callback Windows policy and the
host's preceding callback timing are gaps in the evidence.

The additional direct recorder and scheduler collector began after both startup
losses. They subsequently failed independently: 43 recorder stream errors and a
scheduler byte-limit/cleanup timeout. Their outputs are not accepted audio or
causal scheduler evidence. All owned sessions and observers retired. Normal
controls restored recovery1 component bytes and both reference native binaries
under new selection/publication locations. Neither full comparison establishes
dependable audio; startup delivery and the separate editor-interval silence are
the next bounded attribution targets.

The next short recovery5 trace directly attributed a final-callback loss:
presentation occurred only 1.340 ms after admission, SDK processing took
1.943 ms, and publication followed the gap by 0.894 ms. No control work occupied
that request. A sample delay did not guarantee elapsed worker time. The speaker
capture has no internal exact-zero span during its musical signal, so this is
a measured delivery failure at close, not a claimed audible dropout in that run.

The [recovery6 result](../evidence/audio-recovery/2026-10-02-queued-completion-deadline.json)
adds one preallocated native completion notification and one actual-block-duration
budget for already-due protocol-14 output. It preserves delay, epochs, ownership,
counted timeout silence and late-result expiry. The unpaced source regression
fails on recovery5 and passes with exact delayed output and zero callback
allocations. Source `85c444ff` passed 96 Linux backend tests (one existing fixture
ignored), five Rust/five Python scheduling checks, and all ten build steps.

Installed recovery6 passed two short complete Bitwig lifetimes: 9,095 / 8,702
traced and 8,399 / 8,014 untraced instrument/effect blocks, with zero gaps,
rejections or expiry, clean retirement and no internal exact-zero capture span.
Neither exercised a wait. A separate diagnostics-disabled physical SDK A/B used
the same application, Windows host, runtime, module, state and consumer, changing
only the native proxy. A 40 ms suspension of the owned consumer generated
catch-up calls while a note was held. Recovery5 lost 1,536 actual captured frames;
recovery6 used three waits and lost none, with a 1.526402 ms full SDK process-call
maximum. Both arms retained state and retired. The SDK native workers remained
SCHED_OTHER (no finite host realtime budget); Windows RR 5 was verified in both,
and the separate Bitwig runs verified native RR 5.

This completes the focused burst repair, not FC-AUDIO-001. The deliberately
paused SDK consumer is not device-deadline acceptance, and it does not attribute
every historical gap. The frozen artifact still needs the declared interaction
and soak, with separate attribution of silence when bridge counters are clean.
Normal UI publication and package selection restored all reference application
and native bytes afterward, under new selection/publication locations. Other
publications, runtime/environment and original/reference projects are unchanged.
All owners/recorders are retired and the builder is stopped. The retained
reference itself remains unqualified for dependable audio.

The 2026-10-02 [audio recovery comparison](../evidence/audio-recovery/2026-10-02-late-note-off.json)
preserves missing audio in both the earlier usable Deck pair and an internal57
control. FC-MIDI-002 now has a physical note-release repair, but its short clean
SDK and Bitwig runs do not close this independent deadline failure class.

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

### User posture

Blocked for dependable musical use. Historical sound, editor and recall results
retain their stated scope; no clean whole-session workaround is established.

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

### Historical fix chain before the Deck recovery comparisons

- **Source correction:** AP16's private-tmpfs hot-transport mapping addressed the demonstrated backing-store class. AS1 [PR #172](https://github.com/kasselvania/Linux-VST-bridge/pull/172) removes recurring bridge-owned allocation from the covered request/reply path at the source stage only; no residual deadline-miss mechanism or fix has been established.
- **Built artifact:** AP16 corrected software revision `f6a19c78100fce548ae380ac043489d85d1543497ea20806e029526ad8fb8f0b`; no residual repair artifact.
- **Profile/candidate:** AP16 retained ordinary revision-7 LoFi/FRAGMENTS profiles; no residual candidate.
- **Installed generation:** AP16 corrected transport revision was installed on the Deck; Ubuntu FRAGMENTS remained on its accepted revision 12.
- **Physical result:** AP16 matched result for the backing-store class; residual gaps persisted in later Deck and Ubuntu sessions. The staged Pi A/B in [the exact Serum default-state result](../evidence/pi-reconciliation/serum-default-2026-09-25.md) retained a 1,280-frame first-note gap in all eight runs despite lower mean request service in the AS1 arm. No installed-generation or support claim follows.
- **Attribution:** FN1 [Pi default-state evidence](../evidence/fn1/serum-first-note-2026-09-25.md) records caller CPU/run time, FEX-boundary samples, minor faults and 116 new Serum-named guest JIT regions during the slow first-note call. This is diagnostic source and physical attribution only, not a repair or support change.
- **Pi source candidate:** PW1 [exact default-state evidence](../evidence/pw1/serum-default-prewarm-2026-09-25.md) records a fresh FN1 baseline with the 1,280-frame first-note gap and three final candidate runs with zero missing frames, exact second state readback and clean retirement. The cold 43 ms call moved into bounded startup. No installed-generation or general support claim follows.

### Product coverage

The integrated source successor adds startup/processing counters keyed to an
exact successful START epoch. Its callback uses fixed atomic counters only;
the worker retains bounded phase records and writes after processing/retirement.
All 19 queued-backend tests pass, including 1,100 production callback blocks
with zero allocation/reallocation/free calls, wrong-epoch refusal and whole-total
reconciliation. This is instrumentation, not an audio-gap repair. Internal32
does not contain it; no installed or physical dropout-free claim follows.

Arturia Deck sessions and FRAGMENTS Ubuntu sessions contain retained gap counters. Functional use is accepted; dropout-free operation is not claimed.
The internal36 Ardour reference instrument/effect run now retains explicit
processing-phase misses: 599,294 frames for the instrument and 303,104 for the
effect. Its ten-second output capture is all zero. The effect separately
refused input events with code 258; the exact rejected event was absent from
that generation's report. A source diagnostic retains the first bounded invalid
event without callback allocation, I/O or changing admission. Three native
SDK tests pass, including the real preloaded callback audit. This is attribution
instrumentation, not a repair or a passed musical session; the causal audio
work remains open. See the [retained phase and failure receipt](../evidence/self-service-delivery/native-input-and-graphical-startup-2026-10-01.json).
The separate Pi standalone Serum source candidate is unqualified for musical
use. FN1 retains the first-note gap; PW1 removes it only in the named
default-state fixture under an opt-in startup sequence.

Deck internal43 still failed: Pure LoFi retained 39,680 missing processing
frames and FRAGMENTS 6,912. Temporary exact-worker scheduling promotion did not
establish a fix; FRAGMENTS retained 512 more missing frames afterwards. Later
policy readback was absent and Pure LoFi's early trace capacity was exhausted.
The next native diagnostic preserves the first 16 and most recent 16 gaps and
each correlated mailbox reply's Windows timing, with displaced coverage counted.
All 83 serial native-library tests and Clippy pass, with one Windows-fixture
test ignored. It changes observation only and is not installed in internal44.
See the [internal43 diagnostic](../evidence/self-service-delivery/internal43-audio-priority-diagnostic.json)
and [integrated delivery status](INTEGRATED_BETA_DELIVERY.md).

Internal52's coherent reference publications, using the whole-block host47,
passed sixteen installed SDK record/recall consumers. The independent oracle
compared 15,667,200 stereo samples; all processing-phase missing-frame counters
were zero and actual owners retired positively. Startup and processing priming
remain retained, and this is shorter development instrumentation on a VM, not
the declared ten-minute DAW workload or a commercial/hardware qualification.
The [SDK receipt](../evidence/self-service-delivery/internal52-installed-sdk-lifecycle.json)
does not close the prior Deck commercial gaps or qualify kit52's unpublished
successor proxy bytes.

Internal56's physical Deck update/save/reopen completed ordinary controls,
but audio remains failed. First traced recall lost 2,048 Pure LoFi frames before
its first note. A second reopen was clean for the ten-minute musical subset,
then ended with 2,560 Pure LoFi and 2,048 FRAGMENTS missing frames after editor
activity. A tracing-disabled comparison still lost 1,536 Pure LoFi frames.
Tracing is therefore not the sole cause. Retained request 66595 spent 1.175 ms
in FRAGMENTS SDK processing but 19.574 ms in native admission-to-publication;
CPU accounting queries polluted the broader diagnostic brackets. The source
successor removes those queries and reports the existing exact SDK duration
with each gap. This is a measurement correction, not a residual-audio fix.
See the [physical results and limitations](RESTART_VALIDATION_2026_10_01.md#internal56-physical-deck-update-recall-and-audio-failure).

### Claim limit

Counters are not automatically audible-dropout evidence. Elapsed Windows
`process()` time is not automatically thread CPU time. AS1's allocation result
does not establish a CPU, deadline, dropout or instrument-failure improvement.

### Related failure classes

FC-AUDIO-002, FC-CAP-001, FC-LIFE-002.

### Remaining gate

For the selected Deck recovery, attribute the first startup request/reply loss
with observation present before audio begins; independently qualify the output
recorder before attributing editor-interval silence. Recovery5's automatic native
scheduling passed its capability check and failed continuity. A later render
policy readback does not prove first-callback readiness.

The separate FN1/PW1 source reviews do not authorize installation or a
general support claim. PSL1 is proposed next to make preparation product-owned
and state-aware. Other residual misses still need their own exact thread/queue
capture; the Pi result does not assign them a shared cause. Later resilience
work must reconcile bounded recovery on the shared core without importing
callback-side policy from the old fork.

### Evidence and historical sources

[AP16](AP16.md), issue #90, [Pi matched result](../evidence/pi-reconciliation/serum-default-2026-09-25.md), [FN1 Pi result](../evidence/fn1/serum-first-note-2026-09-25.md), [PW1 Pi result](../evidence/pw1/serum-default-prewarm-2026-09-25.md).

### Tracking issue

[#90](https://github.com/kasselvania/Linux-VST-bridge/issues/90).

### Last reviewed

2026-10-02.

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

### Delivered-runtime demo follow-up

The continued disposable check selected explicit 48 kHz and visibly entered
512, but Bitwig 6.1.1 native PipeWire reverted to 1024. ALSA reported the device
busy; JACK had no server. The original PipeWire backend was restored. This
rules out those ordinary settings as a completed repair on this fixture.
Source now adds a generic explicit 1024-frame testing preference, available
only for the exact matching prebuilt successor that declares the larger
envelope. Defaults and historical support claims remain unchanged. See
[delivered host-block configuration](SELF_SERVICE_AUDIO_BUFFERING.md). Built
artifact and installed DAW result are still pending for this successor.

Internal23 on the fresh Ubuntu application account requested 1024 samples at
48 kHz against a selected 512-frame bridge. Bitwig's numeric 512 edit was visible
but reverted to 1024, confirmed in native lifecycle readback. No effective 512
configuration, processing callback or editor open is claimed. This is a
self-service configuration gap in the delivered journey. The same attempt also
observed an independent vendor initial-state refusal, documented under
FC-MGMT-003's installed follow-up. No new audio-delay or RT implementation is
introduced by runtime delivery.

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

### Integrated retained-proxy capability follow-up

Internal33 selected a changed native kit on the populated Ubuntu account while
retaining the original FRAGMENTS registration. Home then refused
`candidate_runtime_contract`. The 1024-frame publication check consulted only
the successor kit; its changed native digest could not prove the older proxy's
capacity. Host-pair retention alone does not retain every publication capability.
This is a new measured boundary; it does not assign that cause to older host
reports. Internal33 is a failed development candidate.

The source correction follows the exact published candidate's retained recipe
when the selected kit does not describe that proxy. The publication, original
runtime/host/source binding, immutable kit digest and module/class/native index
must all match. It neither searches arbitrary kit directories nor enlarges a
proxy based on a new manager. The regression changes the selected kit, retains
the old publication and 1024-frame selection, and rejects foreign targets,
missing retained runtime and changed recipe bytes. All 37 preparation tests and
264 manager binary tests pass (two existing tests ignored); all-target Clippy
passes. The actual ordinary Stop → Restore previous version → Start route
restored internal32, its running frontend and identical software/registry hashes
without state edits or manual process termination. Home again showed Bridge
ready / compatibility unqualified. See the
[internal33 development receipt](../evidence/self-service-delivery/ubuntu-internal33-development.json).
The fixed internal34 successor selected on the same populated Ubuntu account;
Home reported Bridge ready with the existing FRAGMENTS registration unchanged.
It imported the root-owned first-party Recovery installer through Setup. Visible
Stop cancelled the exact held operation while general capacity was unavailable,
confirmed zero remaining owned processes, then the same card offered a fresh
isolated retry. That attempt discovered both reference products and published
the reference effect through ordinary controls. Focus, DAW recall and the full
recovery matrix remain open. The installer still reported partial installation:
its detector ignored `.vst3` files and required a registered `.exe`. The source
successor captures exact changed PE plug-in images without claiming completeness,
authorization or first use. A separate unknown descendant exit 1 remains retained
and unattributed. See [internal34](../evidence/self-service-delivery/ubuntu-internal34-development.json).

Internal36's signed native installer staged on Ubuntu and SteamOS through the
ordinary file browser and installer. On the populated Ubuntu account, visible
Stop → Select → Start selected the successor and retained all three published
registrations. SteamOS staging preserved the selected software and registry
digests exactly and changed no protected-system files. Its Setup preflight
refused `publication_component_generation_unavailable`: the selected preparation
kit already owned the 5fa090 host/source pair used by four publications, but
pair resolution consulted only the default host and static catalogue. The Deck's
working generation remains selected. This is a measured migration gap, not lost
module bytes or a license problem.

The source correction follows only the exact immutable kit selected by a
software generation and its verified staged preparation runtime. It preserves
that generation's supervisor/ownership pair through subsequent package
predecessors, without searching arbitrary historical kit directories. The
regression covers a changed default host, supervisor, ownership and preparation
kit, and refuses missing runtime, changed host bytes and mismatched source.
All 39 package-authority tests pass on macOS. The delivered correction and
commercial project recall still require installed verification. See the
[internal36 receipt](../evidence/self-service-delivery/internal36-installed-delivery.json).

Internal37's delivered correction selected the populated Deck successor without
changing its eight publications. A licensed Bitwig project with Pure LoFi and
FRAGMENTS reopened with meaningful state and automation and produced a captured
signal. Its native proxies remained the predecessor binaries. The subsequent
normal rollback restored exact software and preserved registry/project digests,
but final activation readback required a package version record absent from the
verified legacy installation. Setup could not offer Start, leaving the service
inactive. The update/rollback journey failed. The source successor reuses the
exact verified legacy classification for selected status and explicit restart,
refuses missing modern records, and compares the full selected component set
before and after activation. Forty package-authority tests passed; installed
restart and repeat musical acceptance remain open. See the
[internal37 receipt](../evidence/self-service-delivery/internal37-installed-delivery.json).

The following signed user-space package route stages verified PKG0 inputs without
changing the selected installation or protected operating system. Source tests
cover invalid signatures, mismatched bytes, links, low disk and Python mismatch;
the real SteamOS staging/update/rollback journey is still required. See
[user-space delivery](USER_SPACE_PACKAGE.md).

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

## FC-MGMT-003 — Whole-runtime hashing blocks bounded setup admission

### Shared boundary

Runtime identity/integrity verification versus manager status projection and
action serialization. This is independent of the selected vendor installer.

### Understanding

causal

The disposable Ubuntu delivery fixture recorded a 19.067-second installed
Overview and a 10.000114-second canonical-lock timeout for Continue setup.
The request was refused before environment creation or installer launch.
Readback traversed and hashed the entire newly delivered runtime under
projection serialization. This does not reattribute older manager failures.

### Implementation

source-fixed

`c99d64da` keeps a completed byte-observation cache outside the immutable
runtime. Projections check exact per-file device/inode/size/owner/mode and
modification/change times, directory roster and symlink targets. Missing or
invalid caches require byte verification before projection locks. Execution
ignores the persistent observation cache and verifies runtime bytes.
`823cff4f` also buffers the existing size-limited JSON reader. Internal15
installed the cache correction but still recorded 13.014-second warm status
and a cold frontend timeout; that partial result is retained.

### User posture

resolved at setup admission on internal16; package update status then timed out
on internal17 with a retained failed environment. Internal18 installed the
retirement correction and completed ordinary update, selection and activation;
Bootstrap status took 1.322 seconds. Commercial execution remains open.

### Fix chain and product coverage

- Source: `c99d64da`; focused changed-file, added-file and forged-cache launch
  refusal checks passed. Full serial manager checks passed (253, one ignored).
- Build/candidate: internal16 at `823cff4f`; package-time native rebuild checks
  passed.
- Installed generation: internal16 selected and activated normally; internal15
  retained. Two installed Overviews took 0.557 and 0.564 seconds. Normal Continue
  setup completed in 43.272 seconds with environment_ready. Earlier failed and
  slow attempts remain retained. The long creation job temporarily made status
  unavailable; this is not a broad responsive-UI qualification.
- Follow-up: internal17 system package installed, but Bootstrap status timed out
  before user selection. `all_retired` had loaded execution-verified records.
  Its correction reads exact custody and correlated retirement reports without
  requiring intact executable bytes. Launch admission keeps full verification.
  Focused damaged-runtime, unknown-cleanup and foreign-operation tests passed.
  Internal18 installed this fix; Bootstrap status took 1.322 seconds and the
  normal package switch and activation completed, retaining internal16.
- Physical result: installer import and the setup refusal were observed through
  the normal app on Ubuntu 26.04.1. No plug-in execution or audio result.

### Scoped product readback follow-up

Internal20 discovered FRAGMENTS, but its product controls exceeded the frontend
deadline. Direct product readback completed in 28.110 seconds. The capture used
validated runtime observations; subsequent preparation and scoped environment
projections left that readback scope and rehashed runtime bytes. Source now
extends the same observation scope over the complete read-only product response
and current-offer validation. The queued mutation/execution owner remains
outside that scope and performs full verification. Internal21 installed through
normal successor selection and activation, retaining internal20. Product readback
took 2.173 seconds, and the normal GUI loaded exact FRAGMENTS controls and
accepted Check compatibility. The earlier 28.110-second failure remains retained.

### Claim limit and remaining gate

The setup admission retest passed. Prove actual runtime execution and retain
commercial outcomes separately. This cache is a performance observation,
not a sandbox, authorization flow or compatibility qualification.

### Evidence and tracking

[Ubuntu delivery result](../evidence/self-service-delivery/ubuntu-fragments-trial-2026-09-29.json),
[PR #200](https://github.com/kasselvania/Linux-VST-bridge/pull/200).
Last reviewed: 2026-09-29.

### Native admission follow-up on the delivered runtime

Internal22 guest-demo Bitwig recognized the published FRAGMENTS proxy but its
first instance failed after 69.783 seconds with setupProcessing kResultFalse.
No Windows session or memory transport was created. The manager later reported
Broken pipe. Native admission's retained-authority path repeats full runtime
byte hashing before it can stage an environment keeper, exceeding the existing
65-second native admission envelope. This is a launch validation gap, separate
from status-cache behavior and from Windows plug-in processing.

The correction scopes freshly computed hashes to one native admission. Every
new admission hashes all runtime bytes; repeated checks reopen and revalidate
exact file identities before reuse. No persisted observation cache authorizes
execution, including nested readback calls. The scope ends before DSP supervisor
launch; deadlines, keeper policy, security posture and DSP are unchanged.
Regression tests reject changed bytes during the same scope and forged runtime
observation stamps. Internal23 was independently built and normally selected
and activated, preserving internal22. The native load reached exact Windows
module session `c1ed490a85ec31578c97e29d89749b17` and one witnessed shared-memory
mapping/connection. This closes the earlier before-host boundary on this one
fixture; it does not establish successful DAW activation. Load still failed
after 70.481 seconds, with no processing or editor opens. Initial vendor state
capture returned SDK result 1 and zero bytes/writes, with no stream failure;
subsequent state requests were explicitly refused. Its cause is not established
and must not be relabeled as licensing or bypassed with fabricated state. The
actual request was 1024/48000 against 512 selected frames (FC-AUDIO-002). Normal
product controls have no action to open this installed trial editor before DAW
activation. The official vendor trial choice was not reached. A command-line
access mechanism is not a delivered self-service action. Windows-host and
transport retirement was positively confirmed; zero callback frames were
processed. Retest the trial-access, exact state and supported block boundaries
separately; the earlier Ubuntu-lab workaround does not qualify this runtime.

### Integrated startup follow-up

Internal30's retained cold refusal identifies `keeper_starting`. That proves
the observed refusal phase, not a universal cause of prior slow launches. The
integrated successor performs fresh immutable-byte verification before taking
registry admission, joins the existing environment keeper, releases admission
while waiting, and rechecks publication/performance before exposing a lease.
The wait retains the existing bounded startup budget; failures and actual
capacity limits remain explicit. Control-worker phase records separate binding
verification, keeper preparation, transport and supervisor readiness. Source
tests establish reservation release, repeated admission, refusal and deadline
behavior. Internal31's ordinary cold live preview was accepted without Reload
in 44.610 seconds: verification 24.586 seconds and keeper readiness at 44.202.
This exceeds the 30-second responsiveness target; complete vendor initialization
and steady audio were not measured. The next source correction shares verified
byte preparation within one manager process, reopens/checks exact identities,
refuses stale/changed bytes and disk-cache authority, and releases its bounded
preparation lock before keeper/DSP work. Focused tests pass. It is absent from
internal32; repeated installed cold/warm measurements remain pending.

Internal52's installed SDK suite passed both reference roles. First state access
took 24.806 seconds; the remaining fifteen warm launches took 11.367–12.674
seconds. Retained phases place 8.070–8.599 seconds
in initial binding verification and about three milliseconds in ready-keeper
admission. A separate read-only probe observed 6,799 runtime file identities
change and 1.46 GB of manager reads across two successful launches. The acquired
runtime omits the component enabling `NativeProtonSession`; the supervisor
therefore creates a fresh outer runtime for every instance. This invalidates
process-owned observations of the mutable runtime-copy hardlinks.

The source successor derives only the exact acquired runtime's command
client/service pair from its already pinned tree. It reuses the existing keeper
and custody owner without changing runner files, prefixes, bounds or byte
verification. The differential selection test refuses the old absence and all
32 command-owner tests pass against the correction. Installed startup and
retirement validation is still required. The [warm-delay receipt](../evidence/self-service-delivery/internal52-warm-verification-delay.json)
preserves observations, inference and nonclaims separately.

Internal53 passed normal package selection but failed its first installed state
capture before Windows processing. Its command keeper assumed a cache parent
existed and claimed the endpoint before exclusive creation; finalization then
disputed a nonexistent directory. A separate actual runtime probe also proves
that the host-compiled supervisor bytecode cannot load in runtime Python 3.13.5.
The connected source correction creates/verifies private roots, claims and
retires only its exact directory, and sends a fixed isolated text bootstrap
through the unchanged inherited-fd/kernel-identity handshake. Exact failed keeper
reports now refuse capacity instead of projecting available work. Frozen53
fails both fresh-cache finalizer reproductions; 37 corrected owner tests and an
actual pinned command-service/client interface test pass. Installed successor
audio/state/retirement and complete frontend qualification remain pending. See
[failed53](../evidence/self-service-delivery/internal53-runtime-owner-failure.json).

Internal54 installed that correction. Its first effect record/recall pair
exercised the actual acquired-runtime command bootstrap and passed meaningful
state, automation, processing and exact retirement, with no processing missing
frames. First state access was 18.759 seconds; warm state access was 4.324 seconds.
This uses retained native51/host47 publication bytes and is not DAW qualification.

The subsequent full SDK workload failed consumer eleven: one active 1024-frame
instrument block was missing and expired, with 2,044 mismatched stereo samples.
Ten earlier consumers passed; state capture and exact retirement also passed
in the failing consumer. Callback time was below the declared period and task/
memory ceilings were not exhausted. Neither establishes the cause of the late
block. The independent host lacks wake-lateness and first-gap timing; bounded
observations are required before another audio correction. This failed run
does not close FC-AUDIO-001. See [the retained SDK failure](../evidence/self-service-delivery/internal54-sdk-full-failure.json).

Ordinary inspection and prebuilt preparation then each completed their own work
but refused service restoration. The first recovery created keeper specifications
about 55 seconds after service start, near its 60-second overall deadline; ready
reports appeared roughly 18 seconds later. The pre-staging interval is measured,
but its internal attribution remains a gap. No deadline is increased. Recovery
also expanded one prior keeper to all registered environments, and pure prebuilt
construction unnecessarily stopped them. The selected correction removes that
unneeded suspension and restores the exact previous owned bindings, retaining
fresh verification and readiness for each. Legacy records and failed results
remain distinct. See [the failed installed run](../evidence/self-service-delivery/internal54-frontend-restoration-failure.json)
and [the ownership explanation](RESTART_VALIDATION_2026_10_01.md#internal54-installed-result-and-restoration-reproduction).

The subsequent unchanged-candidate timing repetition failed again with only
0.276 ms of consumer wake lateness. Native protocol-14 observation was disabled,
and the Windows trace switch was lost when HOME was isolated. The source
successor corrects both observation paths; the processing gap remains unresolved.
Recovery now restores only the exact operation's control-service availability,
leaving environment initialization and processing readiness to native admission.
This supersedes prior-keeper reconstruction; prebuilt preparation also avoids
suspension. A separate post-SIGKILL cleanup race is corrected within the existing
deadline. All 347 runtime tests pass with an init reaper. These are source results,
not installed recovery or audio qualification. See the
[closeout continuation](RESTART_VALIDATION_2026_10_01.md#closeout-continuation-timing-and-source-corrections).

Internal55 subsequently passed installed control-service restoration and
preparation without restarting its keeper. Its new proxies passed an effect
probe and four full SDK suites, including three at the earlier two-environment
scope. Both trace paths now produce observations. These traced diagnostic runs
had zero processing gaps; they do not establish a repair of the original
intermittent gap or real DAW/commercial reliability. The exact
[installed results](RESTART_VALIDATION_2026_10_01.md#internal55-installed-recovery-and-diagnostic-regressions)
retain that distinction.

The subsequent combined Ardour run failed: 276,353 effect frames and 279,862
instrument frames were missing during processing. The exact fixture used
PulseAudio; its first retained consumer intervals were only 249–264 microseconds.
Later queue/reply stalls remain separately unattributed. All whole-lifetime
counters and successful owner retirement are retained in the
[internal55 DAW failure](../evidence/self-service-delivery/internal55-ardour-pulseaudio-failure.json).
This does not establish an audio repair or a general scheduler cause. The ALSA
comparison was stopped after 468 DAW xruns; even the subsequent no-plug-in
baseline accumulated 20 xruns with zero bridge DSP owners. The audio fixture
itself fails. Actual reference-state reopen and fresh recapture did match both
saved payloads, separately from the still-failed audio result. See
[the bounded comparison](../evidence/self-service-delivery/internal55-daw-recall-and-fixture-failure.json).
The approved two-CPU no-plug-in comparison also failed (41 xruns, zero bridge
DSP owners). Its independently bracketed interval averaged 0.190 outer core
with no outer throttling, so expanding CPU alone did not resolve the fixture.
The earlier no-plug-in one-CPU bracket averaged 0.435 core; do not substitute
the separate near-one-core plug-in transition for that baseline. The tests have
different durations and do not establish an xrun-rate improvement. See the
[two-CPU record](../evidence/self-service-delivery/internal55-two-cpu-no-plugin-failure.json).

Internal56's subsequent JACK/PipeWire comparison also failed with diagnostics
disabled: 677,464 effect frames missing in 263 gaps, and 183,296 instrument
frames missing in 56 gaps before a distinct input refusal. The native witness
records a note-off at signed offset -1,661 in a 1,024-frame block. That explains
the instrument's permanent refusal, not the preceding or sibling gaps. Its
state save failed and the new snapshot has no managed instrument envelope;
original snapshots are unchanged. The connected virtual sink capture was
silent. Both Windows owners retired, while native instrument lifecycle remained
failed. The no-plug-in JACK baseline and combined graph errors remain visible,
without a unique scheduler attribution. See the
[internal56 audio and save failure](../evidence/self-service-delivery/internal56-jack-audio-and-save-failure.json).

## FC-PLAT-002 — Delivered runtime lifetime lock cannot be opened

### Shared boundary and understanding

Causal runtime extraction/pressure-vessel startup failure, before vendor launch.
Internal16's normal Run installer exited during prefix initialization with status
1. The retained diagnostic identifies permission denied at the exact zero-byte
SLR platform `files/.ref`. The extractor had set it to mode 0400. Upstream opens
this lifetime lock read/write; this is not an Arturia activation or editor failure.

### Implementation and user posture

Source correction: the exact empty lock receives mode 0600; all other regular
files remain 0400/0500. Its empty digest and size still require verification.
The corrected runtime identity is `managed-ge-proton11-7-slr4-20260805-r2`.
Existing runtimes/environments are retained, never rewritten in place.
Revision -r3 below addresses a subsequently observed platform-mode conflict.

A fully retired receipt that positively records prefix initialization failure
before target launch may offer a new isolated attempt. A target launch, dropped
stages, unresolved cleanup or installed result still refuses that recovery.
The original durable-outcome report is preserved; no vendor state is inferred.

### Fix chain, coverage and remaining gate

- Source: focused lifetime-lock extraction and isolated-retry refusal tests.
- Build/installed: internal18 at `06a33e49` acquired the distinct corrected
  runtime through ordinary Setup in 303.67 seconds: 16,911 verified entries and
  the exact lock mode 0600. Earlier runtime/environment records remain retained.
  Its retry message lacked a button for the indeterminate durable outcome;
  internal19 at `8e6dfea9` installed the narrow pre-target recovery correction.
- Observed recovery: normal New isolated attempt completed in 118.289 seconds,
  retaining and linking the failed attempt. Ordinary Run installer completed
  prefix initialization with exit 0, observed the exact target SHA and displayed
  the official Arturia Efx FRAGMENTS 1.0.0 license agreement. No security
  protection was disabled and no runtime permission repair was performed.
- This startup result did not qualify plug-in compatibility. The operator later
  confirmed the agreement and installation completed; the next shared failure
  and its remaining gates are retained below.

### Post-install platform-mode failure

The operator confirmed the agreement; the official installer completed with
exit 0, installed durable outcome, zero owned live processes and confirmed
cleanup. The following Overview refused `managed_runtime_file_changed`.
Read-only diagnosis checked all 13,070 regular runtime files: no changed bytes,
but 6,274 platform files changed 0400 to 0644 and 434 changed 0500 to 0755.
Upstream `pv_runtime_create_copy` hard-links the platform payload into a mutable
sysroot and calls mtree application with chmod normalization. This is another
conflict in the extraction permission policy, not an Arturia licensing failure.

Source revision -r3 gives platform payloads their canonical 0644/0755 modes
inside private 0700 directories. Exact byte, mode, owner and roster verification
remains; other runtime components remain 0400/0500 and the lifetime lock 0600.
The regression exercises hard-link chmod and rejects changed bytes through
that shared inode. All six runtime tests passed. Earlier runtime/environment
artifacts remain unchanged. Internal20 on a fresh application account acquired
-r3 normally, installed the official vendor product with exit 0 and confirmed
cleanup, passed post-install Overview in 4.129 seconds and discovered the exact
FRAGMENTS class. No trial usability, native publication, sound or persistence
result is claimed.

[Ubuntu delivery result](../evidence/self-service-delivery/ubuntu-fragments-trial-2026-09-29.json),
[PR #200](https://github.com/kasselvania/Linux-VST-bridge/pull/200),
[upstream runtime lock](https://gitlab.steamos.cloud/steamrt/steam-runtime-tools/-/blob/main/pressure-vessel/runtime.c),
[upstream file-lock implementation](https://gitlab.steamos.cloud/steamrt/steam-runtime-tools/-/blob/main/steam-runtime-tools/file-lock.c).
Last reviewed: 2026-09-29.

### Final native-admission recheck

Internal25's cold attempt again expired before DSP while the keeper started.
A warm ordinary reload loaded in 38.029 seconds and rendered the vendor DEMO
editor with the installed 1024-frame configuration. The final keeper/history
recheck was outside the fresh-byte preparation scope and reread the complete
runtime. The follow-up moves that exact final recheck inside the scope, after
all fallible preparation and before exposure. File identities are rechecked;
new admissions still read every runtime byte. Internal26 loaded from an idle
service without a manual reload, rendered the demo editor and produced captured
processed audio. The complete DAW load still took 72.440 seconds. This does not
qualify startup responsiveness or the inspect-to-DAW keeper transition. The subsequent editor-removal failure is FC-UI-008, not this class.

## FC-MGMT-004 — Managed publication is mistaken for a static catalogue fixture

Internal21 completed normal prebuilt preparation and managed test publication
for the exact official FRAGMENTS trial module. Registry, ELF bytes and the DAW
link remained present, but subsequent Overview failed in 0.292 seconds with OS
NotFound. A package without a static native catalogue still delegated every
nonempty registry to the legacy sealed FRG1 fixture. That is a shared catalogue
ownership gap, not an Arturia installation or authorization failure.

The catalogue boundary now recognizes exact preparation-owned publications:
candidate-derived registration and retained authority, committed publication
transaction and exact published/removed physical disposition. No static
catalogue is invented. Unknown legacy entries, changed registration, missing
completion and pending or foreign links refuse. The historical FRG1 contract
remains separate. Setup, operator readback and onboarding use that shared owner.
The source regression covers publication, removal and negative identity and
completion outcomes. Internal22 passed normal successor selection and service
activation, retaining internal21; Overview passed in 2.198 seconds and product
readback in 5.413 seconds. Normal controls show Available experimentally and
untested DAW capabilities. Exact native bytes and publication remained intact. Bitwig's separate agreement is now accepted and guest demo mode opened. The
first DAW load failed during admission before a Windows host session existed;
sound, editor and persistence remain open.

[Ubuntu delivery result](../evidence/self-service-delivery/ubuntu-fragments-trial-2026-09-29.json),
[PR #200](https://github.com/kasselvania/Linux-VST-bridge/pull/200).
Last reviewed: 2026-09-29.

## FC-MGMT-006 — Exact prebuilt catalogue blocks unfamiliar plug-ins

The operator reported a missing exact prebuilt proxy while preparing newly
installed Nibbi. Source `9a0766afbf71fbb336c897c98b003b9e4bdda737` confirms the
shared refusal: `tools/mf3/native_builder.py::prebuilt` requires exactly one
shipped class-ID/module-digest match before it generates or validates the
descriptor. An absent entry raises
`prebuilt_proxy_unavailable_for_exact_plugin_build`. This is not evidence of
a Nibbi DSP, graphics or authorization defect.

`tools/ap8_descriptor.py` embeds discovery metadata in generated C++ constants;
some initial parameter observations also participate in the exact prebuilt
descriptor match. Stable native class IDs already derive from the logical
vendor class independently of the module digest and must be preserved.
`preparation_cli.rs` offers preparation when a kit and inspection are available,
without resolving the missing exact entry. The refusal arrives after that offer.

The selected repair replaces module-specific compiled metadata with validated
data for a reusable native engine. Assessment, local advanced settings and
candidate publication must share the configuration described in
[architecture section 18.6](ARCHITECTURE.md#186-general-preparation-and-compatibility-experimentation).
Existing candidate observations, experimental publication and history are
retained; exact qualification is not a prerequisite for a supervised local trial.

Understanding: causal in the selected source. First implementation: kit schema 4
uses one reusable engine with Rust-validated discovery data, carried by existing
candidate/publication records. Generated classes chosen after compilation load
simultaneously through an independent Linux SDK consumer; a simulated module
update preserves DAW IDs. Actual manager preparation and publication, malformed
data, exact rollback and all 16 interrupted-publication boundaries pass source
tests. The graphics source step binds assessment to a candidate's requested
and applied launch settings, offers a typed process-only Wine D3D11/DXGI trial,
and restores its exact previous publication through the existing transaction
owner. Refresh retains local settings without rebuilding an identical engine.
Trial interruption, stale baseline, optional-editor, launch compatibility and
cleanup cases pass native Linux source checks, including manager dispatch and
the full runtime suite. Older supervisors refuse a graphics trial before Windows
launch, while retained capable generations remain usable across updates. Those
source checks did not establish Windows processing or installed runtime behavior;
the subsequent comparison is recorded below. Physical Deck/Nibbi repaired
coverage: none. Posture: still blocked in the unchanged Deck package for builds
missing from its index. This is distinct
from FC-MGMT-004, which repaired readback after an existing proxy was published.
No cause is assigned to the separately reported Deck input pauses or readiness
changes. The installed work below uses only a disposable Ubuntu fixture.

The subsequent installed comparison reproduced an additional packaging caller:
`tools/beta/candidate_inputs.py` still read the retired `proxies` list and failed
with `KeyError: proxies` on the reusable kit. It now verifies and stages the
schema-4 engine under a generic name. The real input-builder/roster regression
passes without any exact plug-in entry. The repaired path assembled both
packages used in the installed comparison below.

The `0.12.0general1` installed comparison then processed both post-freeze Windows
fixtures through the unchanged reusable engine, including automation, state
recall and retirement. The Wine D3D11 trial also processed correctly, but restore
refused with `rollback_performance_mismatch` after an explicit 512-to-1024 buffer
change. The publication snapshot was incorrectly used as authority over the
independent current preference. The focused regression reproduced the installed
failure. The source repair preserves current buffering while checking the target
engine's exact capacity, including retained-kit capacity after a package update;
unknown capacity refuses without changing publication or buffering. All 213
macOS library tests pass. At `6bbd6178`, native Linux checks also pass: 218 library,
297 manager (two existing ignored), five example, 356 runtime, 86 frontend and
23 package tests; native, Windows, portability and policy workflows pass.
Installed `0.12.0general2` restored the exact graphics predecessor with 1024
buffering retained and the sibling entry unchanged. Both original saved states
then recalled and processed after the restart and manager update. Both packages
have the same engine digest. Default and Wine D3D11 probes ran in the selected
Proton runtime; the fixtures have no editor and no actual GPU device was
observed, so editor coexistence and acceleration remain unqualified.

Both 1.0.2 modules also installed, prepared, replaced the publications, processed
and recalled their own state with stable DAW IDs and no new engine. The effect's
1.0.1 state was rejected by the native module-digest check; see FC-STATE-001.
The [installed evidence](../evidence/preparation/2026-10-03-unfamiliar-installed.json)
retains these partial results and the original graphics-restore failure.
The subsequent [state-update comparison](../evidence/preparation/2026-10-03-state-update-installed.json)
repairs and passes reference cross-version recall and normal-manager restoration
of both exact original publications. Next check: the actual DAW workflow.
No repaired Nibbi/Deck claim follows.
The [beta contract](INTEGRATED_BETA_DELIVERY.md#unfamiliar-plug-ins-and-advanced-compatibility)
also requires dependency and alternate-runtime experiments and failed-trial
recovery. Last reviewed: 2026-10-03.

## FC-STATE-001 — Saved state rejects an explicitly selected module update

Installed `0.12.0general2` on the disposable Ubuntu 26.04.1 fixture reproduced
this with LVB Unfamiliar Effect 1.0.1 to 1.0.2. Both revisions have the same
logical class and unchanged first-party DSP/state implementation. Both pass
their own SDK processing and state round trip through the same frozen native
engine. Loading the intact 1.0.1 state after explicitly publishing 1.0.2 fails
at `IComponent::setState`, before activation or vendor state restore.

At that failing head, `native-vst3-proxy/backend/src/state.rs::payload_for`
required equality of both class and module digest. `Processor::setState` called
`ap8_validate` before the state session and marked the instance failed on refusal.
The retained envelope's
class and payload hash match; its module digest identifies the original build.
This establishes a bridge refusal, not vendor state incompatibility. The
controller mirror and worker restore boundaries must be included in the repair;
silently changing saved bytes or simply deleting all identity checks is not a
repair. Actual execution must still use the explicitly selected exact module.

The consumer exited through its failure path. Windows ownership cleanup was
confirmed, but transport retirement failed with a broken pipe and the manager
retained one uncertain DSP lease. No recovery action was offered in that state.
A normal disposable-VM restart reconciled the ended kernel generation and
preserved `successful_session: false`; no lease or registry was hand-edited.
This is a separate cleanup/recovery observation under FC-LIFE-002, not proof that
the same failure occurs after a DAW's orderly rejection handling. Instrument
cross-version recall was not attempted after the failed effect run.
Existing managed rollback commands restored both original publications exactly;
both original states then recalled and processed correctly with clean retirement.
Normal history controls did not offer experimental ancestors. Their CLI
restoration is not a complete frontend rollback pass.

Understanding: causal at the native restore boundary for this exact fixture.
The coordinated source repair now distinguishes historical same-class restore
from exact-snapshot/readback validation, leaves saved objects unchanged, and
synchronizes the native controller from selected current readback. The SDK
consumer unwinds completed lifecycle steps and uses a separate migration oracle.
The supervisor proves exact native-generation death independently of its final
acknowledgement, and manager history offers retained experimental predecessors.
The controller regression fails with the prior controller and passes with the
repair in all four state/connection-order combinations. Linux runtime tests pass
(359). Installed testing then found and repaired two additional cleanup defects:
an inactive failed processor returned termination failure despite positive
backend cleanup, and a pending owner failure notice was mistaken for the final
retirement acknowledgement. Their focused regressions fail on the prior code;
actual cleanup refusal and missing acknowledgement still refuse.

Frozen `0.12.0general5`, source `9da4dc66`, passes ten installed Ubuntu SDK cases
at 48 kHz/1024 frames. Both reference roles restore 1.0.1 opaque state into
1.0.3's changed schema (24 to 32 bytes) and parameter inventory (two to three).
The migrated added parameter is .75 rather than its .25 default and affects
the sample oracle. Connected and preconnection restore, truthful current capture
and reopen pass. Same-version recall remains exact. Wrong-class, corrupt and
oversized envelopes refuse; complete and partial vendor restore refusals and
abrupt native loss retire without reboot or record deletion. Six healthy-sibling
comparisons pass during those failures. Original saved objects remain unchanged.

The normal manager restores both exact original 1.0.1 publications and retains
1024 buffering. Their original saved states then pass SDK audio comparison and
retirement through the restored original engine. In total the final ten cases
contain 18 separate audio comparisons and 41,287,680 checked samples with zero
mismatches; this is functional coverage, not a soak. General3/general4 failures
and the failed general4 VM checkpoint remain retained. Manager refresh briefly
showed unavailable status before returning ready; that UI observation is open.

Fix stage: installed reference repair passed. The later
[general5 Deck installation](../evidence/audio-recovery/2026-10-03-general5-deck-installation.json)
adds real Bitwig recall with unchanged commercial binaries and a saved/reopened
Macro2 change. It does not test a commercial vendor-version migration. That
physical interaction later fails in terminal instance handling; both transports
retire, but temporary state saving fails for the terminal instrument. The reference
migration regression remains resolved at its declared scope; commercial updates
and the complete musician workflow remain unqualified. Further engineering follows
the platform reassessment in CURRENT_SLICE.md rather than automatic soak retries.
The original
[failure](../evidence/preparation/2026-10-03-unfamiliar-installed.json) and final
[comparison](../evidence/preparation/2026-10-03-state-update-installed.json) remain
separate evidence. #204 remains draft and unmerged.
Last reviewed: 2026-10-03.

## FC-UI-008 — Vendor editor removal crashes the Windows host

### Shared boundary and understanding

Windows host `IPlugView::removed()`; bounded. This identifies the failed SDK
call, not whether the underlying defect belongs to the host, vendor or runner.

### Fix chain and coverage

Observed failure: Ubuntu 26.04.1, Bitwig 6.1.1 guest demo, internal25,
official Efx FRAGMENTS 1.0.0.2925 trial module
`edb358f124bd35290dc2597e68c2d7f1f4061d5cf650a008f097c463fc6aef89`,
class `41727475415649536772616E50726F63`, application-owned runtime r3 and
native proxy `bbee707e64e9cfb96b58762aadde125c096779f9c165bfb9c4b6bc9623e9ba41`.
The normal warm reload rendered the vendor DEMO editor. Closing its title-bar
window reached view stage 212, raised Windows access violation `c0000005`
and ended the supervised Windows host with exit 5. The native side reported a
terminal instance failure after 8620 transport blocks. Input was silent; this
does not establish audio through. Host and transport cleanup were confirmed.

Internal26 loaded from an idle service and captured actual processed output.
Its normal editor close reproduced the same removal-stage access violation;
105348 transport blocks completed before terminal failure. Host and transport
cleanup were confirmed. No dropout-free or successful editor retirement claim.

The ordinary host called removed() before clearing IPlugFrame; retained-view
retirement already used the opposite order. The shared helper now always
clears the frame before removal, matching [Steinberg editorhost](https://github.com/steinbergmedia/vst3_public_sdk/blob/master/samples/vst-hosting/editorhost/source/editorhost.cpp).
A refused frame detach retains the exact parent/view and prevents removal or
release. The production SDK fixture asserts this order, refusal and positive
retry, alongside ordinary and retained editor cycles. This source correction
does not yet establish the cause or resolution of the commercial crash.

The corrected Windows host passed CI and was included in internal27 with
independently compared native proxies. The populated account refused normal
package selection because its required host/source pair changed; internal26
remained selected. The account with no registry/catalogue selected internal27
through normal controls for the focused commercial retest. This does not
establish a safe migration of existing published products.

Internal28 completed normal preparation/publication and captured processed
stereo output, then reproduced the same editor-close failure twice with the
corrected host. Same-process mappings and the bounded editor fault header
locate the fault in Wine `uiautomationcore.dll`, `create_uia_node_from_elprov`
+0x18 (RVA 0x7988), with a null first argument and zero access address. The
vendor caller remains unproved. This is a runtime invalid-provider defect;
frame detach did not fix the observed commercial failure.

The existing AP11/AP12 process-scoped accessibility policy was missing from
new managed preparation. Source now declares it for the exact measured
FRAGMENTS module/class and runtime, includes its Windows screen-reader
limitation, and preserves the review-candidate claim and predecessor.
The isolated LGPL Wine guard also passes the direct null-provider reference:
delivered DLL exit 5 at the same RVA, corrected DLL exit 0. The saved builder's
independent output passed in another fresh reference prefix. Neither reference
establishes commercial editor behavior or delivery of a patched runtime.
See [construction and policy boundaries](RUNTIME_UIA_GUARD.md).

Internal29 prepared and explicitly replaced the exact policy candidate, but a
separate retained-buffering guard refused admission before a Windows DSP host
existed. Internal30 delivered that admission correction. The ordinary inserted
instance completed three normal editor close/reopen cycles and final close,
then retired through Bitwig. A fresh demo instance also completed close/reopen,
captured altered stereo audio afterwards, closed again and retired. Both
terminal observations recorded stage 217, editor failure/exception zero and
confirmed host/transport cleanup, without a terminal instance fault. No patched
DLL was adopted. These positive policy results preserve the prior failed
default-accessibility fixtures and the exact screen-reader limitation.

### User posture and next gate

Review candidate with bounded installed close/reopen and retirement results.
State/project recall, audio quality, startup responsiveness and ordinary
installer recovery remain unqualified. Do not apply this exact Ubuntu policy
result to the separately accepted Deck fixture or another module/runtime.

### Evidence

[Installed result](../evidence/self-service-delivery/ubuntu-fragments-trial-2026-09-29.json),
`fresh_account_retest.daw_demo_retest_internal25`, `daw_demo_retest_internal26`,
and `clean_account_host_retest_internal27.daw_demo_internal28` / `daw_demo_internal30`;
draft [PR #200](https://github.com/kasselvania/Linux-VST-bridge/pull/200).
Last reviewed: 2026-09-30.

## FC-AUTO-001 — Automation refusal collides with terminal silence

The internal44 Ubuntu reference effect reported successful contained silence
without terminal custody, then refused state capture. Both references retained
processing-phase missing frames. The Windows owner confirmed normal cleanup;
this observation does not establish a vendor DSP crash or a recall pass.

The planner's unknown-anchor/capacity refusal reused `0x106`, already IF2's
terminal-silence result. The SDK therefore took its success/silence branch and
latched the state refusal. Source now uses `0x107`, with a static distinction and
an isolated production SDK regression. The refusing installed input was swallowed
by the collision and is unretained; its exact curve cause remains inferred.

An exact parameter-ID table is configured outside processing, with no initial
numeric values. Previously accepted last-sample values can anchor subsequent
sparse curves across a seek. GUI/state changes and Stop/recovery invalidate them;
recovery retains the exact census. Unknown baselines still refuse rather than
substitute metadata defaults. Actual processing entry-point regressions retain
zero allocations, reallocations and frees.

The result-code correction is **deployed** in internal46, but the musical
capability is not accepted. The effect returned `0x107` on a 1008-frame callback
with two input events, then refused state. No false IF2 terminal-silence success
was retained. The exact parameter points and invalidation origin are absent;
the report does not prove a particular GUI event caused the missing anchor.
The [internal46 receipt](../evidence/self-service-delivery/internal46-populated-reference-failure.json)
preserves its failed snapshot, processing gaps and confirmed owner retirement.

The current source successor transports one whole DAW block through protocol 14
and mapping 3 instead of reconstructing implicit curves across 256-frame
chunks. The incoming queue is unchanged and its implicit previous value remains
vendor-owned. GUI/state/seek invalidation cannot require an invented bridge
anchor on this path. Legacy protocol fixtures retain their separate planner.
Whole 1008/1024-frame callbacks, zero-frame flush, mapped multi-output guards,
native SDK activation and production Windows decoder/SDK queues have source
regressions; the callback allocation counters remain zero. A matching Windows
host and proxy kit must be delivered and retested. The correction is source-only
and does not close the installed state, recall or audio gaps.

Native Rust,
SDK and X11 regressions passed, as recorded in the
[installed failure and source receipt](../evidence/self-service-delivery/internal44-populated-reference-automation-failure.json).
The coherent successor must repeat saved automation, state and reopening.
The distinct [FC-AUDIO-001](#fc-audio-001--residual-audio-deadline-misses)
processing gaps remain unresolved.

## Maintenance rules

When a PR changes one of these classes:

1. update the index row;
2. update the detailed card;
3. update affected rows in [SUPPORT_MATRIX.md](SUPPORT_MATRIX.md);
4. preserve historical documents rather than rewriting old observations;
5. state the new fix-chain stage and claim limit in the PR body.

A new class should represent a reusable mechanism, shared architectural boundary, current user limitation, or accepted fix that future work must preserve. Do not add one entry for every failed command or harness mistake.


## FC-MGMT-005 — Partial installation retry omitted from Setup

Internal55's packaged partial reference installer wrote only the instrument.
Ordinary Setup Stop retired every owned process, preserved that file, restored
the service and cleared pending recovery. The same card offered discovery but
hid the backend's already-authorized isolated retry. This is a causal projection
gap: `setup_projection_current` retained only Stop as a secondary action when
partial files made discovery primary. The source correction retains the exact
backend retry offer beside discovery, including its disabled reason; ownership,
retirement and new-attempt admission remain with their existing owner.

The installed failure and successful partial-stop subset are retained in
[the internal55 record](../evidence/self-service-delivery/internal55-partial-stop-hidden-retry.json).
The regression fails against the old projection. Internal56 then exposed the
exact retry through installed Setup. Two fresh attempts passed partial Stop,
positive retirement and service restoration, preserving the prior partial
attempts. Setup offered discovery and another isolated retry after each. The
[installed record](../evidence/self-service-delivery/internal56-installed-partial-recovery.json)
binds the source, package selection and operation identities. The complete
recovery matrix and other platforms remain open.

## FC-MIDI-002 — Late note-off permanently fails processing

**Understanding:** causal at the native SDK timestamp conversion.
**Implementation:** source-fixed, installed and physically compared on the Deck.
**User posture:** the declared late-note-off recovery passed for exact Pure LoFi
1.0.0.6121; dependable musical use and deadline continuity remain unqualified.

The retained internal56 Ardour witness supplies a note-off at SDK offset -1661.
`Processor::process` cast that signed input directly to an unsigned wire offset.
Strict event validation correctly refused it with 0x102; the proxy then entered
`Failed`, rejecting later audio and state capture. The witness precedes bridge
translation; it does not explain the earlier missing audio blocks.

The correction defines one host-boundary recovery: a negative-timestamp note-off
in a nonempty block releases its identified voice at sample zero. Identity,
channel, pitch, velocity, tuning and subsequent valid events are preserved.
The lifecycle report counts `late_note_offs` away from the callback. Negative
note-ons, future offsets and invalid event fields gain no new support. The wire
validator, ownership, epochs, delay, worker, Windows host and runtime are unchanged.

The pinned SDK regression fails on 37b5d41 and passes with the correction for
-1661 and -1, followed by valid processing on the same instance. Callback audit
reports zero prohibited effects. All eight non-graphical native tests passed;
the two X11 panel tests were not run in this builder, which lacks xvfb-run.
`ap18-late-note-off-host` supplies the installed control/injected-input comparison
and records whole output for independent analysis.

The physical comparison used the same consumer, module, Windows host, pinned
runtime, 48 kHz / 512-frame configuration and KDE Wayland session. Internal57
rejected 600 of 720 callbacks after the -1661 note-off, silenced the later note
and failed state capture. Recovery1 normalized one late release, accepted all
720 callbacks, produced captured later-note audio, captured state and retired
cleanly. All four control/late before/after owners had confirmed supervisor
cleanup and transport retirement. The old late run's native termination still
failed; supervisor cleanup is a separate fact.

Both repaired short SDK runs had zero presentation gaps. A short copied Bitwig
chain also had zero gaps in its initial and reopened native lifetimes and
exercised both editors and existing automation. Speaker output was captured for
the initial playback only. These are not a longer soak, meaningful unique-state
recall, reboot, lower-buffer or reconfiguration result. The earlier reference
and internal57 control retain real gaps; FC-AUDIO-001 stays open. Exact artifacts,
whole-session counts, private-output hashes and limitations are retained in the
[physical comparison](../evidence/audio-recovery/2026-10-02-late-note-off.json).

## FC-UI-009 — Owner exception unwinds a live processing thread

### Boundary, understanding and repair

Causal at the production Windows processing-owner boundary. The pinned-SDK
fixture captures correct output, throws from `service_owner()` while the worker
is live, and reproduces `std::terminate` before controlled shutdown on source
`30859670dca8f654e7c31befc8d5172398dc3bb3`. This is generated instrumentation,
not a commercial incident or a GPU diagnosis.

The owner now catches before unwinding the joinable thread, requests cancellation,
and waits up to five seconds before deliberate process containment. Socket and
mailbox storage remain owned until join. State waits observe cancellation even
if notification is missed. Windows testing also established that local socket
shutdown alone did not reliably end an idle receive; explicit cancellation
checks now cover idle, partial-message and reply waits while preserving their
normal deadlines. Vendor exception text is replaced by a fixed failure message.

The separate production `MappedSession` fixture holds, refuses or throws from
controller synchronization while SDK processing runs. Refusal now publishes
explicit controller terminal status. Its existing audio-stop and state-save
refusal remain intentional in this repair; safe continued audio after a display
synchronization failure remains a distinct open decision.

### Fix stage and support posture

Source repair and generated Windows SDK tests only. The exact test/build outcomes,
including the failed socket attempt, are in the
[source-repair evidence](../evidence/graphics-runtime/2026-10-02-source-repair.json).
No installed generation or commercial physical result. BEAM, Pure LoFi and
FRAGMENTS retain their existing support rows. No attribution of earlier crackling,
no graphics qualification and no beta-ready claim.

Tracking: [PR #204](https://github.com/kasselvania/Linux-VST-bridge/pull/204).
Last reviewed: 2026-10-02.
