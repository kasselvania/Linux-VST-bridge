# AP6 — Recover one failed Windows instance

Issue #62 extends accepted AP5 from `4cc7ac48bd7dd1f5855d73254bad1cdc11495ad9`. The focused desktop recovery and subsequent project recall passed. One deliberately terminated Windows AGain instance recovered its confirmed complete state and correct processing while its sibling kept running in the same native Bitwig host. This is development evidence for review, not a general reliability claim.

## Implementation and use

Open the proxy's plug-in window in Bitwig. The small **Bridge instance recovery** view displays the confirmed snapshot's revision, capture/restore provenance and full envelope SHA-256, the controller gain and status. **Recover this failed instance** is an explicit action. The window warns that later edits and audio tails are not recovered. It is a native SDK/X11 control using the host's `IRunLoop`, not the Windows plug-in editor. Bitwig's generic parameter panel did not expose the non-automatable recovery parameter, so that panel is not the action surface.

The Rust backend retains confirmed complete component-state envelopes per logical instance, outside the failing connection. Confirmation requires a successful real getState or setState with exact readback. Recovery storage treats the bytes as opaque; the existing reference-specific envelope validation and test oracle remain separate. No gain value is used to construct replacement state. A failed later capture preserves the earlier confirmed snapshot without returning it as a successful save.

Recovery requires a failed instance and the explicitly selected revision. It excludes that instance's callback leases, waits for its worker and requires the private owner's positive confirmation that containment and environment retirement completed. EOF, missing acknowledgement or uncertain cleanup cannot authorize replacement. Reporting success does not decide retirement. The owner retains its AP5/R1 admission and error-isolation rules.

A fresh endpoint, wire session, generation and queues replace only the failed transport. Real Windows setState/readback must succeed; then controller `setComponentState` and the host parameter-cache notification must be acknowledged before activation and processing resume. Missing, changed or invalid snapshots and failed restoration stay failed. No automatic retry, default substitution, queued-audio replay or sibling restart occurs. A failed replacement attempt remains blocked for that logical instance.

All restart/state work is outside audio callbacks. The callback remains bounded and silent during failure/recovery, with fresh 1024-sample restart alignment. Snapshot metadata alone no longer invalidates host controls or marks a saved project dirty. The actual restore still synchronizes them.

The terminal worker records the first fault and bounded native I/O explanation before cleanup. Native records now live outside the disposable Windows stage, so later SDK status and termination records survive retirement. The existing launcher, state transport and Windows binary are reused. There is no new broker or special DAW launch.

## Focused verification

At final implementation `428870b3345177e722548a144558f1a516d5dc9a`:

- 21 Rust backend tests and Clippy with `-D warnings` passed.
- 12 private-owner tests passed, including complete/incomplete containment with reporting failures, retirement failure and healthy-sibling admission isolation.
- Five SDK-loaded Linux cases passed: complete recovery, a deliberately late response, missing state, bad restore readback and unconfirmed retirement. These use substituted Windows peers, not actual Windows DSP.
- Two concurrent audio threads exercise one shared native host. The complete-state case stores non-gain state, makes an uncaptured gain edit, fails the endpoint, and verifies the full original bytes, controller and host values, fresh delay alignment and exact output after recovery. The sibling continues. All callback-audit effects and checked audio errors were zero. Negative cases did not resume processing or return a successful save.
- The loaded cases also check distinct parameter IDs, view creation/unattached cleanup, and that snapshot metadata does not invalidate host control values. Earlier loaded AP5 isolation/failure and AP4 state checks passed on `6222f3c`; their valid results were retained.

The final native manifest is `716e9beb6b637f9ea51014d045b647853df9c43648f2d69e3291e926ed9d50b4`, queued binary SHA-256 `7fb528083f29fc634c676f6a887b500a16558a2e38c13b39408c4dfb06ec257b`. The Linux build used the retained VST3 SDK and Flatpak SDK runtime. The native view adds Xlib at the SDK boundary.

## Actual desktop recovery and recall

Bitwig Studio 6.1 was launched from **Applications → Multimedia → Bitwig Studio** on the maintainer's Steam Deck. Hosting was **Together**; both copies mapped into one native plug-in host. Neither Bitwig nor its native host had `LVB_` environment keys. The owner ran source `079f6baf0bf861c67d6512b2a443114bd9996c75`, using the unchanged AP4 Windows host manifest `f00325df1cefe08c4496d3007211d7ecebdd02a49d4dc42a6a2e0a94881ec615` under the prepared Proton 11.0-2c / Steam Linux Runtime 4 fixture.

The existing AP5 two-track project supplied distinct stereo inputs and saved gains 0.6750 / 0.3500. Both recalled controls and actual Windows output were checked before fault injection. Only A's owned Windows endpoint was killed, after checking its process start identity and mapping to its owned stage. The first fault retained `ConnectionReset: Connection reset by peer (os error 104)`. A became silent; B continued correct processing.

