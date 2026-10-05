# Current task: stop the audio dropouts

## Goal

Pure LoFi plays in Bitwig on the Deck at 512 frames / 48 kHz, same-callback
delivery, editor open, for ten minutes with no dropped cycle. Then thirty
minutes with normal interaction.

## Symptom

Occasional dropouts of exactly one 512-frame cycle, each with a Bitwig graph
error: nine in three minutes with the editor open, none seen in about 2.5
minutes with it closed. Bitwig's native test tone is clean for ten minutes.

## Best current explanation (about 70%)

One bridged call takes 3.9 ms typically and up to 8.4 ms of the 10.67 ms cycle.
The calling thread sleeps while the Windows host processes the block, so the
long "blocked" calls are most likely the Windows side running long. Bitwig's
calling thread and the Wine server both run at ordinary priority, and the caller
waits up to 2.9 ms for a CPU. With the editor competing in the same Wine
process, some calls exceed the cycle.

## Do next, in this order

The operator authorizes every change below. Use only the per-call timer for
measurement: three minutes per run, count calls over 10.67 ms and dropped cycles.

1. Report the Windows host's own processing time for the same calls, to split
   the 3.9 ms into vendor DSP and everything around it.
2. Run as-is, then with the Wine server at realtime priority. Compare.
3. Run editor open, then editor closed. Compare.
4. Find out why Bitwig's plug-in thread is not realtime inside the Flatpak, and
   fix it or document the setup step.
5. Keep whichever changes remove the dropouts, revert the rest, and rerun the
   ten-minute goal.

If the Windows processing time in step 1 is small, the delay is in the bridge or
Wine layer: follow that instead and say so.
