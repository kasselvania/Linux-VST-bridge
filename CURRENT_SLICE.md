# Current task: isolate repeated controller dispatch

## Goal

Pure LoFi plays in Bitwig on the Deck at 512 frames / 48 kHz, SameCallback,
editor open, for ten minutes without a dropped cycle, then thirty minutes with
normal interaction. Preserve projects, licensed state and machine headroom.

## Works now

Installed direct mailbox v4 connects callback to Windows render without the
native relay/socket pump. Reference output/state and second setup pass; killing
an owned reference peer wakes the pending call in 3.14 ms and retires cleanly.
Typical bridge remainder is now 0.121 ms, down from roughly 0.64 ms.

## Still broken

The full 600-second editor-open capture still has ten silent 512-frame blocks
and ten graph errors. All 76,926 lifetime calls return successfully; no measured
call exceeds nominal 512/48k cadence. Actual device budget remains distinct.
Nine arrivals exceed 16 ms; the longest has 15.835 ms outside the prior call.

## Best explanation (about 60%)

The controller produces two synchronous cross-host polling messages every
10 ms, even with a closed editor. Empty polls do not wait for Wine or state;
nonempty polls reenter host edit/restart handlers. This may contend with host
call dispatch. Routing to the actual audio caller is not yet established.

## Next change

One opt-in diagnostic paces recurring AP10/AP11 polls to 100 ms while retaining
the 10 ms close/focus timer, immediate bootstrap/commands and bounded queued-event delivery.
It adds notification latency and is not an accepted production default.
Reuse Windows/runtime bits, build the native candidate, and repeat the same
600-second captured fixture. Keep or revert according to the drop count.

## Done

The targeted controller regression preserves gestures, latency and lifecycle.
The installed comparison measures output/graph errors and complete call timing.
If drops remain, revert pacing and examine exact host dispatch boundaries.
Do not start thirty minutes until ten minutes is clean. No priority/buffer trial.

Source/SSH/Deck: Sol6.1 xhigh; GUI: Sol6.1 high; root orchestrates.
