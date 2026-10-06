# Current task: remove Pure LoFi stalls with Buffered 512

## Goal

Pure LoFi plays in Bitwig on the Deck at 512 frames / 48 kHz for ten minutes
without missing blocks or graph errors, then thirty minutes with normal use.
Preserve projects, licensed state, pinned runner and machine headroom.

## Works now

The unchanged installed build, Together hosting, Buffered 512, editor closed:
600 seconds, zero missing blocks, Bitwig ERR+0 (3 to 3), recorder ERR0.
Largest call gap 12.75 ms versus 20.15 ms with the editor open.
All 56,249 DAW calls succeed: median/p99/max 0.038/0.057/0.084 ms.
Normal quit/retirement succeeds; project, publications and preferences survive.
Buffered 512 stays selected. No rebuild or scheduling change for this run.
Source caller grants remain opt-in, off by default; that successor is uninstalled.

## Still broken

The editor-open Buffered baseline has two missing 512-frame blocks and ERR+2.
Misses occur about 177 and 273 seconds after playback starts, both mid-run.
No operator save or GUI interaction; GUI queue counters never change during
capture. Redraw and meter events were not individually timestamped.
Retained playback ps CPU: Part B lvb-audio 13.5%, wineserver 16.1%, session
supervisor 3.7%, Bitwig data-loop.0 1.6%, each audio-1..8 0.2%.
These are per-thread lifetime averages, not instantaneous utilization.
Part A's filtered samples did not retain the renamed Windows host threads.
Thirty-minute normal-use/export confirmation remains outstanding.

## Most likely cause

Editor-associated Wine/rendering load delaying the ordinary-priority Bitwig
engine, about 75%. Closing the editor removes both measured symptoms in one
run; the exact redraw/scheduler mechanism still needs the next comparison.

## Doing next

Wait for the operator's explicit confirmation that PR #218 is merged before
rebuilding/installing its supervisor switch. Source already contains the merge;
the Deck remains on the previous installed build. Then repeat editor OPEN with
LVB_AUDIO_SCHEDULING=0, verify disabled records/no bridge RR threads, and restore
the environment afterwards. Buffered 512 throughout; raw captures outside Git.

Source/SSH/Deck: Sol6.1 xhigh; GUI: Sol6.1 high; root orchestrates.
