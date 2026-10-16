# Architecture

This is the governing product design selected on 2026-10-16. It replaces the
earlier policy of selecting separate production runtimes for different plug-ins.
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

The service owns the canonical selected configuration. Profiles, UI, CLI,
publication and launch all consume that model; none invents a competing selection.
It identifies the release/runtime, exact module/class, vendor setup revision,
dependencies, content locations, renderer and applied options.

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

## One musician status and primary action

The service derives one displayed status and available actions from the canonical
configuration and owned operation facts. The UI presents them consistently.

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

## Logical owners

These are code responsibilities, not a requirement for nine services or new
databases. Rust owns product policy and lifecycle. C++ stays at the VST3 SDK and
Win32 edges behind a versioned C ABI; process boundaries use versioned protocols.

| Owner | Responsibility |
| --- | --- |
| Runtime build | Pinned sources, common patches, reproducible component set and regression recipes. |
| Windows host | Load the exact Windows module; implement vendor SDK, editor, state and audio calls. |
| Linux proxy | Native DAW interfaces, stable class identity and bounded transport; never load a Windows binary. |
| Service | Canonical configuration/status, affected-resource coordination and action admission. |
| Supervisor | Launch, observe, cancel and retire exact process generations and their transports. |
| UI | Musician-facing status, primary action and advanced controls from the service model. |
| Profiles | Typed settings advice and separately identified tested support facts. |
| Release | Coherent signed delivery, acquisition, update, retained predecessors and rollback. |
| Tests | Contracts, combined-runtime regressions and actual installed musician journeys. |

Converge or replace existing code toward these owners. Existing modules, schemas
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
