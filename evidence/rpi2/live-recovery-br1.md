# BR1 live recovery: source-owned result

Base: `36b87ad1c5f55db5c241e5c961a604cd2b18a3be` (tree `150736ab379cff98d08e952595c713b587f51dd2`). This is an opt-in same-instance queued-backend repair. PR #167 and its Serum evidence were not changed.

## Policy and state machine

Ordinary paths remain disabled. The Pi appliance may select `LVB_RPI2_LIVE_RECOVERY=horizon_frames:worker_deadline_ms:1` before its first START. The horizon is 256–8192 frames, the worker-return deadline is 20–5000 ms, and one automatic recovery is allowed per instance. Invalid values are refused. The callback compares `current_position - presentation_delay - next_result_position` with the configured horizon, after consuming already published results. Queue high-water counters are historical observations, not the trigger.

`normal -> requested -> restarting -> acknowledged -> reconciliation -> re-prime -> normal`. Once requested, ordinary admission closes and output is silence. The callback tracks admitted and current NOTE_ON/NOTE_OFF/PARAMETER intent in fixed 256-note/256-parameter views. Post-ack note edges are bounded; repeated parameter movements coalesce to the latest value. An unrepresentable note or event history terminates with recovery-capacity fault 8, without truncation. The callback performs fixed-size work, atomics, queue operations and monotonic reads only; it does not wait for the worker, allocate, log, or perform lifecycle/I/O.

After an in-flight call returns, the worker suppresses its obsolete result, discards old requests from the consumer side, and sends STOP/START on the same Session and plug-in instance. The callback discards old results, clears retained audio and returned events, advances the epoch, sends old-note releases followed by currently held notes and latest parameters, and withholds presentation through re-priming. A second late incident is fault 7 (`recovery_exhausted`); a missed worker-return deadline is fault 6 (`recovery_timeout`); failed/conflicting lifecycle work is fault 9. No `ap6_recover()` or old opaque snapshot is used. STOP/close after a completed recovery use the existing lifecycle.

## Source-owned validation

- Mode A: mapped/TCP fake peer delayed one process call while the callback received an old note-off, new held note and parameter updates. It resumed with old note inactive, new note active, latest parameter 0.9, finite nonzero stereo sample 0.72, exactly one new epoch, and normal STOP/DEACTIVATE/CLOSE.
- Mode B: worker held past the 100 ms deadline, then released for containment. Fault 6 was observed, with admission closed and no queue-overflow fault or second recovery.
- Mode C: a second delay after successful recovery produced fault 7, without a second epoch or queue growth.
- Focused checks also cover explicit ledger-capacity refusal, coalesced pending parameters, and discarding one old result plus pending returned state before the new epoch.
- macOS backend suite with `--test-threads=1`: 82 passed, 1 fixture-dependent test ignored. macOS native event suite: 12 passed. macOS standalone appliance library suite: 28 passed. The unchanged broad parallel backend run can contend for the global test registry and returned two `Full`/`Busy` test setup failures; both pass in the serial suite. No base-equivalence claim is made for that parallel contention.

## Pi smoke

The final smoke ran only the source-owned finite-stall mapped transport fixture on the Pi 5. Its callbacks were paced at 256 frames per 48 kHz block, with a 2048-frame presentation reserve. JACK remained at 48 kHz/512 frames; no plug-in, Wine, FEX, priority, affinity or device setting changed. The staged `queued.rs` SHA-256 was `2bd058464e4ec93f6fc8b7790cd3a66f2fdafdaeaab9236f87dd7ddda6788f81`; the source-owned fixture SHA-256 was `acfe843cd7e04043c5c27ddcd9684bee26b756e4ab8ed0dfb4e28fba1c5ed99a`. Both hashes matched the local files.

The paced run passed: epoch 1 -> 2, one recovery, historical request high-water 9 of 2048, 9 old requests discarded, 0 queued old results to discard, 17.192 ms from recovery request to worker return, and 49.997 ms from worker return to first resumed current stereo audio in the paced fixture. The old stalled completion was not published. Normal source-owned retirement completed. JACK had system-only ports before and after, with no leftover fixture process. A post-run device read was 47.7 C and `get_throttled=0x0`.

The mapped peer and paced callback exercise the actual queued worker, epoch lifecycle and audio output on ARM; they do not establish JACK callback deadline behavior, analog latency, a vendor preset transition, or DSP capacity. This slice stops before another Serum session.

Two earlier short source-owned Pi runs informed the fixture: the first advanced callbacks faster than real time and was excluded from timing evidence; the next paced run passed before the final discard-count accounting correction. The numbers above are from the exact staged source after that correction. No commercial plug-in ran in any of these checks.
