# Current task: the DAW thread that calls the bridge is not real-time

## Goal

Pure LoFi plays in Bitwig on the Deck at 512 frames / 48 kHz for ten minutes
without a missing block or graph error; then thirty minutes with normal use.
Preserve projects, licensed state, the pinned runner and machine headroom.

## What the last two runs established

Denormals are not the cause: the FTZ/DAZ guard left vendor time unchanged
(2.61 vs 2.65 ms median) and misses went 10 to 8, within noise. No bridged call
exceeds the 10.67 ms period (max 6.4 ms). Yet entry spacing between calls
reaches 19.4 ms, with 16.2 ms lying outside our call. The caller's thread,
recorded at every SDK entry, is SCHED_OTHER priority 0. Our native worker and
Windows render thread are SCHED_RR 5. The render grant itself was flaky
(first request refused, second found no thread, third effective); PR #215
retries it and records the refusal reason.

## Best explanation (about 75%)

In Bitwig's "Together" hosting the plug-in runs in Bitwig's plug-in host
process, and that process's audio thread has no real-time policy on this Deck.
Every ordinary-priority thread on the machine (Wine UI, wineserver, our
supervisor, the desktop) can delay it, and our 2.6 to 6.4 ms call occupies a
quarter to 60 percent of the period, so a few ms of caller delay becomes a
missed block. The two RT grants we do hold protect threads that wait on the
unprotected one.

## Next changes, one at a time

1. Deck, no code: during playback, record thread policies and CPU for Bitwig's
   engine, Bitwig's plug-in host, the Wine host and the supervisor:
   `ps -eLo pid,tid,comm,cls,rtprio,pcpu | grep -iE 'bitwig|wf0-factory|wineserver|python'`
   sampled a few times. Expect the plug-in host's audio thread to show TS/-.
2. Deck, no code: set Bitwig's plug-in hosting mode to "Within Bitwig" for this
   plug-in (same project, clip, editor open) and repeat the ten-minute capture.
   Compare missing blocks and maximum entry spacing against 8 and 19.4 ms.
3. Code, if 1 and 2 confirm: at the first process() call the proxy records the
   caller's TID and the supervisor requests SCHED_RR for it through the existing
   RealtimeKit path, at least as high as the render thread. Then re-test in
   "Together" mode.
4. If 2 does not remove the misses: run Buffered with 512 remembered frames to
   take the vendor call off Bitwig's critical path, and compare again.

## Done

Ten clean minutes in the normal product configuration, then thirty, with the
change that removed the misses named. Raw captures stay outside Git.

Source/SSH/Deck: Sol6.1 xhigh; GUI: Sol6.1 high; root orchestrates.
