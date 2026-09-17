# NAD1 implementation-agent handoff

Act as the implementation engineer for Linux-VST-bridge slice **NAD1 — Exact Native
Access dependency service owner**.

## Exact branch and authority

Work only on:

```text
branch: codex/nad1-ntkdaemon-dependency-owner
basis:  922bb01d170b7b8696fdc7745493628af68f33cc
tree:   5b2e79b13f84e8cbf3529256594432950ec8fa75
```

Read first:

```text
CURRENT_SLICE.md
docs/NAD1.md
```

NAD1 is active implementation authority. Do not ask the operator to choose another
slice or repeat facts already in those files. Make a best engineering decision inside
the closed contract and document any unresolved evidence honestly.

## Context you must preserve

NAUI2 is finished and merged. For exact Native Access 3.26.0 application identity
`7228a542c01b89daa5d04b9c8566af235ee7a2358e52f741b8918239c3a7d26e`, the
white/content/white comparison selected exactly operation-scoped `--disable-gpu`.
Do not reopen rendering or add any other Chromium switch.

The current blocker appeared after the first usable rendered Native Access view: an
NTKDaemon dependency failure. The application was closed cleanly; no owned processes
remain; the bridge service and both keepers recovered; products and projects are
unchanged. Dependency-failure logs were retained privately.

Leading operation source:

```text
B software-rendering operation:
627e52ca72b7533a5774862da7df11cb
```

Treat that only as a lead. Prove the exact private source before attributing the
failure.

Exact managed identities:

```text
environment:
627d2cba97edbecf113c22504eb4c81b

application identity:
7228a542c01b89daa5d04b9c8566af235ee7a2358e52f741b8918239c3a7d26e

Native Access.exe:
7b2413e90db79538cd1edc7c6169ff384aea181a85635c43a363f4aa18b9e841
225757168 bytes

installed software record:
1267f4233cee79d267f7e768dff6ff34d954dcbbca18416ef08c58f112b0d769
```

## Comparative sources to inspect, not copy blindly

Pin and read these exact sources:

```text
Mark12870/cabinet
commit 1203830c341b4514adc92c7524adae931ad0cea0
data/library/native-instruments/native-access.sh

selimbucher/native-instruments
commit 139b8bf3dd4a4f0fad3dd22d5fe3af9e7d423e12
src/ni_wine/daemon.py
src/ni_wine/config.py
src/ni_wine/setup_cmd.py

theaaronmartin/nixos
commit 2ac9e204bd214fa1e2742f80d2c4225d7a13893d
modules/native-instruments.nix
```

They suggest, but do not prove for our prefix:

```text
bundled installer:
Program Files/Native Instruments/Native Access/resources/daemon/win/
NTKDaemon*Setup PC.exe

installed executable:
Program Files/Common Files/Native Instruments/NTK/NTKDaemon.exe

Windows service:
NTKDaemonService

start law:
service manager / net start, not direct executable

possible readiness endpoints:
127.0.0.1:5146 and 127.0.0.1:5563
```

Verify locally before admitting any of these into project authority.

## Required work order

### 1. Establish exact repository and machine basis

- Confirm branch parent and clean worktree.
- Confirm current main has not moved in a way that changes installed authority. If it
  has, record the later main but do not silently rebase away this planning basis.
- Read the NAUI2 source, installation receipt and A/B/C evidence.
- Read the current renderer owner, operator service-recovery owner, installer owner,
  process ledger and service/cgroup custody code.
- Confirm installed state is idle before any Deck observation.

### 2. Implement sealed read-only NAD1 observer first

Create `tools/nad1/` with a closed package/input/observer/classifier/test structure.
The observer accepts no arbitrary runtime path, operation or prefix.

Bind:

- exact merged source/tree;
- exact installed software and artifacts;
- exact environment and registered application;
- exact A/B/C evidence hashes;
- exact private log and operation sources inspected;
- before/after preservation witnesses.

Read only:

- exact Native Access bundled daemon directory;
- bundled daemon installer candidates;
- installed daemon directory and executable candidates;
- system registry service registration;
- bounded `sc query` or equivalent read-only service state;
- same-prefix and foreign-prefix daemon processes;
- bounded listener/readiness state;
- operation-owned and vendor log evidence;
- relevant stale-lock evidence as a separate lead.

Do not start Wine, Native Access, NTKDaemon, an installer, updater, DAW or product
scan. Do not mutate registry or files.

Generated tests must cover duplicate keys, aliases, unstable files, ambiguous
installers, changed bytes, missing service, stopped service, false-running service,
foreign prefix, deleted prefix, listener without process authority, process without
readiness, log loss and privacy rejection.

