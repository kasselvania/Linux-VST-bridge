# Current task: finish smooth effect insertion

## Goal

Keep effect insertion audible from its first callback through the latency-aligned
50 ms dry-to-wet transition. Discuss the remaining initial hiccup before changing it.

## Current result

Test 23 is installed on all 11 Deck publications. The operator says BEAM's
dry-to-wet transition is much better and super smooth; a slight earlier loading
hiccup before the UI appears remains. All 1,891 calls are retained, maximum
6.891 ms (first 5.546 ms); server ERR stays 0 over 54 Bitwig samples at 512/48k.

## Best explanation

One silent 512-sample callback (10.7 ms) matches the initially empty dry delay
history with D=512 and L=0. This is a strong candidate for the earlier hiccup,
but its audible attribution is unproven. The later Wine wake fix remains parked.

## Next changes

- Discuss insertion sequencing and the initial delay-history priming with the operator.
- Keep Buffered nonwaiting startup and its current, aligned 50 ms linear fade.
- Preserve SameCallback D=0 synchronous completion, instruments, public N0 and
  offline rendering; repeat startup automation and fresh save/reopen.

## Done

Loading BEAM, Nibbi and Serum 2 over a playing session produces no audible gap,
and no call in an already playing instance exceeds half a block (5.3 ms at
512/48k). Startup automation reaches the plug-in and save/reopen preserves it.
The earlier editor-gesture stall remains open until a repeat captures it.

## Separate open work

- Later Wine wake stall: nice 0 failed; baseline restored. Nibbi touch, multi-touch and the six-instance ceiling.
- Ctrl cleared after reboot locally without streaming; its source remains a gap.
- Inspection crash rollback, general preparation/manager/UI, supervisor replacement and portability.
