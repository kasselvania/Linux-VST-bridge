# Architecture

**Status:** Provisional repository architecture accepted for design seeding. It governs slice selection unless amended, but it does not claim implementation.  
**Primary target:** Native Linux DAW loading an exact native proxy for a supervised Windows VST3 module.  
**First real fixture:** Serum 2 VST3 in Bitwig Studio Flatpak on the maintainer's Steam Deck.

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
