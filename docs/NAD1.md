# NAD1 — Exact NTKDaemon dependency identity and managed service lifecycle

## Decision

NAUI2 established that the exact registered Native Access 3.26.0 generation renders
usable application content only under the closed software-rendering policy, which
maps to exactly `--disable-gpu`. That result is complete and merged. NAD1 must not
reopen graphics, sandbox or renderer policy.

The next observed boundary is a Native Access dependency failure involving
NTKDaemon. It appeared immediately after the first rendered application view. The
application was then closed normally; exact process cleanup and bridge-service
recovery passed. Private dependency-failure logs were retained, but their exact
source, the installed daemon state and the service-readiness state have not yet been
bound into project authority.

NAD1 selects a single product boundary:

> Establish and own the exact NTKDaemon dependency required by the registered Native
> Access application, from installed identity through Windows service readiness,
> without broadening the bridge into a general Windows-service manager.

The implementation PR is source, generated proof and one read-only current-state
observation only. It does not mutate the actual Native Access prefix.

## Exact basis

Repository basis:

```text
main: 922bb01d170b7b8696fdc7745493628af68f33cc
tree: 5b2e79b13f84e8cbf3529256594432950ec8fa75
```

Installed and registered identities retained by the accepted NAUI2 evidence:

```text
managed environment:
627d2cba97edbecf113c22504eb4c81b

registered application identity:
7228a542c01b89daa5d04b9c8566af235ee7a2358e52f741b8918239c3a7d26e

Native Access.exe:
7b2413e90db79538cd1edc7c6169ff384aea181a85635c43a363f4aa18b9e841
225,757,168 bytes

installed software record:
1267f4233cee79d267f7e768dff6ff34d954dcbbca18416ef08c58f112b0d769
```

Controlled renderer operations:

```text
A inherited:
c44ae35d794f0a1f47ee97468e848a1c -> blank_white

B software_rendering:
627e52ca72b7533a5774862da7df11cb -> rendered_nonblank

C inherited restored:
ac91282207c833b49de0779ebb1a635a -> blank_white
```

The dependency failure was observed after usable content first appeared. B is the
leading evidence source, but the observer must prove which exact operation-owned
private log or retained application record contains the failure. Similar timestamps,
filenames or prose are not enough.

## What is known from comparative implementations

These are pinned comparative sources, not project authority.

### Cabinet

`Mark12870/cabinet@1203830c341b4514adc92c7524adae931ad0cea0`, file
`data/library/native-instruments/native-access.sh`, currently:

- locates a bundled installer below
  `Program Files/Native Instruments/Native Access/resources/daemon/win`;
- matches `NTKDaemon*Setup PC.exe`;
- invokes a fixed silent installer command;
- expects the installed executable at
  `Program Files/Common Files/Native Instruments/NTK/NTKDaemon.exe`;
- verifies Windows service `NTKDaemonService` with `sc query`.

### ni-wine

`selimbucher/native-instruments@139b8bf3dd4a4f0fad3dd22d5fe3af9e7d423e12`, files
`src/ni_wine/daemon.py`, `config.py` and `setup_cmd.py`, currently:

- states that Native Access's own elevated PowerShell dependency-install route is
  unreliable under Wine;
- installs the exact bundled daemon separately;
- distinguishes installed executable from registered service;
- reads service registration from `system.reg`;
- starts through `net start NTKDaemonService` rather than direct executable launch;
- requires an actual same-prefix NTKDaemon process after service start;
- treats another prefix's daemon as unsafe because Native Access communicates over
  fixed localhost endpoints;
- retains vendor logs below
  `C:\users\Public\Documents\Native Instruments\Logs`;
- identifies stale NI lock files as a later possible library-readiness boundary.

### NixOS Native Instruments module

`theaaronmartin/nixos@2ac9e204bd214fa1e2742f80d2c4225d7a13893d`, file
`modules/native-instruments.nix`, currently:

- reports that executing `NTKDaemon.exe` as an ordinary process is not equivalent to
  starting it as a Windows service;
