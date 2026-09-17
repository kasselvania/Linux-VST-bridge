# NAUI1 — Native Access renderer identity and blank-window attribution

## Decision

NAUI1 is the next implementation slice. It does **not** apply a graphics workaround.
It establishes the exact installed Native Access application/runtime identity and
uses the already-retained operation evidence to select, or explicitly refuse to
select, the rendering boundary responsible for the blank white window.

The selected order is:

1. exact application identity;
2. exact renderer/process evidence;
3. closed classification;
4. independent review;
5. only then a separate operation-scoped remedy experiment.

This ordering preserves the distinction between a highly plausible community fix
and a source-owned result on this exact managed environment.

## Exact basis

- Merged and installed source: `8fb2f20c822320340e44f9cbae399633dd6e8a53`.
- Installed source tree: `947461f96dc2648b13c9b450c761beae97880571`.
- Installation-only receipt: `175330055156c828ad8015301544ca8a03afb68b`.
- Retained Native Access continuation: `2a4846958ad176785b7bece9a5e99d263b4d9e5a`.
- Retained continuation tree: `c43c0ea0521e562585c04ed5228a93c722b131f9`.
- Managed environment: `627d2cba97edbecf113c22504eb4c81b`.
- Installer operation: `e4143128adc87de3fdbbdfb44f186ee5`.
- Imported installer SHA-256:
  `82d7b7977d4fbc19db32aee8778c75906f6ddf935bcf2958f2d8d45141b2b2cc`.
- Imported installer size: `179877344` bytes.
- Private transaction SHA-256:
  `1e1096d66befab602726fde1f7e11b1425cea1c1fdb2ddb544843c833f92053d`.
- Public result SHA-256:
  `fc9325cd947059e0d453ba69c9a332028197d871e3ef9ab1c94b2fe7d91586a0`.

The exact operation reached a visible window titled `Native Access` whose client
area was blank white. The operator closed it using the window close control. The
owned operation returned outer exit 0, retired with zero survivors and positive
cleanup, and did not record an attributed startup failure. Durable witnesses remain
`partial_installation`: 15 added files, four uninstall records and three logs were
observed, with no matched application registration and one incomplete observation
surface. Therefore neither complete installation nor application absence may be
inferred from the current classification.

The runner observation buffer dropped records. Missing GPU, renderer or network
records are therefore unavailable evidence, not proof that those processes or
failures did not occur.

## Comparative research input — not project authority

A current MIT-licensed community project, `dguedry/nilinux` at commit
`e332e69c03d697c60b369c0fbca73be34ba31e65`, treats modern Native Access as an
Electron/Chromium application. Its source launches Native Access with fixed
`--disable-gpu` and `--no-sandbox` switches and documents a blank-window GPU-process
failure on its own stack. This is a strong route-selection input, not evidence about
our exact installed files, process generations or failure.

Electron itself separates the main process, renderer processes and utility/GPU
processes. A white BrowserWindow can accompany renderer or GPU-process failure, and
disabling hardware acceleration is an application-start decision. Consequently,
NAUI1 must distinguish at least GPU failure, sandbox/child failure and generic
renderer failure rather than collapsing all three into “Electron does not work.”

No source is copied from the comparative project in this slice. Any later reuse must
retain license/provenance and still pass this project’s exact ownership boundaries.

## Ownership boundary

NAUI1 is a source-owned diagnostic package. It has no production launch authority.

Recommended layout:

```text
tools/naui1/
  identity.py
  records.py
  classify.py
  run.py
  package.py              # only if a sealed package is needed
  test_identity.py
  test_records.py
  test_classify.py
  test_run.py
```

The implementation may reuse generic repository helpers by importing or factoring
source-owned code where ownership remains clear. It must not expose an arbitrary
path, command, environment value, PID, process name, application argument or vendor
file mutation through the manager, CLI or frontend.

## Closed input manifest

The observation begins from a committed closed manifest containing only exact
identities needed to locate retained custody:

```text
schema
source head/tree
environment identity
installer operation identity
imported installer digest/size
private transaction digest
public result digest
expected managed-root relation
```

