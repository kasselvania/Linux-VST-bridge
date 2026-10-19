# Product architecture

This is the sole current architecture. It replaces the 2026-10-16 recovery
contract, retained only as [history](history/ARCHITECTURE-2026-10-17-runtime-recovery.md).
`AGENTS.md` sets working rules. `CURRENT_SLICE.md` names the immediate behavior to
deliver and cannot redefine this architecture or the release scope.

## 1. Product and first beta

The product is a managed library for Windows audio plug-ins used from native
Linux DAWs. A musician installs and authorizes vendor software, sees the resulting
plug-ins in the DAW, plays them, saves and reopens projects, and updates, repairs
or removes them without administering Wine, Proton or prefixes.

Vendor applications such as Native Access are supported because they install,
authorize and maintain plug-ins. This is not initially a general Windows
application or DAW-management platform.

The first closed-beta target is:

- CachyOS with native REAPER and SteamOS with Bitwig;
- the Reference fixtures and Tyrell as repeatable regressions; and
- Native Access through Kontakt and Serum 2 as named commercial journeys.

A claimed combination must complete its ordinary install, authorization,
discovery, playback, usable editor, state recall, restart, supported maintenance
and removal journey. Blackhole and BEAM remain recovery work; preserve their
existing installations, but do not advertise them on a new default runtime until
useful behavior is physically demonstrated there. This is a target, not a claim
that every cell already passes.

## 2. One maintained compatibility line

Wine implements Windows behavior. The Proton/GE-derived runtime packages a
coherent Wine build, supporting components and launcher against a pinned Linux
userspace. The Steam Linux Runtime is that userspace; the Steam client is not a
product prerequisite.

The production maintenance line starts from the known GE-derived reference. The
current r5 generation is the recovery reference, not an automatic beta claim: a
current package must prove it again.

One maintained generation is recommended for new setups. Runtime files are
immutable and shared; every setup records the generation it uses. Older
releases remain while working setups or rollback require them. A manager update
never silently changes a setup's runtime.

New managed setups use product-owned runtime files, independent of Steam-owned
Proton builds, Steam Linux Runtime copies and prefixes. Retained foreign-runtime
setups are migration inputs: package updates preserve them, their own launch
admission reports changed files, and ordinary product operations move them to a
product-owned runtime. A runtime changed outside the product is a condition on
its affected setups; it never blocks a product update or activation or stops
unrelated setups.

A new plug-in failure does not create a private engine. It first exercises
existing capabilities, then scoped profile advice, then—only when source evidence
requires it—a reusable runtime correction. A second maintained lineage requires
an explicit product and support decision.

The r6 construction is parked as experimental evidence. It changed both the Wine
lineage and Proton launch composition; it need not be salvaged. Measure the pinned
giang fork against its documented upstream base before judging its size or
coherence, and retain its working results as evidence.

Wine-Staging's DirectComposition series is the preferred first integration
candidate for the GE-derived line. It enters the default only after the real
editor renders and the existing baseline still passes. Graphics composition is
part of that capability: a giang path using Wine builtins and a staging path using
DXVK's DXGI are not interchangeable. Profiles may select only a path shipped and
validated by the selected runtime. Do not reuse r6 DirectComposition results until
the actually loaded DLLs are established.

Runtime build records retain pinned upstream source, the complete patch series,
build invocation, notices and output identities. Executable and matching-source
delivery may be separate where licences permit, but their release relationship
remains exact. Do not graft loose DLLs from another Wine lineage into production.

## 3. Runtime, setup and live instance

These are different objects:

- **Runtime:** shared immutable compatibility code.
- **Setup:** one mutable Windows installation environment—vendor apps, plug-ins,
  services, registry and setup-owned dependencies.
- **Live instance:** one DAW plug-in instance with its own Windows host process,
  transport and VST state.

Products that genuinely share a vendor hub or service family normally share a
setup. Independent or unfamiliar installers normally receive a new setup. A
vendor name alone neither requires nor forbids sharing; the installation and
service relationship decides it.

Default execution is one Windows host per live DAW instance. Two Kontakt
instances may share the Native Instruments setup and its services, but not their
host process or live state. Instances from different setups may run simultaneously
through the same runtime generation.

Retiring one instance stops that instance and transport, not siblings or unrelated
setups. Installer, uninstaller, dependency or runtime maintenance is exclusive to
the affected setup and waits for its users to stop. A running setup never mixes
runtime generations. Service conflicts are reported rather than hidden by creating
another prefix.

