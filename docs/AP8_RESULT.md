# AP8 commercial instrument — implemented, awaiting review

Serum 2.0.18 generated note audio through the native bridge and reopened in
normally launched Bitwig 6.1 with the saved Main Vol value. PR #67 implements
issue #66. This is a bounded Steam Deck fixture result, not general Serum or
Linux support. The Windows module performed synthesis and state restoration.

## Installed candidates and failure boundaries

| Candidate | Observed result |
| --- | --- |
| Pure LoFi 1.0.0 | Installed module and factory; component creation succeeded, but `initialize()` returned `kResultFalse`. The observed resource-path setting and existing graphical session did not change that result. The vendor cause remains unestablished; activation is not asserted as the cause. |
| Efx FRAGMENTS 1.0.0 | Installed stereo effect with sidechain; component/controller lifecycle initialized. The inspection then exceeded the old per-parameter diagnostic event bound. Compact metadata output repairs that harness problem, but FRAGMENTS processing was not subsequently exercised. An effect does not satisfy AP8's instrument claim. |
| Serum 2.0.18 | Installed instrument and FX classes; the explicit instrument class passed lifecycle, real parameter enumeration, state capture/synchronization, connected note processing and the desktop check below. |
| Scaler 3 / Scaler 3 Audio, imagiro piano 2.2, autochroma | Windows modules present in the bounded inventory; not executed as fallback fixtures after Serum succeeded. |

Early attempts also exposed an incorrectly mandatory factory `setHostContext`
result and vendor stdout being parsed as bridge JSONL. Both were repaired;
the earlier failures remain in [inspection evidence](../evidence/ap8-commercial-instrument/installed-module-inspections.json).
An initial environment-setup exception and a connected owner lost when its SSH
command ended also remain in private diagnostics. The connected owner was
moved to the existing user-service launch route. No installer or new activation
flow was used. Existing module hashes remained unchanged. The existing
installation's license channel and long-term authorization posture were not
established; successful processing does not fill that gap.

## Connected implementation

The prepared descriptor comes from actual SDK metadata: no audio input, stereo
main output, 16-channel note input, 2,623 controller parameters and zero vendor
latency. Stable native IDs derive from the vendor class, while state and the
owner handshake also bind the exact module digest. Publication is separate
from the original yabridge wrapper. AP8 uses the existing per-instance native
queue, mapped/TCP Windows transport and supervised owner, with one commercial
endpoint at a time in the persistent prepared environment.

Bounded note-on/off events carry note identity, channel, pitch, velocity and
sample offset alongside real parameter IDs and points. Changed protocol data
has its own version. Expired presentation output retains ordered input
processing; stop/restart preserves AP7's explicit gap/failure behavior. Actual
component and supported controller state remain opaque, with real Windows
controller synchronization/readback. AGain's state decoder and project IDs
remain separate. AP7 N1 is included: bounded observation records are emitted
individually; a regression retains eight complete traces.

## Exact fixture and source provenance

- SteamOS 3.8.16, build 20260716.1, x86_64 Steam Deck;
  Bitwig 6.1 Flatpak, Freedesktop Platform 25.08.
- Proton `11.0-2c`, build `25118279`; Steam Linux Runtime 4,
  `4.0.20260805.254769`. Launch identity digest
  `20064247dc297d0228bbf63d5a49050238c518d14fbb77b61a8c73351db5ebf7`.
- Windows Serum module: 18,062,336 bytes, SHA-256
  `838bc7ab42d5d039156768680ffc3e0175d6e6bed9f99802989a01d695e13175`;
  instrument class `56534558667350736572756D20320000`.
- Native build source `65adb96285b664d2fe394ea157178f71ce14c8cf`, binary
  `2f2b78ed92cfe238486ce8298cad399c39c095665ed9665a67560fac428d5525`;
  desktop owner source `31e020e3f1a5b93e9758d156d35f66c39ad6c325`.
