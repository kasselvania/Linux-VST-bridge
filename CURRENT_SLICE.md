# Current task: find what still stalls Bitwig with the vendor call off its thread

## Goal

Pure LoFi plays in Bitwig on the Deck at 512 frames / 48 kHz for ten minutes
without a missing block or graph error; then thirty minutes with normal use.
Preserve projects, licensed state, the pinned runner and machine headroom.

## What the runs so far established

Through the identical bridge the reference plug-in (0.66 ms call) has zero
Bitwig errors; Pure LoFi (3.3 ms median, 6.9 ms max) has them in every run.
Denormals, hosting mode, the caller grant and the Flatpak RealtimeKit
permission each changed nothing or made Bitwig worse. Every Bitwig audio
thread on this Deck runs at ordinary priority and we cannot change that. The
caller grant is opt-in (LVB_CALLER_SCHEDULING=1), off by default.

## Buffered result

Buffered 512, no rebuild: the DAW-side call fell to 0.038 ms median (3.2 ms
max, two calls above 1 ms). Missing blocks 8 to 2, Bitwig graph errors +8 to
+2, recorder 0, largest gap 20.15 ms with 20.10 ms outside our call. The first
change that improved both numbers without new harm. Buffered becomes the
default for effects; SameCallback stays the opt-in for live instruments.

## Best explanation for the remaining two (about 60%)

Bitwig stalled for 20 ms while our call cost it nothing, so the residue is
what our process family does to the machine, not the audio path. Suspects, in
order: our two real-time threads (render RR 5, worker RR 5) preempting
Bitwig's ordinary-priority engine for up to 7 ms per period; the Wine editor
window's rendering; the supervisor's 20 Hz process census. Bitwig alone, with
none of these present, was clean for ten minutes.

## Next changes, one at a time, Buffered 512 throughout

1. Deck, no code: same run with the editor closed. Compare 2 and +2.
2. Deck, after the switch lands: same run with LVB_AUDIO_SCHEDULING=0 in the
   manager's environment, so no thread of ours is real-time. Compare 2 and +2.
   If clean, ordinary priority is the right default in Buffered mode.
3. Free data from the existing samples: per-thread CPU for the Wine host,
   wineserver and the supervisor during playback, and the timestamps of the
   two misses (start-up or mid-run).

## Done

Ten clean minutes in the normal product configuration, then thirty, with the
change that removed the misses named. Raw captures stay outside Git.

Source/SSH/Deck: Sol6.1 xhigh; GUI: Sol6.1 high; root orchestrates.
