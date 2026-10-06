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

- Both sides and wire codecs now accept 256 MiB; oversized read-only capture
  refuses cleanly. Partial restore failure remains terminal.
- Specific bridge errors survive; versioned C ABI returns actual-sized bytes.
- Windows/Linux builds and recorded-state/refusal/audio regressions pass.
- Test 10's matched kit is assembled. The existing package signer is missing;
  locate it, then use the manager's normal update and test Nibbi save/reopen.

## Done

Both builds pass. The Deck has the matching pair. Nibbi with recorded audio
saves and reopens in Bitwig; a capacity refusal keeps a healthy host usable.
Original projects, plug-in installations, activation and the selected runner
are preserved. Raw build/test logs stay outside Git.

## Separate open work

Touch knobs, editor/audio contention and broader plug-in intake remain open.
Wine's built-in Direct3D 11 stays the default; Nibbi uses its already-selected
DirectComposition reference runner.
