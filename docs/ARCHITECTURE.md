# Architecture

This is the governing recovery contract adopted by the operator on 2026-10-16.
It replaces production runtime selection by plug-in or qualification campaign.
AGENTS.md retains priority. Detailed schemas, conflict rules and physical module
layout are not selected by the examples or logical owners below.

> Where semantics are unresolved, I will bring a concrete decision with alternatives and consequences before implementation—not quietly choose a schema or architecture while fixing something else.

The [current slice](../CURRENT_SLICE.md) owns immediate work. The
[retained bridge contracts](components/BRIDGE_CONTRACTS.md) preserve active SDK,
transport, custody, state and transition invariants in accessible documentation.
Existing mechanisms and tests remain until a replacement proves their invariants;
existing structure has no authority merely because it exists.

## Product and release

A musician installs, authorizes, plays, saves, reopens, updates, repairs and removes
Windows audio plug-ins through the product in a native Linux DAW. The product
acquires its runtime; customers need no Steam, Wine installation, SDK or compiler.
Preserve existing workspaces without expanding the target to all Windows apps or DAWs.

For each supported platform/architecture, a bridge release has **one qualified
binary compatibility component set for new work**: one common Wine engine with
its runtime, graphics and integration components. Configuration selects supported
behavior inside that set. Profiles cannot select private per-plug-in production
engines, and several vendor engines in one archive/name do not satisfy the rule.

The release binds manager/control protocol, proxy, Windows host/scanner,
supervisor protocol, runtime and shipped policy revisions as a tested combination.
Choose Proton or GE-Proton by combined compatibility and regression results;
separate successful patches do not qualify their combination. A binary correction
creates a runtime generation; a supported option creates a configuration change;
a dependency installation changes a setup revision; discovery and qualification
record observations. None substitutes for another.

WebView2 is a managed setup dependency, as selected by the operator on 2026-10-16.
Its exact browser dependency belongs to the setup revision and changes through
an explicit owned transition. Preserve predecessor browser bindings and browser
user data/account state; retaining browser binaries alone does not prove usable
rollback. The common Wine engine remains independent of browser servicing. This
classification does not accept publisher terms or settle acquisition versus
redistribution, delivery APIs or the required servicing implementation.

The setup owner owns the installed dependency set. The configuration owner binds
an exact browser version for each launch purpose in the immutable applied
configuration, as selected by the operator on 2026-10-16. The supervisor receives
the resolved exact identity, folder and scope; it never selects or upgrades a
browser, uses an implicit latest version, or rewrites configuration at launch.
Keeper conflicts, browser-data compatibility and failed partial setup mutations
require explicit transition semantics before implementation.

Git owns original source/patches, pinned inputs/toolchain, build/composition recipes,
tests and distribution notices. Build coherent components from one selected source
and SDK basis, without transplanted prebuilt DLLs from different Wine bases or
machine-local construction steps. Customer installers, plug-ins, presets, content,
credentials and activation material are never Git build inputs or diagnostic exports.
Record acquisition, licences, notices and servicing obligations before distribution.

Retain exact immutable predecessors for continuity, migration and rollback. Do not
silently switch working setups or delete predecessors to shorten the runtime list.
Lab generations remain isolated from ordinary selection and confer no support
claim. A successful lab mechanism enters production through the common build and
combined regression. Incompatible binary fixes require an explicit release decision,
not a quiet profile-selected engine fork.

## Four layers and their identities

| Layer | Identity and responsibility |
| --- | --- |
| Release runtime | Immutable release/runtime generation: common binary component set and paired product/protocol/policy inputs. |
| Vendor setup revision | Stable setup identity with revisions of the mutable Windows environment, installed modules, vendor services and content relationships. |
| Applied plug-in configuration | Immutable resolved snapshot for one exact module/class, setup revision and release, with selected options and their provenance. |
| Live instance | One admitted DAW use of an applied snapshot, with actual audio format, editor, process generations and effective runtime observations. |

One runtime does not mean one prefix, account environment or Windows process for
all vendors. Isolate vendor setups and plug-in processes by default. Shared services
have explicit owners and affected users. An immutable setup-revision record does
not make vendor disk state immutable: mutation requires an owned revision transition.
Preserve licensed machine identity and mutable user state through supported changes.

Identity uses actual content and logical classes, not paths/display names. Stable
native DAW class and automation identities survive runtime/configuration updates.
Never substitute a same-name build, preset, environment or content root. Retaining
files alone cannot promise reversal of vendor activation or external changes.

<a id="186-general-preparation-and-compatibility-experimentation"></a>

## Configuration lifecycle

Keep discovered facts, shipped advice, user intent, proposed resolution, applied
selection, effective live observations and qualification separate. Their references
connect them; one combined profile/candidate record must not impersonate them all.

