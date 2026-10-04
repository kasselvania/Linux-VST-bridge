# Architecture

**Status:** Provisional repository architecture accepted for design seeding. It governs slice selection unless amended, but it does not claim implementation.  
**Primary target:** Native Linux DAW loading an exact native proxy for a supervised Windows VST3 module.  
**First real fixture:** Serum 2 VST3 in Bitwig Studio Flatpak on the maintainer's Steam Deck.

The operator selected audio recovery on 2026-10-02 and a whole-platform assessment
on 2026-10-03. Section 18 records the current production design and takes precedence
over provisional choices below where specified. The original proof questions remain
historical context. The [platform assessment](PLATFORM_ARCHITECTURE_REVIEW.md) traces
current implementation gaps; SUPPORT_MATRIX.md and retained evidence bound observed
support. A selected design is not an implementation or qualification claim.

## 1. Architectural ruling

The system has four independently versioned planes:

```text
Management plane
  install, authorize, scan, organize, profile, publish, update, diagnose

Native host plane
  Linux VST3/CLAP proxy loaded by the DAW

Windows plug-in plane
  Windows host process loading the proprietary module under a pinned runner

Transport plane
  versioned control/callback protocol + preallocated real-time audio/event memory
```

No plane may impersonate another. The manager does not enter the audio callback. The native proxy does not administer prefixes. The Windows host does not choose compatibility policy. The transport does not interpret user-facing product state.

## 2. Language ruling

### 2.1 Rust owns the product

Rust is the primary implementation language for:

- manager application and CLI;
- canonical product-state model;
- environment and runner registry;
- installation transaction supervision;
- process broker and watchdog;
- authorization-session coordination;
- compatibility profile parser/validator;
- host registration and publication;
- scanner orchestration and normalized census storage;
- protocol framing and version negotiation;
- shared-memory allocation/lifecycle;
- diagnostics and redaction;
- update/rollback transaction logic;
- future user interface unless evidence selects another boundary;
- test harnesses and deterministic fixture controllers.

Reasons:

- strong ownership and concurrency discipline around many process and lifecycle states;
- memory safety in a system that handles untrusted vendor processes and binary metadata;
- excellent Linux systems support;
- straightforward static/small binary distribution for manager/broker tools;
- existing project familiarity and maintainability;
- clear serialization, schema, testing, and CLI ecosystems;
- ability to share exact state and protocol types without making the plug-in ABI itself a Rust experiment.

### 2.2 C++20 owns the VST3 SDK edge

C++20 is permitted for two narrow shells:

```text
native-vst3-proxy-cpp
windows-vst3-host-cpp
```

They own:

- official VST3 SDK includes and helper classes;
- factory entry points and bundle/module glue;
- COM-style interface querying and reference counting;
- exact VST3 object proxy/host adapters;
- platform view/window structures at the SDK boundary;
- conversion between SDK objects and the project's closed C ABI / IPC messages.

Reasons:

- VST3's object model is COM-like, interface-rich, C++-defined, and dynamically queried;
- the official SDK, examples, validator, and host utilities are C++;
- existing pure-Rust bindings are useful research but are incomplete/older and can carry incompatible licensing or coverage assumptions;
- first-product risk is lower when the format owner’s native boundary stays in its documented language;
- C++ can be contained without making the manager or protocol C++.

This is not a declaration that Rust cannot implement VST3. It is a risk-boundary decision for the first clean implementation.

### 2.3 C ABI between Rust and C++

The in-process Rust/C++ seam is a small `extern "C"` ABI containing only:

- fixed-width scalar types;
- opaque handles;
- pointer + length borrowed buffers with declared lifetime;
- explicitly versioned POD structs;
- callback tables with declared thread affinity;
- result/error enums;
- create/retain/release functions;
- no exceptions crossing the boundary;
- no STL or Rust-owned layout crossing the boundary;
- no implicit allocation ownership.

Candidate shape:

```c
struct lab_abi_version { uint16_t major; uint16_t minor; };
struct lab_bytes_view { const uint8_t* data; size_t len; };
struct lab_owned_bytes { uint8_t* data; size_t len; void (*release)(void*, uint8_t*, size_t); void* ctx; };
struct lab_result { int32_t code; uint32_t detail; };
```

The exact ABI is selected in a bounded slice. The law is that object ownership and thread requirements are explicit.

### 2.4 CLAP posture

CLAP has a stable C ABI and explicit thread requirements. A later CLAP proxy/host can likely be predominantly Rust using current bindings, while sharing manager, broker, environment, transport, and diagnostics contracts. CLAP is not implemented merely because the core is format-neutral.

### 2.5 UI toolkit remains open

No GUI framework is architecture authority yet. Initial management proof should expose a typed CLI and structured event stream. A later UI can be native Rust, webview-based, or another reviewed choice. Product-state truth remains in the Rust service/core, not a frontend store.

## 3. Logical system

```text
+---------------------------------------------------------------+
| Manager / CLI / future UI                                     |
|                                                               |
| environments · runners · profiles · install · auth · scan     |
| publication · content · presets · diagnostics · rollback      |
+------------------------------+--------------------------------+
                               | typed local management API
                               v
+---------------------------------------------------------------+
| Broker / supervisor                                           |
|                                                               |
| exact launch · process groups · watchdog · health · cleanup   |
| protocol negotiation · shared-memory grants · editor routing  |
+----------------------+----------------------+-----------------+
                       |                      |
          control/callback IPC               | process lifecycle
                       |                      |
+----------------------v----------------+     v
| Native Linux proxy inside DAW         |   +--------------------+
|                                       |   | Audio runner       |
| VST3 factory/classes/components/views |   | Wine/Proton-derived|
| exact host callbacks                  |   | immutable revision |
| audio/event shared-memory endpoint    |   +---------+----------+
+----------------------+----------------+             |
                       |                              v
                       |                  +------------------------+
                       +----------------->| Windows VST3 host      |
                                          | module + exact class   |
                                          | Win32 GUI main thread  |
                                          | DSP/event/state side   |
                                          +------------+-----------+
                                                       |
                                                       v
                                          proprietary Windows VST3
```

The manager may be absent while an already published plug-in runs. The broker may be a long-lived user service or launched on demand, but its API and lifetime are explicit.

## 4. Canonical state model

Product state is stored in a transactional registry, likely SQLite plus content-addressed manifests in early versions. The exact storage engine is not selected; the model is.

### 4.1 Identity types

Minimum strong identities:

```text
VendorId
ProductFamilyId
ProductBuildId
InstallerArtifactId
EnvironmentId
EnvironmentRevisionId
RunnerId
RunnerRevisionId
WindowsModuleId
ModuleDigest
PluginClassId          # format-defined class identity
ScannerCensusId
ProxyBuildId
ProxyPublicationId
HostInstallationId
CompatibilityProfileId
CompatibilityProfileRevisionId
ContentRootId
PresetRecordId
AuthorizationPostureId
ProjectBindingObservationId
DiagnosticReceiptId
```

Friendly names, paths, and aliases are indexed attributes.

### 4.2 Transactional revisions

Environment, runner selection, profile binding, publication, content relocation, and updates create revisions or event records. Avoid mutable rows whose history cannot explain a working project.

Conceptual event:

```text
EnvironmentRevisionCreated
  environment_id
  parent_revision
  runner_revision
  operation_kind
  installer_artifact
  observed_file_manifest
  safe_registry_manifest
  discovered_modules
  authorization_posture
  content_roots
  created_at
  accepted_at / abandoned_at
```

### 4.3 Readback