- waits for loopback readiness rather than trusting command completion;
- names ports 5146 and 5563 as observed daemon endpoints.

NAD1 must independently verify this exact installed generation. Paths, service name,
installer arguments, ports and readiness law may be admitted only after exact local
observation or generated proof.

## Ownership boundary

NAD1 owns one dependency of one exact application generation:

```text
application = native-access
application identity = 7228a542...
environment = 627d2c...
dependency = exact NTKDaemon generation bundled or installed in that environment
```

It does not introduce:

- a generic service registry;
- an arbitrary Windows executable launcher;
- an arbitrary installer runner;
- arbitrary registry reads or writes;
- arbitrary ports or endpoint probes;
- arbitrary Wine-prefix selection;
- arbitrary command-line arguments;
- process-name killing;
- global Wine service management.

The production model may use a closed application dependency record, for example:

```text
schema
dependency_id = native-access-ntkdaemon
application_identity
exact environment and revision
bundled installer artifact, if admitted
installed executable artifact, if admitted
service_name = exact observed fixed value
service-registration witness
readiness contract
comparative-source identities
observation source/seal
```

No operator request may carry a path, service name, registry key, port, process ID or
command line.

## Phase 1 — sealed read-only attribution

Implement a source-owned observer under `tools/nad1/` before any production mutation
owner is selected.

### Inputs

The sealed input manifest must bind:

- current merged source/tree;
- current installed software record and exact installed owner artifacts;
- exact Native Access environment and application record;
- A/B/C result and presentation hashes;
- every private operation/log file inspected;
- exact current registry/service-state source;
- exact current system readback and preservation witnesses.

### Retained failure source

Inspect operation-owned private evidence only. The observer must determine whether
the dependency failure is bound to A, B, C or another exact operation. It must retain:

- operation identity;
- private source-file digest and bounded source class;
- exact application/software identity;
- allowlisted dependency category and message hash;
- any service/process identity actually established;
- loss or truncation counters.

Do not publish account data, machine identifiers, tokens, URLs, complete command
lines, full vendor logs or raw private paths.

### Prefix census

Read only the exact managed environment. Census and verify:

1. Native Access bundled daemon directory.
2. Every candidate bundled daemon installer with hash, size, PE architecture and
   stable file identity.
3. Installed NTK directory and candidate executable identities.
4. Exact service registration from retained registry data or bounded read-only
   service query.
5. Current service state.
6. Current same-prefix NTKDaemon process generations.
7. Foreign-prefix or deleted-prefix NTKDaemon processes.
8. Bounded localhost listener/readiness state, with exact process relation where
   possible.
9. Relevant bounded vendor log tails and their source identities.
10. Any stale daemon/product lock evidence, retained as a separate non-selecting
    lead unless it explains the current dependency failure directly.

No filename alone confers identity. Refuse aliases, duplicate roots, unstable files,
ambiguous candidates and changed bytes.

### Closed dispositions

Return exactly one:

```text
NAD1_DEPENDENCY_ABSENT
```

No admitted installed daemon executable and/or no admitted bundled installer needed
to recover it.

```text
NAD1_SERVICE_UNREGISTERED
```

The exact installed executable exists but the exact Windows service is not registered,
or the bundled installer exists and installation did not establish the service.

```text
NAD1_SERVICE_STOPPED
```

Exact installed executable and service registration exist; no same-prefix ready
daemon is active.

```text
NAD1_SERVICE_RUNNING_NOT_READY
```

A same-prefix service/process exists, but the selected readiness contract is not met
or Native Access's retained dependency probe failed.

```text
NAD1_FOREIGN_DAEMON_CONFLICT
```

A daemon from another or deleted prefix can answer the shared endpoint or otherwise
conflicts with this exact environment.

```text
NAD1_DEPENDENCY_READY
```

Exact installed identity, service registration, same-prefix process and admitted
readiness all hold; the blocker must be elsewhere.

