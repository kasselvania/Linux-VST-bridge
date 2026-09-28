# MIDI0 — Push note release through the native VST3 proxy

## Fixture and evidence

- Base: canonical main `c920f2eca09bb27e92172b1f32e5899d45d2630b`, tree
  `656049fe96aa9a0685c8326ca5a93058b3b0e08b`.
- Physical fixture: Push 3 in controller mode through the Steam Deck dock,
  Bitwig and native bridge publications. The operator reports a held note on
  each Push pad release across bridged plug-ins; an OMX-27 and Bitwig's native
  devices do not show that symptom.
- A bounded read-only capture on the Push User MIDI port showed channel 10,
  pitch 63 note-on and matching note-off, alongside expressive CC/pitch data.
  The raw controller did send the release. Recent bridge session reports show
  callback rejections. Those facts do not identify the exact VST3 input event
  type Bitwig submitted in the rejected callback.

## Source failure and correction

The native `Processor::process()` accepted only VST3 note-on and note-off
input events. A poly-pressure or note-expression event caused an immediate
failure of the **whole** callback before later note events could be forwarded.
That source path can discard a note-off in a mixed callback.

MIDI0 keeps the current note/parameter transport unchanged. It recognizes
poly-pressure and note-expression value/text as expression types that this
transport does not yet carry, skips them without rejecting the audio block,
and counts them for the non-real-time lifecycle report. Note-on, note-off,
sample offsets, note IDs and channels still reach the existing transport.
Unknown event types and other invalid callback inputs still refuse.

No allocation, filesystem access, process work or logging was added to the
audio callback. The count is emitted only at normal lifecycle reporting.

## Acceptance and limits

The source-owned SDK test sends note-on, poly pressure, note-expression value
and note-off in one processing block; only the two note events reach the
backend, in order, and the callback succeeds. It also checks an expression-only
block, a later note-off block, and an unknown-event refusal.

This repair does **not** transport Push pressure, slide or per-note pitch into
the Windows plug-in, and does not qualify MPE. It does not change MIDI hardware
settings, audio configuration, the manager or installed Deck state. A later
exact physical check must confirm that the selected successor releases notes
with Push in ordinary Bitwig use. If it still sticks, the next bounded step is
event-type evidence at the native VST3 input boundary; no further pad test is
needed merely to reconfirm the present symptom.
