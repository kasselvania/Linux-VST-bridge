# Product and platform support matrix

This matrix records current accepted product/platform claims. It is not live manager state and does not replace exact profiles, candidates, revisions, or physical readback.

Status terms:

- **supported** — the listed exact behavior has a physical product result;
- **supported with workaround** — normal use is available with the listed limitation;
- **unqualified** — source, build, or partial physical evidence exists, but the listed behavior is not accepted;
- **blocked** — a known failure prevents the stated use.

A source patch, build, candidate, or publication is not a physical support claim.

## Steam Deck / SteamOS

| Product | Exact fixture | Current posture | Runner/policy | Physically accepted behavior | Known limitations | Failure classes | Last retained evidence |
|---|---|---|---|---|---|---|---|
| Pure LoFi | 1.0.0.6121 | supported | exact pinned Arturia Proton/SLR runner | audio effect, editor, automation, state/save-reopen, sibling independence, retirement; individual fleet smoke passed | residual deadline misses; 256 unqualified | [FC-AUDIO-001](FAILURE_CLASSES.md#fc-audio-001--residual-audio-deadline-misses) | AP12–AP18 records; current fleet smoke |
| Efx FRAGMENTS | Deck exact Arturia fixture; historical README lists 1.0.0.2925 | supported | exact pinned Arturia Proton/SLR runner | audio effect, editor, parameter/state, save/reopen, retirement; fleet smoke passed | residual deadline misses; complete arbitrary I/O not claimed | [FC-AUDIO-001](FAILURE_CLASSES.md#fc-audio-001--residual-audio-deadline-misses) | AP12/AP17 and current fleet smoke |
| Pigments | 7.0.1.6772 | supported | exact pinned Arturia runner; accepted pump-fairness host | note audio, editor, preset/control automation recall, save/reopen, sibling independence, process-scoped retirement; fleet smoke passed | one-instance posture; residual deadline misses; older touch-release tail unresolved | [FC-UI-001](FAILURE_CLASSES.md#fc-ui-001--generic-editor-input-starvation-behind-posted-work), [FC-UI-004](FAILURE_CLASSES.md#fc-ui-004--windows-touch-release-processing-continues-long-after-x11-release), [FC-AUDIO-001](FAILURE_CLASSES.md#fc-audio-001--residual-audio-deadline-misses) | AP18, UIR1, UIO3, current fleet smoke |
| Serum 2 | 2.1.5; class `56534558667350736572756D20320000` | supported with workaround | candidate B/current runner; candidate C touch runner pending in PR #151 | instrument audio, ordinary controls, editor, processing reconfiguration, clean retirement; mouse/trackpad waveform menu works | physical touch on waveform dropdown stalls editor; save/reopen coverage remains narrower than Arturia products; residual gap counters | [FC-UI-002](FAILURE_CLASSES.md#fc-ui-002--x11-raw-touch-release-retains-contact-on-pointer-up), [FC-UI-003](FAILURE_CLASSES.md#fc-ui-003--touch-opened-serum-popup-stalls-the-editor-message-loop), [FC-UI-006](FAILURE_CLASSES.md#fc-ui-006--touch-triggered-editor-loss-on-non-arturia-products), [FC-AUDIO-001](FAILURE_CLASSES.md#fc-audio-001--residual-audio-deadline-misses) | controlled mouse/touch classification; draft PR #151 |
| Blackhole Immersive | 1.4.4 | supported | DirectComposition reference runner | editor renders, mouse and physical touchscreen drag worked, processing session and cleanup retired cleanly; recent physical open accepted | no direct audio claim in first gate; historical broad touch-disappearance report not causally resolved; Gaming Mode not fully generalized | [FC-GFX-001](FAILURE_CLASSES.md#fc-gfx-001--directcomposition-presentation-capability), [FC-LIFE-001](FAILURE_CLASSES.md#fc-life-001--graphical-session-and-keeper-authority), [FC-UI-006](FAILURE_CLASSES.md#fc-ui-006--touch-triggered-editor-loss-on-non-arturia-products) | Blackhole physical gate and fleet readback |
| Kontakt | exact installed managed build; version retained in product evidence | supported with limitations | exact current managed runner/candidate | current physical open and normal operation accepted; clean manager state retained | first-load quirk retained; broad touch behavior and Gaming Mode need explicit product coverage; full bus matrix not claimed | [FC-LIFE-001](FAILURE_CLASSES.md#fc-life-001--graphical-session-and-keeper-authority), [FC-UI-006](FAILURE_CLASSES.md#fc-ui-006--touch-triggered-editor-loss-on-non-arturia-products) | current physical open; Kontakt evidence directory |

## Ubuntu

| Product | Exact fixture | Current posture | Runner/policy | Physically accepted behavior | Known limitations | Failure classes | Last retained evidence |
|---|---|---|---|---|---|---|---|
| Efx FRAGMENTS | 1.3.1.6566; module SHA-256 `5846dfe91396596715f01a85d51c5ca21a808ef7dad1bfb44b02ab444345e1a5` | supported | CPI2 canonical product with Ubuntu shared-private-loopback adapter; exact registered proxy/revision retained | inventory, publication, Bitwig discovery, editor, audible processing, parameter interaction, save/reopen, second launch, clean retirement, controlled restart, cold boot, exact same-revision reselection | recorded underruns remain; no dropout-free claim; abrupt active-owner power-loss recovery not universal | [FC-PLAT-001](FAILURE_CLASSES.md#fc-plat-001--nativewindows-transport-requires-shared-private-loopback), [FC-BOOT-001](FAILURE_CLASSES.md#fc-boot-001--volatile-runtime-and-publication-restoration-after-boot), [FC-AUDIO-001](FAILURE_CLASSES.md#fc-audio-001--residual-audio-deadline-misses), [FC-LIFE-002](FAILURE_CLASSES.md#fc-life-002--failed-launch-cleanup-and-truthful-recovery-state) | Ubuntu-lab merged PR #5 and reboot receipt |
| Other fleet products | — | unqualified | — | no accepted Ubuntu product result in this matrix | do not infer from Deck support | relevant Deck entries only | — |

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
