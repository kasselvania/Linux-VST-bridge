# Delivered reference plug-ins

These original instrument/effect modules and Rust installers are first-party
test instrumentation. They use the official SDK at its C++ boundary. They do
not substitute for commercial acceptance or provide vendor DSP, authorization,
presets or state.

Build both module targets and their contract tests through the Windows artifact
workflow. `tools/beta/build_reference.py` embeds those exact module bytes in
three installers and retains their hashes in `LVB_REFERENCE_MANIFEST.json`:

- **LVB_Reference_Plugins_1_0_0.exe:** ordinary install of both modules, with
  explicit Install/Cancel and completion screens.
- **LVB_Reference_Recovery_1_0_0.exe:** waits at a source-owned window until the
  user continues or stops the exact installation from the manager.
- **LVB_Reference_Partial_1_0_0.exe:** waits after installing the instrument,
  before installing the effect. Cancellation must retain that partial result;
  discovery and retry must follow the manager's actual classification.

Use the real delivered frontend to import, install, discover, check, prepare and
publish these modules. A direct module copy or private bridge helper is not an
installed journey. Reopening the manager while the held installer is running
must recover the same operation; neither installer requires an account or EULA.

Each module has Level and Colour parameters, 24-byte versioned state and
controller state. The instrument has sixteen bounded voices and sample-offset
note onset/release; the effect preserves stereo polarity while applying its
gain. Use the DAW's generic parameter controls. A custom vendor editor is not
implemented or claimed by these fixtures.

The SDK tests verify recognizable component/controller state, refusal of
truncated/wrong-role/non-finite state without mutation, sample-offset automation,
stereo output, and instrument note release. They do not establish project
save/reopen, reboot, installed recovery or audio timing. Those require the
delivered package and actual DAW. Existing AP10 malformed-output/delay fixtures
remain separate failure instrumentation.

The audio-completion slice builds a separate compile-time variant of this same
original producer, with distinct completion class identities and two default
active stereo outputs. The main output follows the reference equations after
13 samples of vendor latency; auxiliary left/right are respectively one half and
minus one half of the main channels. The fixture declares a finite 13-sample tail.
Component/controller state remains the recognizable opaque reference state.

Completion parameters 0/1 remain Level/Colour. Parameter 32 is the read-only
observed actual process mode: realtime 0, prefetch 0.5, offline 1. Each completion
also returns final Level and inline MIDI CC 74/channel 2 carrying the actual mode.
First/final notes are echoed with their original identity and sample offset.
Zero-frame inputs are parameter-only; zero-frame output points and the supported
inline non-note event still return. This does not widen zero-frame note admission.

Transient parameter 31, `Fixture operation`, has five exact declared values:

- 0: normal operation;
- 0.25: valid 12-second work, admitted only under actual offline processing;
- 0.5: explicit vendor process refusal;
- 0.75: 65-second offline work for the independent consumer's 60-second deadline test;
- 1: delay the next state reply 200 milliseconds for ordered capture/audio overlap.

These fixture controls are not serialized vendor state or environment/runtime
overrides. Real-time/prefetch may switch without setup; an offline boundary must
match a new inactive setup. No slow work is inserted in ordinary real-time audio.

`lvb-completion-instrument-tests` and `lvb-completion-effect-tests` test the actual
producer and its production bus-census parser: every N 0..1024, mode switching,
legal rate/block setup, in-place stereo, both outputs, original first/final audio
and events, zero-frame results, state and explicit refusal. Their optional
`--slow` additionally executes the 12-second producer check. Windows CI builds
and runs the fast tests and uploads the exact two modules with source/digests.
The existing reference installers still embed only the baseline modules;
completion installed acceptance needs its own supported setup artifact. Module
bytes and source tests alone do not establish that installed journey.

The separate `tools/beta/completion_host.cpp` loads an installed native proxy
through the official SDK and independently checks these equations/results.
Neither producer tests nor one installed consumer replaces real-DAW or soak
qualification. `tools/beta/lifecycle_host.cpp` and the original reference contract
remain the baseline comparison.
