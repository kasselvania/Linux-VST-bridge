# Engineering a Windows-audio compatibility platform

## Product purpose

Build a managed compatibility platform that lets native Linux DAWs use Windows
audio plug-ins across supported machines, distributions and runtime configurations.
Users install, authorize, prepare, use, save, reopen, update, investigate and recover
through the product. They must also be able to try unfamiliar plug-ins and explicit
compatibility options without waiting for a maintainer to compile a special proxy.

The unit of design is a reusable platform capability and its complete user journey.
A plug-in is a fixture that exposes requirements; a particular crash or missing
block is an observation about that system. Steam Deck, Bitwig, a vendor, a known
catalogue and one successful session do not define the architecture. Small patches
are useful delivery units, but do not reduce the product goal to isolated repairs.

## Decisions and current work

Follow the operator's current instruction, then this file, `GOVERNANCE.md`,
`CURRENT_SLICE.md`, `docs/ARCHITECTURE.md` and applicable accepted decisions.
Code describes the implementation; evidence describes observations; support records
bound published claims. None acquires authority merely by existing. If these
sources conflict, identify the conflict instead of inventing a compromise.
Ask when a product term or authority boundary is genuinely unresolved; choose
ordinary implementation details without making the operator manage engineering.

Read the current task and relevant architecture, failure classes and support rows.
Historical task pointers, campaign documents and previous experiments are context,
not standing launch instructions. Keep `CURRENT_SLICE.md` short: current outcome,
shared contracts affected, remaining work, acceptance and machine custody. Replace
superseded directions; retain history in Git and original evidence. Do not use the
fusion skill for reviews.

## Design the shared system before selecting patches

Before changing behavior, trace the user journey and the data, control, thread,
process and ownership boundaries that implement it. Ask whether the contract and
implementation can serve the intended range of plug-ins and platforms. Inspect
adjacent consumers, including normal setup, updates and recovery, rather than
stopping at the function that emitted an error.

Use this coverage map to find architectural gaps and select representative tests:

| Platform capability | Questions the design must answer |
| --- | --- |
| Machine and host integration | Which architecture, loader, sandbox/IPC, audio access, display system, graphics APIs, storage, quotas and scheduling capabilities exist? Which are required, optional or unobserved? |
| Runtime and dependencies | How does the product acquire/select a coherent pinned runtime, install declared dependencies, preserve environment identity and restore a predecessor? What actually changes with a Proton/runtime option? |
| Discovery and preparation | Can an unfamiliar module expose classes, interfaces, buses, parameters and requirements through supervised inspection and reusable native machinery? Which specific unsupported contract prevents use? |
| Configuration and recommendations | How do observations, implemented defaults, profile advice and explicit user overrides produce one explainable configuration with effective readback? |
| DAW/audio integration | How are negotiated capacity, actual block length, sample rate, precision, events, automation, latency, real-time deadlines and offline completion handled through legal lifecycle changes? |
| Editors and graphics | Who owns event dispatch, embedding, resizing, input, rendering and teardown? Which work can interfere with DSP/control, and where are failures allowed to propagate? |
| State and change | Can projects survive save/reopen, restart, module updates, runtime changes and rollback? Are class identity, state provenance and current executable admission distinct? |
| Supervision and recovery | Can installer, scanner, editor, vendor host and native consumer failures be identified, contained and retired without damaging healthy sessions or hiding uncertain cleanup? |
| Product delivery | Can the normal manager complete the journey on clean and populated systems, explain a failed stage and offer usable recovery without maintainer administration? |

This map guides design and coverage selection; it does not require rerunning every
platform combination for every edit or creating another approval checklist.
Portability, graphics, audio, configuration and recovery are designed together.
Their implementations may proceed in dependency order, but portability is not a
packaging phase to append after one Deck demonstration works.

Recurring symptoms, accumulating exceptions, duplicated settings, incompatible
owners or a design that excludes unfamiliar inputs require an architectural
review before another local workaround. Retain, repair, replace or retire code
according to the platform contract and observed consequences. Existing proofs do
not make an early design permanent. Depth of change follows the evidence: neither
an arbitrary small-patch limit nor a speculative engine rewrite is the default.
Reuse sound ownership and transport machinery; do not create competing frameworks.
Remove or quarantine production paths that contradict the accepted contract;
preserve needed user state and evidence, not wrong behavior because it is old.

