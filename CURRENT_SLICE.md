# Current task: finish the Pure LoFi claim, then the instrument class

## Goal

A musician plays a bridged plug-in in Bitwig on the Deck without dropouts,
editor open or closed, through thirty minutes of normal use. Preserve projects,
licensed state, the pinned runner and machine headroom.

## Works now

Pure LoFi, Buffered 512, 48 kHz: 1,800 seconds with transport and mixer use
and an offline export, zero missing blocks, Bitwig ERR+0, DAW-side call
0.038 ms median. Editor open with no screen streaming: 600 seconds clean.
FC-AUDIO-001 is resolved on this fixture; Buffered 512 is the product default.

## Established

The misses scaled with the vendor call inside Bitwig's window on a DAW with no
real-time audio threads. Buffered delivery removes the call from that window.
The two editor-open misses were Moonlight's video encoding: never stream the
screen during an audio measurement. Final acceptance is a person at the Deck
with headphones. The bridge's real-time grants stay; the caller grant and the
all-grants-off switch remain opt-in experiment knobs.

## Next changes, one at a time

1. Deck, no code: Pure LoFi, Buffered 512, editor OPEN, Moonlight disconnected,
   thirty minutes with knob changes in the editor, offline export at the end.
   Clean completes the Pure LoFi claim as a musician uses it.
2. Deck, no code: the instrument class. Serum, Buffered 512 then Buffered 256,
   same ten-minute method, no streaming. Record its call cost median/p99/max at
   each block size and the misses. This is the first data point for choosing
   delivery from measured cost, and it shows whether a live instrument can
   have less than a block of added latency on this machine.
3. Source: a prepare-time cost measurement per plug-in and block size, stored
   as profile data, and a delivery default chosen from it with the rule
   "worst-case call above a quarter of the period, or no real-time DAW thread,
   means Buffered". Plain-language status for the musician.

## Done

Both plug-ins thirty minutes clean in the configuration the product selects
by itself, with the latency each one carries stated to the musician.

Source/SSH/Deck: Sol6.1 xhigh; GUI: Sol6.1 high; root orchestrates.