Every frontend refreshes from canonical readback after a transaction. A progress UI may stage local events, but it may not assume completion before the service commits the resulting revision.

## 5. Component boundaries

## 5.1 Manager core

Responsibilities:

- canonical state and transactions;
- exact identity resolution;
- user-level workflows;
- compatibility-profile selection;
- installer artifact registration;
- environment creation/snapshot/update/rollback;
- runner inventory and selection;
- authorization-session orchestration;
- scanner orchestration;
- proxy generation/configuration/publication;
- content and preset indexing;
- diagnostics redaction/export;
- compatibility claim state.

Must not:

- run on audio thread;
- implement VST3 interfaces;
- execute arbitrary profile scripts;
- store vendor credentials;
- silently mutate an active working environment.

## 5.2 Runner builder and inventory

The runner is a redistributable Wine/Proton-derived execution environment.

Builder responsibilities:

- exact source and submodule pins;
- reproducible containerized build where practical;
- selected audio-relevant patches/components;
- removal/disablement of game-only assumptions where necessary;
- 64-bit-first binaries;
- license/notice/source-offer material;
- capability manifest;
- signing/digesting;
- smoke tests;
- immutable installation.

Inventory responsibilities:

- install/remove runner revisions without deleting those still bound;
- verify digests;
- report compatibility/profile constraints;
- permit side-by-side revisions;
- mark known-regressed/withdrawn revisions.

The first proof may use an existing Wine or Proton/UMU runner. That runner is fixture input, not automatically the final builder result.

## 5.3 Environment manager

Responsibilities:

- create environment family and revision;
- preserve stable machine identity;
- own prefix path and metadata;
- expose controlled filesystem roots;
- record non-sensitive changes;
- snapshot/clone only under declared policy;
- detect external drift;
- bind exact runner revision;
- coordinate background services/companion apps;
- provide cleanup and orphan detection;
- retire without deleting external content roots.

Default sharing unit is vendor or compatible vendor family, not global and not automatically one-per-plugin. A profile can require isolation.

## 5.4 Installer supervisor

Responsibilities:

- run exact user-provided installer/manager artifact;
- display foreground windows and modal children;
- provide declared network/filesystem posture;
- record process tree and exit outcome;
- observe declared file/registry roots;
- detect installed modules/content roots/services/URL handlers;
- support cancellation and bounded forced termination;
- form a proposed environment revision;
- retain partial-install evidence and rollback.

Installers are untrusted. The supervisor does not infer success from exit code alone.

## 5.5 Authorization broker

Responsibilities:

- classify and present vendor-owned flows;
- open external browser URLs without logging secret query material;
- route declared custom-scheme or localhost callbacks;
- maintain the correct environment process session;
- allow user selection of offline license files without copying them into logs/evidence;
- report vendor-confirmed success/failure/unknown;
- track non-secret continuing requirements;
- preserve machine identity.

It does not parse credentials, automate login forms, intercept TLS, patch licenses, or manufacture authorization.

## 5.6 Scanner service

The scanner runs each unknown module in a disposable supervised Windows process. It has staged operations so a failure can be assigned:

```text
module_open
module_entry
factory_get
factory_info
class_enumeration
class_instantiate
interface_census
bus_census
parameter_census
state_probe
process_setup_probe
editor_probe
module_exit
```

Output is a normalized, versioned census. Raw SDK-specific data can be retained separately.

Scanner rules:

- bounded wall time, CPU, memory, child processes, and output sizes;
- no DAW involvement;
- exact module digest;
- per-stage crash/hang result;
- quarantine on unsafe result;
- deterministic reference modules for positive/negative tests;
- commercial modules are not copied into test artifacts.

## 5.7 Native Linux VST3 proxy

The proxy is an ELF VST3 bundle loaded by the DAW. It contains:

- official Linux VST3 entry points through the C++ shell;
- exact factory/class metadata or a safe cache lookup;
- proxy objects for supported VST3 interfaces;
- broker client;
- protocol negotiation;
- shared-memory audio/event endpoint;
- bounded host-failure posture;
- diagnostic instance identifier;
- no installer/environment/profile UI.

The proxy must be discoverable without starting Wine where practical. Factory/class census is cached and integrity-bound to module/environment/profile identities.

A proxy mapping may represent one class or a module with multiple classes. The external bundle structure must preserve VST3 host expectations and stable class IDs.

## 5.8 Windows VST3 host

The host is a Windows executable running under the runner. Responsibilities:

- load exact module path/digest from the bound environment;
- call module entry/exit;
- expose factory/class operations;
- instantiate exact processor/controller/view objects;
- query and represent supported interfaces;
- enforce Win32 main-thread/message-loop obligations;
- map host callbacks to protocol callbacks;
- access shared-memory audio/event buffers;
- own plug-in-created child windows/popups;
- report crash/exception/exit state through supervision;
- unload cleanly where the plug-in permits.

VST3 modules can expose multiple classes. A module-level host process may be required to preserve module semantics. Instance/process grouping is a declared policy, not improvised per call.

## 5.9 Broker and process supervisor

The broker owns:

- proxy authentication to local product state;
- exact mapping from proxy/class to environment/runner/module/profile;
- protocol version negotiation;
- Windows host process creation;
- process-group membership;
- shared-memory region creation and grant;
- watchdog/heartbeat;
- editor-session routing;
- crash classification;
- orphan cleanup;
- non-real-time diagnostics;
- shutdown/idle policy.

The proxy does not directly run arbitrary `wine` commands.

## 5.10 Host publisher

Responsibilities:

- discover supported DAW installations;
- run native-proxy probe;
- select publication mode;
- create/remove exact proxy bundles and metadata;
- verify host-visible paths inside sandbox;
- trigger or guide rescan safely;
- retain host registration;
- migrate Linux-facing bundles across Flatpak runtime changes without mutating Windows environments.

## 5.11 Preset/content service

Responsibilities:

- register exact content roots;
- index supported metadata without rewriting vendor files;
- distinguish factory/user/external/DAW/bridge snapshot classes;
- support user tags/favorites as separate data;
- observe missing/relocated content;
- invoke vendor-supported locate flows;
- retain state-snapshot compatibility metadata;
- avoid indexing paid content into distributable artifacts.

## 5.12 Diagnostics service

Responsibilities:

- structured events with correlation IDs;
- per-layer health state;
- real-time counters written without callback logging;
- crash and timeout summaries;
- environment/profile/version manifests;
- redaction and allow-list export;
- evidence bundles for exact fixtures;
- no full-prefix default export.

## 6. VST3 technical constitution

## 6.1 Factory and class identity

A VST3 module exposes a factory and can export multiple classes. The system preserves:

- module identity and digest;
- factory metadata;
- class IDs exactly as reported;
- class categories/subcategories;
- component/controller linkage;
- class lifecycle and supported interfaces.

A friendly product name cannot be used to regenerate or substitute a class ID.

## 6.2 Interface proxying

The bridge uses explicit supported-interface adapters. For each interface:

- detection/query behavior is defined;
- object ownership/reference counting is defined;
- method thread affinity is defined;
- request/response and callback direction are defined;
- reentrancy is defined;
- payload limits are defined;
- unsupported behavior has a result.

Avoid one untyped “invoke interface method” protocol. Typed protocol messages allow coverage review and safer evolution.

## 6.3 Reentrancy

Host-to-plugin calls can cause plugin-to-host callbacks before the original call returns. Editor size/focus/context operations are especially recursion-prone. The transport needs:

- call IDs and parent call IDs;
- multiple in-flight calls;
- separate direction lanes;
- ability to service callbacks while a caller waits;
- thread-affinity dispatch queues;
- cycle/depth limits and diagnostics;
- no lock ordering that assumes strict request/response alternation.

## 6.4 Thread affinity

Classes:

```text
DAW audio thread
DAW main/GUI thread
proxy worker/control threads
broker event loop
Windows host main/Win32 GUI thread
Windows plug-in audio thread
Windows host callback/control threads
scanner process/thread classes
```

Every protocol method declares allowed caller and required execution class. Crossing to the main thread is asynchronous or uses a bounded recursion-safe dispatch path as required by the SDK/host.

## 6.5 Component/controller/state

Processor and edit controller may be separate objects. The bridge preserves:

- connection points and messages;
- component state;
- controller state;
- parameter IDs and normalized/plain conversion;
- units/program lists;
- host context and restart notifications;
- bus activation/arrangement;
- latency/tail changes;
- state blob ordering and size bounds.

State acceptance requires byte-level round-trip evidence for deterministic fixtures and behavioral/project evidence for commercial fixtures.

## 6.6 Editor

First mode: detached top-level Windows editor associated with one exact plug-in instance.

Required features:

- platform/view support census;
- Win32 message pump on correct thread;
- open/close/reopen;
- initial and resizable size;
- scale/DPI classification;
- keyboard/mouse focus;
- child popups/menus/tooltips;
- always-on-top/transient relation to DAW where feasible;
- failure independent from processor;
- clean destruction.

Embedded X11/XWayland or rendered/captured editor modes are separate architecture amendments/slices.

## 7. Transport architecture

## 7.1 Channels

Minimum conceptual lanes:

```text
lifecycle/control: proxy -> Windows host
callbacks: Windows host -> proxy/DAW
per-processor real-time control/event synchronization
audio shared memory
editor/window control and asynchronous events
health/diagnostics: Windows host -> broker
```

One underlying Unix-domain socket may multiplex lanes only if reentrancy, ordering, and thread isolation are proven. Logical lanes remain separate.

## 7.2 Protocol framing

Every frame includes:

```text
magic
protocol major/minor
message kind
flags
connection/session ID
instance/object ID
call ID
parent call ID
payload length
optional integrity field
```

Rules:

- bounded payloads;
- unknown optional messages can be rejected or skipped by version law;
- incompatible major versions fail before plug-in lifecycle;
- timeouts are operation-class-specific;
- diagnostics refer to IDs, not raw secret paths where avoidable;
- serialization cannot allocate unboundedly from hostile lengths.

Exact codec remains open. Candidate evaluation should compare a hand-rolled fixed schema, Cap'n Proto, FlatBuffers, or another bounded approach. Product code must not bind architecture to a convenience crate without review.

## 7.3 Audio shared memory

A session negotiates maximum block size, sample format, channel capacity, event capacity, and queue depth before activation.

Regions are:

- created and sized outside audio callback;
- memory-mapped on both sides;
- prefaulted/touched where possible;
- laid out with cache and false-sharing awareness;
- addressed through validated offsets rather than raw cross-process pointers;
- versioned;
- closed only after both sides stop using them.

Control synchronization may use futex/eventfd/Unix socket or Wine-compatible primitives proven by experiment. The signal is not itself the audio payload.

## 7.4 Real-time failure posture

For each process call:

- deadline derived from sample rate/block size and host context;
- bounded wait;
- timeout counter outside callback;
- declared output on failure;
- no process restart inside callback;
- broker asynchronously marks/restarts/quarantines according to policy;
- DAW receives appropriate restart/latency/state signals only from safe threads.

## 7.5 Events and automation

Event transport preserves:

- sample offsets;
- note IDs;
- note on/off/poly pressure and other supported events;
- parameter queues and multiple points;
- input/output parameter changes;
- process context and transport fields;
- bounded counts and overflow policy;
- deterministic ordering.

MIDI-only testing is insufficient for VST3 event correctness.

## 8. Process topology and isolation

Candidate default:

```text
one broker per user session
one scanner process per scan attempt
one Windows module host per module/process group
one or more plug-in instances inside that host where module semantics require
separate installer/authorization process sessions
```

Process-group policy is profile- and host-aware:

- per instance for maximal crash isolation where legal/possible;
- per module because VST3 module classes share module process semantics;
- per vendor family only when inter-instance/vendor communication requires it;
- never global by default.

Bitwig's own plug-in sandboxing mode is fixture input. The bridge must not silently defeat it by placing unrelated plug-ins in one Windows process.

## 9. Compatibility profiles

Profiles are declarative and schema-validated.

Conceptual structure:

```yaml
identity:
  vendor: xfer-records
  product: serum-2
  match:
    format: vst3
    module_sha256: ...
    reported_version: ...

runtime:
  supported_runners: [...]
  preferred_runner: ...
  graphics_backend: wined3d | dxvk | auto
  synchronization: ...
  environment_family: xfer
  isolation: module

installation:
  acquisition: user-provided
  expected_modules: [...]
  allowed_child_process_classes: [...]
  dependencies: [...]

activation:
  class: xfer-owned-browser | splice-rto-companion | unknown
  network: first_launch | periodic | required
  browser_callback: ...
  offline_authorization: supported | unknown | no

editor:
  mode: detached
  dpi: ...
  known_popups: ...

content:
  roots: [...]
  relocation_owner: vendor | product | unsupported

claim:
  fixture_matrix: [...]
  verified_capabilities: [...]
  known_limitations: [...]
  evidence: [...]
```

Schema does not permit arbitrary commands. Implementation maps declared capabilities to reviewed code.

Exact profile matching bounds recommendations and support claims; it is not a
prerequisite for attempting an unfamiliar plug-in. Local experiments use the
same implemented operations and publication owner. See section 18.6 for the
preparation and advanced-control contract.

## 10. Flatpak deployment architecture

## 10.1 Host probe first

Each DAW adapter must prove:

- native proxy bundle shape;
- scan path visible inside exact sandbox;
- native dependencies;
- local IPC namespace;
- shared memory;
- process creation or broker route;
- graphics/editor route;
- persistent writable locations.

## 10.2 Development user-path mode

Possible initial shape:

```text
~/.vst3/<project>/<proxy>.vst3
~/.local/share/<project>/state
~/.local/share/<project>/environments
~/.local/share/<project>/runners
$XDG_RUNTIME_DIR/<project>/broker.sock
```

Only exact fixture evidence can accept these paths.

## 10.3 Linux Audio extension mode

A later extension can contain Linux-facing proxy/runtime support built against Bitwig's matching Freedesktop branch. Mutable vendor environments remain in persistent user data visible under declared permissions.

The extension is not the canonical plug-in database; it is one publication artifact.

## 10.4 Host broker mode

If Windows host execution outside the sandbox is necessary, define a narrow authenticated local broker:

```text
start exact binding ID
stop exact instance
open/close exact editor
query health
allocate/grant exact transport region
```

No arbitrary command execution or path launch.

## 11. Preset and content architecture

Canonical metadata never replaces vendor truth.

Content root record:

```text
id
owner/vendor
kind
current URI/path
filesystem characteristics
required layout/version
portability class
sensitive/proprietary class
index revision
last verification
relocation mechanism
```

Preset record points to source and optional user metadata. State blobs are encrypted/sensitive product data where stored and are never committed.

## 12. Security architecture

Threats include malicious/broken installers, plug-ins, companion apps, malformed protocol messages, hostile path lengths, unbounded scanner metadata, local socket spoofing, support-bundle leakage, and destructive repair.

Minimum controls:

