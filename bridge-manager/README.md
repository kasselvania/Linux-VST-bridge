# Registered bridge startup

This is the installed management core for the current exact Arturia vertical. Rust owns exact environment/runner bindings, registration, publication, service admission, volatile transport allocation, and immutable software selection. The installed Python supervisor owns each Windows host. Neither runs in an audio callback.

This source is not, by itself, commercial or everyday-workflow qualification. See the retained AP12–AP16 evidence for the actual vendor-access, editor, project, and delivery results.

## Setup surface

Build `linux-vst-bridge` for the Linux fixture. `setup PACKAGE` installs a private, immutable software revision containing that executable, `session.py`, `ownership.py`, and the existing `host.exe`. `host-source.json` records the SHA256 of the accompanying `host-source-manifest.json`; both identify the exact host source and build receipt. Setup enables the user service `linux-vst-bridge.service`. The source checkout and build directory are then unnecessary for playback. Updating software requires closed devices and a stopped service; live software updates are not implemented.

The typed CLI also supports:

- `environment-create RUNNER.json`: create a private persistent environment with exact runner entry points and critical-file hashes.
- `environment-import ENVIRONMENT.json`: import an existing bridge-owned setup environment without recreating its prefix or modifying vendor/account state.
- `install ENV_ID INSTALLER_PATH SHA256`: execute a verified normal vendor installer in a bounded user unit, preserving its real foreground UI and exit.
- `inspect INSPECTION.json`: inspect an exact module and selected class in an inactive environment.
- profile-driven managed preview, publication, status, reconciliation, rollback, and unpublish operations.
- bounded vendor-editor and engineering qualification routes where explicitly compiled and admitted.

All record types are in `src/lib.rs`; unknown registration fields, ambiguous SDK roles, missing artifacts, and changed bindings are refused. An installer or standalone inspection requires exclusive environment access. Compatibility profiles may select exact process-scoped behavior such as disabling Windows accessibility for one product host; the shared environment owner does not inherit that setting.

## Playback and ownership

`~/.vst3/LVB_<Windows-class-ID>.vst3` points atomically to an immutable installed native bundle. The SDK processor/controller IDs retain their established UUIDv5 derivation; display names, paths, and build changes do not generate new IDs. Registration stores the installed native artifact, not its input build location.

The registered proxy requests an exact class/module binding from the service outside the audio callback. The service validates the registration and the connecting DAW process, creates a fresh private tmpfs session under `/run/user/<uid>/linux-vst-bridge`, and transfers the exact session binding. The supervisor preserves the pinned Windows host’s durable `C:\bridge\sessions\...` readiness/gate contract while exposing only the fixed native-created hot transport files as verified views into that RAM session.

A bounded environment owner starts Wine’s shared infrastructure before DSP instances and loads no plug-in. Each admitted DSP instance has its own supervised Windows process, transport, controller, state owner, and editor.

### AP17 capacity candidate

`capacity` asks the running owner for canonical version-1 status. It reports live DSP leases, each class, the separate keeper/maintenance owners, current request workers, the native hard ceiling, the enforced admission limits, and the engineering envelope. The current candidate admits six DSP instances globally, at most three of the exact LoFi class and four of the exact FRAGMENTS class. The native hard ceiling stays four per loaded image/process. Six is a conservative engineering selection; project recall and recovery qualification remain in progress.

Six long-lived DSP owners are separate from the 16-worker mechanical service ceiling. One inspection or standalone vendor-access job requires inactive DSP; the environment keeper loads no plug-in and consumes no DSP slot. An exclusive existing registry lock reserves startup before binding/session/lease exposure. Durable owner leases, not an in-memory permit count, retain capacity until exact positive retirement. Unresolved retirement blocks new admission and survives service restart.

The non-RT registered startup uses `LVB3` and receives a bounded correlated `LVR3` success or typed refusal. Global, per-class, native-image, maintenance, cleanup and service-worker limits are distinct. Only an unowned service-busy refusal may retry: at most 64 attempts with 20 ms backoff inside one ten-second startup deadline. Actual capacity refusal, malformed/stale replies, partial bindings and EOF do not retry. The audio protocol and callbacks are unchanged. Old `LVB1/2` binaries retain their historical startup encoding; they do not acquire typed-refusal support from a manager update alone.

`qualify-capacity stage PACKAGE`, `qualify-capacity publish`, and `qualify-capacity restore` reuse the immutable publication transaction for the finite AP17 native candidates. The compiled candidate roster is currently absent pending exact builds. Ordinary managed activation stays verified-only. An AP17 candidate must preserve the exact verified revision-7 parent, Windows host, descriptor, runner, module, class, editor/state/precision/performance constraints and external IDs. Startup/reconcile restores engineering publications to their retained verified parents when inactive. AP15 qualification remains distinct.

## Volatile transport and durable recovery

High-frequency session mappings are intentionally ephemeral:

- `ap1.control`
- `ap1.audio`
- `ap10.delivery`
- `ap11.ui`
- `ap12.status`

They live in private tmpfs and disappear at user-session or machine restart. Durable owner specifications, readiness/gate files, reports, receipts, software, environment, profiles, publications, vendor state, and projects remain in their existing persistent locations.

The manager validates tmpfs type, uid, mode, marker, device/inode, and the actual connecting peer’s mount namespace before exposing a binding. There is no disk-backed fallback. Positive descendant cleanup and native release precede removal of the exact session directories. Failed physical retirement cannot produce a success receipt.

Ongoing Linux tracking uses process identity as PID plus start time rather than names alone. A terminal SDK failure is observed even if a companion keeps an outer launcher alive. Unconfirmed cleanup blocks new admissions and survives service restart through persisted ownership leases; it does not stop already healthy siblings. Unexpected service loss remains an explicit native failure and cannot silently replace DSP or restore guessed state.

## Performance posture

Registered proxies retain 512 added frames and the mailbox delivery path without reading historical preview settings. Optional correlated tracing reads `managed/runtime/trace-enable` at inactive setup. The supervisor passes the same explicit opt-in to the Windows host; inspection and vendor-access jobs do not inherit it.

AP16 moved the hot mappings from journaled environment storage to private tmpfs and repaired one demonstrated 27–29 ms native preparation-stall class. It did not claim gap-free operation. Residual startup, queue/reply, editor/removal, and shutdown-window classes remain tracked in issue #90.

Two serial proxies add 1,024 bridge frames before vendor/device latency. The exact accepted LoFi → FRAGMENTS chain reports 1,264 frames / 26.333 ms at 48 kHz, excluding DAW, device, and acoustic latency.

## Focused checks

```text
cargo test --manifest-path bridge-manager/Cargo.toml --locked
python3 -m unittest discover -s bridge-manager/runtime -v
```

Rust tests cover exact metadata, managed publication/rollback, artifact replacement, software ownership, transport storage, duplicate service ownership, and independent registrations. Linux process tests cover sibling survival, stat-only tracking, terminal host failure with a lingering launcher, native release before transport retirement, and the tmpfs/C: handshake law.

They do not substitute for normally launched Bitwig and actual vendor tests. AP17 must use the real fixture to establish process topology, resource scale, project recall, clean over-limit refusal, and recovery at the selected supported capacity.
