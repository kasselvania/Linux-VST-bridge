# Ordinary Restore without transient lock refusals

## Goal

Make ordinary Restore tolerate brief registry readback contention, while
preserving real mutation exclusion, exact ownership and the current setup.

## Completed musician results

Test43 manager Update/pause/reopen passes. At 48 kHz/Buffered 512, its diagnostics-OFF
30-minute interaction (580.350s deliberate pauses) and separate two-hour playback
pass with finite output, graph+0, recorder0 and clean native retirement. Changed
Macro3=.4242 fresh recall and retained flat Macro2=.2538 automation pass.
Original projects, vendor registrations, preferences and environments stay exact.
Ubuntu Test29→Test28 Restore also preserves changed state and 110 protected files.

## Current result and best explanation

Restore still acquires several registry guards fail-fast, unlike Update's shared
bounded wait. Best explanation for the intermittent refusal: brief readback
contention, about 90%. A live service releases this guard normally; one owned
1.500258s hold reproduces installed Test43's Error: LockBusy before any selection
change. The two historical failures lack exact lock-owner attribution.
An 80ms source regression fails before the change and passes after it.

## Next change and done

Restore now uses Update's shared 75s registry-wait budget and separate finite
recovery allowance. Package/setup exclusion and fresh ownership/identity checks
remain intact. Timeout, partial-publication recovery and changed-state regressions pass.
Review and green CI precede landing. Build Test44, repeat the same bounded hold
through ordinary Restore, then Update and verify the current manager and custody.
Done means the installed comparison works; no repeated audio endurance is needed
for this manager-only change. Preserve the existing Test43 audio qualification.

## Remaining beta gates

[Audio #90](https://github.com/kasselvania/Linux-VST-bridge/issues/90), [update #240](https://github.com/kasselvania/Linux-VST-bridge/issues/240), [startup #241](https://github.com/kasselvania/Linux-VST-bridge/issues/241), [readback #239](https://github.com/kasselvania/Linux-VST-bridge/issues/239),
[restoration #242](https://github.com/kasselvania/Linux-VST-bridge/issues/242), [capture #249](https://github.com/kasselvania/Linux-VST-bridge/issues/249) and [REAPER editor #261](https://github.com/kasselvania/Linux-VST-bridge/issues/261) remain open.
Edited automation, lower-block/SameCallback and integrated beta remain open.
The [audio/recovery](docs/AUDIO_RECOVERY_ROADMAP.md) and [delivery](docs/INTEGRATED_BETA_DELIVERY.md) requirements govern those gates.
