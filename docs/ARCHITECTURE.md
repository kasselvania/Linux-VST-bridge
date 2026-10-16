# Architecture

The product constraints below reflect the operator's direction on 2026-10-16,
including replacement of separate production runtimes for different plug-ins.
The ownership and status proposals are for design review; their schemas, physical
modules, storage and migration rules are not settled by this document.
Implementation and observed support are recorded in [CURRENT_SLICE.md](../CURRENT_SLICE.md)
and [SUPPORT_MATRIX.md](SUPPORT_MATRIX.md). Earlier designs remain in Git history;
historical implementation notes do not override this design or AGENTS.md.

## Product and release

A musician installs, authorizes, plays, saves, reopens, updates, repairs and removes
Windows audio plug-ins through the product in a native Linux DAW. The product
acquires its runtime; customers need no Steam, Wine installation, SDK or compiler.
Existing workspaces and projects are preserved; they do not expand this target to
all Windows applications or DAWs.

Each bridge release delivers one qualified, versioned compatibility component set
for its supported platform packages. It includes one common Wine engine and the
required runtime, graphics and integration components. Typed configuration may
select supported rendering or library paths within that set. Packaging several
vendor-specific engine builds under one runtime name does not meet this contract.

The release binds the manager, Linux proxy engine, Windows host, protocols,
runtime and profile definitions as a tested combination. Choose the Proton or
GE-Proton base by actual combined compatibility and regression results. Previously
successful patches must be recovered, integrated and tested together; their
separate results do not qualify the combination.

Git owns the original product source, common patches, pinned upstream inputs,
build recipes, tests and distribution notices. Build coherent components from the
selected pinned source/SDK, without transplanting prebuilt DLLs from another Wine
base or requiring unrecorded machine-local steps. Customer plug-ins, installers,
presets, content, credentials and activation
material are never Git build inputs or diagnostic exports. Distribution requires
the applicable component licences and notices to be recorded.

Retain exact predecessor release/runtime pairs only for project continuity,
migration and rollback. New installations use the current qualified set. A local
developer experiment is isolated from production and gains no support claim until
it is incorporated into a qualified release. Do not silently switch a working
setup or delete its predecessor to make the runtime list shorter.

## Isolation and identity

Sharing runtime code does not share every vendor's installed environment. Keep
vendor setups isolated, with independent plug-in processes by default. A shared
vendor service has an explicit owner and affected setups; maintenance retires
those actual users before changing their resources.

Identify vendor modules and content by their actual bytes and logical classes.
Keep native DAW class IDs stable across bridge/runtime/configuration updates.
Never substitute a same-name build, preset, content root or environment. Preserve
licensed machine identity and mutable user state through supported transitions.

<a id="186-general-preparation-and-compatibility-experimentation"></a>

## One resolved configuration

The service exposes one canonical selected configuration. Profiles, UI, CLI,
publication and launch consume that model; none invents a competing selection.
It identifies the release/runtime, exact module/class, vendor setup revision,
dependencies, content locations, renderer and applied options. This shared view
does not settle the domain ownership proposed below or make one profile dictionary
the authority for every concern.

Profiles are typed data for supported renderer/library choices, Windows version,
dependencies, services and runtime options. They contain no commands, binary
patches, hidden downloads or vendor licensing material. General mechanisms apply
them; a vendor-specific recommendation needs a vendor-specific reason.

Resolve implemented defaults, applicable profile advice, then explicit user
overrides. Preserve overrides across updates and show their affected scope.
Shared advice changes do not silently rewrite selected working configurations.
The supervisor reports effective settings so requested options cannot impersonate
what actually ran.

Tested support is separate data: exact plug-in build/class, bridge/runtime and
configuration, platform, DAW, date, conditions and observed results. Generate the
support matrix from those facts as that mechanism is implemented. Unknown inputs
are untested, not forbidden; refuse only a specific missing capability or invalid
binding. A successful local trial is not universal support. Sanitized settings
and results may inform a reviewed shared profile without exporting private data.

<a id="187-platform-execution-convergence"></a>

## Proposed musician status and primary action

Platform convergence uses the resolved configuration above and the proposed
capability owners below; this status view consumes their results.

The agreed goal is one displayed status and clear primary action. The proposed
service projection derives these from canonical configuration and owned domain
facts, rather than a second persisted status. The UI presents them consistently.
The following meanings and conflict rules require design review.

| Status | Meaning |
| --- | --- |
| Ready | Connected for DAW use; tested conditions and known limitations remain visible. |
| Needs you | A specific choice, installer or vendor authorization step needs attention. |
| Working | A named product operation is in progress, with progress and cancellation where supported. |
| Problem | A named operation failed, with a useful explanation and next action. |
| Removed | Removed from the selected integration, with retained history and data scope explained. |

