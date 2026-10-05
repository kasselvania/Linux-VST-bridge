# Current task: residual Bitwig timing after the caller grant

## Goal

Pure LoFi on the Deck, Bitwig 6.1, Together hosting, editor open, 512 frames /
48 kHz: ten minutes without missing blocks or graph errors, then thirty minutes
with normal use. Preserve projects, licensed state, runner and machine headroom.

## Works now

The caller grant is installed. The session's native_audio_scheduling caller
record is effective, RR 5, on bitwig-remote-p. The eight PluginsThreadPo workers
remain TS. One offline loop-region WAV export completed with 674 offline calls;
the same plug-in host survived. Both sessions exited normally and retired cleanly.

## Still broken

The ten-minute Together run captured two missing 512-frame blocks versus baseline
eight, but the largest call gap grew to 33.37 ms versus 19.4 ms. Bitwig graph
ERR increased by 16; recorder ERR by one. The longest bridge call was 5.71 ms;
29.44 ms of the largest entry gap lay outside the preceding bridge call.

With Bitwig closed, the approved RealtimeKit permission was added and Bitwig
restarted. Three playback samples showed no change: Bitwig data-loop.0, audio-1
through audio-8, Audio Task workers and all eight PluginsThreadPo workers stayed
TS. Per operator instruction, no second ten-minute run or export was attempted.
The added talk permission was removed; other overrides and projects were preserved.

## Most likely cause (about 70%)

Delay in Bitwig's remaining host dispatch/audio-engine path, whose sampled
workers are still ordinary priority. Raising the actual bridge caller did not
remove the problem, and the Flatpak permission alone changed no target policy.

## Doing next

Operator's Part B/C comparison is complete; no further experiment is running.
The next proposed comparison is Buffered with 512 remembered frames, taking
vendor processing off Bitwig's critical path. Await the operator's next task.
Raw captures stay outside Git; gap-free musical use remains unfinished.
