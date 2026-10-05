# Current task: residual Bitwig timing after buffered delivery

## Goal

Pure LoFi on the Deck, Bitwig 6.1, Together hosting, editor open, 512 frames /
48 kHz: ten minutes without missing blocks or graph errors, then thirty minutes
with normal use. Preserve projects, licensed state, runner and machine headroom.

## Works now

The installed caller-grant build was reused without rebuilding. The manager
restored Buffered with 512 remembered frames; the reopened project confirmed
512 bridge frames plus 48 vendor frames. DAW-side process-call median/p99 fell
to 0.038/0.065 ms from Part B's 2.653/4.100 ms. Bitwig exited normally and the
session retired cleanly; original and working projects and publications survived.

## Still broken

The complete 600-second capture still has two missing 512-frame blocks:
baseline eight, Part B two. Bitwig graph ERR +2 versus +8/+16; recorder ERR 0.
Largest call gap 20.15 ms versus 19.4/33.4 ms. The preceding call took 0.045 ms,
leaving 20.10 ms outside it. Two of 56,249 calls exceeded 1 ms; maximum 3.157 ms.
All 64,144 lifetime calls exported without loss. This run is not clean.

Source now makes the caller grant opt-in; that successor was not installed.
Bitwig audio and all eight PluginsThreadPo workers still sampled ordinary policy.

## Most likely cause (about 80%)

Host scheduling or dispatch delay before bridge entry. Taking vendor work off
the DAW call greatly shortened typical calls but did not remove missing blocks.
Almost all of the largest entry gap remains outside the measured bridge call.

## Doing next

Stopped after the requested ten-minute comparison: no thirty-minute run or
offline export. Buffered 512 remains selected; Bitwig is closed. Raw captures
stay outside Git. Next proposed task is a bounded host scheduling/dispatch
comparison; no further experiment is running. Gap-free musical use is unfinished.

Source/SSH/Deck: Sol6.1 xhigh; GUI: Sol6.1 high; root orchestrates.