- Windows source `f872701e4f2b403a3aeb4c10fbe190a385248827`,
  [build 34060332214](https://github.com/kasselvania/Linux-VST-bridge/actions/runs/34060332214),
  executable `80451fef2e3f0111ad127616c079eedf8fe216d27bcbf5d4761fadfe579fdd57`.

The delivered branch subsequently added focused regressions, corrected owned
cleanup, and replaced two equivalent OR patterns with Rust range patterns for
lint. No subsequent DSP, state, event or queue behavior change is presented as
a new desktop observation. [Desktop evidence](../evidence/ap8-commercial-instrument/desktop-result.json)
also records the exact SDK, descriptor, app commit, original log hashes,
project hash, permissions and source distinctions.

## Verification

The [SDK connected test](../evidence/ap8-commercial-instrument/serum-sdk-connected.json)
used the actual native proxy and Windows Serum. MIDI pitches 60 and 67 used nonzero
sample offsets and distinct note IDs. Measured windows had RMS 0.1146 and
0.0426646; Main Vol changed to 0.25. The final release window was exactly zero.
Opaque capture/restore returned Windows controller value 0.25. There were no
rejected callbacks, underrun frames or terminal faults. The SDK callback audit
reported no forbidden callback effects. The original cleanup report counted
the owner itself as a stage process; its original failure is retained, with
the later process readback and ownership-predicate repair documented separately.

In Bitwig, the existing S1 clip played distinct pitches at 110 BPM. The operator
reported notes triggering audio at handoff. The continuation observed track
and master meter activity over Moonlight, changed Main Vol from 0.5000 through
0.3900 to **0.1150**, resumed playback, stopped and saved **AP8 Serum Recall**.
Bitwig and the Windows endpoint exited. After a new launch from KDE
**Applications > Multimedia > Bitwig Studio**, the project reopened with
**0.1150 visible before any edit**. The untouched recalled clip produced meter
activity; transport stop returned the meters to silence. Both instances were
then closed and positively contained.

| Desktop counter | Before close | Reopened |
| --- | ---: | ---: |
| Processed frames | 15,887,104 | 2,226,432 |
| Delivered frames | 15,884,032 | 2,225,408 |
| Priming frames | 3,072 | 1,024 |
| Underrun frames / gaps | 0 / 0 | 0 / 0 |
| Rejected callbacks / terminal faults | 0 / 0 | 0 / 0 |
| Successful silent frames, including idle/priming | 8,781,056 | 1,488,896 |
| Owner retirement / process containment | PASS | PASS |

The desktop observer's first 32 numerical windows covered idle setup. Later
windows were explicitly unretained. Consequently **desktop RMS/peak and exact
waveform recall are not claimed**. Streamed meters and state readback support
the desktop behavior; the separate SDK run supplies numerical note/release
measurements. Legacy AGain gain/reference fields in commercial logs are not
Serum control values or an exact-DSP comparison. Acoustic capture was not
performed by this continuation.

Focused checks pass: 9 native-client tests, 27 backend tests and 18 fixture/owner
tests, including event/parameter ordering, nonzero offsets, opaque identity
and integrity, vendor reserialization, deadline/disconnect/correlation failures,
queue epochs, observation bounds and cleanup. Native-library and backend
Clippy pass with warnings denied. A broader CLI lint invocation also found a
pre-existing range-loop warning in unchanged `native-audio-client/src/main.rs`;
that unrelated CLI cleanup is deferred. [Check scope](../evidence/ap8-commercial-instrument/checks.json)
records the corrected Python test invocation and lint boundary.

## Cleanup and remaining limits

Original preferences and VST3 metadata were restored and compared with their
pre-test backup. The experimental bundle was moved out of publication into
`<HOME>/AP8-Commercial-Test/completed-desktop/CommercialInstrumentBridge.vst3`.
The owner service is stopped, its socket is absent, the session directory is
empty, and no Bitwig/native/Windows test workload remains. Persistent vendor
environments, private diagnostics and build artifacts remain for reuse.
No remote-access configuration was changed.

The saved project remains at
`<HOME>/Bitwig Studio/Projects/AP8 Serum Recall/AP8 Serum Recall.bwproject`,
SHA-256 `faa8dfb8cf75fec0f951fb21e703bdb5ee1d150d34189c49b99f9872f53dc0d2`.
It requires the prepared preview publication and owner to be made available
again before use. A direct save to the host's AP8 task directory failed in
the file chooser; saving under Bitwig's existing Projects location succeeded
without a permission change. Vendor state and the project remain outside Git.

No reboot, editor/preset browser, MPE, full automation matrix, activation,
multiple commercial instances, latency tuning or long-run reliability is
claimed. Pure LoFi's initialization cause remains unresolved. This slice is
ready for review in #67 and remains unmerged.