Stable DAW class and automation identities, content references and opaque vendor
state survive supported changes. A prefix copy alone does not promise rollback of
activation, accounts or external data. Capacity is generic finite reservation of
instance, transport and memory resources; campaign- or product-specific class
limits are historical constraints to remove.

## 4. Profiles are scoped data

Profiles select implemented mechanisms. They contain no shell commands, arbitrary
executable hooks, binary patches, hidden downloads, credentials or licensing
material.

The catalog has three concerns:

- **Vendor/application:** setup-sharing family, hub, services, dependencies,
  content conventions and managed application launch behavior.
- **Installer:** applicable installer family/build, supported launch options,
  preparation and completion/handoff behavior.
- **Plug-in:** applicable module/classes, renderer or SDK workarounds and known
  operating restrictions.

Every option declares its scope:

- **runtime:** compiled Wine functionality and component versions;
- **setup:** dependencies, services and shared registry/files;
- **process:** supported arguments, accessibility and renderer/DLL selection; or
- **instance:** live VST state and negotiated processing configuration.

A process option cannot mutate setup-wide files, and profile data cannot invent a
missing runtime capability. Wider conflicts are resolved before launch or
maintenance.

Resolution is ordinary product logic:

```text
selected runtime capabilities
+ observed setup and module
+ applicable profile defaults
+ explicit safe user choices
= validated setup/launch plan
```

The service stores the selected local configuration. A catalog update does not
silently reinterpret a working setup. The supervisor executes the plan and reports
what actually ran; it does not merge policy.

Portable advice, local installation/configuration and tested support are separate.
An unfamiliar plug-in may use the generic path; lack of a support claim is not a
permission gate. Sanitized successful adjustments may become reviewed profile
updates without exporting prefixes, credentials, activation or proprietary state.

## 5. Service, supervisor and bridge boundaries

The control service owns setups, profiles, installation, discovery, publication,
maintenance and musician-facing status. The native Linux VST3 proxy is the DAW
SDK edge. The Windows host loads one Windows VST3 instance. The bridge transports
audio, events, state and editor/lifecycle calls between them; the native DAW owns
the audio device, so ordinary processing introduces no Wine/ASIO device path.

The host supervisor stays outside the Wine application container so it retains
host PID, pidfd, `/proc` and graphical-resource responsibilities. It receives a
closed launch plan and owns exact processes, services and cleanup. It does not
choose a runtime or compatibility policy.

Until moved function-by-function to Rust, ship `session.py` and `ownership.py` as
source in the signed package with a declared supported host Python range. Do not
ship bytecode coupled accidentally to a distro CPython minor. A bundled interpreter
is the fallback if an acceptable host floor cannot be declared. Do not move the
supervisor across a namespace boundary merely to obtain an interpreter.

Installer completion comes from an exact retained process handle and structured
result. Wine debug text is diagnostic only. Installer completion, usable
application, vendor authorization and plug-in readiness are separate outcomes.

Rust is the target product language. C++ stays at VST3/Win32 edges behind
versioned contracts. Existing Python is a bounded migration, not a place for new
product policy. UI and CLI consume the same service view and do not write setup,
publication or process facts directly.

On the audio callback path: no filesystem, network or process work, profile
resolution, ordinary logging, allocation after activation or unbounded wait. Any
synchronous cross-process wait has a stated bound and measured result. Existing
instances do not depend on the manager or editor remaining open.

## Regression, records and authority

One mixed-use workload is the regression for this runtime/setup model:

- two instances of one instrument from the same setup, with distinct state;
- one plug-in from another setup;
- simultaneous playback and editor open/close;
- removal and reload of one instance while siblings continue; and
- save, full DAW exit and fresh-process recall.

Use one workload specification with separate native REAPER and Bitwig projects.
Runtime or host changes rerun it. A changed installer, renderer, state or other
capability also runs its focused journey; this regression is necessary, not
sufficient.

`docs/SUPPORT_MATRIX.md` contains only physically demonstrated claims.
`docs/FAILURE_CLASSES.md` is a concise map of open shared defects. Historical
campaigns and superseded designs live under history or closed PRs and do not
govern. New production modules, tools and CI use durable responsibility names,
not campaign codes.

Implement ordinary source, test and internal-structure decisions within this
architecture. Review before applying a change to runtime lineage, setup-sharing or
process boundaries, saved-state/DAW identity guarantees, licensed data or the
closed-beta scope.

The current assignment is [#465](https://github.com/kasselvania/Linux-VST-bridge/issues/465): restore Native Access through Kontakt on current `main` using r5 in a
fresh CachyOS account, comparing with the retained working package at the first
difference. Then run the mixed-use regression. No further consolidation or beta
qualification begins before that working path is recovered.
