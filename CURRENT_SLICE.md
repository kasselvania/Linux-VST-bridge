# Current task: finish playing-audio continuity

## Goal

An effect on a sounding track passes incoming audio through until it can process,
then engages. Instruments stay silent. The operator approved this on 2026-10-06.

## Current result

Test 22 is installed on all 11 Deck publications with expanded diagnostics; audio and scheduling behavior are unchanged.
On 2026-10-07, local Test 22 had no audible pause per the operator; all 8,867 calls were retained, maximum 29.749 ms.
Server ERR rose 0 to 5: first +1 unassigned; the +4 batch is consistent with the later stall, with exact attribution open.
Test 21's 2026-10-07 BEAM loading/touch pass and 0.293 ms first-ready result remain.

## Best explanation

Dry audio removed the observed loading silence. The later delay is in the Windows
receive handoff: at least 38.086 ms from native send until receive completion,
with a 1.007 ms vendor call. A Wine wait/scheduling dependency is likely (about 70%).

## Next changes

- Compare a fresh Test 22 host with LVB_HOST_NICE=0; verify ordinary tasks at nice 0 and native/render RR 5.
- Repeat editor gestures and loading with Nibbi and Serum 2; check instruments,
  startup automation, fresh save/reopen and offline export on the installed pair.

## Done

Loading BEAM, Nibbi and Serum 2 over a playing session produces no audible gap,
and no call in an already playing instance exceeds half a block (5.3 ms at
512/48k). Startup automation reaches the plug-in and save/reopen preserves it.
The earlier editor-gesture stall remains open until a repeat captures it.

## Separate open work

- Nibbi touch, per-plug-in multi-touch and the configured six-instance ceiling.
- Ctrl cleared after reboot while the Deck stayed local with Moonlight/Sunshine disconnected; its source remains a gap.
- A plug-in inspection crash can roll back an update for unrelated plug-ins.
- General preparation, manager/UI and replacing the Python supervisor continue
  after this stability repair; portability remains part of the product.
