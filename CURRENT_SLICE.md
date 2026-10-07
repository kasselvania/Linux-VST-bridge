# Current task: keep playing audio audible while a plug-in loads

## Goal

An effect on a sounding track passes incoming audio through until it can process,
then engages. Instruments stay silent. The operator approved this on 2026-10-06.

## Best explanation

Test 20 answers promptly but deliberately mutes the loading effect's track.
About 95%. Its latest BEAM run returned 281 silent startup blocks (3.0 seconds).
The whole callback maximum was 5.78 ms, median 0.041 ms, with no bridge gap.
The audio-server recorder had stopped and did not cover that run.

## Next changes

- Keep dry effect audio at the reported bridge plus vendor latency through
  startup and priming, then crossfade to the ready result over one maximum block.
  Use the admitted main stereo buses; auxiliary outputs stay silent until ready.
- Retain final parameter values from skipped startup blocks and flush them in
  order before the unchanged first ready curve. Keep one originating deadline.
  Refuse a fresh save while accepted edits have not reached the plug-in.
- Before offline rendering, synchronize pending settings internally at N0 under
  the first call's 60-second bound. Add no DAW audio/tail blocks or samples;
  report failure if synchronization cannot complete. Preserve export length.
- Build the reviewed pair as Test 21 with the existing builder signer, install,
  and repeat loading BEAM over sound. Measure the first ready callback too:
  deferred parameter synchronization can itself take time.

## Done

Loading BEAM, Nibbi and Serum 2 over a playing session produces no audible gap,
and no call in an already playing instance exceeds half a block (5.3 ms at
512/48k). Startup automation reaches the plug-in and save/reopen preserves it.
The earlier editor-gesture stall remains open until a repeat captures it.

## Separate open work

- Nibbi touch, per-plug-in multi-touch and the configured six-instance ceiling.
- Control is held in XWayland; the unread SteamOS virtual input remains a gap.
- A plug-in inspection crash can roll back an update for unrelated plug-ins.
- General preparation, manager/UI and replacing the Python supervisor continue
  after this stability repair; portability remains part of the product.
