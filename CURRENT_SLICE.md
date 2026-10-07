# Current task: preserve smooth insertion through ordinary use

## Goal

Keep BEAM's smooth insertion and 50 ms dry-to-wet handoff, then verify automation
and fresh save/reopen with its selected SameCallback delivery.

## Current result

The operator reports completely smooth insertion and handoff in unchanged Test 23
at 512/48k, SameCallback D=0/L=0, direct mailbox v4. All 2,538 SDK calls are complete,
with zero fully silent callbacks. First call: 5.598 ms; first wet: 1.067 ms;
later maximum: 1.768 ms. The first call retains the half-block START wait.
BEAM now retains D=0 with Buffered 512 remembered; other class settings and the
global default are unchanged. All 11 publications retain their identities.

## Still broken and best explanation

The SameCallback comparison eliminated the reported hiccup, implicating empty Buffered
delay history or the DAW's latency-compensation response, about 80% confidence.
The comparison does not distinguish those causes. PipeWire's 64 Bitwig samples
show ERR 0 to 1 later, 20.516–23.516 s after the first callback; the 281 calls
in that bracket peak at 1.025 ms. The server error's cause remains unassigned.

## Next changes and done

- Verify ordinary BEAM automation and fresh save/reopen with D=0.
- Repeat normal use and resolve the later server error; retain whole-call bounds.
- Extend smooth loading to Nibbi and Serum 2 before claiming it for them.
- Done: these workflows preserve sound and state, with no audible gaps or later
  server errors and no already-playing call above 5.3 ms at 512/48k.

## Separate open work

- Later Wine wake repair remains parked; earlier editor-gesture stall remains open.
- Nibbi touch, multi-touch, concurrency ceiling and other platforms remain open.
- Ctrl source, inspection rollback, manager/UI, supervisor replacement and portability.