- local broker authentication/capability token scoped to exact binding;
- Unix socket permissions under user runtime directory;
- length/offset validation;
- scanner resource limits and timeouts;
- process-tree ownership and cleanup;
- allow-listed environment observation/export;
- redaction tests;
- no arbitrary profile code;
- signed/digested runners and proxy builds;
- user confirmation for network/filesystem/destructive operations;
- backups before repair/update;
- no false sandbox claims.

Wine is compatibility, not a security sandbox. Flatpak permissions may be broad. Security claims require separate threat-model evidence.

## 13. Diagnostics model

Every operation receives a correlation hierarchy:

```text
transaction_id
  environment_revision_id
  scan_id / publication_id / launch_session_id
    process_id / instance_id
      call_id / audio_counter epoch
```

User errors are stable categorized codes plus plain-language ownership. Raw Wine output is retained locally with redaction controls but is not the only diagnostic surface.

Example categories:

```text
HOST_PROXY_NOT_DISCOVERED
PROXY_PROTOCOL_INCOMPATIBLE
RUNNER_MISSING_OR_TAMPERED
ENVIRONMENT_MACHINE_IDENTITY_CHANGED
MODULE_LOAD_FAILED
VST3_FACTORY_FAILED
VST3_CLASS_NOT_FOUND
VST3_INTERFACE_UNSUPPORTED
AUDIO_DEADLINE_MISSED
EDITOR_PLATFORM_UNSUPPORTED
AUTH_BROWSER_CALLBACK_FAILED
AUTH_RECURRING_CHECK_REQUIRED
CONTENT_ROOT_MISSING
PROFILE_KNOWN_REGRESSION
```

## 14. Build and repository topology

Planned topology after implementation begins:

```text
crates/
  lab-model/                 canonical IDs/state/events
  lab-protocol/              process protocol schemas and validation
  lab-shm/                   shared-memory transport primitives
  lab-broker/                process supervisor and local API
  lab-manager/               transactions/workflows
  lab-scanner-controller/    scanner orchestration/census normalization
  lab-profile/               compatibility profile schema/validation
  lab-host-publisher/        DAW/Flatpak publication adapters
  lab-diagnostics/           structured events/redaction/export
  lab-cli/                   first user/admin surface

cpp/
  native-vst3-proxy/         official Linux VST3 SDK shell
  windows-vst3-host/         official Windows VST3 host shell
  vst3-test-fixtures/        redistributable deterministic fixtures

schemas/
  protocol/
  compatibility-profile/
  evidence/

compatibility/
  proposed/
  verified/
  withdrawn/

evidence/
  fixtures/
  benchmarks/
  matrices/

tools/
  runner-builder/
  fixture-capture/
  support-bundle-inspector/
```

This is planned source topology, not a requirement to create empty crates before a slice owns them. Add only the smallest components required by the active claim.

## 15. Test architecture

### 15.1 Deterministic open fixtures

Create or use redistributable modules that deliberately expose:

- one normal instrument/effect;
- multiple classes in one module;
- separate component/controller;
- parameter/unit/program lists;
- state round-trip;
- variable buses;
- sample-accurate automation;
- editor resize and popup;
- reentrant callbacks;
- controlled crash/hang;
- malformed counts/large state within safe test limits;
- slow processing/deadline misses.

### 15.2 Commercial fixture harness

Commercial binaries remain local. Tests consume a local fixture descriptor containing hashes/versions, not the binary. Results retain no proprietary state beyond safe hashes/metadata.

### 15.3 Matrix dimensions

Where relevant:

- kernel/distro/session;
- native vs Flatpak DAW and runtime branch;
- Bitwig version and sandbox mode;
- runner revision;
- module build/license channel;
- sample rate/block size;
- audio channel/bus arrangement;
- single/multiple instances;
- editor open/closed/resized/reopened;
- online/offline authorization posture;
- content internal/external/missing/relocated;
- update/rollback;
- process crash points;
- reboot/project reopen.

### 15.4 Claim levels

```text
protocol_unit
open_fixture_exercised
commercial_fixture_observed
commercial_fixture_exercised
project_recall_exercised
exact_matrix_verified
multi-matrix_supported
```

Do not skip labels.

## 16. Architectural nonclaims

- The exact C ABI is not yet designed.
- The exact protocol codec is not selected.
- The exact runner base is not selected.
- The exact broker placement for Bitwig Flatpak is not selected.
- The exact database is not selected.
- The UI framework is not selected.
- Embedded/streamed editor modes are not selected.
- Audio deadline targets are not yet measured.
- Profile schema is conceptual until a slice owns it.
- VST3 interface coverage is not known until scanner/proxy slices enumerate it.
- Serum 2 and Kontakt compatibility are unproved.

## 17. First architecture decisions that implementation must prove

The first implementation sequence should produce evidence for these exact questions:

1. Can the current Bitwig Flatpak discover and instantiate our native Linux VST3 probe from a user-controlled path?
2. Can a project-owned broker and Windows host communicate across the exact sandbox boundary without broad host escape?
3. Which runner can lawfully install/launch the maintainer's exact Serum 2 license channel, and what authorization/window/content behavior occurs?
4. Can the Windows scanner produce a stable exact VST3 factory/class/interface census for both an open reference module and Serum 2?
5. Which VST3 interfaces are required by Bitwig and Serum 2 before audio processing begins?
6. Which synchronization primitives remain bounded and reliable through Wine on the Steam Deck fixture?
7. What detached-editor window and DPI behavior occurs under the exact session type?

Until answered, later architecture remains bounded intention rather than implementation fact.

## 18. Audio recovery and portable execution

