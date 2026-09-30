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