Clicking A's recovery button restored **snapshot #3**, originally confirmed by component restore, envelope SHA-256 `ef796d3d393a3c14ea0ade6ab181bf8b3ce0baf175cb1ed683dd35c53aaff43d`. Its complete Windows payload retained SHA-256 `2346a6c4bf2fb57fd9cc3e217a3f903b6548614fdb31172d80119585290a9d64`. A resumed actual 0.6750 processing without any setting re-entry. Its endpoint/session changed and generation advanced from 1 to 2. Bitwig, the shared native host, and B's Windows endpoint all retained their process start identities. B's state stayed at SHA-256 `c4462717ee17596492abf5ccf5b4d662ddf2f5ba6eda68e2f22736a8dbd562a6` / 0.3500.

That recovery ran on native source `5791256dc0e14237c008f85847cf380605d840bf`, manifest `8c6a35ddd05bf8774ef52320aeb227099f2657031eb162765b0bf2a6254e6a41`. Subsequent code removed the unnecessary snapshot-metadata notification and added its regression and view-lifetime checks. The recovery implementation and actual restore notification were preserved. The final build passed the focused loaded recovery cases and the following normal desktop reopen; the successful Windows fault/recovery was not replayed under another label.

The recovered project was saved separately as `AP6-Recovered.bwproject` in the existing two-track fixture folder, then Bitwig closed. After a fresh Applications launch on the final build, both complete payload hashes, displayed controls and actual processing matched before any edits. Normal Save completed and quit required no extra save prompt. The saved/reopened file SHA-256 is `1849faf11a683e83f52e0ca07f64ef3b4e91c2aff852f67dd660d925d49e8231`. This later Save reused Bitwig's cached project state; it is not counted as a new Windows capture.

The existing transport-worker witness compared every returned audio sample in these intervals:

| Interval | Compared samples | Nonzero samples | Maximum error | Gain edits |
| --- | ---: | ---: | ---: | ---: |
| Recovered A, generation 2 | 11,792,384 | 5,222,930 | 0 | 0 |
| Healthy B across A's failure/recovery | 12,438,528 | 11,863,561 | 0 | 0 |
| Final-build reopened A | 5,105,664 | 1,972,416 | 0 | 0 |
| Final-build reopened B | 3,945,984 | 1,974,394 | 0 | 0 |

Supplemental read-only samples of completed guarded audio planes also matched before the kill, for B during failure/replacement, after recovery and after reopening. Those selected matching snapshots are not protocol acknowledgements; the exhaustive comparison is the worker witness above. A's logical lifetime recorded 8,316 rejected/silent callbacks while failed or recovering. B and both reopened instances recorded zero callback rejections and no queue faults. The replacement endpoint itself had no queue fault.

All five Windows sessions were contained and retired, including the deliberately failed one. Replacement, sibling and reopened Windows hosts exited zero. The original AP5 and accepted AP4 project bytes stayed unchanged. The saved AP6 project and private artifacts remain available. Test publication was removed, original preferences and metadata cache restored byte-for-byte, the owner stopped, its socket disappeared, and no test Windows/native/Bitwig processes or stages remained. Sunshine stayed active.

## Failures and remaining limits

During initial setup, one idle-processing instance encountered native queue underflow (`fault=1`) before fault injection. Its first fault and later I/O/cancellation detail were retained, its sibling survived, and cleanup completed. No root scheduling cause was established. The targeted substituted-peer late-response case also produces underflow and confirms first-fault retention. **Neither result explains the historical AP5 loss**, whose retained Windows `control disconnected/IO` lacked the useful native explanation. That loss remains unresolved; no indefinite soak or historical replay was performed.

Recovery restores the last confirmed complete snapshot, not necessarily the most recent edit or a new capture on every Bitwig Save. The UI reports its identity and warns about uncaptured changes; it does not restore lost audio/tails. Storage remains in the live native component, not a separate durable recovery service. Replacement startup is visibly slow and synchronous on the SDK owner thread: control interactions may wait, although the healthy sibling's audio continued. There is no automatic retry after failed replacement or uncertain containment.

This remains a private, prepared AGain preview: float32 stereo, 48 kHz, up to 256-frame callbacks, 1024 samples added latency, four native slots and four owner sessions, with actual desktop evidence for two. It requires the prepared runtime/artifacts and existing private owner. X11 embedding and the host SDK run loop are required for the recovery view. Commercial plug-ins, Windows editor integration, instruments, reboot persistence, packaging and low-latency/reliability qualification remain unclaimed.

[Sanitized evidence](../evidence/ap6-instance-recovery/result.json) retains state, audio, faults, cleanup, artifact identities and private raw-record hashes. [AP5](AP5_RESULT.md), [AP4](AP4_RESULT.md), and [original D9/A2 provenance](AP4_ATTEMPT_STATUS.md) remain unchanged. No consumed candidate, acceptance campaign or full historical test sequence was replayed.