The manifest does not contain account details, host-user paths or raw logs. Runtime
location is derived from the existing managed root and exact environment identity.
Caller-supplied alternate paths refuse.

Before reading application or operation evidence, require:

- service state compatible with read-only inspection;
- no live owner for the retained operation;
- exact environment record and environment-root relation;
- exact retained result and private transaction digests;
- exact installed software record and source generation;
- regular, owner-controlled, non-symlink inputs;
- bounded file sizes and counts.

Any mismatch produces an identity refusal before classification.

## Exact application identity

Search only inside the exact environment’s Windows prefix. Do not scan the operator
home, other prefixes, system-wide Wine locations or other managed environments.

The census must be bounded and alias-safe. Record candidate files by stable relative
location, content digest, size and PE/resource metadata without publishing the
private absolute root.

An application root is admissible only when exactly one closed candidate satisfies
all required relations. Candidate evidence should include, where present:

- exact `Native Access.exe` regular file and PE metadata;
- exact sibling `resources` relation;
- `resources/app.asar` identity;
- Chromium/Electron resource identities such as `chrome_100_percent.pak`,
  `chrome_200_percent.pak`, `resources.pak`, `icudtl.dat`,
  `snapshot_blob.bin`, `v8_context_snapshot.bin`, `libEGL.dll` or `libGLESv2.dll`;
- bounded ASAR header/package metadata and main-entry identity;
- exact version facts obtainable without executing the program.

Do not require every optional filename across Electron releases. Instead implement a
closed evidence law with positive combinations and explicit ambiguity tests. A
file named `app.asar` by itself is insufficient. Similar filenames outside the exact
application root confer no identity.

The ASAR reader is read-only and bounded. It may inspect the package manifest and
selected entry metadata but must not extract the whole archive, execute JavaScript,
rewrite the archive or retain source content publicly.

The result separates:

```text
application_identity: established | unresolved
renderer_family: electron_chromium | other | unresolved
application_version: exact fact | unavailable
renderer_version: exact fact | unavailable
```

An unavailable renderer version does not invalidate an otherwise exact application
identity.

## Retained process and diagnostic evidence

Consume the exact operation-owned private records and bounded private log without
relaunching anything. Validate their operation/root/source identity before parsing.

Recognize only allowlisted renderer-role facts, for example:

- `--type=gpu-process`;
- `--type=renderer`;
- `--type=utility` and a bounded utility subtype such as network service;
- exact create/self-exit generations;
- canonical Chromium/Electron diagnostics for GPU-process exit/restart,
  unusable GPU process, child/sandbox launch refusal, network-service crash,
  renderer termination or render-process-gone;
- exact GDI-handle exhaustion or bounded font-enumeration storm evidence;
- ANGLE/D3D/OpenGL initialization failures when retained exactly.

Do not publish complete command lines. Public evidence retains only role, generation,
exit domain/status where authoritative, allowlisted diagnostic code/category, counts
and hashes. URLs, tokens, account data, file contents and raw private paths remain
private.

Windows and Linux process identifiers remain separate domains. Similar numeric PIDs,
process names or temporal proximity never establish cross-domain identity. Repeated
process names require exact generation/parent/root custody.

Observation-buffer loss is retained. A category may be selected only from positive
retained evidence; absence in a lossy stream is never decisive.

## Classification law

The classifier returns exactly one disposition:

```text
NAUI1_ELECTRON_GPU_FAILURE_SELECTED
NAUI1_ELECTRON_SANDBOX_CHILD_FAILURE_SELECTED
NAUI1_FONT_GDI_FAILURE_SELECTED
NAUI1_RENDERER_PROCESS_FAILURE_SELECTED
NAUI1_ELECTRON_IDENTITY_ONLY_CAUSE_UNRESOLVED
NAUI1_OTHER_RENDERER_CAUSE_UNRESOLVED
NAUI1_APPLICATION_IDENTITY_UNRESOLVED
```

Minimum laws:

### GPU failure selected