### 3. Run exactly one physical read-only observation

Commit the observer source and seal it before execution. Run one pass on the Deck.
No second pass unless the first did not execute any observation because source input
validation refused before reading the environment; retain such a refusal separately.

Return exactly one disposition:

```text
NAD1_DEPENDENCY_ABSENT
NAD1_SERVICE_UNREGISTERED
NAD1_SERVICE_STOPPED
NAD1_SERVICE_RUNNING_NOT_READY
NAD1_FOREIGN_DAEMON_CONFLICT
NAD1_DEPENDENCY_READY
NAD1_IDENTITY_UNRESOLVED
```

Retain sanitized evidence under `evidence/nad1/observation/`; keep raw logs and raw
registry/process material private.

### 4. Implement the closed production owner selected by the observation

Build one exact Native Access dependency capability. The installed Rust and Python
owners must independently bind all production identities and refuse foreign
self-consistent specs before mutation.

No operator request may carry:

```text
path
service name
registry key
port
process ID
command
args
environment
Wine prefix
```

Use a closed action such as `Prepare Native Access dependency`, or two closed actions
only if the observed state proves install and start require separate operator gates.

Required state law:

```text
absent/unregistered -> exact bundled install -> verify executable + service
stopped             -> exact service start -> verify same-prefix readiness
running_not_ready   -> bounded refusal/diagnostic; no blind reinstall
foreign_conflict    -> refusal; do not kill
ready               -> exact no-op verified receipt
identity_unresolved -> refusal
```

Installation, if implemented:

- exact bundled installer only;
- one fixed source-owned argument vector;
- operation-bound log path;
- no trust in outer exit alone;
- verify installed executable and service registration;
- immutable before/after witnesses;
- no Native Access reinstall;
- no arbitrary PowerShell/UAC expansion.

Service start, if implemented:

- exact Windows service manager route only;
- never direct `NTKDaemon.exe` execution;
- preserve uncertain submission and recovery ownership;
- require exact same-prefix process authority;
- require the admitted readiness contract;
- reject a foreign/deleted-prefix daemon;
- retain exact Stop and cleanup behavior;
- make daemon lifetime relative to Native Access explicit and owned.

Integrate Native Access launch gating so software-rendered launch is unavailable or
refuses until dependency readiness is exact. Do not change the selected renderer
policy or introduce a default in this slice unless the existing closed software-
rendering action requires only readiness gating.

### 5. Build generated qualification

Exercise the real production owners with source-owned fixtures. Include at least:

- exact bundled installer and installed daemon identity;
- service-only start semantics;
- direct executable non-readiness;
- absent, unregistered, stopped, running-not-ready and ready states;
- foreign/deleted-prefix conflict;
- fixed listener readiness and false listener cases;
- stable hash/alias/replacement laws;
- installer/start acknowledgment loss;
- interruption and exact recovery;
- wrong-operation and exact-operation Stop;
- duplicate prepare refusal;
- immutable receipt and cleanup;
- unchanged Native Access renderer, Arturia and ordinary installer/product routes.

Introduce a source-owned Windows service fixture under AP8 if necessary. Do not use a
fake that can also succeed when run as an ordinary process; the fixture must preserve
the service-controller distinction being qualified.

### 6. Validate and return one draft PR

Required final validation:

```text
focused NAD1 tests
affected manager tests
affected frontend tests
runtime tests
strict manager/frontend Clippy
AP8
AP12
PX2
```

AP10 is required only if native/audio code changes.

Final readback must prove:

```text
Native Access closed
real NTKDaemon unchanged
installed software unchanged
service active
two keepers
zero leases/transactions/stale transports
capture off
products/publications/imports/environments unchanged
307 retained witnesses unchanged
five projects unchanged
A/B/C evidence unchanged
no new renderer operation
```

Open one draft PR from this branch. Include exact head/tree, physical disposition,
generated results, nonclaims and next physical gate.

## Explicit prohibitions

Do not:

- launch Native Access;
- install/start/stop/remove the real NTKDaemon;
- reinstall Native Access;
- launch updater, DAW, plug-in or product installer;
- sign in or interact with account content;
- change registry state in the real prefix;
- run `wineserver -k`;
- kill by process name;
- use another prefix's daemon;
- add `--no-sandbox` or any renderer flag;
- expose arbitrary Windows-service or command authority;
- install or replace production software;
- merge the PR.

Stop at the draft PR for independent review. The later real dependency transition and
one software-rendered Native Access readiness observation are separately gated.