These decisions implement the operator's recovery direction at the design level.
They do not claim a new installed implementation. The [roadmap](AUDIO_RECOVERY_ROADMAP.md)
sets the sequence, the [platform assessment](PLATFORM_ARCHITECTURE_REVIEW.md) identifies
current code and gaps, and [Integrated beta delivery](INTEGRATED_BETA_DELIVERY.md#acceptance-method)
owns the release acceptance contract.

### 18.1 Product ownership

| Owner | Responsibility |
| --- | --- |
| DAW | Audio device/session, transport, negotiated format, automation, project persistence and delay compensation. |
| Native proxy and Rust backend | Faithful SDK adaptation, bounded audio/event transfer, actual-block processing, result presentation, stream epochs and truthful latency. |
| Windows SDK host | Exact module/class, vendor lifecycle and state calls, Windows editor thread and actual render thread, respecting vendor/SDK concurrency. |
| Manager and supervisor | Capability observation, reviewed policy selection, pinned runtime/environment/publication identities, preparation, process custody, recovery and rollback outside audio callbacks. |
| Proton-derived runner and Linux runtime | Windows compatibility, selected graphics/synchronization components and a controlled userspace, with effective host integration verified. They do not own DAW semantics or bridge presentation deadlines. |

Keep Rust as the product language and C++ at the SDK edges. Preserve the existing
versioned ABI, protocol, exact component pairing, ownership and epoch protections.
Do not add a second lifecycle framework or a separate host architecture to work
around an unmeasured failure in those boundaries.

### 18.2 Capabilities and stable identity

The manager observes host capabilities, matches them to declarative plug-in and
runtime requirements, and prepares one explicit execution configuration. A distro
name identifies a test target; it does not substitute for capability probes.

Bounded observations cover architecture and available CPU resources, effective
scheduling permissions and quotas, kernel/runtime synchronization support,
graphics renderer/driver and desktop route, native or sandboxed DAW integration,
audio/IPC access and storage needed by the selected operation. Record observation
source and freshness. Read effective scheduling on the native worker and Windows
render thread after launch. A launcher preference alone is not evidence.

Keep capability eligibility separate from exact-fixture qualification. Missing
required capability has an actionable failure; an unfamiliar but eligible system
remains unqualified until exercised. Preserve identity, cleanup and integrity
refusals. Never mark a system supported solely because its probes pass.

The capability snapshot is not vendor machine identity. Preserve the licensed
environment identity across observation refresh, software update and ordinary
DAW reconfiguration. Never recreate a prefix to refresh a hardware assessment.
Live DAW format facts come from the processing instance; they remain unknown
while absent rather than being inferred from PipeWire defaults or device settings.

### 18.3 Prepared processing configuration

One prepared instance configuration binds the exact publication/runtime/profile
to sample rate `Fs`, maximum block `M`, precision `P`, process mode, bus/event
capacity, delivery mode, bridge delay `D` and vendor latency `L`. The callback
receives actual length `N`, where `0 <= N <= M`; `N` is not a new configuration.
Allocate for `M` while inactive. Preserve event offsets, parameter curves,
transport context and returned results when adapting any block.

The native float32 interface and existing advertised format limits remain truthful
until successors are implemented and qualified. A rejected format must not be
silently substituted. Ordinary changes follow legal host stop/deactivate/setup/
activate/start transitions, retire old work and reuse the existing stream epoch
mechanism. They do not require installing or republishing the plug-in.

Keep `setProcessing` lightweight, including when called on the processing thread.
Move preparation, process launch and state I/O outside that transition and the
audio callback. State/control scheduling must respect vendor thread safety while
allowing valid audio to meet its deadlines. Do not introduce concurrent state
restoration and DSP as a shortcut around control contention. Distinguish a
recoverable host-input error from a dead or corrupt transport.

Report `D + L` as plug-in latency and expose the two components separately in
the manager. Use the existing processor/controller notification route for latency
changes so the DAW can recompute compensation. These requirements follow the
[VST3 processing interface](https://steinbergmedia.github.io/vst3_doc/vstinterfaces/classSteinberg_1_1Vst_1_1IAudioProcessor.html)
and [setup fields](https://steinbergmedia.github.io/vst3_doc/vstinterfaces/structSteinberg_1_1Vst_1_1ProcessSetup.html);
the pinned SDK and real host transitions remain the implementation test boundary.

### 18.4 Delivery and deadline policy

First restore continuity in the installed queued path. Keep its `D >= M` guard,
position checks, output ownership and stale-result rejection. A sample delay does
not guarantee an equal amount of wall-clock execution time between callbacks.
Test callback bursts and offline processing without artificial pacing.

The selected low-latency design to evaluate is same-callback request/reply over
the existing transport ownership: prepared audio/event memory, an event-driven
handoff to the actual Windows render thread, and a bounded completion policy.
Its target is no added bridge presentation delay (`D = 0`): return the current
block's result in that callback, retaining any vendor latency `L`. Transport and
processing still consume time; this is not a zero-overhead claim. Merely accepting
64-frame host calls while retaining a 512-frame bridge delay does not meet it.
Select and measure the wake mechanism before promoting it. This is a DAW-execution
qualification target, not an implemented or accepted fast path. Keep queued mode
as an explicit buffered compatibility option only if it proves independent value.

Separate three bounds: startup/control timeouts, transport failure containment,
and audio completion deadlines. The current worker's five-second reply timeout
must never become a synchronous DAW-callback wait. A new callback completion
budget must be declared for its format and processing mode before testing, with
defined silence/failure and asynchronous recovery if it expires. No callback
allocation, filesystem/network operation, ordinary logging, manager wait or
unbounded lock acquisition becomes permissible through this design.

Bounded, preallocated observations must attribute a missing span to its actual
stage: admission, queue/control wait, worker service, Windows wait/DSP, reply
validation, publication or presentation. Export away from processing. Acceptance
uses diagnostics disabled and actual captured output, with observer effects
measured separately. Existing phase counters remain evidence, not a replacement
for delivered signal.

The 2026-10-03 audio-completion workstream selects the following implementation
contract; its source, installed and physical acceptance remain separate gates:

- The existing class performance preference carries typed Buffered or SameCallback
  delivery and remembered buffered frames. Missing/legacy records mean Buffered.
  Switching modes preserves that value. SameCallback has effective D=0, while
  Buffered retains D>=M. Neither mode changes DAW capacity, rate or vendor latency.
  Exact paired native/Windows capability is verified on selection and execution;
  restoring an older unsupported publication requires an explicit supported choice.
- Real-time and prefetch may alternate per call without setup. Crossing offline
  requires legal inactive setup. Carry actual mode through the C ABI and PROCESS
  message; do not reuse the setup mode as a substitute. Every operation has an exact
  completion ticket, including consecutive zero-frame calls at the same position.
- A real-time/prefetch completion allowance ends N/Fs after native C++ callback
  entry. In either delivery mode N=0 instead has a 1-ms flush allowance and must
  return its exact supported event/parameter results or explicit failure. Offline
  operations have one 60-second absolute bound across the callback and worker,
  with cooperative cancellation observations no farther than 4 ms apart. Vendor
  process() is not thereby interruptible; storage remains owned until joins or
  existing supervisor containment establish safe retirement. Offline failure is
  explicit, never successful timeout silence. Buffered offline still presents at
  D after completing each operation; the host supplies its own latency/tail calls.
- Session owns a dedicated paired notification connection, isolated from bulk
  state replies. Mailbox publication remains authoritative; hints can coalesce,
  race or duplicate without changing ownership. Native callbacks do no network
  work. A Windows transport pump owns notification and active lifecycle/control
  socket bytes, exchanging bounded prepared handoffs and Win32 events with the
  render thread. The render thread performs no socket reads/writes, including
  Start/Started, control-flag handling and error paths. Vendor state calls retain
  their existing owner and restoration remains exclusive.
- Worker completion waits service already-admitted state capture incrementally,
  retaining its independent deadline. Request/control/capture/fault/quit wakeups
  and control-flag consumption cannot depend on repeated 50-microsecond sleeps.
  Cancellation is published before waiting for native callback leases, then all
  owned workers/pumps are joined or contained before storage release.
- For IPC15 START, the same Session may publish one ordered mapped request after
  sending START while its exact Started reply is pending. Publication does not
  mean readiness: validate the original session/sequence/epoch acknowledgement
  before accepting any result or releasing another control operation. The pending
  reply has one bounded reader. Cancellation ends native access before releasing
  its local mapping view; the supervisor retains custody of the independently
  mapped Windows endpoint and transport stage until confirmed retirement. It never
  authorizes slot reuse or stage removal from an unconfirmed reply. Windows still
  calls vendor setProcessing(true) before consuming
  AUDIO and retains its existing synchronous acknowledgement handoff. Legacy
  exchanges remain sequential. This removes a request-publication dependency;
  it does not enlarge the callback allowance or establish a timing improvement.
- Buffered presentation storage is sized by retained frames (`D + M`), not by
  assuming that one host call contains a full block. Copy completed extra output
  planes into that prepared history before releasing their transport slots.
  Returned events and automation retain bounded operation packets separately;
  their declared per-callback limits still apply. The native SDK output owner
  supports 512 payload-bearing events per callback, 512 outstanding note-ons,
  and at most 1536 nonempty drain packets followed by an empty read. These are
  independent bounds, not consequences of retained-packet capacity. Exceeding
  them or a host sink's capacity fails explicitly; it cannot authorize dropping
  due results while reporting successful delivery. Inactive reconfiguration must
  prepare the new capacity without depending on the previous maximum block.

This contract uses IPC minor15 with an explicitly versioned bootstrap, mailbox
layout3, notification schema1, admission LVB4 and the separate ap23 C ABI. Their
versions are independent. Legacy admission remains buffered-only and older
executable publications retain their original identities. None of these choices
establishes lower-block qualification, whole-DAW scheduling guarantees or the cause
of earlier physical missing audio. Those require the installed/physical comparisons
in the current workstream.

### 18.5 Runtime selection and production convergence

Retain pinned runtime acquisition, side-by-side revisions and exact rollback.
Preserve normal Proton initialization and the coherent selected runtime, rather
than swapping isolated DLLs or adopting an ambient newer runner. Runtime
operations have implemented, reviewed contracts; selections and local overrides
are declarative data with exact identities and effective readback. An unfamiliar
runner satisfying an implemented adapter may be tried as an unqualified local
configuration under section 18.6 without an exact support-profile entry.

Graphics acceleration and fallbacks are selected and verified per rendering path.
Scheduling/affinity changes require measured benefit and actual-thread readback;
they are not blanket product requirements. Do not change unrelated services or
licensed machine identity to obtain an optimization. Proton's documented
[runtime options](https://github.com/ValveSoftware/Proton#runtime-config-options)
are configuration inputs, not audio qualification.

Consolidate duplicate production settings into the prepared configuration, after
checking callers. Isolate comparison-only controls and preserve useful tests;
delete superseded implementations once their replacement is verified. Historical
AP names alone do not prove code is unused or wrong. Keep app-owned runtime and
reusable prebuilt proxy-engine delivery; no customer Steam, Wine, SDK or compiler
prerequisite. Plug-in metadata and configuration are prepared data, not a finite
catalogue of separately compiled per-module proxies.
Release qualification binds the complete component roster, including retained
predecessors needed by installed projects, to declared platform packages.

### 18.6 General preparation and compatibility experimentation

The operator's 2026-10-02 correction makes an unfamiliar plug-in and an advanced
user's experiment ordinary product cases. Automatic preparation and advanced
controls use one manager-owned configuration and the existing supervised
installation, inspection, candidate, publication and recovery mechanisms. This
is the required design; the complete workflow is not implemented or qualified.

The original dossier already includes runtime dependencies, graphics choices,
advanced overrides and rollback. The restriction to a shipped catalogue of
module-specific proxy binaries contradicts that goal. A compatibility database
helps choose settings and explain evidence; it must not determine which new
plug-in builds are allowed to enter preparation.

#### One configuration, distinct responsibilities

| Concern | Owner and prepared result |
| --- | --- |
| Installed module | Supervised installer and scanner retain exact module/class identities, interface and bus/parameter facts, dependency hints and unresolved requirements. |
| Machine and runtime | Capability assessment records the selected runner, actual platform capabilities, graphics paths, audio/IPC access and effective resource permissions. A distro name is not a policy selector. |
| Suggested setup | The manager combines implemented defaults, applicable profile advice, current observations and explicit local overrides. Every selected value has a reason, scope and observed-or-untested status. |
| Native publication | One precompiled engine per supported platform/ABI reads validated per-plug-in descriptor/configuration data outside the audio callback. Stable DAW class IDs are independent of module digest, renderer and runtime choice. |
| Runtime execution | Existing supervisor and Windows host apply the selected configuration before use and report effective settings. Installation, editor, control and audio retain their thread and failure boundaries. |
| DAW processing | The DAW supplies sample rate, maximum and actual blocks, precision and transport. The bridge prepares supported capacities while inactive and reports actual added latency. Installation does not invent these live facts. |

The configuration identifies the engine/host protocol pair, module/class,
descriptor, runner, environment revision, dependencies, renderer/editor policy,
process policy and explicit overrides. Existing candidate/history records own
these bindings; do not introduce a second candidate database or lifecycle system.
Machine capability observations are not vendor machine identity. Presets,
parameters, saved state and authorization data remain legitimately mutable.

Hashes identify versions and detect stale observations. A vendor update triggers
rescan and preparation of new data; it does not require a new bridge release or
silently inherit the previous build's support claim. An unexplained changed file
must not execute under an old receipt. The user gets a managed refresh/update
operation, with the previous publication retained where its exact bytes remain
available. Never promise to restore a vendor build that is no longer present.

#### Normal use and advanced controls

Normal use is install, assess, try and use. The manager offers a plausible initial
setup and explains a failed stage. Missing historical qualification is visible
but is not itself a refusal. Missing required capabilities, an unsupported
interface or invalid data retain specific failures. Optional editor assessment
cannot make an audio-only plug-in unsupported. Assessment failure preserves
useful discovery results and offers another configuration when feasible.

Advanced controls expose the same configuration with these operations:

| User intent | Product operation and scope |
| --- | --- |
| Try another compatibility runtime | Select an installed revision, acquire one through product delivery, or import a user-supplied coherent runner through an implemented runner adapter. Check its structure and required capabilities; unfamiliar provenance remains explicit without becoming a plug-in whitelist. Keep the previous selection. |
| Change editor rendering | Select a supported graphics backend or fallback, DPI and editor mode. Retain requested versus effective renderer facts and test the actual editor as well as independent capability probes. |
| Add or change a dependency | Run a lawful user-supplied dependency installer, or a declared package acquisition, through existing installation supervision. Record version, source, resulting environment changes and affected plug-ins; rescan afterward. Fonts, redistributables and companion applications need real setup operations, not arbitrary profile scripts. |
| Try library or synchronization options | Apply typed options supported by the selected runtime, with a visible configuration difference and reset-to-default operation. Runtime component replacements form a new coherent runner revision; do not silently replace DLLs in a working shared runner. |
| Change process or CPU behavior | Offer implemented process grouping and bounded scheduling/affinity options. Read actual worker/render-thread policy and measure results. Shared vendor services, quotas and other applications remain accounted for. |
| Investigate system drivers | Report actual driver/API capabilities and a specific unmet requirement. User-level runtime component selection is distinct from changing an OS/kernel driver; a plug-in check never silently changes the latter. |

Reviewed operations and declarative option types are the execution boundary;
they are not a closed list of permitted plug-in builds. A user may try an
unqualified combination through those operations without a maintainer compiling
a proxy or approving a new compatibility profile first. A genuinely new runner
interface or unimplemented operation needs an adapter implementation, with the
missing capability named. Do not disguise it as an unknown-product refusal.

Each control shows whether it affects one instance, all classes of a module,
a shared vendor environment or the whole host. Conflicting per-plug-in requests
for one shared environment must be resolved visibly, not silently applied to
siblings. Precedence is implemented defaults, applicable profile advice, then
explicit local overrides; required interface/capacity constraints are checked
separately. A profile refresh must preserve the user's overrides and show advice
that conflicts with them.

#### Try, compare, keep or restore

An experiment starts from the current configuration and retains its difference
and predecessor. Launch-only options are scoped to the trial. Dependency or
runner changes can mutate shared environment state: determine affected owners
and a viable restore/checkpoint strategy before applying them. Preserve licensed
machine identity; do not assume copying or recreating a prefix is harmless.
Where a vendor operation cannot be undone, state that fact before the user's
action rather than promising transactional rollback that does not exist.

The manager can suggest the next useful trial from a failed stage and observed
capabilities. Record why that change might help. Compare one relevant change at
a time by default; allow an explicit group of dependent changes and record the
whole difference. A user can run bounded checks or enable the candidate in the
DAW for musical testing before it has a support claim. Ordinary interaction is
not a request to run every diagnostic again.

Bridge buffering remains an explicit class preference. A publication records
its value at publication time; that historical snapshot does not override a
later user choice. Publication restore preserves the current preference and
checks the target engine's supported capacity, including its exact retained kit
when the installed package has changed. Unknown capacity refuses before the
publication changes. Restoring graphics settings must not silently alter audio
buffering or require it to equal an obsolete snapshot.

Saved state preserves its producing module digest as provenance. Exact recovery
snapshots and current readback validate that digest against the selected module.
A host project restore is a distinct operation: validate the saved object's
format, integrity, bounds and logical class, then offer its unchanged opaque
state to the explicitly selected and independently admitted implementation.
Only that implementation migrates it. The saved object never chooses an
executable, publication or runtime. A successful new capture identifies its
current producer; the original saved object remains unchanged.

The historical parameter mirror cannot define a successor's inventory or values.
After vendor restoration, use current authoritative readback and validate it
strictly against the selected descriptor. A native controller synchronized before
connection defers that readback until connection; it does not invent positional
parameter mappings. Changed automation IDs remain a separate compatibility claim.

Failure retirement distinguishes orderly native release from native death. A
live consumer still owes the transport handshake. An outstanding `F` failure
notice does not replace the independent `R` retirement acknowledgement: teardown
may consume one notice, but still requires `R` within its original deadline.
Unknown bytes, EOF and timeout cannot prove retirement, and positive cleanup
cannot reclassify the original failed restoration as successful. If the
authenticated native process generation has ended, socket closure, exact Windows-owner cleanup and
retirement of the session's transport establish cleanup without an impossible
acknowledgement to the dead consumer. Unknown identity or incomplete cleanup
remains visible. No reboot or manual ownership-record deletion is a recovery step.

Keep loader, audio, editor, state/recall and cleanup outcomes separately. A
renderer probe is not editor acceleration; editor success is not uninterrupted
audio. Combined editor/automation load requires captured audio and timing
comparison. Keep/restore acts through the existing publication and history owner
after affected processing stops. Failed or interrupted trials leave useful
results and an identified recovery action rather than global unexplained
"bridge not ready" state.

A successful local setup is reusable local configuration, not universal support.
The user can export sanitized settings and results; profiles contain no licenses,
account state, arbitrary commands or proprietary binaries. Wider recommendations
need evidence from the actual conditions being claimed. Reuse observations when
their inputs remain valid and rerun affected checks when module, runtime,
dependency, driver or host conditions change.

#### Preparation history and current integration gap

The following assessment describes source
`9a0766afbf71fbb336c897c98b003b9e4bdda737`, before reusable preparation. It is retained
for the origin of these decisions, not as a new instruction to repeat completed work:

| Existing component | Required treatment |
| --- | --- |
| `tools/mf3/native_builder.py`, `tools/ap8_descriptor.py`, native SDK factory | Replace exact per-module binary selection and compiled metadata with the reusable engine and validated descriptor data. Preserve existing stable class-ID derivation and strict binding checks. |
| `bridge-manager/src/preparation/{model,mod,history}.rs` | Extend the existing candidates, observations and predecessor relationships to carry the coherent configuration; do not recreate them. |
| `bridge-manager/src/graphics_cli.rs` and `graphics/` | Consume the shared assessor during requested preparation/checks. Retain its distinction between import hints, runtime probes and actual editor observations. It currently reports; it does not choose or apply a complete setup. |
| `bridge-manager/src/profiles.rs`, runtime and installer owners | Separate exact support claims from permission to try; provide typed runtime/dependency/override operations without name-based policies or arbitrary command hooks. |
| `bridge-manager/src/preparation_cli.rs` and manager UI | Present recommended setup, advanced differences, trial results and keep/restore through the same owner. Replace the late generic missing-proxy refusal with a working general preparation path. |

The reusable engine, unfamiliar-class preparation, graphics settings restoration
and state-update recovery now have bounded installed reference evidence. They do
not yet provide a complete resolved configuration, generic runtime/dependency
workflow or dependable physical audio. The [2026-10-03 assessment](PLATFORM_ARCHITECTURE_REVIEW.md)
records current source and dispositions. Dependency changes and alternate-runtime
trials extend the same candidate/history path. The full acceptance journey is specified in
[Integrated beta delivery](INTEGRATED_BETA_DELIVERY.md#unfamiliar-plug-ins-and-advanced-compatibility).
Removing the catalogue refusal alone does not complete that journey.

Valve's [Proton documentation](https://github.com/ValveSoftware/Proton#runtime-config-options)
describes per-game runtime overrides, restoring defaults by removing overrides,
and selection of locally built compatibility tools. Those concepts inform the
user workflow; their game launch commands are not the bridge protocol, and
runtime options do not establish audio correctness.

### 18.7 Platform execution convergence

Selected engineering direction from the 2026-10-03 whole-platform assessment,
recorded in D-028. These are implementation requirements. They do not reclassify
current support, the failed Deck run or PR #204.

#### Shared configuration, separate facts

Use existing candidate/history, registration, environment and class-preference
owners. Do not create another configuration database, transaction engine or support
registry. Their shared resolution contract distinguishes:

| Fact | Authority and use |
| --- | --- |
| Observed capability | Bounded observation with consuming context, relevant input identities, freshness and an explicit unknown result. It neither modifies licensed machine identity nor establishes support. |
| Declared requirement | A required or optional implemented interface, runtime, resource or integration capability. Missing required capability refuses the affected operation specifically; missing optional evidence does not become a global refusal. |
| Configuration choice | Implemented defaults, applicable advice and explicit user overrides, in that precedence. Retain source/reason, affected scope, predecessor and whether application needs processing to stop. Required constraints are checked separately. |
| Resolved launch configuration | Exact module/class, descriptor, engine/host/protocol, coherent runner, environment revision/dependencies and applied editor/graphics/process options. Admission and launch consume this same prepared result. |
| Live processing configuration | The DAW's negotiated and actual format under section 18.3. Installation cannot invent it; legal inactive reconfiguration does not require republishing. |
| Effective behavior | Readback of actual worker policies, applied launch options, actual editor observations and measured workload results. Requested settings and independent probes cannot impersonate it. |
| Qualification | A bounded support claim for observed versions and conditions. It is advice/status, not a closed permission list for trying eligible unfamiliar inputs. |

The manager, CLI, admission service, supervisor and native publication must agree
on the resolved configuration and its schema. Keep strict executable binding without
binding mutable presets or parameters to an installation digest. Update/restore
checks its affected inputs and supported capacities, preserves explicit unrelated
preferences, and uses retained predecessors rather than guessing another build.
Migrate existing records without changing class IDs or saved objects.

Ordinary configuration readback validates bounded owned records and their current
bindings, including transaction state and the selected physical publication. This
is control-record validation, not a fresh verification of executable payloads.
Present that distinction in readiness and offered actions. Do not traverse complete
runtime trees, hash bulk executable payloads or execute preparation tools merely
to display a product or validate an offered choice. Offer admission still rechecks
the exact token, expected predecessor, settings and affected owners. The queued
worker executable is freshly verified; mutation and launch owners retain deep
execution verification and final binding/ownership rechecks. Neither a readable
record nor a previously successful observation grants permission to execute it.

Incomplete retained history from an older installation stays readable as needing
recovery. An explicit scoped action invokes the existing history-completion owner
after rechecking the selected candidate and recorded history. Projection does not
perform migration or invent missing lineage. Corrupt existing records remain
refusals; completing history does not replace executable admission. Paired clients
must recognize the recovery action's operator schema, while retained earlier
operation records preserve their original schema and bytes.
The shared preparation owner retains valid lineage before publishing a candidate
record. An older incomplete record without authority for its original predecessor
stays an explicit history gap; it cannot acquire invented lineage through readback.
Independently valid publication and exact rollback facts remain available.

The initial implementation uses optional typed local settings on the retained
candidate, preserving the serialized identity of older records when that field is
absent. Preparation resolves graphics and Windows accessibility choices into the
candidate profile. Publication validation, assessment and the launched host consume
the same resulting registration. Default advice is resolved at preparation time;
later advice cannot reinterpret an already retained configuration. Refresh carries
explicit preferences forward and creates fresh configuration evidence.

Preparing a settings trial records an immutable successor without changing a live
publication. Applying or restoring it rechecks the exact predecessor and affected
class under registry admission, including after artifact copying. Buffering remains
an independent class preference and is retained when the predecessor supports it.
The selected publication owns the main configuration readback and Keep/Restore
workflow. A newer preparation is an explicit proposal, not a new selection.
Prepared alternatives remain reachable alongside the selected workflow, with
their own candidate identity and predecessor checks; stale proposals cannot
displace the current selection or borrow another trial's admission result.
The manager distinguishes selected options, their sources and scope, independent
probe results and observed behavior; selecting a graphics option is not evidence
of an editor's effective renderer.

Cache observations by their actual dependencies. Module changes invalidate module
inspection; runtime/dependency changes invalidate affected runtime and compatibility
observations; driver/display/host changes invalidate affected consuming-context
checks. Unchanged evidence can remain reusable. Do not rebuild the engine or recreate
a prefix merely because support prose, an observation or a user preference changed.

#### Capability and ownership scope

Host integration is an adapter contract for native and sandboxed DAWs: observable
loader/architecture, publication visibility, IPC/audio/display access, storage and
resources in the actual consuming context. Distro/DAW names identify evidence and
select a relevant adapter, not universal eligibility. Report adapter absence as
such. Require a display only for an operation or runtime route that needs one.

Maintenance and admission scope follows affected publication, shared environment,
runner/service and host resources. Distinguish structural capacity bounds from
configured limits and tested workload recommendations. Preserve finite reservations
and exact generation/ownership checks. Independent instances must not be stopped
merely because a current implementation uses a global inactivity shortcut; shared
mutations must not proceed until their actual affected owners are inactive.

Class publication and buffering controls permit independent DSP owners while
requiring every instance of the affected class to retire. Their final mutation
uses the service's canonical owner classification: unresolved custody, inspection
and vendor maintenance cannot masquerade as independent DSP. Scanner, installer,
runtime and environment operations retain global exclusion until their narrower
shared-resource contracts are implemented. That remaining restriction is explicit;
it is not evidence of environment-scoped maintenance.

The shared-runtime launch adapter must authenticate the final Windows host, not
assume that its pre-exec launcher remains its parent. The existing per-launch
channel may carry kernel-authenticated writer credentials and a pinned process
handle. Admission additionally binds that writer to this launch's exact readiness
identity and mapped session-status object. A claimed PID, process name, shared
keeper membership or ordinary diagnostic line cannot grant custody. Recheck the
binding on render restart. Unsupported kernel/runtime observation remains explicit;
missing final-host custody cannot become confirmed retirement. Preserve exact
instance cleanup and keeper/sibling isolation, including pre-admission failure.
Close new custody admission before cleanup starts. Diagnostic draining cannot
expand the ownership set after its final retirement signal or turn missing
admission into a successful cleanup claim.

An unfamiliar class with a valid managed publication may reserve the existing
native image slot count under the shared service ceiling. This is a structural
admission limit, not a qualified workload recommendation. Readiness separates that
permission to try from support history. Active native/sandboxed consumer observations
bind to authenticated process generation, mapped proxy and consuming mount paths;
an idle host or missing observation remains unknown without inventing a failure.

Advanced runtime/dependency operations apply through those same owners. An imported
runner must satisfy an implemented coherent-runtime adapter; a new name does not
need a new permission list. Environment mutations expose affected siblings, stable
identity implications and a truthful recovery route before application. OS driver
changes are outside automatic plug-in setup. No arbitrary command or profile-script
interface replaces typed, supervised operations.

#### DAW and editor execution contracts

Implement event-driven request and completion notification over the existing owned
transport. Notification is a hint to inspect authoritative queue/mailbox state;
missed or raced notifications must not lose requests. Preserve epochs, positions,
buffer ownership and bounded data. Compare effective wake/service tails on the
actual threads, not requested sleep intervals or launcher priority.

Real-time completion has an explicit local budget and failure policy; measure whole
SDK callback duration and serial chains before claiming a supported workload.
Offline completion uses bounded interruptible waits with its own progress,
cancellation and containment policy, allowing valid work slower than real time.
It must return the ordered results required by the declared latency/tail contract
through the final host-supplied blocks, or an explicit failure. It must not invent
extra host calls or report success with timeout-substituted silence.
These requirements do not permit an unbounded real-time callback wait. Cover legal
real-time/prefetch mode switches without inactive setup; offline transitions follow
the SDK setup sequence. Advertise current support truthfully until implemented.

Discovery, descriptor, native interface exposure and Windows implementation must
agree on supported buses, parameters, events and host callbacks. Preserve vendor
parameter conversions and reentrant/thread-affine behavior where implemented;
report unsupported required contracts specifically. Generic engine reuse does not
prove full VST3 coverage. Add contracts through both SDK edges and their consumers,
not only by widening parser limits or returning placeholder interfaces.

Failure severity belongs to the operation that can establish it:

| Failure | Required policy |
| --- | --- |
| Optional view operation refused or bounded GUI channel overloaded, with valid owner/transport | Return an editor/control result; safely close/reset that facility where supported. Preserve DSP only when its safety is established. Do not automatically classify it as corrupted audio. |
| Invalid identity/protocol, unsafe vendor state or vendor process crash | Contain the affected instance under existing ownership; preserve saved state and healthy independent siblings. A shared-process editor crash cannot be promised independent containment. |
| Vendor state restore refused or partially applied | Preserve the original error and saved object; safely retire the affected instance and offer the retained predecessor. Never silently use defaults. |
| Native consumer disappears | Reconcile exact process and transport ownership independently of polite SDK teardown. Unproven cleanup remains visible. |
| Audio deadline missed | Record its stage/position and apply the declared processing-mode failure result. A counter, process exit and observer disappearance are different signals. |

Use existing terminal records and control/result protocols; version them when their
semantics change. Product policy belongs in Rust with narrow SDK/OS adapters.
Consolidate production Python/C++ policy as affected owners change, preserving
verified custody and without making a language rewrite the delivery objective.

#### Integrated completion

The next capability is one managed unfamiliar-plug-in configuration from preparation
through ordinary use, meaningful recall and settings restoration. Runtime/dependency
operations and DAW execution then extend that contract; rendering and resource
requirements inform it from the start. The [implementation programme](PLATFORM_ARCHITECTURE_REVIEW.md#implementation-programme-and-completion)
states owner changes and representative acceptance. A new type, probe, narrowly
passing fixture or source-only report does not finish an integrated capability.

Keep the existing commercial catalogue, platform and low-latency goals. Use shared
contract fixtures for unfamiliar identities, SDK semantics, rendering/refusal,
state evolution and owned failures, then validate combined installed workflows on
frozen artifacts. Required release interaction/soak duration, project persistence,
update/recovery and distribution remain governed by Integrated beta delivery.
