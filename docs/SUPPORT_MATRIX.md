# Product and platform support matrix

This matrix records current accepted product/platform claims. It is not live manager state and does not replace exact profiles, candidates, revisions, or physical readback.

Status terms:

- **supported** — the listed exact behavior has a physical product result;
- **supported with workaround** — normal use is available with the listed limitation;
- **unqualified** — source, build, or partial physical evidence exists, but the listed behavior is not accepted;
- **blocked** — a known failure prevents the stated use.

A source patch, build, candidate, or publication is not a physical support claim.

## Tested processing conditions

**Steam Deck AP17 fixture:** 48 kHz, float32, actual host maximum no greater than 512 frames, and 512 added bridge frames. This is the accepted AP17 configuration, not a claim for other host maxima or the opt-in 256-frame setting.

**Ubuntu FRAGMENTS fixture:** Bitwig Audio settings explicitly selected 512 samples at 48 kHz, with Bitwig-only `PIPEWIRE_QUANTUM=512/48000`. That environment request alone did **not** change Bitwig's observed 1024-frame internal maximum; the explicit Bitwig setting was required for the accepted load. See [FC-AUDIO-002](FAILURE_CLASSES.md#fc-audio-002--host-block-exceeds-the-selected-bridge-presentation-envelope).

## Steam Deck / SteamOS 3.8.16, Bitwig 6.1

