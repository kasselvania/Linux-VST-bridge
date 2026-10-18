# Product architecture

This is the sole current architecture for the product. It replaces the
2026-10-16 runtime-recovery contract, which is retained only as
[history](history/ARCHITECTURE-2026-10-17-runtime-recovery.md). `AGENTS.md` sets
working rules; `CURRENT_SLICE.md` names the immediate assignment and cannot
redefine this architecture or the release scope.

## Product

The product is a managed library for Windows audio plug-ins used from native
Linux DAWs. A musician installs and authorizes vendor software, sees the resulting
plug-ins in the DAW, plays them, saves and reopens projects, updates or repairs the
installation, and removes it without administering Wine, Proton or prefixes.

Vendor applications such as Native Access are supported because they install,
authorize and maintain plug-ins. This is not initially a general Windows
application or DAW-management platform.

The first closed-beta target is:

- CachyOS with native REAPER and SteamOS with Bitwig;
- the Reference fixtures and Tyrell as repeatable compatibility regressions;
- Native Access through Kontakt and Serum 2 as named commercial journeys; and
- ordinary install/authorize/discover/play/editor/state/restart/update/repair/remove
  behavior for the combinations actually claimed in `docs/SUPPORT_MATRIX.md`.

Blackhole and BEAM remain explicit recovery work, and their existing working
installations must be preserved. They are not advertised on a new default runtime
until the useful behavior is physically demonstrated there. This scope is a target,
not a declaration that every cell already passes.

## Compatibility stack

| Component | Responsibility |
| --- | --- |
| Wine | Windows APIs, loader, registry, processes, services and graphics behavior. |
| Proton/GE-derived runtime | A coherent Wine build, supporting components, launcher and expected Linux userspace. |
| Steam Linux Runtime | The pinned Linux container/userspace used by that runtime; it is not a requirement for the Steam client. |
| Native Linux VST3 proxy | The DAW-facing SDK edge and real-time transport endpoint. |
| Windows host | Loads one Windows VST3 instance and owns its VST3/Win32 calls. |
| Control service and manager | Setups, profiles, installation, discovery, publication, updates, repair and musician-facing status. |
| Host supervisor | Executes an already selected setup/launch plan and owns exact processes, services and cleanup. |

The native DAW owns the audio device. The bridge carries audio, events, state and
editor/lifecycle calls between the native proxy and the Windows host. Ordinary
plug-in processing does not introduce a second Wine/ASIO audio-device path.

## Runtime strategy

The production maintenance line starts from the known GE-derived reference. The
current r5 generation is a recovery reference, not an automatic beta claim. A
current product package must prove it again.

One maintained runtime generation is recommended for new setups. Runtime files are
immutable and shared; each setup records the generation it uses. Older generations
remain available while working setups or rollback require them. A manager update
does not silently change a setup's runtime.

A new plug-in failure does not create a private engine. It first exercises existing
capabilities, then scoped profile advice, then—only when source evidence requires
it—a reusable runtime correction. A second maintained runtime lineage requires an
explicit product decision and support boundary; it is never the incidental result
of debugging one plug-in.

The r6 construction is retained as experimental evidence, not the production
foundation. It changed both the Wine lineage and Proton launch composition. The
pinned giang source must be measured against its documented upstream base before
being judged, and its working results remain useful evidence.

Wine-Staging's DirectComposition series is the preferred first integration
candidate for the GE-derived line. It enters the default only after the real
editor that needs it renders and the existing baseline still passes. The graphics
composition is part of that capability: a giang-style path using Wine builtins and
a staging path expecting DXVK's DXGI are not interchangeable. Profiles may select
only a graphics path implemented and validated by the selected runtime. Advice
from one DirectComposition implementation must not be reused for another without
that validation. Do not reuse an r6 DirectComposition result until the actually
loaded DLL path is established.

Runtime build records retain the pinned upstream source, complete patch series,
build invocation, notices and output identities. Executable delivery and matching
source delivery may be separate artifacts where the licences permit, but their
release relationship remains exact. Do not graft loose DLLs from a different Wine
lineage into a production runtime.

## Setups and simultaneous plug-ins

The runtime, a setup and a live plug-in instance are different things:

- **Runtime:** shared immutable compatibility code.
- **Setup:** one mutable Windows installation environment, including vendor apps,
  plug-ins, services, registry and setup-owned dependencies.
- **Live instance:** one DAW plug-in instance with its own Windows host process,
  transport and plug-in state.

Products that genuinely share a vendor hub or service family normally share a
setup. Independent or unfamiliar installers normally receive a new setup. A vendor
name alone neither requires nor forbids sharing; the installation and service
relationship decides it.

Default execution is one Windows host process per live DAW plug-in instance. Two
Kontakt instances may share the Native Instruments setup and its services, but
not their live VST state or Windows host. Instances from different setups run at
the same time through the same runtime generation where supported.

Instance retirement stops that instance and its transport, not sibling instances
or unrelated setups. Setup maintenance—installer, uninstaller, dependency change
or runtime transition—is exclusive to that setup and waits for its affected live
users to stop. A running setup never mixes runtime generations. Shared services
have explicit setup ownership and conflicts are reported rather than hidden by
creating another prefix.

