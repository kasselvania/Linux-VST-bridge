# Kontakt four-part routing continuation

This continuation uses the existing experimental publication documented in
`shared-bus-playback.md`. Bridge scope base: `fd23d756`; installed product source:
`9d47606`. The existing Jev worker is `jev-application-harness` at `f658435`.
The operator subsequently withdrew this routing task; the current scope is at
the top of `CURRENT_SLICE.md`. This file retains the partial setup accurately.

## Setup observations

The original KontaktStereo project was read back before mutation: 119,281 bytes,
SHA256 `5f6094ad6e52e09edfe755dcc114a58b15497c99caf6bd79f51f83d58151b453`.
Bitwig 6.1 / freedesktop Flatpak runtime 25.08 and an idle bridge were observed.
A separate KontaktFourParts project owns all changes and recordings.

Four Factory Selection instruments were loaded through Kontakt's actual browser:
Pad - Noir, Bass - Monster, Lead - Retro Mono and Pad - Lesotho. The configured
MIDI channels are 1, 2, 3 and 4. Their Kontakt output strips are Out 1, Out 2,
Out 3 and Out 4. Out 4 was explicitly mapped to host channels 63/64.

The actual SDK census is **28 ordinary stereo outputs plus four auxiliary stereo
outputs**, not 32 ordinary instrument-menu choices. The highest bus is index 31,
named `KT Aux 4`. Its two channels are `63 - KT Aux 4 [1]` and
`64 - KT Aux 4 [2]`. Kontakt's output-configuration notice required saving and
reopening the instance. The owned project was saved and Bitwig fully closed
before reopening; no global output default or environment was replaced.

Setup session `b97271b4dcee9dcc591b7828a7293d9f` reported 48,000 Hz, 512 host
frames, 256 transport frames and 512 reported bridge latency frames. Windows
raw exit was -15, error null and cleanup confirmed. This retains the earlier
graceful-retirement limitation; it is not a normal vendor exit claim.

The streamed UI reordered characters during Save As. Filesystem readback caught
the mistyped new project path. While Bitwig was confirmed closed, the owned copy
was backed up privately and normalized to `KontaktFourParts/KontaktFourParts.bwproject`.
Its four-instrument save was 312,455 bytes, SHA256
`427c1ce21cad346217c04ca159079c1ac898dcc66d4fcca1a87b27202abf7b46`.
No working project was overwritten. Moonlight also delivered some UI changes
after the first returned screenshot, so action return alone was not completion.

## Scope withdrawn before acceptance

A generated MIDI audition was imported as four tracks. Bitwig inserted Organ
devices; removing those and routing all four note tracks was not completed.
A routine GUI attempt accidentally deleted the second track; the observed
"Undo Delete Instrument Track" action restored it. The operator also confirmed
that some unexpected input came from concurrent use of Moonlight. The owned test
copy may still contain unsaved edits; the original KontaktStereo was preserved.

Draft MIDI-generation and WAV-checker scripts were retained privately at the
operator's scope change, not committed as a new test suite. No four-stem result,
effect-isolation result, deliberate-mute failure or new project-recall pass was
produced. These requirements are withdrawn rather than silently counted complete.

The subsequent real Jev attempt is recorded in `jev-usability.sanitized.json`.
It used seven decisions, made three successful GUI clicks, and exposed the
existing worker's exact-pixel target-check problem over the Moonlight stream.
The opt-in streamed-text repair belongs to the existing Jev harness, commit
`9cd52a7` on `codex/jev-streamed-target-check`. After the
repair, Jev selected the device and opened its menu; it stopped before the actual
Kontakt editor. Existing Bitwig process identities survived the bounded attempt.

## Routing references

- [NI output configuration](https://docs.native-instruments.com/ni-tech-manuals/kontakt-manual/en/classic-view)
- [Bitwig multi-output chains](https://www.bitwig.com/support/technical_support/how-do-i-use-multi-out-vst-plug-ins-27/)

These references informed setup; they do not establish a routing acceptance.

## Jev readiness follow-up

The subsequent Commander experiment remains blocked: Jev reached Bitwig's
command browser, but the fixed editor-command search did not appear. It did
not open Kontakt's editor. Fifteen validation calls and one cleanup call were
used; no Codex GUI actions occurred. The failed Commander-specific adapter was
removed from the active harness and retained privately. Existing Bitwig process
identities survived, and the original KontaktStereo hash remained unchanged.
See `jev-commander-blocked.sanitized.json`; this does not expand compatibility
or audio acceptance.
