# Native late-note-off recovery and installed comparison

The SDK input boundary preserves a negative-timestamp note-off's exact voice
fields and places its release at sample zero of a nonempty block. Other input
validity is unchanged. This exception does not normalize note-on timestamps,
future note offsets or zero-frame notes. The wire validator remains strict.

The production repair is extracted from `2e159ca7` onto trunk
`13ed1d85e830d581ec297760e04e9f9433bfd671`; the helper retains `bad5c69c`'s
truthful vendor latency and early-failure retirement corrections. The actual
Processor SDK test covers -1661/-1, exact note identity and following valid
audio under callback audit. Separate negative cases inspect unchanged offsets
at a refusing transport stub; they do not exercise a real Windows worker.

The source and CI checks for this extraction establish their own narrow scope.
The [original physical comparison](https://github.com/kasselvania/Linux-VST-bridge/blob/a308fbf93e623ba2b8b5bc5e029f06f440ae390c/evidence/audio-recovery/2026-10-02-late-note-off.json)
belongs to stacked base `37b5d41e` and tested source `bad5c69c`. It is retained
provenance, not an installed qualification of this extracted tree.

## Optional installed comparison

Build `ap18-late-note-off-host` with `AP10_RESULTS_TESTS=ON` against the pinned
SDK. This is maintainer instrumentation, not a customer's runtime requirement.
Run only with the fixture idle, its exact installed module/proxy/host hashes
verified, and its selected bridge delay at 512 frames.

`ap18-late-note-off-host BUNDLE PRIVATE_OUTPUT_PREFIX 0` supplies the control.
Repeat with `-1661`, using a new prefix and a fresh instance. Both runs use
48 kHz, float32, maximum and actual block 512, 720 paced callbacks, notes at
blocks 8/360 and releases at 120/472. The first release is the only differing
input. Factory identities, every callback result and duration, later state
capture and SDK retirement results are emitted after processing.

The `.f32le` capture contains interleaved stereo samples returned by the actual
installed instrument for the entire sequence. Keep commercial output private.
Analyze onset, release and the later note separately; a successful callback is
not an audio-continuity oracle. No diagnostics, callback audit, GUI or state
operation runs concurrently with the audio sequence. A failing callback is
retained and later callbacks continue, exposing permanent refusal. A nonzero
exit is expected on the defective negative-offset run. Independently verify
product lease retirement and cleanup before the next run.

This is an installed physical SDK-host test. It does not establish ordinary
DAW behavior, deadline reliability, save/reopen or low-latency qualification.
