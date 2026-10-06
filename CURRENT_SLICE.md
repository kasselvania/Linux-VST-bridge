# Current task: keep the DAW's audio running while a plug-in loads

## Goal

Loading a bridged plug-in, or opening its editor, must not interrupt audio
that is already playing. This is the last stability item before the operator
soaks the Deck; portability to other distributions follows the soak.

## Best explanation

Bitwig's audio thread waits inside the bridge's callback while the Windows
side is slow on a new plug-in's first block. About 70%. A BEAM session on the
Deck has one callback of 2.88 seconds and no lost frames; the editor opened
and audio started within a tenth of a second of each other. The frame
counters stay at zero through this, so they are not the measure.

## Next changes

- Read the per-callback trace (`LVB_PROCESS_CALL_TRACE=1`, switched on for
  Bitwig on the Deck) and the Windows-side delivery trace from one session
  that loads BEAM over playing audio. They say which callback stalled and
  whether the time went in the vendor's call or in waiting for it.
- If the stall is the instance's own first blocks: answer a block that is not
  ready during start-up with silence at once, and keep the fixed delay.
- If it is another instance's callback: find the shared resource and remove
  it.
- Rerun the same load on the Deck and compare the longest callback.

## Done

With sound playing, loading BEAM, Nibbi and Serum 2 one after another on the
Deck gives no callback longer than half a block (5.3 ms at 512 frames and
48 kHz) in an instance that was already playing, and no audible gap.

## Separate open work

- Nibbi's knobs do not follow touch; BEAM's do, on the same driver.
- Control sticks on the Deck desktop; cause unknown, a passive logger runs.
- Multi-touch capability per plug-in, and raising the six-instance ceiling.
- The supervisor is still Python; it costs about 1% of a CPU thread per
  running environment and nothing when none is loaded.
