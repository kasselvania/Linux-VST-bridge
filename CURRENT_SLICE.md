# Current task: save recorded plug-in audio without ending the instance

## Goal

Build and install a matching Linux proxy and Windows host that can save and
recall plug-ins whose state includes recorded audio. Nibbi is the reported
failure; the fix applies to every plug-in.

## Best explanation

The operator's recordings fail while the no-recording comparison saves.
Nibbi starts at 730,156 state bytes. Both SDK streams and both wire codecs
cap state at 1 MiB. Exceeding that cap throws an ordinary exception instead
of the existing recoverable save refusal, ending the Windows host. High
confidence; the old host discarded the exception text.

## Changes

- Raise the opaque payload cap to 256 MiB on both sides and the wire codecs.
- Refuse oversized read-only capture without retiring a healthy instance;
  keep restore failure terminal because partial application is unsafe.
- Keep the specific bridge error while keeping vendor/private text out of logs.
- Return actual-sized owned bytes across the versioned SDK C ABI, including
  recovery, rather than allocating 256 MiB for every ordinary save.
- Exercise recorded-state save/restore, oversized component/controller and
  combined state, and continuing audio after a refused capture.
- Build both halves and use the manager's normal matched update on the Deck.

## Done

Both builds pass. The Deck has the matching pair. Nibbi with recorded audio
saves and reopens in Bitwig; a capacity refusal keeps a healthy host usable.
Original projects, plug-in installations, activation and the selected runner
are preserved. Raw build/test logs stay outside Git.

## Separate open work

Touch knobs, editor/audio contention and broader plug-in intake remain open.
Wine's built-in Direct3D 11 stays the default; Nibbi uses its already-selected
DirectComposition reference runner.
