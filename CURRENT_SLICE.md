# Current task: direct audio completion

## Goal

Pure LoFi plays in Bitwig on the Deck at 512 frames / 48 kHz, SameCallback,
editor open, for ten minutes without a dropped cycle, then thirty minutes with
normal interaction. Preserve projects, licensed state and machine headroom.

## Works now

Whole native calls retain correlated Windows/vendor process-call time. Typical
baseline time was 3.18 ms whole / 2.55 ms inside Windows, not pure DSP CPU time.

## Still broken

Matched editor-open and editor-closed runs each had three graph errors and three
silent blocks. Wine-server priority, caller priority and in-process hosting did
not remove drops. Priorities and Together/Auto 512 preferences are restored;
the interrupted 1024 comparison was never played.

## Best explanation (about 70%)

The synchronous audio dependency crosses a native relay and TCP notification
pump and can service state parsing/hash/snapshot work before inspecting an audio
reply. This structure adds contention even when vendor processing is bounded.

## Next changes

First prove an original, product-built shared-futex adapter on the selected
Wine/Linux x86-64 runner. Then give the prepared native callback and actual
Windows render thread their own AUDIO request/completion slot and direct wake.
Keep control/state processing outside their completion dependency. An off-path
watcher must wake both waiters on death/cancel; expiry never releases host-owned
storage. Keep legal lifecycle, state round trips and exact epoch/ticket bounds.

## Done

Focused audio/control-overlap, cancellation, expiry and retirement checks pass.
The paired installed build preserves output/state, then passes the editor-open
512/48k checks above with actual output and graph counters. N/Fs is cadence
context; it is not a fabricated device deadline. Old build stays available for
comparison/rollback. No runner, priority, manager or licensed-state expansion.

Source/SSH/Deck: Sol6.1 xhigh; GUI: Sol6.1 high; root orchestrates.