Ready describes operational availability; qualification still requires the actual
musician journey. Missing support history alone is not a Problem. Keep installation,
loader, editor, audio, saved-state and cleanup outcomes distinct in the details.
A companion application needing attention does not erase an already usable plug-in.

Offer one clear primary action while retaining necessary Show, Stop, Cancel and
advanced controls. Every action identifies its target and affected siblings.
Check inspects the current setup; Repair addresses the observed failed stage;
fresh Reinstall creates an explicit replacement while preserving recoverable old
state; Remove states whether it affects DAW integration, a vendor setup or the
bridge. These are consistent operations, not a compulsory sequence of gates.

Advanced controls change the same configuration, show the difference and support
restore where it is real. A vendor installer, fresh prefix or runtime reversal
can change licensed identity or external state; retaining old files does not make
that reversible. Explain the consequence before an affected user action. Never
delete uncertain ownership records or original projects to make recovery succeed.

## Ownership proposal for review

One service is not one undifferentiated module. Each domain needs a single writer
for its facts and a clear extension boundary. The table proposes those boundaries;
it does not choose a database, Rust crate layout or new process per row. Data names
describe concerns, not selected schemas. Whether policy authoring/versioning is
one common component or separate domain writers is an open design decision.

| Concern | Proposed source of truth and writer | Consumers | Scope / affected objects | Extension mechanism |
| --- | --- | --- | --- | --- |
| Vendor policy | Versioned vendor advice; authoring/versioning owner open | Resolver, installer planning | Shared vendor setup, dependencies and service advice | Typed recommendations with vendor-specific reasons |
| Installer policy | Installer advice; authoring/versioning owner open | Installer planner | Installer type/build and intended target setup | Data for implemented installer adapters; no command hooks |
| Plug-in policy | Plug-in advice; authoring/versioning owner open | Resolver, preparation | Module/classes and applicable versioned advice | Typed settings; unfamiliar builds remain eligible to try |
| User control and resolution | User overrides / selected configuration; configuration owner proposed | UI/CLI, publication, admission, launch | Explicit instance/module/setup/release scope | Scoped typed actions; resolver derives one result from policy, facts and overrides |
| Installer execution | Install intent and observed result; installer owner proposed | Discovery, resolver, status | One operation, exact target setup and affected siblings | Supervised installer adapters; process facts reference the supervisor |
| Setup revisions | Vendor-setup identity and revision facts; writer unassigned | Installer, resolver, admission | Mutable setup and its affected users | Revision/commit rules require design review |
| Discovery | Observed module bytes/classes; writer unassigned | Resolver, publication, support | Exact module/class within a setup | Discovery/inspection contracts; no path/name identity substitution |
| DAW integration | Native publication and admission facts; writer unassigned | DAW, resolver, status | Stable class identity and selected implementation | Publication/admission transitions require design review |
| Managed storage and content locations | Placement / content references; storage owner | Installer, resolver, host, project restoration | Runtime/cache, vendor install, user library and externally shared content | Typed storage/move operations with explicit ownership and preservation rules |
| Renderer capabilities | Editor capability/effective-renderer observations; renderer assessor | Resolver, editor adapter, status | Actual runtime, editor, driver/display context | Capability adapters and supported backend types; settings do not write observations |
| Audio topology | Observed SDK bus/layout capabilities; inspection owner | Audio negotiation, preparation | Exact module/class and declared SDK capabilities | SDK contracts implemented at both edges with tests |
| Live audio negotiation | Processing configuration; DAW-facing audio owner | Windows audio worker, transport, latency readback | Actual instance, negotiated format and inactive reconfiguration | Versioned contracts and bounded capacities; profiles cannot invent live topology |
| Platform capabilities and quirks | Capability observations; platform adapter owner | Acquisition, resolver, admission | OS/CPU, driver, desktop/sandbox and consuming DAW context | Capability adapters; distro-specific handling names a measured need |
| Tested support | Support observations; test/qualification owner | Matrix generator, profile reviewers, UI | Exact builds/configuration/platform/DAW and measured conditions | New measured checks; observations do not publish policy |
| User export/import | Allow-listed settings/results transfer; writer unassigned | User, configuration owner, review tools | Explicitly selected configuration/results; never private payloads | Versioned data exchange; import acceptance/conflict rules open |
| Community recommendations | Submissions and review decisions; review owner unassigned | Profile authors, users | Proposed advice with its provenance and support limits | Review/trust rules open; no automatic promotion to shipped advice |
| Shipped profile publication | Published advice versions and update provenance; writer unassigned | Release, acquisition, resolver | Reviewed shared definitions and their updates | Authoring/trust/versioning/distribution ownership requires review |
| Status and actions | Derived domain view; service projection | UI and CLI | Plug-in/setup/operation, including affected siblings | Named outcomes/actions with a domain owner and user journey |