## Work in complete capability increments

1. State the user-visible capability being delivered, its shared contracts and the
   gap between current behavior and the platform goal. A symptom alone is not the
   task definition. Use a short note in the existing task/architecture documents;
   do not create a new campaign or receipt system.
2. Review the relevant implementation against that contract. Use failure evidence
   to test the design, including how the same assumption affects other plug-ins,
   host behaviors, machines and configuration changes. Distinguish measured causes
   from hypotheses and missing evidence.
3. Implement the coherent change across the necessary owners. Make reviewable
   commits and retain working comparisons. Do not leave required connections to
   the manager, prepared configuration, state or recovery for the operator to
   discover and commission separately.
4. Verify the shared contract, affected failure paths and installed user journey.
   Report completed work, remaining gaps and actual support scope. If an external
   blocker prevents delivery, finish independent work and name the blocker exactly.

A vendor-specific workaround needs evidence of a vendor-specific requirement and
an explicit scope. Prefer reusable behavior or declarative compatibility data.
Do not accumulate product-name branches, per-binary builds or allowlists to conceal
a missing general mechanism. Fixing one fixture can establish a regression repair;
it cannot establish platform coverage or close the surrounding product capability.

## Compatibility, identity and experimentation

Qualification and permission to try are different. An unfamiliar product, version
or distribution is unqualified, not automatically forbidden. Refuse actual missing
capabilities, unsupported required interfaces, invalid data or incompatible component
pairs with a specific reason. Profiles improve setup and communicate evidence;
they are not the catalogue of software allowed to run.

Observe capabilities rather than choosing policy by distro or product name. Keep
static import hints, independent runtime probes, actual editor/render-thread
observations and measured workload behavior distinct. Do not infer the editor's
renderer from a successful probe or invent live DAW settings during installation.

One managed configuration connects discovery, runtime/dependencies, graphics,
process policy, publication and recovery. Normal setup supplies explainable defaults;
advanced controls expose implemented options with their scope, expected effect,
limitations, comparison and keep/restore behavior. Explicit user overrides survive
profile refresh. Ordinary selection must not rerun deep diagnostics.

Hashes identify exact versions and stale inputs; they do not make legitimate state,
presets or installations immutable. Managed updates refresh observations and prepare
new data. Stable logical class identity, producing-state provenance, vendor machine
identity and the selected executable/runtime remain separate. Saved state never
selects code, and only the selected vendor implementation migrates its opaque state.
Paths are locations and friendly names are metadata, not durable plug-in identities.
Never guess parameter replacements or silently substitute another build or preset.

Runtime acquisition is the product's responsibility. Steam, customer-installed
Wine/Proton, SDKs, compilers and remote maintainer builds are not normal prerequisites.
Use coherent pinned runners and observable updates. Graphics, synchronization and
CPU options need measured benefit and effective readback; a Proton label is not
proof of acceleration, real-time scheduling, audio correctness or portability.

## Verification that matches the claim

Choose tests by the contract's meaningful variations: instrument/effect roles,
capabilities and unsupported interfaces, changing parameter/state schemas, rendering
paths, native/sandboxed hosts, platform capabilities and failure modes. Include an
unfamiliar identity when preparation or recommendation generality is claimed.
Reference modules provide deterministic coverage; lawful commercial fixtures and
physical targets validate the actual workload. Neither replaces the other.

Use controlled comparisons to attribute a defect and combined workflows to qualify
the product. Change related variables together only when the experiment explicitly
measures that combined change. A soak measures endurance after functional checks;
it is not a substitute for architecture review or failure attribution.

Validate the observer before relying on it. Keep recorder/harness faults, missing
observations, native admission failures, deadline misses, transport/protocol faults,
vendor DSP failures and editor failures distinct. A missing status file is not
proof of lost audio; a successful callback is not proof of delivered signal.
Correlate actual output, timing and lifecycle evidence before assigning a cause.
Correct a mistaken report promptly and retain the original failure evidence.