Require exact Electron/Chromium application identity plus positive GPU-process
failure authority: an exact associated GPU generation with abnormal exit/crash-loop,
or a canonical GPU fatal/restart diagnostic bound to the operation. A blank window,
GPU DLL presence or community report is insufficient.

### Sandbox child failure selected

Require exact Electron/Chromium identity plus positive child-process/broker failure
such as the exact Chromium sandbox launch refusal or retained `error_code=39`
sequence. `--no-sandbox` is not selected from Electron version alone.

### Font/GDI failure selected

Require positive retained GDI exhaustion or bounded font-enumeration pathology.
Missing Microsoft font names or comparative reports alone are insufficient.

### Renderer failure selected

Require exact renderer-generation abnormal termination or canonical renderer-gone
record where no more specific GPU, sandbox or font/GDI cause is established.

### Identity-only / unresolved

Use when Electron identity is exact but decisive failure evidence is absent,
ambiguous or lost. This is a valid scientific result and may select a later bounded
instrumented comparison rather than a repair.

The human screenshot contributes the observed presentation only. It cannot select
any technical category.

## Generated proof

Generated fixtures must exercise the real classifier and identity reader, including:

- exact Electron application tree;
- misleading partial Electron-like tree;
- two candidate roots;
- symlink/hardlink/replacement races;
- changed ASAR or executable bytes;
- clean renderer startup;
- repeated GPU-process crash-loop;
- sandbox child error and network-service failure;
- renderer-only failure;
- GDI/font storm;
- unrelated diagnostics containing similar words;
- missing, truncated, saturated and dropped-record cases;
- duplicate/reordered generations;
- Linux/Windows PID-number collision;
- account/path/token privacy rejection.

Each decisive scalar/category must have a mutation or removal test demonstrating
that classification fails closed or weakens to the correct unresolved result.

## Physical read-only observation

After committed source and generated tests pass, run exactly one read-only NAUI1
observation against the retained environment. It must not start:

- Native Access;
- its installer or updater;
- Wine/Proton target processes;
- NTK or vendor services;
- Bitwig or a plug-in host;
- a product scan or installation.

No keyboard, mouse, window focus or account action is permitted. No vendor file or
registry write is permitted. Temporary source-owned output lives outside the managed
prefix and is removed after sanitized evidence is committed, except for intentionally
retained private evidence under the existing custody convention.

Before and after readback must establish:

- service active and keepers healthy;
- zero DSP/maintenance leases, pending transactions and stale transports;
- cleanup unblocked and capture off;
- products/publications/projects unchanged;
- installed software unchanged;
- no new environment, operation, unit or Wine cohort;
- the retained Native Access environment and prior attempts unchanged.

## Validation

Required before review:

- focused NAUI1 unit/negative tests;
- affected repository tests;
- strict checks for any changed Rust source;
- AP12 with NAUI1 tests included;
- PX2;
- AP8 only if Windows executable/helper source is added;
- no AP10 unless native/audio behavior unexpectedly changes, which is outside scope.

Commit executable diagnostic source before the physical read-only observation.
Generated/sanitized evidence belongs in a later evidence-only commit. The draft PR
must name the executed source head/tree and prove that the final evidence commit did
not change executable diagnostic source.

## Explicit exclusions

NAUI1 does not authorize:

- reinstalling or updating Native Access;
- relaunching Native Access;
- `--disable-gpu`, `--no-sandbox`, `--disable-gpu-sandbox`, DXVK or other launch
  policy changes;
- `app.disableHardwareAcceleration()` or any `app.asar` mutation;
- changing PE stack reserve, fonts, C runtime, NTK daemon or registry state;
- adding a general application command-line API;
- importing a community project as runtime authority;
- scanning, installing, licensing or publishing NI products;
- automated vendor/account input;
- treating `partial_installation` as proof of failure or absence.

## Conditional next cut

After independent acceptance:

- `NAUI1_ELECTRON_GPU_FAILURE_SELECTED` may prepare NAUI2 with one exact
  operation-scoped `software_rendering` launch mode mapping only to
  `--disable-gpu`.
