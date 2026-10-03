# Compatibility-platform architecture assessment

Assessment date: 2026-10-03. Source reviewed:
`01590fa05f99c71b141664becd7d9fadf7fa94a7`, tree
`b4625b53a7df4eb89287ad0c65e8a12d509b9d4e`, on
`codex/graphics-runtime-investigation`. Executable source is unchanged from
`9da4dc66111c500d4ca7bada5f0fad06ab19f93d`, the source used for general5;
subsequent changes are instructions, documentation and retained evidence.

This assessment traces production callers and adjacent consumers across all nine
capabilities in [AGENTS.md](../AGENTS.md). It is not a line-by-line audit of the
repository, a new benchmark, or a new hardware run. No product behavior or installed
machine was changed. [Architecture section 18.7](ARCHITECTURE.md#187-platform-execution-convergence)
records the resulting design; [the roadmap](AUDIO_RECOVERY_ROADMAP.md) orders its
implementation. Existing failure cards and support records remain the result registry.

## Decision

Retain the reusable native engine, exact process/transport ownership, stream epochs,
state migration and focused audio repairs. Change the shared configuration and
execution boundaries that still encode early fixtures. The necessary work reaches
manager policy, runtime setup, both SDK edges and the delivery mechanism. It is
larger than tuning a queue or fixing one vendor, and does not require discarding
all of the working machinery or starting an unrelated host engine.

The central gap is between the intended platform and the implemented product:
section 18 already describes broad capabilities, but production selection still
mixes exact support claims with execution configuration; advanced trials mainly
change graphics; readiness and resource policy encode particular fixtures; and
audio, editor and controller failures still have overly coarse propagation.
Documentation of the intended boundaries has outrun their integration.

## What executes today

```text
Manager / CLI / ordinary UI
  -> candidate, profile, module/class and environment selection
  -> reusable engine + validated descriptor publication
  -> Rust admission service -> Python runtime/ownership supervisor
  -> selected coherent Proton/Linux runtime -> Windows SDK host

DAW process(N)
  -> native C++ SDK adaptation and guard
  -> Rust queued request (epoch + sample position)
  -> native worker
  -> shared-memory Windows request/reply mailbox
  -> Windows render thread -> vendor DSP
  -> native completion publication -> delayed sample presentation

DAW controller/editor requests
  -> separate GUI/control transport -> Windows owner/message thread
  -> vendor controller/editor in the SAME process as vendor DSP
```

The shipping kit uses `--features registered`, recipe schema 4 and
`LVB_REUSABLE_ENGINE=ON`. `AP3_PREVIEW`/`AP8_PREVIEW` names occur on that production
build path; they are not evidence that a path is merely experimental. Conversely,
the AP9 file-based delay selector is not the installed registered selector.
See [kit production](../tools/mf3/prebuilt_kit.py),
[native build selection](../tools/mf3/native_builder.py),
[CMake](../native-vst3-proxy/CMakeLists.txt), and
[performance configuration](../native-vst3-proxy/backend/src/performance.rs).

## Shared boundaries and dispositions

| Capability | Implemented foundation to retain | Gap and treatment |
| --- | --- | --- |
| Machine and host | Bounded observations; requested/live audio facts distinguished; effective native/Windows thread scheduling readback. | Replace fixture-oriented readiness decisions with operation-specific capability requirements and host adapters. A support label must not determine whether an eligible unfamiliar setup can be attempted. |
| Runtime and dependencies | Product-owned pinned runtime acquisition, coherent runtime selection, stable environments and supervised installers. | Generalize selection/import through implemented adapters and dependency installation into the selected environment. Current specialized runner/dependency routes do not supply the promised advanced workflow. |
| Discovery and preparation | Unknown-class discovery, reusable engine, validated descriptor, stable class IDs and exact publication pairing. | Extend the actual VST3 contract where needed. A generic binary does not make its current stereo/event/controller restrictions universal. Preserve strict unsupported-capability results. |
| Configuration and advice | Candidate/history lineage, explicit graphics trials, settings carry-forward and independent bridge-delay preference. | Separate observations, resolved execution settings and support claims inside those existing owners. One resolver must serve preparation, launch, UI, update and restore. |
| DAW/audio | Preallocated transport, positioned results, epoch rejection, bounded completion notification, note-off correction and legal inactive setup machinery. | Replace polling handoffs and incomplete mode/deadline policy coherently. Validate the full SDK lifecycle, offline correctness and callback bursts; qualify same-callback delivery over existing ownership. |
| Editors and graphics | Separate owner/render threads, GUI transport, dependency hints, runtime probes and typed graphics selection. | Distinguish recoverable optional-editor/control failures from unsafe instance failure; observe actual renderer use and combined load. Shared-process vendor crashes still require containment. |
| State and change | Historical state reaches an admitted successor; authoritative current parameter readback; unchanged saved bytes; predecessor restore. | Keep this path as a regression foundation and carry its semantics through every configuration/runtime change. Changed automation IDs and commercial state compatibility remain separate claims. |
| Supervision and recovery | Exact owner generations, independent consumer-death cleanup, retirement acknowledgement, bounded cancellation and joined workers. | Reuse custody while separating operational failure, cleanup result and support state. Scope maintenance/resource policy to affected owners. Consolidate product policy currently split across Rust, Python and SDK code. |
| Product delivery | Packaged runtime/engine, installer supervision, current-state UI queries, history and ordinary predecessor restoration. | Integrate all configuration operations and failure actions into that normal workflow; qualify clean/populated systems and finish signing/distribution. Package checks are not musical qualification. |

### 1. Configuration and qualification are still entangled

[Candidate](../bridge-manager/src/preparation/model.rs) already owns selection,
inspection, profile, artifacts and predecessor/trial data. Keep that owner.
[Configuration preparation](../bridge-manager/src/preparation/configuration.rs)
currently accepts a graphics-backend difference; its carry-forward mechanism
preserves graphics settings, not a complete runtime/dependency/process configuration.
[Profiles](../bridge-manager/src/profiles.rs) mix exact support posture, executable
requirements, renderer selection and fixture-specific capability policies.

Unknown plug-ins are no longer fundamentally blocked by missing compiled proxies:
schema 4 publishes the reusable engine with new descriptor data. Review candidates
also have a qualification route. The remaining gap is a coherent ordinary path
from an unfamiliar plug-in to explainable settings, execution and recovery.
Do not regress to a verified-profile requirement for permission to try.

**Repair:** use existing candidate/history records for one resolved launch
configuration. Keep observations and support claims as separate typed inputs/views.
Explicit overrides survive advice refresh. Invalidate observations by the inputs
they depend on; do not regenerate a licensed environment or rebuild an unchanged
engine when a driver observation or support description changes. Live DAW format
is a separate prepared instance contract, joined to the launch configuration.

### 2. Portability remains partly a fixture policy

[Readiness](../bridge-manager/src/readiness.rs) recognizes an exact Deck/SteamOS/
Bitwig Flatpak combination and several Arturia identities. Its home-page path asks
for Bitwig Flatpak when that installation is absent and uses broad Flatpak host/X11
permissions as a visibility test. It records JACK availability, but its proposed
actions still rely on PipeWire/Flatpak assumptions. An unfamiliar platform can be
reported as unknown; this is not evidence that every non-Deck execution is banned.
It is evidence that the main readiness experience is not host-neutral.

[Runtime session environment](../bridge-manager/runtime/session.py) also requires
`DISPLAY` during session construction. Display needs should derive from the chosen
operation/editor route, including explicit XWayland requirements, rather than all
work inheriting the interactive fixture's assumptions.

**Replace:** operation-specific assessment of loader/architecture, selected DAW
context, reachable publication and IPC paths, audio access, display route, storage,
resource quotas and scheduling capabilities. Probe in the consuming context.
Native DAWs and sandboxed DAWs need implementations of that same contract. Keep
support qualification separate from capability eligibility and service liveness.

### 3. Proton integration needs a configuration workflow, not more named trials

[Runtime delivery](../bridge-manager/src/runtime_delivery.rs) supplies a pinned
Proton/Linux-runtime combination without requiring Steam. Preserve it.
[Runner selection](../bridge-manager/src/onboarding.rs) enumerates installed or
registered choices; [experimental runners](../bridge-manager/src/experimental_runner.rs)
still implement specialized DComp/touch variants with fixture bindings.
[Dependency setup](../bridge-manager/src/dependency_cli.rs) is a specialized Native
Access route. [Manager actions](../bridge-manager/src/operator_model.rs) expose
runtime installation and graphics assessment/trials, not the full runtime import,
selected-environment dependency and typed-override workflow in section 18.6.

**Repair and replace:** keep validated acquisition and custody; replace named
experiments as the ordinary control surface with implemented runner adapters,
typed options and scoped environment operations. Reject an unsupported adapter or
missing dependency specifically. Do not reject unfamiliar vendor/runtime names.
The planner must show affected siblings and reversibility before an environment
mutation. Runtime files, prefix identity and OS drivers are different owners.

Proton offers documented runtime/graphics options; selecting one does not supply
DAW timing or VST3 semantics. See Valve's
[runtime configuration documentation](https://github.com/ValveSoftware/Proton#runtime-config-options).
DLL import hints can suggest a check, but cannot establish the editor's effective
renderer, required dependency version, GPU benefit or audio coexistence.

### 4. The delivery model has shared problems beyond one lost block

The native [queue](../native-vst3-proxy/backend/src/queued.rs) sleeps for 50 microseconds
when its worker is idle or in certain control handoffs. The
[native mailbox](../native-vst3-proxy/backend/src/mailbox.rs) polls for the Windows
reply, and the [Windows mailbox](../windows-factory-probe/source/delivery_mailbox.h)
polls for requests. The native futex completion notification does not wake the
Windows render thread; it covers a later native-worker-to-callback boundary.
A requested sleep interval is not a worst-case wake bound.

The buffered completion allowance begins at Rust entry and is `N / Fs`, including
for offline processing. Earlier C++ work and later copying are outside that
allowance; a serial chain consumes more than one instance's budget. Timeout can
produce silence while the processing call succeeds. Sample delay cannot provide
wall-clock execution time during callback bursts. Offline output also needs validated final-block behavior
under the declared latency/tail contract, not only a longer wait inside the same loop.

**Replace the completion policy and handoffs, retain transport ownership.** Define
real-time, offline and cancellation behavior end to end. Measure full callback
duration and serial-chain behavior. Use event-driven request/reply wakeups and
qualify same-callback output with `D = 0`; retain buffered mode only for demonstrated
compatibility value. Never turn the worker's five-second containment timeout into
a callback wait. Keep queued `D >= M`, epochs, position checks and strict wire
validation until a replacement actually changes that contract.

[Processor lifecycle](../native-vst3-proxy/source/processor.cpp) already supports
inactive setup and has a nonblocking guard. Audit legal host transitions and
ownership conflicts rather than asserting reconfiguration is wholly absent or
replacing the guard with a blocking mutex. Preserve zero-frame semantics, actual
`N <= M`, sample rate, precision, events, automation and latency notification.
The Windows state owner must remain serialized with DSP where the vendor requires it.

The current processor accepts real-time and offline modes, but rejects prefetch;
it also requires the callback mode to match setup. VST3 permits real-time/prefetch
switches without another setup call, while an offline transition does require setup.
This is a host-contract gap to cover explicitly, not a new explanation for the
Deck incident. See Steinberg's [processing-mode contract](https://steinbergmedia.github.io/vst3_doc/vstinterfaces/namespaceSteinberg_1_1Vst.html).

### 5. A reusable engine still needs faithful SDK semantics

[Descriptor validation](../plugin-descriptor/src/lib.rs) currently restricts audio
buses to stereo and bounds event inputs. The
[native controller](../native-vst3-proxy/source/commercial_controller.h) creates
generic SDK parameters from metadata; it does not forward vendor parameter text/
value conversion or expose a complete unit/MIDI/note-expression interface family.
The processor handles bounded supported events, not arbitrary VST3 event semantics.
The [scanner](../windows-factory-probe/source/inspect_module.cpp) observes core
processor/controller metadata; this is not a complete optional-interface census.

**Repair:** make capability discovery, descriptor representation, native exposure,
Windows host services and failure results agree. Forward vendor-owned semantics
where the contract requires them. Add representative stepped/text parameters,
returned events, reentrant callbacks, bus arrangements and connection orders to
shared tests. Do not map parameters by position, fake optional interfaces or merely
relax a descriptor validator while downstream processing remains incompatible.

Steinberg documents [processing lifecycle and latency notification](https://steinbergmedia.github.io/vst3_doc/vstinterfaces/classSteinberg_1_1Vst_1_1IAudioProcessor.html)
and [interface/thread conventions](https://steinbergmedia.github.io/vst3_dev_portal/pages/Technical%2BDocumentation/API%2BDocumentation/Index.html).
The pinned SDK and representative host calls must verify the implementation.

### 6. Editor separation exists, but error propagation defeats part of it

[Mapped processing](../windows-factory-probe/source/mapped_processing.cpp) runs
owner/editor work separately from DSP and mirrors controller values. Controller
update refusal/exception enters `editor_fatal`; editor channel failures, including
backlog, also propagate to the terminal instance path in
[EditorSession](../windows-factory-probe/source/editor_session.h). This conflates an
optional interaction's refusal with an unsafe processing instance. Bounded command
counts also do not bound the duration of a vendor GUI call.

**Repair:** classify failures at the operation that knows their meaning. A refused
optional view operation, exhausted GUI queue, invalid transport identity and crashed
vendor process cannot share an undifferentiated recovery decision. Where safe,
close/reset the editor channel and preserve DSP and the last saved project. Where
vendor memory/thread safety is unknown, retain whole-instance containment and report
that limit. Do not promise crash isolation between objects in the same process.
State restoration with unknown partial mutation must still fail visibly and retire
safely; preserving playback cannot mean silently accepting corrupt state.

Extend [graphics assessment](../bridge-manager/src/graphics_cli.rs) from hints and
independent probes to attributable effective editor observations, then measure its
interaction with audio. Existing D3D/WGL/DComp probes and short editor pumping are
useful instrumentation, not a GPU or sustained workload claim. See the retained
[graphics review](GRAPHICS_RUNTIME_REVIEW.md).

### 7. Capacity and inactivity rules need shared resource ownership

[Capacity](../bridge-manager/src/capacity.rs) uses a six-owner global DSP envelope,
fixture class limits and a one-instance limit for newly managed classes.
[Native slots](../native-vst3-proxy/backend/src/instances.rs) have a separate four-slot
capacity per engine image. Many [operator actions](../bridge-manager/src/operator_model.rs)
and preparation operations require global inactivity even when their proposed
change is narrower. These are finite safety mechanisms, but their values and scopes
are not a general multi-plug-in resource policy.

**Replace policy, preserve reservation and generation checks.** Determine scope
from affected publication, environment, shared service/runtime and host resources.
Separate structural bounds, configured resource limits and qualified workload
recommendations. Reject a genuine capacity/ownership conflict specifically. Do not
just remove limits or allow concurrent prefix mutation without proving independence.
Read CPU quotas/affinity and actual worker policies; scheduling is not a vendor-name
choice or a reason to take capacity from unrelated services.

### 8. State and custody are foundations, not work to restart

The [installed update result](../evidence/preparation/2026-10-03-state-update-installed.json)
records schema/parameter evolution, both controller connection orders, meaningful
state/audio, malformed/refused loads, abrupt consumer loss and ordinary predecessor
restoration. Ten cases used 18 primary consumers with zero audio mismatches in
41,287,680 compared samples, plus sibling checks. Ordinary runs were 10.24 seconds
at 48 kHz/1024 with 1024 bridge frames. This establishes bounded reference behavior,
not commercial compatibility, endurance or low latency.

Keep historical-state admission distinct from exact current executable admission;
keep current authoritative controller readback and truthful new provenance. Keep
RAII SDK teardown, exact native-generation death recognition, independent failure
and retirement acknowledgements, and Windows cancellation/join containment.

The product supervisor in [session.py](../bridge-manager/runtime/session.py) is
substantial production Python, not merely a test harness. Configuration and failure
policy span it, Rust and the C++ host. Consolidate product decisions in Rust and
version their narrow interoperation as these owners change. Preserve the tested
custody mechanisms while doing so; a language migration is not the first delivery
and must not become an unrelated rewrite.

### 9. Acceptance must close the shared journey

The [SDK consumer](../tools/beta/lifecycle_host.cpp) is a valuable state/audio oracle,
but its current normal runs are paced and high-buffer. The deterministic beta
fixtures do not supply a demanding custom editor. Container package checks do not
exercise the DAW-to-Windows audio path. Extend these tools for the required contracts;
do not infer missing coverage from their successful existing cases.

The [general5 Deck record](../evidence/audio-recovery/2026-10-03-general5-deck-installation.json)
shows installation and short changed-state recall, followed by terminal Pure LoFi
failure during interaction. Terminal counters showed zero queued underruns. Missing
status had initially been misreported as lost audio; the underlying failure trigger
remains unattributed. This assessment identifies structural failure propagation,
but does not establish that it caused that incident. The planned soak never started.

An architecture finding, a deterministic regression, a physical comparison and a
release qualification answer different questions. Reuse unaffected results. Repair
an invalid observer before using its output to judge the bridge. A clean subsection
or larger buffer does not rescue a failed whole session.

## Prior-art comparison

Yabridge's [architecture description](https://github.com/robbert-vdh/yabridge/blob/b580a9f7fc46509767ca156d4f92872552b9e571/docs/architecture.md)
describes a generic native library, prepared shared audio memory, synchronous
cross-boundary calls with callbacks, and thread/recursion handling. Its VST3 design
uses separate processor communication alongside control/callback communication.
Those are useful comparisons with this implementation's delayed presentation and
polling handoffs. They do not establish which stage caused a particular failure
here, or prove that copying one synchronization choice would fix it.

The useful target is faithful host semantics and timely completion across the whole
path. Proton options and priorities complement that work; they do not replace it.
No yabridge implementation code was copied. Any performance comparison must use
matched hardware, plug-in, settings and workload, disclose host/runtime differences,
and follow its [documented native-DAW setup](https://github.com/robbert-vdh/yabridge).
No performance parity or measured advantage is claimed by this source review.

## Retire or quarantine after replacement

- Stop producing new per-module compile recipes through the ordinary product once
  the generic path covers the required contract. Keep schema 2/3 history readers and
  exact artifacts as long as installed predecessors require them; schema 4 already
  avoids per-plug-in compilation for new generic publications.
- Remove fixture-name readiness branches and closed named runtime/dependency trials
  from normal selection when their capability-based replacements pass. Preserve
  exact historical support evidence and genuinely scoped vendor requirements.
- Consolidate installed processing/configuration selectors. Keep comparison-only
  AP settings out of ordinary selection after checking build flags and callers;
  do not delete shipping code merely because its name contains `AP`.
- Replace generic editor-fatal propagation only where a specific recoverable failure
  is established. Keep containment for compromised transport, process or vendor state.
- Remove obsolete product-policy branches as their shared owners take over. Preserve
  saved objects, license identity, predecessor publications and useful regression tests.

## Implementation programme and completion

The next complete capability is **prepare, explain, run and restore an unfamiliar
plug-in through one managed configuration**. It must reach the existing execution
path and normal UI, not stop at a new configuration struct or assessment report.
Implement in reviewable commits; the following are dependent capabilities, not a
fresh campaign of separately approved fixture slices.

| Order | Integrated change and owners | Completion evidence |
| --- | --- | --- |
| 1. Shared configuration and host eligibility | Candidate/history + profile views + readiness + launch/admission + ordinary UI resolve the same module/class, coherent runner/environment, graphics/process policy and explicit preferences. Migrate existing candidate data without changing class IDs, saved state or vendor identity. Distinguish execution eligibility, current liveness and qualification. Scope reservations and maintenance to affected owners. | An unfamiliar class and an existing publication both complete prepare → launch → meaningful save/reopen → configuration trial → keep/restore through ordinary controls. Native and sandboxed host contexts receive capability-specific results; missing optional editor/probe evidence does not block eligible audio. Supported multiple instances and independent siblings remain usable; genuine conflicts remain explicit. |
| 2a. Runtime and dependency operations | On order 1's contract, generalize runner acquisition/selection/import and selected-environment dependency installation through existing custody. Resolve typed options with reasons, differences, scope and effective readback. | A coherent unfamiliar runner using an implemented adapter can be tried without a new name allowlist. A controlled missing dependency is installed, rescanned and exercised; cancellation/refusal leaves a usable retained configuration. Irreversible vendor changes are not falsely promised rollback. |
| 2b. DAW execution and failure semantics | Also on order 1's contract, connect both SDK edges, worker/Windows notification, real-time/offline completion, inactive reconfiguration, parameter/event semantics and editor/control severity. Keep state and retirement ownership. | Deterministic varied blocks, unpaced bursts, slow-but-valid offline work, legal lifecycle/connection orders, malformed input, refused GUI operations and abrupt loss have correct audio/state/cleanup outcomes. Whole callbacks and serial chains are measured. Same-callback `D = 0` must then pass the declared low-block gates; a buffered comparison alone does not finish this capability. |
| 3. Effective rendering and resource configuration | Use the same resolved configuration and order 2b's error policy for actual editor renderer observation, typed fallback/DPI/runtime choices and effective process scheduling/resource limits. Include demanding rendering in shared fixtures. | Capture shows which path the editor used, whether its workload helped or harmed audio, and that keep/restore preserves unrelated buffering and siblings. Unsupported or unobserved paths are accurately reported. No blanket GPU/priority optimization claim. |
| 4. Integrated beta delivery | Freeze the coherent roster and complete normal installation, authorization, update/rollback, diagnostics and release distribution. Run the agreed representative catalogue on the declared platform builds. | The existing [beta acceptance contract](INTEGRATED_BETA_DELIVERY.md#acceptance-method) passes: actual signal, editors/automation, state/reboot, clean and populated setup, recovery and rollback, 30-minute interaction followed by a two-hour soak, plus signing/distribution obligations. Every required failure remains a failed candidate until resolved. |

Orders 2a and 2b share the order 1 contract and can be developed independently;
this does not authorize simultaneous physical workloads or defer audio behind all
advanced UI work. Rendering requirements inform orders 1/2 from the start. Begin
order 1 with a compact vertical implementation, then continue to its whole capability
endpoint; do not turn its subcommits into new product milestones.

Representative contract coverage must include unfamiliar class identities, an
instrument and effect, changed parameter/state schema, absent/custom editors,
recoverable owner-call refusal, returned events and nontrivial parameter conversion,
multiple instances, independent and shared environments, native/sandbox host access,
and required-capability absence. Vary dimensions deliberately; a brute-force Cartesian
matrix is not the plan. Preserve the agreed commercial catalogue and platform goals.

No physical execution is resumed by this assessment. The Deck remains on the retained
general5 candidate with testing stopped; PR #204 remains draft. The next source
implementation follows this programme and current machine custody. Hardware
acceptance, dependable audio, lower latency and beta release remain open.
