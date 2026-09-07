# AP10 technical-lead handoff

Historical handoff: its pending scope decision was closed by review 5128434219 and CURRENT_SLICE at 5be898e. The return path is now implemented in this PR; see `AP10.md` for current verification and remaining device work. The observations below retain their original source and claim level.

PR #71 is draft, open and unmerged against AP9's branch. AP10 is **incomplete**. Do not promote earlier Serum playback into evidence for the current bus implementation. Do not alter AP8 #67 or AP9 #69.

## Concrete decision

The exact SDK census exposes Serum's main event output bus, index 0, 16 channels. Normally Applications-launched Bitwig requests its activation despite the native default-inactive flag. Native source 86d26cff7370904e6d38cfb446d3205195c8b732 records `media=1,direction=1,index=0,active=true` and rejects it before processing. Windows source f885a620a8f4e06a2753e65aef2f933e9d277b47 also supports input events only. Both failed instances retired cleanly. See `../evidence/AP10/serum-bus-activation.json`.

Recommend retaining the exact buses and adding bounded output-event delivery to the same core. Its absent contract needs explicit supported event types/payload limits, SDK bus identity, ordering, sample offsets relative to delayed audio, and overflow/late-note handling. Returning success and ignoring emitted events is not acceptable. Removing or disguising this main bus would need an explicit change to CURRENT_SLICE's bus requirement; it is not the recommended repair.

Implementation entry points: `windows-factory-probe/source/bus_layout.h`, `offline_processing.cpp` (`ProcessData::outputEvents` is not wired), `mapped_processing.cpp` (`Done`), `native-vst3-proxy/backend/src/lib.rs`, `queued.rs`, `include/ap10_backend.h`, and `source/processor.cpp`. Preserve callback preallocation/no-wait laws. Prove return events with a deterministic fixture, especially delayed note-off, expired audio, overflow and variable blocks, before rerunning the narrow Serum comparison.

## Results to preserve

- First socket stall: Windows audio blocked 6.451 ms in a Wine-server dependency while wineserver ran; Serum process call was 141.2 µs and host callbacks remained regularly spaced. Mailbox repair removes this socket notification dependency. Exact monopolizing server operation remains unknown.
- Earlier mailbox Serum: untraced 256-frame run had zero missing frames over 36,262 blocks; 128 had 3,840 missing frames. A longer traced mailbox run still had a 31.9-ms reply-preparation outlier of unknown CPU/blocking/scheduling cause. These use the older instrument bus shape, not the final candidate.
- Current deterministic AGain: exact 256 and 128 added frames, 384,000 samples each, zero error/gaps/rejections, clean state/close.
- Actual FRAGMENTS: input-dependent wet output, Grain Mix control, save/quit/Applications-reopen at 1.0000, frozen output beyond clip/transport stop, and silence after release. Controller synchronization repair changes the actual instance and opaque state. All gaps remain counted (1,792 / 2,176 / 2,560 across respective longer sessions).
- Near-zero audio under optional SDK silence hints caused the first effect failure. The repair preserves sample bits and clears only contradicted hints. No resampling or precision conversion.
- 512 remains recommended. Current-candidate 256/512 musical-load comparison is outstanding. The historical Bitwig close-time crash has no usable stack and remains unexplained.

Detailed sources, CPU/timing and limitations: `AP10.md`, `../evidence/AP10/delivery-comparisons.json`, `effect-development.json`, `reference-current.json`. Hosted Windows and proof-policy checks passed at 86d26cf; the native diagnostic build also passed. Passing builds do not override the observed Serum regression.

## Fixture and resumption

The device is restored and all AP10 owners stopped; `../evidence/AP10/cleanup.json` records exact readback. AP8/AP9 projects are unchanged. The AP10 FRAGMENTS project retains verified Grain Mix 1/Freeze off and has a private backup. No vendor authorization/content reset was performed.

Private work and raw records remain in `/private/tmp/ap10-work` on the coordinator and `~/AP10-Work` on the fixture. The independent checkout is `/private/tmp/ap10-bridge`. Never commit proprietary audio/project/state contents or the raw owner reports. The original shared checkout is dirty and was not changed.

Use the retained build/transfer helpers and normally launched Bitwig. Do not restart a matrix, repeat the already-proven effect work, or create another campaign. Once the return-event decision is implemented, restore Serum playback/state/close, compare the same 256/512 workload, and only pursue 128 if those results warrant it.
