# AP7 result — diagnostic starvation fixed; playback outcome incomplete

AP7 is **not complete**. A forced diagnostic-reader pause reproduced an avoidable underflow in the native worker; that dependency is fixed. The scoped Bitwig check nevertheless encountered another underrun. Its cause is not established, and neither the retained AP6 event nor AP5's earlier transport loss is relabelled as explained.

## Demonstrated cause and change

The worker acquired the sample-observation mutex before taking its next queued audio request. A diagnostic reader descheduled while holding that mutex could therefore prevent otherwise available audio work from reaching the peer. The regression holds that lock through the callback needing the first delayed output: the original blocking publication returns `Err(2)`; nonblocking publication returns the exact expected samples, records one skipped observation, and leaves the audio instance healthy. A later publication can refresh the cumulative diagnostic snapshot.

Only diagnostic publication becomes optional. Audio correlation, genuine underflow/disconnect detection, failure silence, Windows processing/state, owned cleanup and explicit AP6 recovery remain enforced. Queue capacity and the 1024-sample latency are unchanged. Added first-fault progress fields record epoch, worker operation/position and queue counters. Callback counters distinguish rejected/silenced frames, contiguous rejection intervals, successful silent callbacks and latency priming. Callback code performs no added allocation, blocking, clock query or logging.

## Focused checks

- Original forced-contention test: failed at the due callback. Repaired backend: 22 Rust tests pass, including variable-size exact output, stop/restart epoch isolation, missing output and wrong-position rejection.
- Seven preview-owner tests pass.
- The actual Linux SDK-loaded bundle passed independent instances, failure/teardown isolation, disconnect recovery and deliberately late-response recovery. Sample errors and audited callback effects were zero. Healthy siblings had zero rejections/discontinuities. The late response still produced underflow, and recovery's rejected output was counted.
- Native build source: `3438fcb099942406c219e62d60fe1ff3bc7d5807`; manifest: `c2f47288fa8409f76519fcb9fec7880a5b3138609c7f2fb7ef5616df6d9e4a72`. SDK assertions are retained at `0eff1e6`. Windows host manifest `f00325df1cefe08c4496d3007211d7ecebdd02a49d4dc42a6a2e0a94881ec615`, Proton and runtime were reused unchanged.

## Actual desktop result

Bitwig 6.1 was launched from KDE Applications through Moonlight. A disposable copy of the existing two-track project restored A=0.6750 and B=0.3500. Both Windows endpoints mapped into the same native plug-in host process. Actual mapped input/output was checked before any edit; only A's track label was changed. The sequence included playback, pause/resume and one Moonlight reconnect.

Before any deliberate endpoint kill, A had already underflowed. The retained first fault is `fault=1`, epoch 3, position 1,553,408. Worker operation 3 was handling position 1,552,384, exactly the due position after the 1024-sample delay, and the observed result counters were equal. Diagnostic publication skips were zero. Maximum completed service duration **by reporting time** was 30.474 ms; this is not a timestamped duration of that particular missing response. These facts distinguish this observation from a reported correlation error, but do not separate native descheduling, peer response delay or other service delay. They do not establish a causal link to a transport/UI transition or Moonlight.

The observed fault was used for explicit recovery instead of adding another injection. Snapshot 3, envelope SHA-256 `ef796d3d393a3c14ea0ade6ab181bf8b3ce0baf175cb1ed683dd35c53aaff43d`, restored A without entering settings. Bitwig, the native process and B remained alive. After recovery, A compared 11,686,912 samples with zero error; B compared 28,040,704 with zero error and no parameter edits.

A's complete native lifetime recorded **23,877 rejected callbacks / 6,112,512 silenced frames / one discontinuity**, including diagnosis and recovery time. B recorded zero for all three. Successful silent frames were 4,499,200 for A and 1,869,824 for B, including ordinary idle input; latency priming accounted for 4,096 and 3,072 frames respectively. Successful sample accuracy does not erase A's missing output.

The saved project reopened in another normally Applications-launched Bitwig process. A compared 15,832,576 samples and B 11,586,560, both with zero error, zero parameter edits, zero rejected callbacks and zero discontinuities. Successful silent frames were 4,261,888 and 2,138,880, including 2,048 priming frames per instance. This bounded recall check does not negate the earlier underrun.

Cleanup confirmed zero owned stages or Bitwig/native/Windows processes, stopped the AP7 owner and removed its socket/publication. Preferences and the plug-in metadata cache were restored; original AP4/AP5/AP6 project hashes match their retained values. Sunshine remains running. The disposable AP7 project, private captures and build artifacts are retained.

## Remaining engineering question

Ordinary playback can still exceed the available output lead. Separate native scheduling delay from peer-response and post-response work on the specific late request before choosing a scheduling or transport repair. The current aggregate timing cannot decide that. No speculative Windows/runtime change, buffer enlargement, automatic restart or indefinite repeat run was used to obtain a pass. AP6 snapshot-age and synchronous recovery-startup limitations remain unchanged.

[Selected records and original-capture hashes](../evidence/ap7-playback-underflow/result.json), [before](../evidence/ap7-playback-underflow/before-observer.txt), and [after](../evidence/ap7-playback-underflow/after-rust.txt) retain the actual source of each sub-result. Existing AP4/AP5/AP6 and D9 evidence is unchanged. Issue #64 remains open; this PR is left unmerged for review.