Vendor, installer and plug-in advice are separate concerns, not additional
runtime forks or duplicate copies of observed state. Their authoring, publication
and update authorities are gaps to resolve, not supplied by the qualification
owner. User sharing, community review and shipped updates are distinct operations.
Requested renderer choices belong to configuration; renderer observations belong
to the assessor. Installer
intent, actual process custody, storage placement and discovered plug-in facts
likewise remain distinct. No domain writes another's facts to make Ready true.

Runtime build owns pinned common sources/patches and recipes; release owns coherent
signed delivery and retained predecessors. The Windows host owns vendor SDK calls,
the Linux proxy owns DAW interfaces and bounded transport, and the supervisor owns
exact process-generation custody and retirement. Tests establish observed behavior;
the UI consumes the service view. Rust owns product policy/lifecycle; C++ stays at
the VST3 SDK and Win32 edges behind a versioned C ABI and process protocol.

Growth scenarios and open decisions for the technical lead:

- Adding a vendor/service dependency changes vendor advice; a genuinely new
  capability changes the installer/service adapter. The Linux proxy stays generic.
  Who resolves dependency versions, shared-service conflicts and readiness facts?
- A shared-profile update changes advice; the configuration owner retains user
  overrides and resolves the result. The launched host does not merge policy.
  How do conflicting vendor/installer/plug-in advice and differently scoped
  overrides resolve? How are older schemas and selected configurations migrated?
- Moving large content changes storage placement and references; audio callbacks
  do no copying or discovery. Which references may move, who owns shared/external
  files, and what authorization and restoration are real for that move or removal?
- Selecting a renderer changes requested configuration; the assessor owns actual
  capabilities/readback and the editor adapter applies it. Installer policy stays
  unaware. Which fallback and override conflicts need an explicit user decision?
- A different DAW channel layout changes live audio negotiation using inspected
  SDK topology; vendor/installer policy stays unaware. Which supported layouts,
  capacity bounds and inactive reconfiguration rules need new edge contracts?
- A distro exception changes the platform capability adapter, not Windows SDK
  calls or a distribution whitelist. Which exceptions are typed configuration,
  which need an implemented adapter, and which require an operator-owned OS change?

Review which facts are stored versus derived and what must change together.
Define keys, scope, override/conflict and migration semantics before implementation.
Assign setup-revision, discovery and publication/admission writers without
assuming the canonical configuration view owns those facts. Decide who authors,
reviews, trusts, versions and distributes profiles, how user imports differ from
shipped updates, and how reviewed advice can be adopted without erasing overrides.
Choose module/API boundaries that keep new typed values separate from new
capabilities without growing a profile dictionary, name branches or more stores.

Converge or replace existing code toward the reviewed owners. Existing modules, schemas
and experimental routes have no authority merely because they exist. Preserve
user identity/state and useful tests when replacing them; quarantine or delete
conflicting execution paths after their replacement is shown working. Do not add
another status registry, campaign ledger or parallel lifecycle engine.

<a id="18-audio-recovery-and-portable-execution"></a>

## Audio, state and recovery

The DAW negotiates processing format and live block sizes. Prepare bounded
capacities while inactive and report actual added latency. The audio callback has
no heap allocation after activation, file/network/process work, ordinary logging,
unbounded lock/wait or dependence on a manager/editor. Every wait states its bound
and expiry result. A dead host must not stall the DAW. Measure the whole plug-in
call as the DAW sees it; a bridge counter is not delivered-audio proof.

Saved projects preserve stable classes, automation IDs, content references and
opaque vendor state. The saved object does not choose an executable or runtime;
the explicitly selected admitted implementation restores or migrates it. Preserve
the input saved object during restoration and use vendor readback rather than invented parameter
mappings. Reopening earlier projects is a release requirement.

Process custody and successful plug-in readiness are different facts. Cleanup
requires exact owned-process retirement and transport closure, including early
startup failure, while preserving independent instances and shared services.
Incomplete custody remains a visible problem. An editor or cleanup success
cannot turn failed audio or state restoration into a successful operation.

## Delivery and acceptance

Recover the known working recipes and provenance before replacing deployed
runtimes. Preserve active backups and verify their scope and restoration before
destructive work; a running copy job is not a verified backup. The working Deck
stays a preserved reference while disposable fixtures establish the replacement.
Changing licensed installations, accounts or original projects needs the
operator's explicit scope under AGENTS.md.

The release journey is install → authorize → discover/connect → play → save and
reopen → restart/reboot → update/rollback → repair → remove. Exercise it on clean
CachyOS and SteamOS fixtures in actual DAWs with exact authorized plug-in builds.
Test the combined patches against their real cases and retain baseline behavior;
runtime build success alone is not qualification. New platforms use the same
capability-based contracts; a distribution name is not an eligibility list.

Land reviewed working increments. Keep current support and remaining failures
truthful in the existing support matrix and failure classes. The immediate
recovery deliverable, owner handoff and done criteria live in CURRENT_SLICE.md.