Configuration intent records user selections and scoped overrides. A pure Rust
resolver consumes exact release/setup/discovery inputs, typed policy, capability
observations and intent. It proposes a configuration with field provenance,
conflicts, affected objects and consequences; it performs no filesystem mutation,
process launch, network or UI work. Hard capability/safety constraints are distinct
from advice. Supported safe user choices take precedence over advice; competing
advice or incompatible overrides require a visible decision, not dictionary order.

A configuration owner commits a proposal through an owned transition into an
**immutable applied snapshot**. Publication and admission bind that snapshot; UI,
CLI, launch, update comparison, rollback and reports reference the same selection.
No consumer independently merges profile data. The service exposes the canonical
view; this does not make the service projection a writer for every domain.

A profile update produces a new proposal. It cannot reinterpret the existing
applied snapshot at launch. That snapshot continues until a successor transition
commits. Which changes permit safe automatic adoption, which require user action,
and how invalid overrides/schema migrations resolve require explicit decisions
before implementation. Preserve exact project bindings and usable predecessors.
The supervisor reports what actually ran; requested choices cannot write observations.

Profiles are typed data for implemented vendor, installer and plug-in capabilities:
renderer/library choices, Windows version, dependencies, services and closed options.
They contain no commands, patches, arbitrary executable/download hooks or licensing
material. Vendor-specific advice requires a vendor-specific reason. A new operation
requires a reusable implemented capability, not a plug-in-name product branch.

Advice applicability may cover a family/range; support names exact tested builds,
release/runtime/applied configuration, platform, DAW, date, conditions and results.
Generate the support matrix from those facts as that mechanism is implemented.
Qualification does not authorize ordinary execution. An unfamiliar plug-in/distro
is untested, not forbidden; refuse a specific missing capability or invalid binding.
An optional editor failure does not erase usable audio or useful discovery facts.

## Sole writers and dependency boundaries

One owner means one writer for a class of facts, not a daemon/crate/database per
noun. Stored facts, derived proposals and displayed projections remain distinct.
These logical authorities are governing; concrete keys, APIs, atomic transitions,
conflict rules and migrations require a decision before the affected implementation.

| Facts or result | Sole owner | Must not own |
| --- | --- | --- |
| Release/runtime generations and paired bill of materials | Release/runtime catalog | Vendor installations, user intent or support claims |
| Setup identity and committed revisions | Setup owner | Runtime builds, publication or qualification |
| Discovered module bytes/classes and SDK topology | Inventory/inspection owner | Advice, user choices or support posture |
| Shipped policy and immutable advice revisions | Policy catalog | Local setup mutation, process launch or observed facts |
| User intent and committed applied snapshots | Configuration owner | Runtime artifacts or qualification facts |
| Proposed resolution/conflicts | Pure resolver; derived from identified inputs | Mutating stores or executing operations |
| Stable DAW publication and predecessor history | Publication owner | Setup mutation or installer execution |
| Admission reservations and live binding | Admission broker | Choosing policy/runtime composition or rewriting applied snapshots |
| Operation intent/progress/result | Owning domain operation | Other domains' facts or direct Ready status writes |
| Exact process generations, owned services and retirement | Supervisor | Advice, qualification or configuration selection |
| Content placement and owned references | Storage owner | Licensing or guessed external-file ownership |
| Capability/effective observations | Relevant inventory, renderer or platform assessor | User intent or policy publication |
| Tested support and qualification outcomes | Qualification owner | Ordinary product activation/selection |
| Musician status and primary action | Derived service projection; no separate persisted writer | A second status truth store |

An installer adapter executes within the setup owner's operation. That owner
commits the new revision; inventory observes it, configuration proposes successors,
and publication/admission consume committed selections. Cross-domain changes carry
expected predecessor, affected siblings/resources, bounds and consequences to the
actual writers. They do not edit each other's records or set Ready directly.

Vendor advice, installer adapter policy and plug-in advice remain distinct concerns,
not runtime forks or copies of observed state. Policy catalog ownership does not
settle their schemas or conflict rules. User export/import, community recommendation
review and shipped profile publication are distinct operations. Decide their trust,
authoring, versioning, distribution and update semantics before implementing them;
the qualification owner does not acquire those authorities by implication.

Runtime build owns source/patch/composition recipes and is not linked into the
installed control service. Rust owns policy and lifecycle. C++ is confined to VST3
SDK and Win32 edges behind versioned C ABI/protocols. The Linux proxy owns DAW
interfaces and bounded transport; the Windows host owns vendor SDK calls. The
supervisor executes a closed, resolved launch contract and reports custody/effective
facts; it does not decide product policy. Preserve its tested implementation while
narrowing that boundary later, rather than starting with a language rewrite.

