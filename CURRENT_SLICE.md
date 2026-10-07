# Current task: finish playing-audio continuity

## Goal

An effect on a sounding track passes incoming audio through until it can process,
then engages. Instruments stay silent. The operator approved this on 2026-10-06.

## Current result

Test 21 is installed on the Deck; all 11 published plug-ins use the updated pair.
On 2026-10-07 the operator heard no audible gap loading BEAM over sound and
confirmed touchscreen knob dragging. Startup carried dry audio for 334 blocks
(3.56 seconds); the first ready call took 0.293 ms. The server recorder missed this run.

## Best explanation

The loading gap came from deliberately silent effect startup blocks; dry audio
removed the reported symptom. A later 53.7 ms callback remains; vendor DSP took
3.93 ms. The long Windows receive interval is the next candidate; its owner is a gap.

## Next changes

- Use the saved callback trace to locate the later receive delay and choose a fix.
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
