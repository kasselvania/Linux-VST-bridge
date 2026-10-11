# Edited automation survives fresh reopen

## Goal

Save a newly edited, nonflat automation curve in a private project, fully quit
the DAW and hosts, then show the recalled curve driving the actual plug-in.

## Completed musician result

Test45 on Deck/48-kHz/Buffered512 recalls new Macro2 hold points at beats 2/3/4:
.1281/.8471/.1281. Fresh-process playback drives those DAW values; pausing at
meaningful points gives vendor Timbre readbacks .128/.847/.128. The saved project
stays exact through retirement and reopening. Vendor observations are paused.

The diagnostics-OFF, editor-closed 60-second first-pass recording contains 2.88M
finite frames and nonzero output; graph ERR stays 2→2, recorder0. Thirteen later
silence spans leave continuity unqualified. The complete native lifetime retires
cleanly with zero callback rejections/underruns. Originals, vendor registrations,
preferences, environment records, software and restored trace remain exact.
All owned DAW/host/recorder/profiler processes retire; one current Home remains.

## Best explanation for the invalid early attempts

Stale native XWayland Control state caused incorrect input, about 95%: keyboard9
retained modifier4 while XTEST5 and physical Control keys were released. Native
Ctrl-only resets cleared the tested views; neutral checks and paired native input
made the functional check work. Final keyboard3/5/9 state is zero with no temporary
devices. Sunshine and Moonlight were inactive; the earlier state origin is unassigned.

## Retained results and remaining gates

Test43's diagnostics-OFF 30-minute interaction (580.350s pauses) and two-hour
Deck/48-kHz/Buffered512 playback remain scoped to that build and fixture.
Test45's brief registry-contention readback/Restore and ordinary Update pass;
historical refusal owners were not captured. Ubuntu Test29→28 Restore also passes.

[Audio #90](https://github.com/kasselvania/Linux-VST-bridge/issues/90), [update #240](https://github.com/kasselvania/Linux-VST-bridge/issues/240), [startup #241](https://github.com/kasselvania/Linux-VST-bridge/issues/241), [readback #239](https://github.com/kasselvania/Linux-VST-bridge/issues/239),
[restoration #242](https://github.com/kasselvania/Linux-VST-bridge/issues/242), [capture #249](https://github.com/kasselvania/Linux-VST-bridge/issues/249) and [REAPER editor #261](https://github.com/kasselvania/Linux-VST-bridge/issues/261) remain open.
Lower-block/SameCallback musical use and the integrated beta remain open;
[audio/recovery](docs/AUDIO_RECOVERY_ROADMAP.md) and [delivery](docs/INTEGRATED_BETA_DELIVERY.md)
requirements govern them.
