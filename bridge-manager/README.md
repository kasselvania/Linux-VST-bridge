# Registered bridge startup (AP12)

This is the installed management core for the AP12 vertical. Rust owns exact
environment/runner bindings, registration and publication. The installed Python
supervisor owns each existing Windows host. Neither runs in an audio callback.
This source is not, by itself, commercial or everyday-workflow qualification.
See the AP12 evidence for the actual vendor-access and device results.

## Setup surface

Build `linux-vst-bridge` for the Linux fixture. `setup PACKAGE` installs a private,
immutable software revision containing that executable, `session.py`,
`ownership.py`, and the existing `host.exe`. `host-source.json` records the SHA256
of the accompanying `host-source-manifest.json`; both identify the exact host
source and build receipt. Setup enables the user service
`linux-vst-bridge.service`. The source checkout and build directory are then
unnecessary for playback. Updating software requires closed devices and a stopped
service; this slice does not implement live software updates.

The typed CLI also supports:

- `environment-create RUNNER.json`: create a private persistent environment with
  exact runner entry points and critical-file hashes.
- `environment-import ENVIRONMENT.json`: import an existing bridge-owned setup
  environment without recreating its prefix or modifying vendor/account state.
- `install ENV_ID INSTALLER_PATH SHA256`: execute the verified normal vendor
  installer in a bounded user unit, preserving its real foreground UI and exit.
- `inspect INSPECTION.json`: inspect an exact module and selected class in an
  inactive environment. The request contains `environment_id`, `module`
  (`path`, `sha256`), `class_id`, and `compatibility`. No native binary or guessed
  product metadata is required to inspect. Failed vendor calls remain failures;
  earlier discovery facts remain available in the private report.
- `register REGISTRATION.json`: bind inspected SDK metadata and the generated
  native binary to the exact module, environment, installed host and runner.
- `status`, `reconcile`, `unpublish CLASS_ID`: inspect mappings, finish an
  interrupted publication, or remove only the bridge-owned discovery link.

All record types are in `src/lib.rs`; unknown registration fields, ambiguous SDK
roles, missing artifacts and changed bindings are refused. The registry schema is
1. An installer or standalone inspection requires exclusive environment access;
stop the service after closing devices before maintenance. A compatibility mapping
can explicitly disable Windows accessibility for one product's host process.
The shared environment owner does not inherit that setting.

## Playback and ownership

`~/.vst3/LVB_<Windows-class-ID>.vst3` points atomically to an immutable installed
native bundle. The SDK processor/controller IDs retain the established UUIDv5
derivation; display names, paths and build changes do not generate new IDs. The
registration stores the installed native path, not its input build location.

The proxy's private `LVB1` request contains exactly the Windows CID and module
SHA256. Outside the callback, the service checks the mapping and supplies a fresh
random session directory. A bounded environment owner starts Wine's shared
infrastructure before DSP instances. It loads no plug-in. Each admitted instance
has its own supervised Windows process, transport, controller, state and editor.
Eight simultaneous instance admissions are supported by the service; this is a
capacity bound, not a measured eight-instance performance claim.

Ongoing Linux tracking reads process identities (PID plus start time) from stat,
not every process's command line/name. A terminal SDK failure is observed even if
a companion keeps the outer launcher alive; the original launcher exit remains a
separate fact. Positive descendant cleanup precedes native retirement, and only
that instance's private transport directory is removed. Reports remain private
under `managed/runtime/results`. Unconfirmed cleanup blocks new admissions; it
does not stop healthy siblings. Persisted leases keep an unconfirmed cleanup
from disappearing across a service restart; operator diagnosis is then required.
Systemd owns the service's complete process group
on service/session exit. Unexpected service loss remains an explicit native
failure and cannot silently replace DSP or restore guessed state.

Registered proxies retain 512 added frames and the mailbox delivery path without
reading historical preview settings. Existing counters remain enabled; optional
correlated tracing reads `managed/runtime/trace-enable` at inactive setup. The
installed supervisor passes that same explicit `1\n` opt-in to the Windows
audio host; inspection/vendor-access jobs do not inherit it. Windows request
history still arms on actual input/notes, so silent startup can lack its
completed Windows timing detail. The separate bounded
[unfinished-request status](../docs/AP12-FAULT-STATUS.md) is always available,
including idle/pre-note processing. The independent supervisor preserves it
before containment, with editor teardown/exception markers and explicit clock
domains. Detailed histories and owned-thread sampling remain opt-in. Remove
the flag after diagnostics.
Two serial proxies add 1,024 frames (21.33 ms at 48 kHz), before vendor/device
latency. This arithmetic is not a measured whole-chain result.

## Focused checks

`cargo test --manifest-path bridge-manager/Cargo.toml --locked`

`python3 -m unittest discover -s bridge-manager/runtime -v`

The Rust tests cover exact metadata, interrupted/idempotent publication, artifact
replacement, duplicate service ownership and independent registrations. Linux
process tests cover sibling survival, stat-only tracking, terminal host failure
with a lingering launcher, and native release before transport retirement.
They do not substitute for normally launched Bitwig and actual vendor tests.
