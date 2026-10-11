# Ordinary Restore through brief registry contention

## Goal

Make initial package readback and ordinary Restore tolerate brief registry
readback contention while preserving mutation exclusion and exact ownership.

## Completed musician results

Test45 readback and Restore45→43 pass matched 1.5s registry holds; Test44
readback and Test43 Restore refuse the same hold. Ordinary Update43→45 and
return Update45 open current Home; Restore returns exact prior software and
fifteen native bindings. Nine projects, fifteen vendor registrations, five
preferences, nine environment records and the original trace stay exact.

Test43's diagnostics-OFF 30-minute interaction (580.350s pauses) and two-hour
Deck/48-kHz/Buffered512 playback pass, with fresh Macro3=.4242 recall and retained
flat Macro2=.2538 automation. Audio qualification stays scoped to Test43.
Ubuntu Test29→Test28 Restore preserves changed state and 110 protected files.

## Best explanation and repair

Brief registry contention best explains the old intermittent refusal, about 90%:
a live service releases its guard normally and an owned hold reproduces Restore's
refusal before selection changes. Earlier failure owners were not captured.
Restore shares Update's cumulative 10s registry allowance and separate finite
recovery; package readback shares 2s across guards. Package/setup exclusion,
fresh identities and custody checks remain. The UI retains its 10s status
observation; full CLI work has separate bounds.

The first descendant-traced Update hits a Windows load_library deadline; without
that observer, Test45 passes all fifteen modules. Observer overhead is most likely, 95%.

Both source fixes are reviewed, green and landed. Installed comparisons and normal
owned-window cleanup complete; one current Home remains with the service active.

## Remaining beta gates

[Audio #90](https://github.com/kasselvania/Linux-VST-bridge/issues/90), [update #240](https://github.com/kasselvania/Linux-VST-bridge/issues/240), [startup #241](https://github.com/kasselvania/Linux-VST-bridge/issues/241), [readback #239](https://github.com/kasselvania/Linux-VST-bridge/issues/239),
[restoration #242](https://github.com/kasselvania/Linux-VST-bridge/issues/242), [capture #249](https://github.com/kasselvania/Linux-VST-bridge/issues/249) and [REAPER editor #261](https://github.com/kasselvania/Linux-VST-bridge/issues/261) remain open.
Edited automation, lower-block/SameCallback and integrated beta remain open;
[audio/recovery](docs/AUDIO_RECOVERY_ROADMAP.md) and [delivery](docs/INTEGRATED_BETA_DELIVERY.md) requirements govern them.