These are exact-fixture claims, not universal vendor support. The 2026-09-23 fleet readback predates both the merged host-retention repair [#149](https://github.com/kasselvania/Linux-VST-bridge/pull/149) and Serum's installed candidate C; its older `needs_attention` and candidate-B fields are **historical**, not silently treated as current. A fresh fleet-wide manager readback has not been retained in this documentation cut.

| Product / exact class | User posture | Accepted profile or selected candidate | Runner/policy | Physically accepted behavior | Current limitations and linked failure classes | Last physical evidence |
|---|---|---|---|---|---|---|
| Pure LoFi 1.0.0.6121 · `417274754156495350724C4650726F63` (instrument) | supported | ordinary verified revision 10; [profile](../compatibility/arturia-pure-lofi.json); exact host/source continuity [FC-MGMT-002](FAILURE_CLASSES.md#fc-mgmt-002--exact-verified-hostsource-omitted-across-software-generations) | `proton-11.0-2c-25118279-slr4-4.0.20260805.254769`; default policy | Deck instrument/audio, editor, automation, distinct state and save/reopen, sibling isolation and retirement in the exact Arturia fixture | Recorded gaps and no 256-frame qualification: [FC-AUDIO-001](FAILURE_CLASSES.md#fc-audio-001--residual-audio-deadline-misses). Concurrent lease race: [FC-CAP-001](FAILURE_CLASSES.md#fc-cap-001--capacity-enumeration-versus-lease-retirement-race). | [AP17 six-instance/recall result](AP17.md#r1-fixture-completion-and-recovery-result) |
| Efx FRAGMENTS 1.0.0.2925 · `41727475415649536772616E50726F63` (effect) | supported | ordinary verified revision 10; [profile](../compatibility/arturia-efx-fragments.json); exact host/source continuity [FC-MGMT-002](FAILURE_CLASSES.md#fc-mgmt-002--exact-verified-hostsource-omitted-across-software-generations) | same pinned Arturia runner; default policy | Deck effect audio, editor/parameter interaction, saved-state recall and clean retirement | Recorded gaps and unqualified 256: [FC-AUDIO-001](FAILURE_CLASSES.md#fc-audio-001--residual-audio-deadline-misses). Concurrent lease race: [FC-CAP-001](FAILURE_CLASSES.md#fc-cap-001--capacity-enumeration-versus-lease-retirement-race). Arbitrary multibus I/O is not claimed. | [AP17 six-instance/recall result](AP17.md#r1-fixture-completion-and-recovery-result); [AP16](AP16.md) |
| Pigments 7.0.1.6772 · `41727475415649534B61743150726F63` (instrument) | supported | ordinary verified revision 18; [profile](../compatibility/arturia-pigments.json) | same pinned Arturia runner; default policy; accepted host-pump fairness | Deck notes/audio, editor, preset/control and automation recall, save/reopen, sibling independence, process-scoped retirement | Older long touch-release tail remains unattributed: [FC-UI-004](FAILURE_CLASSES.md#fc-ui-004--windows-touch-release-processing-continues-long-after-x11-release). One-instance posture is exact to [AP18](AP18.md). Recorded gaps: [FC-AUDIO-001](FAILURE_CLASSES.md#fc-audio-001--residual-audio-deadline-misses). Generic pump fix is [FC-UI-001](FAILURE_CLASSES.md#fc-ui-001--generic-editor-input-starvation-behind-posted-work). | [AP18](AP18.md); [UIR1](UIR1.md); [UIO3](UIO3.md) |
| Serum 2 2.1.5 · `56534558667350736572756D20320000` (instrument) | supported-with-workaround | engineering candidate C `91b699291eb7b7d1ff6e725d5e6fd1abed88ea721dbd81a621dd2fb39d38d207`, published revision `807828d4b94fccd69e57c352a60dbcd1`; candidate B retained as predecessor; candidate-package host/source continuity [FC-MGMT-002](FAILURE_CLASSES.md#fc-mgmt-002--exact-verified-hostsource-omitted-across-software-generations); source [SV1 profile](../compatibility/sv1/xfer-serum2.json) is not candidate-C physical authority | `proton-11.0-2c-x11-touch-release-v1`; `x11_touch_release_v1` | Deck audio, ordinary controls and mouse/trackpad waveform menu; following note audible and ordinary close clean in candidate-C session | Finger-opened waveform popup still stalls; mouse/trackpad workaround: [FC-UI-002](FAILURE_CLASSES.md#fc-ui-002--x11-raw-touch-release-retains-contact-on-pointer-up), [FC-UI-003](FAILURE_CLASSES.md#fc-ui-003--touch-opened-serum-popup-stalls-the-editor-message-loop). Broad editor-loss reports are separate [FC-UI-006](FAILURE_CLASSES.md#fc-ui-006--touch-triggered-editor-loss-on-non-arturia-products). Save/reopen not accepted for C; one gap recorded [FC-AUDIO-001](FAILURE_CLASSES.md#fc-audio-001--residual-audio-deadline-misses). A candidate-D source proposal is not selected or physically qualified. | [PR #151 candidate-C physical receipt](https://github.com/kasselvania/Linux-VST-bridge/blob/4ae414b33e37f771c422d8a70a9fa78a20231118/evidence/serum-x11-touch-release/physical-attempt-002.json); [action-bound capture](../evidence/serum-x11-touch-routing/candidate-c-action-capture.json) |
| Blackhole Immersive 1.4.4 · `5653544248496D626C61636B686F6C65` (effect) | supported | selected engineering candidate `80e06590b7dc3f4e15d2903237541bdb466c012c8dfb647f6d0843daefb68b7d` in last fleet readback; managed candidate, no root source profile | `proton-11.0-2c-dcomp-c27f058-reference`; `dcomp_wine_builtins_reference_v1` | Exact DirectComposition editor renders and mouse Bypass responds; operator confirmed audible processing; later Deck open accepted | Experimental exact-runner claim, not general graphics: [FC-GFX-001](FAILURE_CLASSES.md#fc-gfx-001--directcomposition-presentation-capability). Broad touch-loss report [FC-UI-006](FAILURE_CLASSES.md#fc-ui-006--touch-triggered-editor-loss-on-non-arturia-products). Same-instance reopen/project recall not established by original graphics receipt. | [Blackhole editor and operator-audio result](BLACKHOLE_EDITOR.md#delivered-bitwig-result-and-stopping-point) |
| Kontakt 8 Player 8.13.1 · `5653544E694B386B6F6E74616B742038` (instrument) | supported | selected engineering candidate `2185e031fee4552232839ef305cceec7760c44271b0b1e2886a992150692949e` in last fleet readback; managed candidate, no root source profile | `proton-11.0-2c-25118279-slr4-4.0.20260805.254769-ni-msi-ed4a47d3b167`; exact NI runner, no generalized policy | Deck Factory Selection played through the declared shared output buses; editor and a fresh Desktop insertion opened; clean retirement observed | Gaming Mode transition is not accepted: [FC-LIFE-001](FAILURE_CLASSES.md#fc-life-001--graphical-session-and-keeper-authority). Broad touch-loss report [FC-UI-006](FAILURE_CLASSES.md#fc-ui-006--touch-triggered-editor-loss-on-non-arturia-products). Full bus matrix and project-state recall not claimed. | [Kontakt physical playback](../evidence/kontakt-native-access-deck/shared-bus-playback.md); [fleet readback](../evidence/fl1-deck-fleet-readonly/README.md) |

## Ubuntu

| Product / exact class | User posture | Accepted profile or selected candidate | Runner/policy | Physically accepted behavior | Current limitations and linked failure classes | Last physical evidence |
|---|---|---|---|---|---|---|
| Efx FRAGMENTS 1.3.1.6566 · `41727475415649536772616E50726F63` (effect) | supported-with-workaround | engineering `review_candidate` revision 12; [exact profile](../compatibility/frg1/revision-12/arturia-efx-fragments.json); retained publication `d1273fdb5a50d9f73009bc6473cd3f36` | `ge-proton11-7-ubuntu2604-frg1`; Ubuntu shared-private-loopback adapter | Inventory/publication, Bitwig editor, audible processing and parameter response, save/reopen, clean retirement, controlled restart and actual cold boot with exact same-revision reselection | Required explicit Bitwig 512/48-kHz configuration: [FC-AUDIO-002](FAILURE_CLASSES.md#fc-audio-002--host-block-exceeds-the-selected-bridge-presentation-envelope). Recorded underruns: [FC-AUDIO-001](FAILURE_CLASSES.md#fc-audio-001--residual-audio-deadline-misses). Abrupt active-owner recovery not generalized: [FC-LIFE-002](FAILURE_CLASSES.md#fc-life-002--failed-launch-cleanup-and-truthful-recovery-state). Namespace and startup laws: [FC-PLAT-001](FAILURE_CLASSES.md#fc-plat-001--nativewindows-transport-requires-shared-private-loopback), [FC-BOOT-001](FAILURE_CLASSES.md#fc-boot-001--volatile-runtime-and-publication-restoration-after-boot). | [Ubuntu-lab merged PR #5](https://github.com/kasselvania/Linux-VST-bridge-ubuntu-lab/pull/5), [machine-reboot receipt](https://github.com/kasselvania/Linux-VST-bridge-ubuntu-lab/blob/473ac0a926e6d95d470c002244b851857b83f41c/evidence/UA1/20260923T1818Z-frg1-machine-reboot/result.json) |

Blackhole, Kontakt, Pigments, Pure LoFi and Serum 2 have **no accepted Ubuntu product row**. Deck results do not transfer to Ubuntu. The Raspberry Pi standalone experiment is a separate appliance claim, not a native-DAW support row here.

## Shared capacity posture

For the accepted AP17 Steam Deck fixture:

- six simultaneous DSP instances globally are supported;
- Pure LoFi: at most three qualified instances;
- Efx FRAGMENTS: at most four qualified instances;
- native hard capacity: four per loaded class image;
- three parallel tracks, serial depth three, and two simultaneous direct editors were exercised;
- seven remains unqualified;
- eight was excluded for the tested workload.

The current six-product heterogeneous project has not yet completed. Individual product smokes do not equal a mixed-six concurrency pass. See [FC-CAP-001](FAILURE_CLASSES.md#fc-cap-001--capacity-enumeration-versus-lease-retirement-race) and [AP17](AP17.md).

## Claim rules

- Keep Steam Deck and Ubuntu claims separate.
- A product version/class/result belongs here only after exact physical evidence or an explicit operator acceptance.
- Link every current limitation to a failure-class entry when a shared class exists.
- Update this matrix in the same PR that changes accepted physical coverage or user posture.
- Exact profile/candidate/revision identity remains in `compatibility/`, manager state, and retained evidence; this file summarizes rather than replaces that authority.