Stable DAW class and automation identities, user content references and opaque
vendor state survive supported runtime, package and configuration changes. A
prefix copy alone is not a promise that activation, accounts or external state can
be rolled back.

Capacity is generic reservation of finite instance, transport and memory resources.
Campaign- or product-specific class limits are historical constraints to remove,
not the permanent policy for adding a plug-in.

## Profiles and configuration

Profiles are versioned data consumed by implemented mechanisms. They do not contain
shell commands, arbitrary executable hooks, binary patches, hidden downloads,
credentials or licensing material.

The profile catalog has three concerns:

- **Vendor/application profile:** setup-sharing family, vendor hub, services,
  dependencies, content conventions and managed application launch behavior.
- **Installer profile:** applicable installer family/build, supported launch
  options, preparation and completion/handoff behavior.
- **Plug-in profile:** applicable module/classes, renderer or SDK workarounds and
  known operating restrictions.

Every option has a scope:

- **runtime scope:** compiled Wine functionality and shipped component versions;
- **setup scope:** installed dependencies, services and shared registry/files;
- **process scope:** supported launch arguments, accessibility and renderer/DLL
  selection for a new process; and
- **instance scope:** live VST state and negotiated processing configuration.

A process option cannot silently mutate setup-wide files, and a profile cannot
invent a missing runtime capability. Conflicts at a wider scope are shown and
resolved before launch or maintenance.

Resolution is ordinary product logic:

```text
selected runtime capabilities
+ observed setup and module
+ applicable profile defaults
+ explicit user choices
= validated setup/launch plan
```

Implemented defaults apply first, then applicable advice, then safe explicit user
overrides. The service stores the selected local configuration and reuses it; a
catalog update does not silently reinterpret a working setup. The supervisor
executes the selected plan and reports what actually ran. It does not merge policy.

Portable profile advice, a user's local setup/configuration, and tested support are
separate records. An unfamiliar plug-in may take the generic path; lack of a support
claim is not a permission gate. A sanitized successful adjustment can become a
reviewed catalog update without exporting a prefix, account data, activation or
proprietary state.

## Component boundaries

The control service selects setups, profiles and operations. The host supervisor
runs outside the Wine application container so it can retain its host PID, pidfd,
`/proc` and graphical-resource responsibilities. It receives a closed launch plan
and reports process/service facts; it does not choose a runtime or compatibility
policy.

Until the supervisor is moved function-by-function to Rust, ship `session.py` and
`ownership.py` as source in the signed package with a declared supported host
Python range. Do not ship bytecode that accidentally depends on the distribution's
CPython minor version. A bundled interpreter is the fallback if an acceptable host
floor cannot be declared. Do not move supervision into a runtime it must acquire or
across a namespace boundary merely to obtain an interpreter.

Installer process completion is established by an exact retained process handle
and structured result. Wine debug text is diagnostic only. Installer completion,
application usability, vendor authorization and plug-in readiness remain separate
outcomes.

Rust is the target product language. C++ remains at the VST3 SDK and Win32 edges
behind versioned contracts. The existing Python supervisor is a bounded migration,
not authority for adding new policy there. The UI and CLI consume the same service
view and cannot write setup, publication or process facts directly.

On the audio callback path: no filesystem, network or process work, profile
resolution, ordinary logging, heap allocation after activation, or unbounded wait.
Any synchronous cross-process wait has a stated bound and measured result. Existing
instances do not depend on the manager window or editor remaining open.

## Release regression and claims

One mixed-use workload is the regression for the runtime/setup strategy:

- two live instances of one instrument from the same setup, with distinct state;
- one plug-in from another setup;
- simultaneous playback;
- editor open/close without stopping DSP;
- removal and reload of one instance while siblings continue; and
- save, full DAW exit and fresh-process recall.

Use one workload specification with separate native REAPER and Bitwig projects.
Runtime or host changes rerun this test. A changed installer, renderer, state or
other capability also runs its focused product journey; the mixed-use test is
necessary, not sufficient.

`docs/SUPPORT_MATRIX.md` contains only physically demonstrated claims.
`docs/FAILURE_CLASSES.md` is a concise map of open shared defects, not a campaign
narrative. Historical campaigns, superseded architecture and dead experiments live
under history or closed PRs and do not govern current work. New production modules,
tools and CI are named by durable responsibility rather than campaign code.

## Working authority

Implement ordinary source, test and internal-structure decisions within this
architecture. Review a change before applying it when it alters the runtime
lineage, setup-sharing boundary, process boundary, saved-state/DAW identity
guarantees, licensed data or the closed-beta scope.

`CURRENT_SLICE.md` names the current product behavior to restore or deliver. It
must fit on one screen, state the comparison and completion result, and cannot
create a new architecture, campaign ledger or substitute intermediate evidence for
working musician behavior.

The current assignment is [#465](https://github.com/kasselvania/Linux-VST-bridge/issues/465): restore Native Access through Kontakt on current `main` using the r5
reference in a fresh CachyOS account, comparing with the retained working package
at the first difference. Then run the mixed-use regression above. r6 need not be
salvaged, and no further consolidation or beta-qualification campaign begins before
that working path is recovered.