```text
NAD1_IDENTITY_UNRESOLVED
```

Required identity or evidence is absent, ambiguous, lost or inconsistent.

Absence in a lossy source is never decisive.

### Physical observation authorization

Exactly one read-only NAD1 observation is authorized during this implementation PR.
It must not start or stop any service, process, application or installer. It must not
modify registry state or vendor files.

Final readback must prove:

- Native Access remains closed;
- service active and two keepers healthy;
- zero DSP/maintenance leases, pending transactions and stale transports;
- capture off;
- installed software unchanged;
- all products/publications/imports/environments unchanged;
- 307 retained witnesses, five protected projects and predecessor files unchanged;
- A/B/C evidence unchanged;
- no new renderer operation.

## Phase 2 — closed production dependency owner

After the read-only disposition is retained, implement one exact owner capable of the
admitted transition. The same PR may contain all generated transition cases, but the
real-prefix transition remains separately gated.

### Admission

The installed Python and Rust owners must independently bind:

- exact current installed software generation;
- exact registered Native Access application generation;
- exact environment/root;
- exact bundled installer and installed daemon artifacts admitted by NAD1;
- exact service name and registration witness;
- exact operation/reservation/report paths;
- exact readiness contract;
- exact current dependency-state observation.

A foreign or self-consistent synthetic spec must refuse before lock, process launch,
registry mutation or result publication to an unbound path.

### Transition law

The manager should expose no arbitrary choice. One action such as
`Prepare Native Access dependency` may select only the exact transition implied by
the verified state:

```text
absent/unregistered -> exact bundled install -> verify executable + service
stopped             -> exact service start -> verify same-prefix readiness
running_not_ready   -> bounded diagnostic/refusal; no blind reinstall
foreign_conflict    -> refusal; no kill
ready               -> no-op verified receipt
identity_unresolved -> refusal
```

If the local read-only result shows that install and service start should remain
separate operator decisions, use two closed actions. Do not expose their underlying
arguments.

### Installation

If installation is selected:

- use only the exact Native Access-bundled installer artifact;
- fixed arguments only, selected and tested in source;
- bind any log path to the exact operation directory;
- never trust installer exit code alone;
- verify exact installed executable bytes and exact service registration afterward;
- retain before/after registry and file witnesses;
- no Native Access reinstall;
- no global UAC or PowerShell policy expansion;
- immutable operation receipt and rollback/readback law.

Comparative implementations use silent forms based on `/s`, sometimes with
`IAgree=Yes` and an exact log path. NAD1 must choose one closed vector from exact
installer identity and generated proof rather than accepting caller arguments.

### Service start and readiness

If service start is selected:

- start through the exact Windows service manager, not by direct executable launch;
- use exact service name only;
- own the start operation under a dedicated manager/systemd operation;
- retain uncertain acknowledgment and recovery authority;
- verify a same-prefix daemon process generation;
- verify the admitted service/readiness endpoint contract;
- refuse if a foreign daemon could satisfy the endpoint;
- do not equate `net start` exit zero, `sc query RUNNING`, a process name or an open
  port individually with readiness;
- preserve exact Stop and cleanup ownership.

The owner must decide whether NTKDaemon stays active as an application dependency or
is retired after the Native Access operation. That lifecycle must be explicit,
generated and reviewable. Do not let a daemon escape into an ownerless wineserver.

### Integration with Native Access launch

After NAD1 is later installed, the Native Access software-rendering action must be
gated on exact dependency readiness. The manager must not silently launch Native
Access and hope it repairs the daemon.

No permanent default renderer change is part of NAD1; the already selected closed
software-rendering action remains the only approved application mode.

## Generated proof

Build source-owned fixtures exercising the real owners. At minimum:

