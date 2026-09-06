# AP5 — Independent instances in one native host

PR #61 implements issue #60 from `ffbc4d2`. The focused normal-desktop recall/removal check passed for two actual Windows AGain instances. **One earlier instance lost its transport; its cause remains unresolved.** This is a reviewable development result, not acceptance or a reliability guarantee.

## Implementation

The Rust queued backend has four independently owned slots, generation-checked handles, and per-instance queues, control mailbox, state, witness, worker and connection. A callback acquires a bounded atomic lifetime lease and its own nonblocking guard. Startup and teardown reserve only their slot; removal stops admission and waits outside audio for that slot's readers. Stale handles and excess capacity are refused.

The existing private preview owner admits up to four sessions, each using the existing Windows launcher, disposable environment, transport and cleanup. A failed session does not stop healthy siblings. Uncertain containment retains that stage and refuses new admission. Stable VST3 class IDs and the AP4 state envelope are unchanged; temporary instance identities do not enter projects. The factory now advertises multiple instances. SDK diagnostic paths belong to the processor instead of thread-local/global state. Linux socket peer credentials record the actual connecting native process.

Windows DSP, complete component-state saving and controller synchronization remain the AP4 implementation. Request/message deadlines, owner/disconnect detection and 1024-sample latency are unchanged. There is no new broker or DAW launcher.

## Focused checks

At implementation source `f678c4e0eaa85215855f231c3ea4b4fa8409dfc1`:

- 18 Rust backend tests passed; Clippy with `-D warnings` passed.
- Seven private-owner tests passed, covering concurrent sessions, capacity, disconnect and failure isolation.
- Four SDK-loaded Linux regressions passed: `instances-isolation`, `instances-failure`, `state-gain`, and `state-lost-set`. These use **substituted Windows peers**, not real Windows DSP evidence.
- The two-instance regressions run two factory-created copies in one process, with two concurrent audio threads. Delayed sibling startup/close and failed sibling transport leave the survivor processing. Each survivor processed another 48,000 frames after its sibling closed; checked output error and callback-audit effects were zero. Failed state retrieval stayed failed, with no default substitution.
- The final native build succeeded through the existing build helper. CI passed at that implementation source.

The final queued binary is `cb73415087c661cbc5009ba4b97d85c4b8aa0c92ffc57ffdf014123cbea166cf`, native manifest `ed2ee1ebe614e8f41377162b3a1977e8b6f02edabebb55e0b21118a7bb65ed0c`. The desktop owner ran unchanged from `d1bf7353c28c355cf11d39b817a67b7c04799ca5`.

## Normal Bitwig project

On the maintainer's Steam Deck, Bitwig Studio 6.1 was launched from **Applications** for both project creation and fresh recall. The bridge used the prepared Proton 11.0-2c / Steam Linux Runtime 4 fixture and the unchanged AP4 Windows host artifact. Bitwig and its native plug-in host had no `LVB_` environment keys. Hosting was **Together**, with AGain's individual-isolation override off.

Two audio tracks used distinct generated stereo WAVs: A at 330/440 Hz and B at 517/733 Hz. Each contained its own AGain Queued Preview. The saved gains were 0.6750 and 0.3500. Their complete 12-byte Windows component-state payloads had different hashes, reproduced exactly by fresh Windows `setState` and readback:

| Track | Saved/restored gain | Component-state SHA-256 |
| --- | --- | --- |
| A | 0.675000012 | `2346a6c4bf2fb57fd9cc3e217a3f903b6548614fdb31172d80119585290a9d64` |
| B | 0.349999994 | `c4462717ee17596492abf5ccf5b4d662ddf2f5ba6eda68e2f22736a8dbd562a6` |

The initial save file hash was `6277fcd8a55cd226aea69a83e8213175e94f69cd31f66db65dda4fa68c85cc31`. It is retained privately, including a backup made before recall. Normal save and Save As after reopening reused Bitwig's cached project state without another component `getState`; these were not counted as fresh Windows saves. No cached bridge defaults were used for restoration.