UI/CLI consume a versioned product API, not manager implementation/store types.
The resolver depends on pure identities/policy inputs, not operations or stores.
Enforce dependency direction through modules/facades before moving folders; a new
crate layout, database or lifecycle framework is not the recovery deliverable.

<a id="187-platform-execution-convergence"></a>

## Musician status and growth decisions

One service projection provides a consistent status and primary action from
separate availability, operation, interaction, health, qualification, publication
and removal facts. Ready, Needs you, Working, Problem and Removed describe those
facts in musician language. Their summary precedence requires an explicit decision;
there is no second persisted status. Missing support history alone is not Problem.
Companion sign-in does not erase usable audio, and cleanup success does not turn
failed restoration into success.

Keep Show/Stop/Cancel and necessary advanced controls available alongside the
primary action. Check inspects the current setup; Repair addresses the failed stage;
fresh Reinstall explicitly replaces a setup while preserving recoverable prior
state; Remove names integration/vendor/setup scope and consequences. These are
consistent operations, not mandatory gates. Restore is offered only where real.

Concrete extension decisions remain visible rather than growing a profile dictionary:

- A new vendor/service dependency changes advice and, for a genuinely new capability,
  an installer/service adapter. The proxy stays generic. Decide version/shared-service
  conflicts and the writer of readiness facts before implementation.
- Updated advice changes a proposal, retaining user intent and the applied snapshot;
  the host never merges advice. Decide cross-domain/scoped conflicts and migration.
- Moving large content changes storage placement/references, never audio-callback I/O.
  Decide shared/external ownership, authorization and actual restoration consequences.
- A renderer choice changes intent/proposal; an assessor reports capability/effective
  behavior and the editor adapter applies the committed choice. Installer policy
  stays unaware. Decide fallback and override conflicts rather than silently choosing.
- A DAW layout changes live negotiation from inspected SDK topology, not vendor or
  installer policy. New layouts/bounds need edge contracts at both ends, not advice.
- A distro exception changes a capability adapter, not Windows SDK semantics or a
  whitelist. Decide whether it is data, a new adapter or an operator-owned OS change.

<a id="18-audio-recovery-and-portable-execution"></a>

## Retain the bridge; recover in order

Preserve SDK behavior, content/class identity, exact custody, immutable publication
history, state/automation continuity, rollback and bounded processing. The linked
[component contracts](components/BRIDGE_CONTRACTS.md) and existing tests govern
replacement. Do not transplant old campaign-specific selection/acceptance policy
into a generic-looking framework or delete safety mechanisms during a folder shuffle.

[#430](https://github.com/kasselvania/Linux-VST-bridge/issues/430) is the sole
implementation slice until a clean Git-owned common build reaches normal delivery
and combined real-case qualification. The old issue queue, new manager window and
control-plane implementation remain frozen. Then proceed through working increments:

1. Introduce a versioned API and read-only configuration correspondence alongside
   current bindings. Compare intent/proposal/applied readback with actual launches;
   the new read model gains no write authority merely because it exists.
2. Move write authority through owned transitions only after correspondence and
   the unresolved semantics are decided. Preserve identities and predecessors.
3. Narrow the supervisor to resolved launch execution and factual reports.
4. Build the plug-in library UI on that API and prove the complete ordinary journey.
5. Migrate preserved setups explicitly; retire old routes only after continuity,
   recovery and rollback pass. Quarantine/delete conflicting code, retaining useful tests.

No new plug-in/vendor gets a permanent product pathway. It exercises an existing
capability, adds typed policy for it, or demonstrates the need for one reusable
capability. Temporary migration code must be explicit and bounded, not a new
production engine-selection escape hatch.

## Delivery and claims

Preserve the working Deck and existing backups. Verify backup scope/restoration
before destructive work; an active copy is not a verified backup. No wipe or silent
licensed-state migration is part of runtime recovery. Use disposable CachyOS and
SteamOS fixtures, preserving accounts, activation and original projects under AGENTS.md.

The ordinary release journey is install → authorize → discover/connect → play →
save/reopen → restart/reboot → update/rollback → repair → remove. Exercise combined
patches against their real cases, baseline effects/stateful instruments, vendor
services, editor/touch and existing state/update/rollback fixtures in actual DAWs.
Normal acquisition and installer/publication paths must deliver the tested common
set; no manual copy or experimental transition substitutes for tester parity.

Report recovered source, clean build, green checks, installed generation and full
musician journey as different results. Retain actual advertised buffer/workload
limits and measure the whole DAW call and captured audio. No probe/counter or build
success proves timely sound or beta readiness. Existing support matrix, failure
classes and beta acceptance retain tested facts; they confer no catalogue permission
gate. No new campaign ledger or per-run report is needed.
