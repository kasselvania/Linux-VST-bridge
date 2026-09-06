# AP4 attempt preserved — incomplete

> Historical checkpoint, preserved as recorded. The later authorized continuation fixed the desktop lifetime/binding defects and completed a focused save/reopen check: see [current AP4 result](AP4_RESULT.md). The old counts, labels and continuation requirements below describe this checkpoint; they are not the current task instructions.

The operator requested that this attempt be preserved on GitHub and work stop. This is an implementation checkpoint, not completed AP4 acceptance. Keep issue #57 open and the PR draft/unmerged. AP3 remains the accepted frontier; all earlier accepted evidence and consumed reservations remain unchanged.

## What actually worked

At executable source `cdadb96fda11f7fa8eb4862ad381f8487074b2a3` (tree `926869b528fdb03e51e8990494ca8d46ee0afce0`), full diagnostic D9 completed all three SDK and all three Bitwig sessions. It compared **20,284,928 actual returned samples with maximum absolute error 0**, at 48 kHz, float32 stereo and 1024-sample declared delay. Actual Windows AGain state retained all 12 bytes. Captured quarter-gain and mute payloads matched on fresh restore; reduction and bypass cases, parameter-only mute and independent restored-audio checks passed.

Bitwig saved gain 0.25, quit normally, reopened with fresh native/Windows sessions, showed 0.25 and played before edits. It then saved mute, quit/reopened again, showed 0 before edits, and returned output after a deliberate edit to 0.5. The last edit was discarded normally, preserving the saved mute project. Native and Windows state records corroborate the GUI observations. This remains a non-authoritative diagnostic, never promoted into acceptance.

| Diagnostic segment | Samples compared | Maximum error |
| --- | ---: | ---: |
| SDK capture | 2,918,912 | 0 |
| SDK fresh gain | 116,480 | 0 |
| SDK fresh mute | 116,480 | 0 |
| Bitwig first save | 7,531,008 | 0 |
| Bitwig gain recall | 4,800,512 | 0 |
| Bitwig mute recall | 4,801,536 | 0 |

D9 reservation: `08504f908295669f054888e5f586aae8b77f7f5dd6615763bfae09a0efda79fd`; classified result SHA-256: `73edc653be9a4559ffd584476933d7aac076441bb7b5c7c4ddd24d8b9e5b47aa`.

## Why acceptance did not complete

A2 ran fresh at the same unchanged source. Its three SDK stages compared **3,151,872 samples with maximum error 0**. During the first Bitwig stage, Sunshine initially failed to provide video; restarting its user service recovered control. The required Save/Quit sequence was not completed within the existing 180-second post-gate window.

The retained classification is `stage_timeout`, Windows raw exit `-15`. The last Windows lifecycle record is `ap4_state_readback`, with no in-flight plug-in call at timeout. Native records show initial real mute restoration and a real 0.25 snapshot, followed by `ap4_native_error`: operation `get`, stage `failed_instance`, fault 3, first position 8,632,832, 33,722 processed blocks, and `ConnectionReset: Connection reset by peer (os error 104)`. The checkpoint recorded 1,463 callback rejections. Bitwig displayed an error saving the VST3 state; the agent declined to publish an incomplete save. It began normal Quit, but the turn was interrupted at the discard dialog; the existing supervisor subsequently completed owned cleanup. Do not claim normal acceptance shutdown or completed acceptance recall.

This establishes failure of the complete timed acceptance run. It does not establish that every state save is broken, nor that the prior successful diagnostic constitutes acceptance. No fresh GUI gain/mute reopen occurred in A2. No product evidence packet was rendered.

A2 candidate: `20d2280bc5e62b824d28fcc153d89987690ef929cf76c21037af35c27df33da2`; reservation: `886ecd09f1ece24c0e94b95762ec47f73431be458fcd95492fb68d0348530de4`; failure SHA-256: `27a682101cd96c66d707f350eb4ec8856ba632db8b3195e7d85925e955e02f02`. The failure receipt's acceptance-eligible field identifies its execution class, not a passing or accepted claim.

## Repair and validation retained

The branch implements actual Windows component state through standard native VST3 methods, reference-controller synchronization, coherent save barriers and bounded opaque state transport using the existing execution/supervision path. It retains original errors before reporting/cleanup and fixes float32 log comparisons and bounded GUI-note timing. The final repair gives native Bitwig desktop-only launch variables and a home working directory, while preserving exact absolute stage/protocol bindings, and recognizes allocator failures in bounded diagnostic excerpts. Neither retained binary was rebuilt for these last launcher/retention changes. The latest focused AP4/AP3 execution and diagnostic-runtime suites passed 38 tests, including the actual worker failure and cleanup paths.

Earlier D8 aborted inside Bitwig's JVM allocator during startup; the initiating cause remains unknown. The operator demonstrated that the same Bitwig Flatpak starts normally from the Applications list. After the launcher correction, D9 completed; this supports the corrected path but does not isolate a definitive cause of the earlier abort.

## Cleanup, retained material and allowance

Final readback: zero Bitwig processes, owned process groups empty, disposable stage absent, temporary native publication removed, original preferences restored, protected state unchanged. The saved mute project remains private on the Deck, unchanged by failed A2, SHA-256 `9ec2a12acb631e201335408ef975dd54b53b36c65859dc70ca426e7a7dafd7c5`. Sunshine remains a separate supporting development service, not an owned plug-in process.

Cumulative consumption: **9/10 diagnostic batches; 2/2 acceptance candidates; 2/6 Windows producer attempts**. One diagnostic remains, but no acceptance candidate remains. No further live work is scheduled. Preserve all historical receipts, counts and artifact bindings. Windows artifact `9978188887` and native manifest `0add784f5f8c996768909792243f7a18ea8d9e08bf148db166fb57d9455fb017` remain reusable when their relevant inputs are unchanged.

Private Mac receipts are under `~/Library/Application Support/Linux VST Bridge/proof/classified-proof/`: D9 under `diagnostic/db719088048bf8da92c0060a4bb852a82945e515a345a369b1c2ab06ece686f5/transactions/<D9 reservation>/`; A2 under `acceptance/<A2 candidate>/transactions/<A2 reservation>/`. Private project files, credentials, addresses, binaries and full logs are not in this PR.

## Connecting and launching Bitwig

Keep Tailscale connected on the Mac and the Deck awake in its logged-in Desktop Mode session. Open Mac Moonlight, select **steamdeck → Desktop**; Sunshine on the Deck supplies video/input. SSH is the independent command/recovery path. If Sunshine is stopped or the stream fails, restart only `app-dev.lizardbyte.app.Sunshine.service` through SSH. The operator authorized a persistent Sunshine-only KDE capture grant; no physical Share prompt was needed after that grant. Reconnect reliability remains imperfect. See [desktop setup and rollback](DECK_REMOTE_DESKTOP.md).

For ordinary manual use, launch **Bitwig Studio from the Deck Applications list**. Do not reuse the old ad-hoc test launch command. The classified verification runner is a separate controlled launch path: its corrected CLI completed D9, so a universal claim that Bitwig cannot be launched from a CLI would be false. GUI actions remain through Moonlight; never use a streamed terminal.

## Concrete continuation

Before another acceptance attempt, make GUI readiness reliable before starting the timed Bitwig workload; verify that locally and through read-only desktop checks. Preserve the existing timeout and failure behavior rather than hiding an expired endpoint behind a successful Save. A new acceptance run requires an explicit additional allowance because the selected two candidates are exhausted. Do not relaunch either consumed candidate or relabel D9. This checkpoint is left for review, with AP4 incomplete.