Both restored controls were inspected before editing or removal. Supplemental read-only snapshots of changing, guarded, non-poisoned shared audio planes matched the saved gain on each distinct input before removal. These selected matching snapshots are not protocol acknowledgements or an exhaustive comparison. The exhaustive returned-audio comparison is the existing per-instance transport-worker witness retained at teardown:

| Fresh recalled instance | Compared samples | Nonzero samples | Maximum error | Gain edits | Queue faults / callback rejections |
| --- | ---: | ---: | ---: | ---: | --- |
| A | 27,062,784 | 26,059,576 | 0 | 0 | 0 / 0 |
| B | 34,433,024 | 30,234,286 | 0 | 0 | 0 / 0 |

All **61,495,808 samples** were after state restoration and before any gain edit. Per-instance input and output fingerprints differed. Both socket connections came from native Bitwig plug-in host PID 2223125; each mapped a distinct Windows stage. Windows host PIDs were 2223457 and 2223799.

While both tracks played, A's plug-in was removed. A terminated cleanly and its owned processes/stage were retired. B retained native PID 2223125 and Windows PID 2223799 with the same process start identities. New B audio-plane snapshots still matched 0.3500, and its final witness above remained error-free. B later closed cleanly. Both Windows hosts exited zero and both cleanup checks found no owned descendants or process group.

The removal was discarded on quit so the saved project retains both instances. The project, WAVs, original save backup and private results remain on the fixture. Test publication was removed from Bitwig's scan directory, original preferences were restored byte-for-byte, the preview owner stopped, its socket disappeared, and no test Windows/native processes or stages remained. The accepted AP4 project hash was unchanged. Sunshine remained active as a separate service.

## Earlier failure and remaining limits

During initial creation on native source `3e9fee96c3c850ea5bb57c6e0eec38c3f45b9276`, B lost its transport after the successful save. Before loss, its witness had compared 10,504,704 samples at zero error and saved its actual 0.3500 state. Windows reported `control disconnected/IO`, exited 110 and cleaned up; A continued and ultimately closed cleanly. The native first-fault report was written too late for the already-retired stage, so the root cause cannot be assigned from retained evidence.

`f678c4e` adds a bounded terminal-worker fault record before retirement. Its loaded failure regression passed. This fixes the diagnostic loss **without claiming to fix the unknown transport-loss cause**. No loss occurred in the subsequent focused recall/removal check. The earlier failure remains a reliability limitation requiring review; it is not relabeled as success or attributed to save timing without evidence.

Capacity is four native instances per process and four owner sessions total; real desktop evidence covers two. This remains float32 stereo at 48 kHz, up to 256 frames, with 1024 samples added latency. Fresh disposable Windows environment startup is visibly slow. Prepared runtime/artifacts and a running private owner remain necessary. Commercial plug-ins, instruments, editors, reboot persistence, general packaging and low-latency suitability are unclaimed.

## Evidence and use

[Sanitized observations](../evidence/ap5-independent-instances/result.json) retain per-instance state/audio/lifecycle records, the earlier failure, topology, cleanup, local tests, exact artifacts and private raw-record hashes. The [supplemental audio-plane observer](../evidence/ap5-independent-instances/observe_audio_planes.py) is read-only development instrumentation.

Use the [existing AP4 preview setup](AP4_PREVIEW.md) with this branch's native bundle and `tools/ap4_preview.py`; a 60-second user-service stop timeout was used for the AP5 check. The socket location and launch contract are unchanged. AP5 replaces that document's historical one-instance restriction with the capacity above; each session owns its resources, and a containment failure blocks new admission while healthy siblings continue. No preview service or test publication is left active after this check.

[AP4's accepted result](AP4_RESULT.md), [original D9/A2 provenance](AP4_ATTEMPT_STATUS.md), main's simplified instructions and historical ledgers are unchanged. No AP4 replay or new acceptance campaign was performed.
