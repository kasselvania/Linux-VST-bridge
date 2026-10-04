# Compatibility platform and beta delivery roadmap

Selected by the operator on 2026-10-02; engineering method and whole-platform
assessment completed on 2026-10-03. The goal remains a portable, managed Windows
plug-in platform with dependable audio, ordinary DAW reconfiguration and proven
low latency, delivered as a self-service beta. Portability, graphics and recovery
inform the architecture throughout implementation.

## Current position

The [platform assessment](PLATFORM_ARCHITECTURE_REVIEW.md) is complete for source
`01590fa05f99c71b141664becd7d9fadf7fa94a7`. It traces all nine platform capabilities,
classifies what to retain/repair/replace/retire, and specifies shared contracts and
integrated completion. The implementation has useful foundations but does not yet
follow those contracts across the whole product. Completing the assessment does not
complete dependable audio, platform qualification or a beta release.

Retain the reusable engine and validated descriptors, installed state-update/
failure-cleanup repair, note-release repair and demonstrated buffered catch-up
correction. Keep their evidence scoped. Earlier usable builds are comparison
artifacts, not a proved gap-free baseline. [PR #200](https://github.com/kasselvania/Linux-VST-bridge/pull/200)
remains frozen and draft; [PR #204](https://github.com/kasselvania/Linux-VST-bridge/pull/204)
remains draft and unmerged. Do not promote their inherited integration wholesale.

[CURRENT_SLICE.md](../CURRENT_SLICE.md) is the current task pointer.
[Architecture section 18](ARCHITECTURE.md#18-audio-recovery-and-portable-execution)
and D-028 own the design. The assessment holds source findings and implementation
criteria. Existing failure cards and SUPPORT_MATRIX.md hold observed support;
this roadmap creates no parallel registry or additional approval cycle.

## Implementation order

| Order | Capability | Complete when |
| --- | --- | --- |
| 1 | One managed configuration and host eligibility | Preparation, normal UI, admission and launch use the same resolved choices. Both unfamiliar and existing plug-ins reach actual use, meaningful save/reopen, configuration trial and keep/restore. Native/sandbox host checks are capability-based; operational readiness and qualification are separate. Affected-owner maintenance and resource reservations preserve independent siblings. |
| 2a | General runtime and dependency operations | The same workflow acquires/selects/imports coherent runtimes through implemented adapters and installs dependencies into the selected environment. Typed choices show reasons, scope and effective readback. Cancellation/refusal and predecessor recovery work without a new plug-in/runner name allowlist or maintainer administration. |
| 2b | Coherent DAW execution | Both SDK edges, delivery wakeups, real-time/offline completion, legal reconfiguration, parameter/event semantics and editor/control failure policy satisfy representative contracts. Whole-callback and serial-chain timing are measured. Actual signal and state survive interactions; same-callback delivery earns the declared 256/128/64-frame gates with truthful latency. |
| 3 | Effective rendering and resource configuration | Actual editor observations, typed rendering/DPI/fallback choices and measured worker/resource policy integrate with the same settings/restore path. Combined rendering, automation and audio pass; independent probes or requested acceleration are not presented as effective performance. |
| 4 | Integrated beta delivery | One frozen component roster passes the complete clean/populated-system musician workflow and failure recovery on the agreed targets/catalogue, with signing and distribution obligations complete. |

The first implementation endpoint is **prepare, explain, run and restore an
unfamiliar plug-in through one managed configuration**. It is not merely a data-model
refactor. Reviewable commits must connect the owners needed for that endpoint.
The [assessment's programme](PLATFORM_ARCHITECTURE_REVIEW.md#implementation-programme-and-completion)
provides concrete source owners and representative acceptance cases.

Orders 2a and 2b depend on the shared configuration contract and can develop
independently. This is a dependency relation, not permission to run competing
physical workloads. Do not defer audio until every advanced control exists, or
portability until one vendor passes. Editor/rendering requirements inform orders
1 and 2; order 3 supplies effective-path and combined-workload qualification.

## Evidence and acceptance

### Review follow-through for the audio-completion work

The operator retained the implementation order above after the #207 review.
The following work closes specific omissions without opening another workstream:

- Trace the remaining 50-microsecond state/start acknowledgement waits in
  `queued.rs` and the reply wait in `mailbox.rs` from the installed IPC 15 path.
  Convert reachable polling that contradicts the completion contract; record
  deliberately legacy-only paths with their callers and scope. Their presence
  alone is not evidence that they caused an observed audio failure.
  The 2026-10-03 caller audit finds `queued::control` still reachable for IPC 15
  setup, activation and state operations; replacing that acknowledgement poll
  remains required by the selected control-wakeup contract. `wait_started` is
  compiled only with `rpi0` and used by the Pi standalone start/restart path.
  `mailbox::receive_while_into` is the explicitly minor<15 reply fallback; IPC 15
  uses `receive_notified_into`. The control-handoff sleep is likewise the branch
  without a notification channel. These legacy branches remain intentional,
  bounded compatibility paths, not evidence for the installed IPC 15 audio claim.
- Format the Windows mapped-processing implementation for a readable thread and
  ownership review before final installed/Deck qualification. Keep a formatting
  change distinct from behavioral repair, preserve the current frozen comparison,
  and rebuild/verify any successor used for acceptance.
  Completed separately at `12ee4383`: all 13,371 tokens/comments and 32
  preprocessor records are unchanged, with no lines over 160 characters. The
  installed audio2 successor includes this formatting and the separately reviewed
  prepared-render repair; its functional results do not establish timing acceptance.
- Make an explicit landing decision for the inherited #200–#207 stack before
  integrating it into trunk: reviewed bottom-up landing or splitting #200 and
  rebasing the retained capabilities. Record the chosen commit/dependency map,
  retained evidence and required integrated checks. Neither adding another layer
  nor passing this audio slice decides that question implicitly. No stack merge
  is authorized by this note.

Use shared contract fixtures for unfamiliar classes, state/parameter evolution,
varied buses/events, actual blocks below maximum, zero-frame handling, callback
bursts, slow valid offline work, legal lifecycle and connection orders, demanding
editors, operation refusal and abrupt owner loss. Test independent/shared environments
and native/sandbox host access. Select representative combinations deliberately;
a Cartesian explosion of fixtures is not the strategy.

Preserve exact source, component, runtime and publication identities, working
comparisons, original projects and vendor authorization. Keep observation validity,
source correctness, installed behavior and platform/release claims distinct. Use
controlled comparisons for attribution and combined workflows for acceptance.
Do not change runtime, graphics, priority, buffer and delivery model together in a
causal comparison. Invalid measurement cannot decide bridge correctness.

[Integrated beta delivery](INTEGRATED_BETA_DELIVERY.md#acceptance-method) retains the
full release requirements: agreed SteamOS/Ubuntu/CachyOS targets and catalogue,
installation/authorization, actual audio, editor/automation interaction, meaningful
save/reopen and reboot, recovery, populated update and rollback. Its endurance gate
is a 30-minute interaction test followed by a two-hour soak on the same frozen
artifact at the declared lowest live-use block. Short reference runs, a clean
subsection, increased bridge delay or package-container success cannot replace it.
A missing target is an unperformed test, not supported-platform evidence.

## Preserved results and machine custody

The [earlier audio assessment](AUDIO_RECOVERY_REVIEW.md) and
[graphics assessment](GRAPHICS_RUNTIME_REVIEW.md) retain their specific findings.
The [installed state-update result](../evidence/preparation/2026-10-03-state-update-installed.json)
proves bounded reference migration, failure cleanup and predecessor restoration;
it does not establish commercial endurance or low latency.

The [general5 Deck installation](../evidence/audio-recovery/2026-10-03-general5-deck-installation.json)
passed installation and short changed-state recall, then failed during interaction
with a terminal instance error. Missing status was initially misreported as lost
audio; final queued-underrun counters were zero. The trigger remains unattributed
and the soak never began. Source-level editor failure propagation is an architectural
finding, not a demonstrated explanation of this incident.

Physical testing remains stopped. This assessment does not launch a new Deck,
runtime, vendor, ARM or Windows-DAW campaign. Existing independent workstreams keep
their authority and machine custody. Builders/VMs used by the stopped run are off.
When build/test work resumes, use one VM or builder at a time, at most two CPUs,
and 256 processes on the declared fixture; reserve host CPU 0–1 and capacity for
Audiobookshelf. The operator authorized the controlled comparison of 3 GiB guest /
4 GiB container with 6 GiB guest / 8 GiB container on 2026-10-03. Combined outer
memory/swap equals the respective container limit; the builder remains at 4 GiB.
The [completed short comparison](../evidence/preparation/2026-10-03-controlled-capacity-observation.json)
had clean reference audio at both sizes and less guest memory pressure in the
larger run. It does not close the original failure or the interaction/soak gate.

Commit and push complete, reviewable capability increments. Report unfinished
integration explicitly. Reducing agreed platform, catalogue or latency goals
requires a product scope decision; selecting ordinary engineering details does not.
