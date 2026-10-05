# Current task: test denormal handling on the real render thread

## Goal

Pure LoFi plays at SameCallback D0, 512 frames / 48 kHz, Together hosting,
editor open, for ten minutes without missing output blocks or graph errors.
If clean, confirm on the same frozen build with thirty minutes of interaction.
Preserve projects, licensed state, the pinned runner and machine headroom.

## Works now

Installed direct mailbox v4 bypasses the native relay/socket pump. Reference
output/state and second setup pass; an owned reference peer death wakes the
pending call in 3.14 ms and retires cleanly. Median bridge remainder is 0.121 ms.

## Still broken

The direct baseline has ten silent 512-frame blocks and +10 graph errors in
600 seconds. Whole-call median/p99/max is 2.770/4.258/6.798 ms; Windows/vendor
elapsed time is 2.646/4.123/6.377 ms. No measured call exceeds nominal 512/48k
cadence, which is distinct from the remaining device budget. Slower controller
polling also retains graph errors; that diagnostic is reverted.

## Working explanation

Missing render-thread FTZ/DAZ could amplify vendor-internal denormal work.
This is a specific untested explanation, not established by the timing split.
The same-thread guard restores the complete original MXCSR after each vendor
process call, including failure/unwind. Vendor worker threads remain unchanged.

## Next change

Build and install the paired denormal host with the restored 10 ms polling path.
Verify actual selected engine/host/helper and direct mode, then repeat the
same full 600-second output capture with sealed whole-call/vendor timings.
Compare missing blocks, active graph errors, recorder errors and call durations.

## Done

Retain the full first result. If clean, run the frozen interaction confirmation.
If it fails, return that concrete result without another variant campaign.
Commit and push completed work. Raw captures/logs remain outside Git.

Source/SSH/Deck: Sol6.1 xhigh; GUI: Sol6.1 high; root orchestrates.