- `NAUI1_ELECTRON_SANDBOX_CHILD_FAILURE_SELECTED` may prepare a separately named
  sandbox-policy comparison; it must not be silently bundled with GPU disablement.
- `NAUI1_ELECTRON_IDENTITY_ONLY_CAUSE_UNRESOLVED` may prepare one instrumented
  inherited-vs-software-rendering comparison, but only after source review.
- Other outcomes select their corresponding narrow boundary or further evidence.

No NAUI2 route is active merely because this document names it.

## Implemented diagnostic law

`tools/naui1/input.json` binds the accepted installed software/artifact generation,
exact retained operation, import, result and private transaction. `package.py`
seals committed diagnostic bytes and the input manifest into a fixed file set.
The remote custodian verifies that detached seal before invoking `run.py` without
arguments. The read-only owner itself rechecks the seal and writes a one-use start
marker outside the environment, so a refusal cannot silently become another pass.

The application law requires exactly one regular, unaliased `Native Access.exe`,
parsed PE resource ProductName, matching ASAR package metadata and an available
selected main entry. Electron-family evidence additionally requires a literal
Electron module import in that entry plus collocated ICU, Chromium PAK and V8
snapshot families. Every present allowlisted resource is hashed. The ASAR header,
package metadata and main-entry bytes are hashed without extracting files; selected
links/unpacked entries refuse. No vendor text is published. Optional resources and
unavailable renderer version remain distinct from application absence. Product
names locate candidates; the conjunction of parsed metadata and exact bytes binds
the resulting observation, not executable admission or authenticity certification.

Only the two fixed readback subprocess commands (verified manager snapshot and
exact installer-unit status) may run. Prefix census uses no-follow directory
handles, skips symlink traversal, and opens selected files with no-follow on every
component. Files with multiple links, foreign ownership or group/other write access
refuse. Bounds: 100,000 census entries, 45 seconds per census, 512 MiB per selected
file, 1 GiB selected identity hash budget, 8 MiB ASAR header, 256 KiB package.json,
32 MiB selected main, 512 Windows generations and 2 MiB of operation-owned log
sinks. Before/after preservation uses metadata for all exact-environment entries,
content hashes for selected application inputs and previously retained witnesses.
It does not claim a complete content hash of every unselected prefix file.

Retained create completions are validated for ordinal order, epoch, active parent,
nonoverlapping PID generations, exact root binding and self-exit domain. Raw create
request/completion pairing confers a role only with the exact retained completion
identity. Current application bytes alone do not retroactively establish launch
bytes: a decisive generation also needs its retained image digest to match.
Unbound canonical diagnostics are counted as leads and cannot select a cause.
Chromium diagnostic headers are interpreted only in the Windows application
context and only with a single matching retained Windows generation across both
epochs; Linux PID records are never used to complete that match. PID reuse removes
this diagnostic authority. Source-location and message grammars must both match;
a helper's numeric status, arbitrary prose, screenshot or accessibility message
cannot substitute for these records. Several distinct specific failure families
produce unresolved, not an arbitrary precedence-selected remedy.

The following primary sources were consulted for format/semantic context; no source
was copied. They are not a claim that the installed application matches upstream
main:

- [Electron process model](https://www.electronjs.org/docs/latest/tutorial/process-model).
- [ASAR disk format owner](https://github.com/electron/asar/blob/main/src/disk.ts).
- [Chromium logging](https://github.com/chromium/chromium/blob/main/base/logging.cc)
  and [Windows process identifiers](https://github.com/chromium/chromium/blob/main/base/process/process_handle_win.cc).
- [Chromium GPU process host](https://github.com/chromium/chromium/blob/main/content/browser/gpu/gpu_process_host.cc).
- [Pinned nilinux comparative launcher](https://github.com/dguedry/nilinux/blob/e332e69c03d697c60b369c0fbca73be34ba31e65/nilinux/native_access.py).

The comparative launcher also performs unrelated modifications. None was copied,
executed or admitted as platform policy. A GDI exhaustion signature can select the
font/GDI family; repeated font names without a handle/time authority cannot.
