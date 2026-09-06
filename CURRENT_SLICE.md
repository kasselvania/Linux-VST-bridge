# Current work: AP8 — First playable Serum 2 instrument

## Outcome

Play a note clip through **actual Windows Serum 2** in Bitwig launched normally from Applications, change a real exposed parameter, and save/reopen that sound. Issue #66; branch `codex/ap8-serum-first-sound`. This is the lead-selected next implementation task following the operator's instruction to move on. Necessary code changes, builds, ordinary debugging, scoped setup and focused verification are included under AGENTS.md. No per-source approvals, campaign replay or GUI countdown.

AP7 is accepted through PR #65, merge `cdfd05be9af2576768d8f2ccc55d3064e9e72a36`, reviewed at `5232870e6ce024efe52c2d6be0af7f62b7f021c3` (review 5126321457). Transient gaps no longer kill healthy instances; optional observation is independent. AP4 state, AP5 isolation and AP6 explicit recovery remain accepted. [AP7's results](docs/AP7_PLAYBACK_REPAIR.md) retain their source distinctions and unresolved historical delays.

## Start with the actual commercial input

[SR0](evidence/sr0-steam-deck-fixture-reconnaissance/SERUM2_OBSERVATIONS.md) recorded a Windows PE module under `~/.wine/drive_c/Program Files/Common Files/VST3/Serum2.vst3/Contents/x86_64-win/Serum2.vst3` and a separate yabridge wrapper. This is a starting location, not current compatibility or authorization evidence. Inspect the present module, class/controller, bus and parameter contracts; do not load the Linux yabridge wrapper as our Windows input.

Keep the existing .wine installation, yabridge publication and user projects untouched. Use a bridge-owned persistent vendor environment for required content and ordinary vendor authorization, with separately owned instance sessions. Do not clone credentials or consume fresh activations per test. Reuse lawful files or the official installer/authorization route. A minimal detached Windows editor for authorization or initial patch selection is in scope if needed; polished editor embedding is not. A genuine account login or permission goes to the operator through its normal flow; continue independent engineering meanwhile. A blocked vendor route must be reported specifically, not replaced with an AGain success claim.

## Implement the connected path

Extend the working proxy, Rust transport and Windows host rather than copy/rename AGain. Obtain the selected module's metadata through SDK queries; a small prepared descriptor is sufficient. Use stable, bridge-namespaced native class IDs derived from vendor class identity, independent of paths, sessions and runtime revisions. Preserve existing AGain project IDs and keep the experimental Serum publication distinct from yabridge.

Replace the assumptions that processing always has stereo input, a single gain at ParamID 0, no note events, and 12-byte component state. Negotiate the actual main audio/event buses. Carry bounded note-on/off events with channel, note identity, pitch, velocity and sample offset, and real parameter IDs/points through the same ordered input timeline. Version changed protocol data; do not disable identity/bounds checks to admit Serum. Expired output must not drop admitted notes or control changes. Handle stop/restart and note releases without stuck voices; never synthesize sound in Linux as a substitute.

Forward the real component/controller lifecycle and required host callbacks. Enumerate real controller parameters for ordinary DAW controls instead of decoding vendor state. Treat component and supported separate controller state as opaque bytes; synchronize through the actual controller. Vendor reserialization need not be byte-identical: assess successful SDK restoration and restored parameter/sound behavior, not AGain's exact-byte oracle. Keep old AGain state compatibility and isolate reference-specific decoding. The engineer chooses private APIs and layout; this does not require every VST3 extension or a general scanner/manager.

Keep callback-safe transport, independently bounded observation, per-instance ownership and AP7 gap accounting. Start with one Serum instance at 48 kHz, float32 stereo main output, up to 256 frames and the existing 1024-sample bridge delay; account truthfully for any vendor-reported latency. A necessary measured compatibility change belongs in the implementation discussion, not a speculative runtime overhaul.

## Enough evidence and finish

Use focused local regressions for event/parameter ordering, nonzero offsets, state routing and genuine failure. An SDK instrument may help diagnose the interface, but cannot complete AP8. In normally launched Bitwig, play distinct notes through real Serum, observe note-off/transport stop, alter one actual sound control, save and reopen. Verify returned audio responds to the notes and restored controls; count gaps and terminal failures separately. Do not demand bit-identical randomized synthesis or another arbitrary sample total. Label acoustic monitoring separately from buffer evidence.

Include AP7 N1 as ordinary housekeeping: emit each bounded observation JSONL record separately into the existing capped sink and test six or more traces locally. No separate slice or AP7 desktop replay.

Reuse SSH/Moonlight and prepared tools. No commercial purchase, general installer, preset browser, full editor/MPE coverage, reboot claim, latency tuning or unrelated cleanup. Preserve proprietary files, credentials, original projects and prior observations; restore temporary DAW settings and retire only owned sessions. Keep a reusable authorized vendor environment rather than erase it after each test. Publish one PR referencing #66, with the first-sound/recall result, actual module/runtime, limitations and cleanup, left unmerged for review.

Targeted sources: `source/factory.cpp`, `source/processor.cpp`, `backend/src/lib.rs`, `state.rs`, `queued.rs` under `native-vst3-proxy/`; `windows-factory-probe/source/mapped_processing.cpp` and `offline_processing.cpp`; `tools/ap4_preview.py`; [architecture](docs/ARCHITECTURE.md). Official references: [VST3 note events](https://steinbergmedia.github.io/vst3_doc/vstinterfaces/structSteinberg_1_1Vst_1_1NoteOnEvent.html), [processor/controller state](https://steinbergmedia.github.io/vst3_dev_portal/pages/FAQ/Persistence.html), [Xfer authorization](https://support.xferrecords.com/article/46-serum2-machine-authorization).
