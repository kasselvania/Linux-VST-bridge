# Current task: smooth the effect startup handoff

## Goal

Keep latency-aligned dry audio audible while an effect starts its current wet
stream, then engage over 50 ms. The operator approved this on 2026-10-07.

## Current result

Test 22 is installed on all 11 Deck publications. The operator clarified that
BEAM's sustained loading silence is gone; a brief dry-to-BEAM hiccup remains.
Touchscreen knob dragging works. The later stall persists with nice 0: all
7,197 calls retained, maximum 27.188 ms, server ERR 0 to 4. Baseline restored.

## Best explanation

The handoff waits for due wet work, then fades over one maximum block (10.7 ms
at 512/48k). A longer transition is a plausible repair; audible proof is pending.
The separate later stall points to a slow Wine event wake after the prior reply.

## Next changes

- Test 23 source keeps Buffered startup dry without waiting for unfinished wet
  work, then commits a current, aligned 50 ms linear fade across variable N.
- Compare BEAM loading and dry-to-wet engagement without moving a knob.
- Preserve SameCallback D=0 synchronous completion, instruments, public N0 and
  offline rendering; repeat startup automation and fresh save/reopen.

## Done

Loading BEAM, Nibbi and Serum 2 over a playing session produces no audible gap,
and no call in an already playing instance exceeds half a block (5.3 ms at
512/48k). Startup automation reaches the plug-in and save/reopen preserves it.
The earlier editor-gesture stall remains open until a repeat captures it.

## Separate open work

- Later Wine wake stall; Nibbi touch, multi-touch and the six-instance ceiling.
- Ctrl cleared after reboot locally without streaming; its source remains a gap.
- Inspection crash rollback, general preparation/manager/UI, supervisor replacement and portability.