Use applicable declared acceptance criteria without quietly shortening durations,
raising latency or dropping difficult cases. Shorter engineering checks retain their
narrow scope. Reuse unaffected evidence with exact provenance, rerun changed contracts
and complete the final journey on the candidate proposed for release. A clean
subsection cannot turn a failed lifetime into a pass.

Capability completion requires its normal product path, meaningful musical/state
behavior and failure recovery at the declared scope. Beta completion additionally
requires the agreed platform/catalogue matrix, clean and populated installation,
authorization, audio/editor/automation, save/reopen, reboot/reopen, update/rollback,
and release/distribution obligations. A source patch, test suite, package, PR or
single plug-in success is not that completion. Keep `docs/FAILURE_CLASSES.md` and
`docs/SUPPORT_MATRIX.md` consistent without creating competing status registries.

## Engineering protections

- The native DAW loads a Linux proxy; a supervised Windows host loads the selected
  Windows module. Rust owns product logic, with C++ confined to justified SDK edges
  and explicit versioned C ABI/IPC boundaries. Preserve SDK reentrancy, lifetime and
  thread affinity. Do not replace vendor DSP or fabricate state. One global Wine
  prefix or one process for every plug-in is prohibited as the default; shared
  vendor services and environment changes need explicit ownership and scope.
- Keep allocation, filesystem/network/process work, ordinary logging, manager/GUI
  dependencies and unbounded waits off the audio callback. Prepare storage outside
  processing; declare each wait's bound and failure result. State/control ordering
  must respect vendor concurrency while preserving the audio contract. Distinguish
  local wait allowances from the full DAW deadline and real-time from offline work.
- Model failure propagation explicitly. A shared vendor process can constrain
  isolation; disclose and contain that limit rather than promising impossible
  separation or treating every editor problem as an audio-transport defect.
- Preserve projects, selected predecessors and licensed environment identity.
  Separate per-instance settings from shared environment changes. Use supported
  selection/recovery operations and clean up exact-owned resources only. Never
  erase uncertain ownership records to manufacture successful recovery.
- Profiles are versioned data, not arbitrary scripts or hidden downloads. Respect
  vendor authorization; never bypass licensing or collect credentials. Keep account
  data, tokens, licenses, proprietary binaries, presets and content out of Git and
  public diagnostics. Exports are allow-listed, not whole-prefix archives.
- Preserve SteamOS's protected base and explicit sandbox/broker boundaries. Do not
  turn root filesystem changes, blanket privilege changes or unrelated-service
  modifications into product requirements. Honor resource/spending limits and
  leave capacity for the user's other services.
- Use public specifications and clean-room prior-art analysis. Do not copy yabridge
  source into this repository. Record applicable third-party licensing and exact
  redistribution obligations; do not assume bundled runtimes or SDKs may be shipped.

## Ownership, tools and communication

Own design, implementation, integration and verification through the selected
outcome. Do not make the operator repeatedly reconnect related subsystems or approve
routine in-scope work. Ask before a genuine goal/security change, destructive change
to user data or new spending. Repeated low-information tests call for reassessment,
not another test campaign or a new permission ceremony.

Use `docs/WORKSTREAMS.md` to coordinate separately authorized lanes. It does not
launch those lanes or expand this task. Do not alter another owner's branch,
environment or experiment. Keep unmerged dependencies explicit. Shared GUI, audio,
service and runtime mutations require coordinated machine custody.

Prefer supported APIs/CLI for inexpensive setup and readback; use the GUI to verify
actual customer interactions. Respect tool permissions. When GUI work is delegated,
honor the operator's model/cost preference and provide a concise target, actions
and stop conditions. The main engineer owns the test and interpretation; an executor
reports observations, not a pass verdict. Never replay a useful test solely to change
executor or tool. Remote access is development tooling, not a customer prerequisite.

Lead updates with what the platform can now do, what still fails and what evidence
changed the decision. Label hypotheses and instrument failures. Do not advertise
motion, commit counts or generated reports as delivered capability. Commit and push
completed work, preserve unfinished work transparently, and keep the PR focused and
reviewable. Merge only within the operator's authority; do not promote a failed beta.
