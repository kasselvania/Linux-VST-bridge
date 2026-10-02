# Installed late-note-off comparison

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