1. Bundled installer absent.
2. Duplicate/aliased/changing installer candidates.
3. Exact installer present; daemon absent.
4. Daemon executable present; service unregistered.
5. Service registered and stopped.
6. Service reports running but process absent.
7. Process present but readiness absent.
8. Exact same-prefix ready daemon.
9. Foreign-prefix daemon on the same readiness endpoint.
10. Deleted-prefix/stale daemon identity.
11. Direct daemon execution refuses or fails to establish service readiness.
12. Exact service start succeeds and retains process/readiness authority.
13. Installer/start acknowledgment loss and interrupted manager recovery.
14. Wrong-operation Stop and exact-operation Stop.
15. Duplicate prepare request refusal.
16. Changed application/software/dependency identities refuse before mutation.
17. Log/source loss weakens to unresolved.
18. Native Access renderer operations remain unchanged except readiness gating.
19. Arturia and ordinary installer/product paths remain unchanged.
20. Full cleanup, service restoration and immutable receipt behavior.

A Windows service fixture is appropriate if needed. AP8 must exercise any new
Windows executable, service program or installer-adapter contract.

## Evidence and privacy

Public evidence may retain:

- exact hashes, sizes and relative semantic locations;
- service-registration status;
- closed service/readiness state;
- exact process-generation ordinals and prefix relation;
- bounded port numbers only if admitted into the fixed contract;
- operation outcome and cleanup;
- disposition.

Keep private:

- raw vendor logs;
- account and machine identifiers;
- URLs and tokens;
- complete command lines;
- raw registry exports;
- raw local paths;
- process IDs where not needed publicly.

## Validation

Required before review:

- focused NAD1 unit, mutation and saturation tests;
- affected manager/frontend/runtime suites;
- strict manager/frontend Clippy;
- AP8, AP12 and PX2 on the final head;
- AP10 only if native/audio source changes;
- exact final preservation readback.

## Stop condition

Return one draft PR containing:

- the sealed read-only physical disposition;
- the closed production owner selected by that disposition;
- generated qualification;
- no real NTKDaemon install/start/stop;
- no Native Access launch;
- no installed software replacement.

After independent review, merge and deliberate installation, a separately authorized
continuation may perform the exact selected dependency transition and one
software-rendered Native Access readiness observation. No product install or account
interaction is automatically authorized by NAD1.

## Implemented lifetime and bounded readiness contract

The dependency has an operation lifetime, not a foreign or host-wide daemon lifetime.
`DependencyPrepare` has no caller parameters. It uses the exact bundled SHA-256 and
size, the fixed `/s` argument, and an adapter-bound installer root. It verifies the
resulting image and exact own-process Windows service registration; outer exit zero
alone is insufficient. The fixed Windows adapter queries or starts only
`NTKDaemonService` through SCM. It never executes `NTKDaemon.exe` directly.

Readiness requires all of: SCM running with exact service image and Windows creation
identity; one exact admitted image mapped by a stable Linux PID/start generation in
the same prefix and owned cgroup; both loopback listeners 5146/5563 owned by that
Linux generation; no ambiguous or foreign/deleted candidate. Windows and Linux PID
numbers are not joined. This is a local dependency readiness contract, not a claim
about vendor authorization, HTTP behavior or successful Native Access use.

Preparation stops and retires its cohort, retaining immutable stage and terminal
receipts. A seven-field `prepared.json` pointer binds that terminal receipt, application,
software and resulting daemon artifact. It is historical preparation, not current
readiness. Each later renderer operation freshly verifies and starts the same service
before application launch, and retires the dependency cohort when the application
closes. Neither Software nor Environment gains persistent fields. Older software
generations and all prior renderer results remain readable and immutable.

Unknown existing daemon bytes refuse until admitted; filename and metadata alone
cannot authorize them. Running-but-not-ready refuses without reinstall. Foreign or
deleted-prefix candidates refuse without adoption or signalling. Submission uses an
exact reservation and writer gate; uncertain acknowledgment preserves service-resume
custody. Only confirmed empty retirement restores the bridge service.

The generated service uses a separately sealed internal entry and compile-time fixed
fixture service/path constants. The installed persisted-spec entry has no fixture or
test-mode switch. The fixture is source-owned and does not establish vendor behavior.
