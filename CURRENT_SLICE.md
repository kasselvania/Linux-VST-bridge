# Current task: take the vendor call off Bitwig's deadline

## Goal

Pure LoFi plays in Bitwig on the Deck at 512 frames / 48 kHz for ten minutes
without a missing block or graph error; then thirty minutes with normal use.
Preserve projects, licensed state, the pinned runner and machine headroom.

## What the runs so far established

Same bridge, same Wine process, same supervisor, same grants: the reference
plug-in (0.66 ms median call) produced zero Bitwig graph errors in its window;
Pure LoFi (3.3 ms median, 6.9 ms max) produced errors in every run. The misses
scale with the vendor call, not with the bridge's presence. Bridge overhead is
0.12 ms. Denormals: no change. "Within Bitwig" hosting: no change. Flatpak
RealtimeKit permission: no Bitwig thread became real-time. Every Bitwig audio
thread on this Deck, including PipeWire's loop inside Bitwig, runs at ordinary
priority, and we cannot change that from our side.

The caller grant works mechanically (effective, RR 5 on bitwig-remote-p) and
gave 2 missing blocks against 8, but Bitwig's graph errors doubled (16 against
8) and the largest gap grew to 33 ms. One run each; a real-time thread inside
an ordinary-priority engine is a plausible cause of the extra errors. It is now
opt-in (LVB_CALLER_SCHEDULING=1) and off by default.

## Best explanation (about 80%)

Bitwig runs its engine with no real-time protection, so its own timing jitters
by several milliseconds. In SameCallback mode our 3 to 7 ms vendor call sits
inside Bitwig's window on top of that jitter; the two tails add up past the
10.67 ms period a few times per ten minutes. The cheap reference plug-in leaves
room for the jitter; Pure LoFi does not.

## Next change

Buffered delivery with 512 remembered frames, nothing else changed. The vendor
call then runs on our real-time render thread during the following period and
Bitwig's thread only copies buffers, so Bitwig's window is no longer shared
with the vendor. In the manager for Pure LoFi: "Restore buffered delivery with
remembered buffering", confirm 512 frames, reopen the project, same clip,
editor open, ten minutes. Compare missing blocks against 8 and Bitwig graph
errors against +8. Report the DAW-side call time; it should fall well under
1 ms. If clean, run thirty minutes, then make Buffered the default for effects
and keep SameCallback as the opt-in for instruments played live.

## Done

Ten clean minutes in the normal product configuration, then thirty, with the
change that removed the misses named. Raw captures stay outside Git.

Source/SSH/Deck: Sol6.1 xhigh; GUI: Sol6.1 high; root orchestrates.
