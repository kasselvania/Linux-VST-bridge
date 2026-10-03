# Compatibility platform and beta delivery roadmap

Selected by the operator on 2026-10-02; engineering method corrected on 2026-10-03.
The goal is a portable, managed Windows
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

## Capabilities and completion

These rows organize related capabilities, not a rule to postpone portability,
graphics or configuration design until one plug-in stops failing. Plan across
their shared contracts under [AGENTS.md](../AGENTS.md), then implement reviewable
increments in dependency order. A fixture failure informs that design; it does
not automatically become the next product assignment.

| Capability | Work | Finished when |
| --- | --- | --- |
| 1. Platform architecture and beta contract | Evaluate the complete compatibility workflow across machine/runtime assessment, preparation, configuration, DAW processing, graphics, state, supervision and recovery. Classify components to retain, repair, replace or retire. | Shared contracts, architectural gaps and a coherent implementation plan cover representative unfamiliar plug-ins, platform capabilities, normal use and failure recovery; a list of isolated repairs is insufficient. |
| 2. Dependable DAW execution | Implement delivery and lifecycle contracts across admission, worker/control scheduling, Windows processing, presentation and editor interaction; distinguish deadline, host-input, vendor and observer failures. | Representative instrument/effect/chain workloads meet declared audio, state, interaction and endurance criteria on the frozen artifact; exact platform coverage and remaining gaps are recorded. |
| 3. Portable runtime selection | Observe host capabilities, match explicit plug-in requirements to pinned runtime policies, verify effective settings, and exercise reference workloads on Ubuntu and CachyOS. | The same product logic prepares declared targets without maintainer environment fixes; physical audio and graphics claims have hardware evidence. |
| 4. Reconfiguration and lower latency | Support legal repeated setup changes without republishing; qualify 256, 128 and 64 frames, irregular blocks, callback bursts and offline processing; evaluate bounded same-callback delivery. | Every claimed configuration has output, latency, timing and lifecycle evidence. Queued mode remains only if independently useful and accurately described. |
| 5. Self-service delivery | Integrate reviewed runtime, reusable proxy engine, installer, compatibility experiments, recovery, update and rollback around the verified audio path; finish ordinary manager behavior and distribution preparation. | Users complete install through recall and recovery, including unfamiliar plug-in preparation, using ordinary controls without compilers, SSH or mandatory manual runtime administration. |
| 6. Frozen beta qualification | Test one coherent release candidate, with declared platform builds, across the agreed catalogue and complete musician workflow. | Installation, authorization, audio, editors, automation, meaningful recall, reboot, recovery, populated update and rollback pass; release artifacts and distribution obligations are complete. |

The earlier [source review](AUDIO_RECOVERY_REVIEW.md) remains useful evidence.
It does not complete the broader platform assessment requested on 2026-10-03.
Architecture assessment is reopened under the rewritten engineering method; the
other capability and beta-completion gates remain open.
[CURRENT_SLICE.md](../CURRENT_SLICE.md) remains the active task pointer. [Architecture section 18](ARCHITECTURE.md#18-audio-recovery-and-portable-execution)
owns the production decisions; [Integrated beta delivery](INTEGRATED_BETA_DELIVERY.md#acceptance-method)
owns the current acceptance criteria. The roadmap does not create a parallel
decision or support registry.

The operator's subsequent preparation correction is part of this same roadmap:
[architecture section 18.6](ARCHITECTURE.md#186-general-preparation-and-compatibility-experimentation)
connects unfamiliar plug-ins, capability assessment, recommended settings,
advanced runtime/dependency experiments and native publication. Stages 3 and 5
must use that shared configuration path. A fixed prebuilt plug-in catalogue is
not the product boundary, and a better refusal message does not complete setup.
The reusable engine and validated metadata now have bounded installed reference
results. They remove the per-plug-in compilation restriction, but do not by
themselves complete assessment, configuration or the ordinary recovery journey.
Alternate runtimes and dependency trials extend the same path; they are explicit acceptance cases, not implied by
graphics-probe success. Dependable audio and physical qualification remain open.

The shared graphics assessment now has one explicit manager path for dependency
hints, editor-time library observations and selected-runtime capability probes.
It reuses inspection ownership and reports exact identities without selecting
policies by plug-in or distro name. Runtime-probe success remains distinct from
the actual editor's rendering device and musical qualification; see
[the implementation scope](GRAPHICS_RUNTIME_REVIEW.md#shared-assessment-implementation).

## Execution

For each change, trace the user action through installation, discovery,
assessment, prepared configuration, publication, processing/editor use, recall
and recovery. Identify the owners affected and the existing mechanism to reuse
or replace before editing one component. Include the unfamiliar-plug-in case,
an existing working installation and an ordinary update in that reasoning.
Select a vertical comparison that reaches the user's result; a passing parser,
probe or package build cannot stand in for that result. Keep unimplemented
connections explicit in CURRENT_SLICE.md so the operator does not have to
rediscover and assemble them between tasks.

Preserve exact source, build, runtime and publication identities. Keep the working
comparison available and protect original projects and vendor authorization.
Use controlled comparisons for causal attribution and combined workflows for
product qualification. Select both from the shared capability contract. Investigate
across the entire relevant path and change its architecture when required; neither
one narrow repair nor one broad soak substitutes for that engineering work.

Capability probes and reference checks for stage 3 can progress independently of
stage 2, but a timing comparison must not change runtime, graphics, priority,
buffer size and delivery model together. Use the existing SDK consumers and
qualified recorder. A failed run remains failed; useful sub-results retain their
original scope.

The operator requested a separate graphics/editor/runtime investigation while
using the Deck on 2026-10-02. Its [source review](GRAPHICS_RUNTIME_REVIEW.md)
separates effective rendering capability, editor/controller failure isolation
and graphics performance from their final coexistence test with audio. Local
source checks passed. The subsequently authorized work adds requested/observed
graphics reporting, optional native diagnostics and controlled owner-exception
retirement with SDK fixtures. [Implementation results](GRAPHICS_RUNTIME_REVIEW.md#earlier-implementation-readback-and-failure-containment)
remain separate from selected-runtime renderer observation and physical
coexistence tests. That source assessment did not change the Deck or qualify a
graphics path. The later [general5 Deck installation](../evidence/audio-recovery/2026-10-03-general5-deck-installation.json)
passed short recall checks but ended its interaction run in terminal instance
failure. Its status-file disappearance was initially misreported as missing audio;
terminal records had zero queued underruns. The trigger remains unattributed,
and no new hardware experiment is selected by the instruction reset.

Complete, review, commit and push coherent capability increments.
A small patch or fixture result does not close unfinished shared integration.
Keep the failure cards and support matrix consistent with their evidence. Do not launch unrelated ARM, Windows DAW,
catalogue expansion or manager redesign work from this roadmap. Existing separate
workstreams retain their own authority.

The resource limit remains one VM or builder at a time, with CPU and memory
headroom for Audiobookshelf. A missing physical target is an unperformed test,
not permission to advertise its support. Reducing the agreed platform, catalogue
or latency goal requires an explicit product scope decision.
