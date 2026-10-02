# Audio recovery and beta roadmap

Selected by the operator on 2026-10-02. The goal is a portable, managed Windows
plug-in platform with dependable audio, ordinary DAW reconfiguration and proven
low latency, delivered as a self-service beta. Portability informs the architecture
from the first stage. Existing implementations must satisfy that architecture;
their age or previous proof status is not a reason to preserve them.

The original diagnostics and installed artifacts are preserved outside the
checkout. The late note-off repair has an installed physical comparison in
[PR #201](https://github.com/kasselvania/Linux-VST-bridge/pull/201). Intermittent
audio gaps remain open. Earlier usable builds are comparison artifacts, not a
gap-free baseline. [PR #200](https://github.com/kasselvania/Linux-VST-bridge/pull/200)
remains a frozen draft; this roadmap does not authorize promoting it wholesale.

## Stages and completion

| Stage | Work | Finished when |
| --- | --- | --- |
| 1. Production architecture and beta contract | Trace the installed path, classify components to retain, repair, replace or retire, settle ownership and configuration, and reconcile release criteria. | A source-grounded review and one current architecture and acceptance contract identify the next concrete repair. |
| 2. Dependable audio | Attribute a missing block across admission, worker, control, Windows processing, publication and presentation; repair the responsible mechanism and exercise normal interaction. | A frozen Deck artifact passes captured-output interaction and soak tests with diagnostics disabled and confirmed retirement. |
| 3. Portable runtime selection | Observe host capabilities, match explicit plug-in requirements to pinned runtime policies, verify effective settings, and exercise reference workloads on Ubuntu and CachyOS. | The same product logic prepares declared targets without maintainer environment fixes; physical audio and graphics claims have hardware evidence. |
| 4. Reconfiguration and lower latency | Support legal repeated setup changes without republishing; qualify 256, 128 and 64 frames, irregular blocks, callback bursts and offline processing; evaluate bounded same-callback delivery. | Every claimed configuration has output, latency, timing and lifecycle evidence. Queued mode remains only if independently useful and accurately described. |
| 5. Self-service delivery | Integrate reviewed runtime, prebuilt proxy, installer, recovery, update and rollback work around the verified engine; finish ordinary manager behavior and distribution preparation. | Users complete install through recall and recovery using ordinary controls, without compilers, SSH or manual runtime administration. |
| 6. Frozen beta qualification | Test one coherent release candidate, with declared platform builds, across the agreed catalogue and complete musician workflow. | Installation, authorization, audio, editors, automation, meaningful recall, reboot, recovery, populated update and rollback pass; release artifacts and distribution obligations are complete. |

Stage 1 is complete at the source-review and design level. Its
[source review](AUDIO_RECOVERY_REVIEW.md) records findings and open experiments;
stages 2 through 6 remain open. [CURRENT_SLICE.md](../CURRENT_SLICE.md) remains
the active task pointer. [Architecture section 18](ARCHITECTURE.md#18-audio-recovery-and-portable-execution)
owns the production decisions; [Integrated beta delivery](INTEGRATED_BETA_DELIVERY.md#acceptance-method)
owns the current acceptance criteria. The roadmap does not create a parallel
decision or support registry.

## Execution

Preserve exact source, build, runtime and publication identities. Keep the working
comparison available and protect original projects and vendor authorization.
Use one causal experiment at a time on the physical audio fixture; investigate
across the entire path and change its architecture when the evidence requires it.

Capability probes and reference checks for stage 3 can progress independently of
stage 2, but a timing comparison must not change runtime, graphics, priority,
buffer size and delivery model together. Use the existing SDK consumers and
qualified recorder. A failed run remains failed; useful sub-results retain their
original scope.

Complete, review, commit and push bounded jobs. Keep the failure cards and support
matrix consistent with their evidence. Do not launch unrelated ARM, Windows DAW,
catalogue expansion or manager redesign work from this roadmap. Existing separate
workstreams retain their own authority.

The resource limit remains one VM or builder at a time, with CPU and memory
headroom for Audiobookshelf. A missing physical target is an unperformed test,
not permission to advertise its support. Reducing the agreed platform, catalogue
or latency goal requires an explicit product scope decision.
