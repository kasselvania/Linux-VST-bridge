# NAUI2 — Controlled Native Access renderer differential

## Decision

NAUI1 established the exact installed application but correctly refused to select a
blank-window cause. NAUI2 is the next slice because a new controlled observation is
now necessary. It prepares the production owner and generated proof for a later
human-authorized renderer differential; it does not perform that differential.

This corrected planning basis is directly above merged NAUI1 main
`eeffae76d0e34e9a2a09cddd05707afac665c4de`, tree `93b015a54211fc3df35f0ce05ea815ed64673fc9`.
PR #116 accepted head `5259d0069297e074501dda292c40239c1da04000` is an ancestor of
that merge and has the same tree. The earlier planning commit `e4062c50f5564024006866c284db333ae1fd94e6`
is superseded; it did not contain the accepted trace-completeness repair.

Implementation is authorized by the operator on this corrected basis. The manager
now has an adjacent exact Native Access application owner, closed rendering modes,
operation-bound observation and recovery, and a generated fixture campaign. This
PR remains uninstalled and contains no real application comparison.

## Exact application basis

The only admitted application is the NAUI1 generation in environment
`627d2cba97edbecf113c22504eb4c81b`:

- Application: Native Access 3.26.0 x64.
- PE file/product version: 3.26.0.962.
- `Native Access.exe`:
  `7b2413e90db79538cd1edc7c6169ff384aea181a85635c43a363f4aa18b9e841`,
  225,757,168 bytes.
- `resources/app.asar`:
  `2df87bef2a7c7113b56374d4f5620519d268496bba9414db2e3c101328783ea5`,
  76,143,407 bytes.
- NAUI1 result source: `f37d7aebe7bb86722aac1349ba79cb80d1439ab3`, tree
  `0bf6fd05707e6f740094166f253a74971df617d5`.
- NAUI1 source seal:
  `539bcf48c961e18b93ce6bc54c4aed20ab7a82822e7563e34b8025b6836b4be9`.

The prior blank window occurred inside installer operation
`e4143128adc87de3fdbbdfb44f186ee5`. That result remains historical evidence, not a
controlled application-launch baseline. Its runner stream lost 4,571,384 bytes and
48,667 records, and its Windows image owner could not hash the 225,757,168-byte application
under the former 64 MiB per-image bound. Therefore NAUI2 must acquire new exact
application/process authority rather than reinterpret the old operation.

## Product boundary

NAUI2 owns a vendor application, not a VST product, application installer or general
Windows-program launcher. It does not change product publication, plug-in scanning,
audio behavior, application authorization, or installer completeness.

Native Access may remain classified as partially installed by the existing durable
installer witness. NAUI2 does not overwrite that classification. Its application
record means only that exact files were identified and are eligible for this bounded
diagnostic launch owner.

## Closed application record

Extend the existing `vendor_application` design or introduce an adjacent exact owner
without weakening Arturia Software Center. The record must bind:

```text
schema
application_id = native_access
exact environment and revision
exact executable artifact
exact app.asar artifact
required Electron-family resource artifact set
NAUI1 evidence/source-seal identities
retained installer operation/result identity
observed application and PE versions
```

Discovery must not accept a caller path, executable, environment, operation, version
or digest. It derives the exact environment from retained manager state and verifies
the committed NAUI1 identity against current bytes. Duplicate roots, changed bytes,
aliases, missing required resources or another environment refuse.

Arturia Software Center records and launch modes must remain byte-for-byte compatible.
No generic `ApplicationId::Other`, arbitrary application registry, path field or
command-line array is permitted.

## Renderer policy

Add one closed operation-bound enum:

```text
RendererPolicy::Inherited
RendererPolicy::SoftwareRendering
```

The exact command law is:

- `Inherited`: launch the exact executable with no renderer-policy argument.
- `SoftwareRendering`: append exactly one argument, `--disable-gpu`.

Both modes otherwise receive the same runner, environment, cwd, supervision,
diagnostic capacity and lifecycle ownership. Do not add `--no-sandbox`,
`--in-process-gpu`, ANGLE switches, logging verbosity, DXVK overrides, arbitrary
Chromium flags or caller environment values.

The manager/operator request carries only exact application identity and the closed
enum. It must not accept `args`, `command`, `path`, `environment`, `flags`, PID,
window ID or process name. The effective result separately records requested and
applied policy; policy becomes effective only after exact target creation and owner
registration.

## Exact launch-root custody

The current vendor-application path predates the IS2 exact Windows launch-root owner.
NAUI2 must not infer the application root from process order or name. Reuse or factor
the accepted exact adapter pattern so that the launch retains:

- operation identity;
- unpredictable token;
- exact executable SHA-256 and size;
- exact child handle and Windows process generation;
- adapter/root creation epoch;
- one root ordinal;
- declared renderer policy and exact argv projection.

The adapter contract must remain application-scoped. It may not become a general
arbitrary-program launcher. A missing, conflicting or changed binding refuses the
operation before public effective-policy authority.

## Large-image and child custody

The known application is 225,757,168 bytes, larger than the former 64 MiB per-image
bound. Increase authority narrowly:

- verify the exact selected application artifact before launch;
- allow a bounded application-image read sufficient for this exact artifact;
- cache the verified digest by stable open-file device/inode/size/generation so
  repeated Electron children do not rehash the same 225,757,168-byte file without bound;
- retain an aggregate byte/read count;
- leave unrelated image and mapped-file limits unchanged;
- refuse replacement, symlink, hardlink alias, deletion, inode mismatch or changed
  bytes.

Every Windows process record remains generation-based. Repeated PIDs require prior
retirement. Parent/child relations require a live exact parent generation. Linux
ledger identities remain independent and never confer Windows identity.

For a causal physical comparison, cause selection requires zero dropped Windows
process observations. Positive diagnostics may survive unrelated textual loss only
when their exact process generation remains independently complete; otherwise the
result weakens to inconclusive.

## Inherited trace-completeness law

The accepted NAUI1 repair distinguishes generation-trace loss from ordinary
runner-tail loss. Preserve the law independently at parsing and classification:

- `windows_dropped_observations` is a required integer in `0..=4294967295`;
  missing, Boolean, negative, fractional or oversized values refuse.
- `chromium_pid_unique_complete_trace` requires zero dropped Windows observations.
- `wine_pid_lifetime_complete_trace` requires zero dropped Windows observations.
- `wine_exact_role_and_self_exit_generation` is a separate authority, also requiring
  zero drops. Its retained self-exit is attached through the Windows PID-generation
  map; an aggregate drop counter cannot prove omitted generations were unrelated.
- Incomplete generation traces retain diagnostics as unbound leads and historical
  process observations, never decisive causal facts. The classifier enforces this
  independently even when given otherwise well-formed facts outside the parser.
- Complete positive generation authority may survive unrelated runner-tail loss.
  A missing record cannot establish absence, success or a selected remedy.
- Preserve closed process/fact schemas, exact category/authority mapping, ordered
  unique generations, hash/role/request coherence and exit-domain/status coherence.

Generated NAUI2 tests must carry the accepted GPU, sandbox, renderer, GDI,
role/self-exit, direct-classifier bypass and malformed-completeness regressions,
including positive controls with complete generations and lost runner text. Do
not weaken this law to make a renderer comparison selectable. Any new independent
handle-based authority needs explicit source-owned proof of its own generation
binding; naming a route differently does not establish independence.

The retained NAUI1 observation, source seal and all prior evidence remain unchanged.
Its exact historical application process generation is unavailable. It is not a
controlled inherited-renderer baseline.

## Diagnostic record

Retain private bounded data sufficient to distinguish:

- main application generation;
- `--type=gpu-process`;
- `--type=renderer`;
- `--type=utility` and network-service subtype;
- exact create/self-exit generations;
- canonical GPU exit/unusable records;
- sandbox/child `error_code=39` records;
- renderer-gone records;
- GDI exhaustion;
- bounded ANGLE/D3D/OpenGL initialization failures.

Do not publish full command lines, raw paths, URLs, account values, tokens, cookies,
window contents or vendor logs. The public result contains only closed roles,
ordinals, exit domains/statuses, allowlisted categories, hashes, counts, loss and
incompleteness.

The diagnostic sink must be large enough for a controlled application observation
but still bounded. Select and document explicit byte/record/line/time limits and test
every saturation path. A run with lost process authority cannot select a renderer
remedy.

## Human presentation observation

Add one closed, operation-bound observation supplied by the human operator:

```text
blank_white
rendered_nonblank
unavailable
```

This field reports presentation only. `rendered_nonblank` does not establish login,
network, authorization, product-list correctness or application usability. The
operator request cannot carry free text or screenshots. Exact Focus and Stop remain
available; the agent never injects input into Native Access.

A read-only X11/window witness may retain title, mapped state and geometry if it can
be bound through existing source-owned window/process custody. It must not infer
client content or replace the human presentation observation unless an independently
reviewed pixel authority is introduced.

## Generated proof

Before any Native Access launch, exercise the production owners with generated
source-owned fixtures.

At minimum:

1. Generate an x64 PE application larger than 64 MiB during the test/build; do not
   commit the binary.
2. Launch through the exact application adapter under the pinned runner.
3. Have the source-owned program spawn self-children with GPU, renderer and utility
   role arguments.
4. Exercise inherited and software-rendering argv laws and prove only
   `--disable-gpu` differs.
5. Emit allowlisted GPU, sandbox, renderer and GDI diagnostic cases through the real
   bounded sink.
6. Prove exact root/child generation, PID reuse, abnormal exit, cleanup and no
   Linux/Windows PID join.
7. Prove changed/aliased/oversized/unregistered application bytes refuse.
8. Prove process-observation loss and competing causes weaken to inconclusive.
9. Prove Arturia application behavior and ordinary installer/product paths remain
   unchanged.
10. Prove Focus/Stop, interrupted owner recovery, uncertain acknowledgment and
    terminal readback retain the exact operation.

AP8 is required because the Windows application adapter/fixture path changes. AP12
and PX2 are required. AP10 remains out of scope unless native/audio source changes.

## Source-review stop

The implementation PR stops after source and generated proof. It must remain draft,
uninstalled and unmerged for independent review. It must not launch Native Access or
create a real application operation.

No reinstall, updater, account sign-in, `app.asar` rewrite, registry mutation,
renderer workaround, sandbox workaround, runtime dependency, Wine/runner change,
product scan/publication or DAW session is part of the implementation PR.

## Later physical campaign

Only after exact-head acceptance, merge, merged-main CI, deliberate immutable
installation and a separate human authorization may the manager run this ordered
campaign:

```text
A. inherited
B. software_rendering
C. restored inherited
```

Each run uses the exact same application/environment/owner generation. The operator
observes the client presentation and closes the application normally. Stop after any
hang, crash, cleanup uncertainty, durable file mutation outside expected application
state, unexpected account action, or inability to bind the exact root.

Stop early after A when inherited renders nonblank; no remedy is then selected. Run B
only after A reproduces blank white with positive cleanup. Run C only after B retires
positively and is needed to establish reversibility.

Possible dispositions:

```text
NAUI2_INHERITED_RENDERED_NO_REMEDY_SELECTED
NAUI2_SOFTWARE_RENDERING_SELECTED
NAUI2_SOFTWARE_RENDERING_NOT_EFFECTIVE
NAUI2_SANDBOX_OR_CHILD_BOUNDARY_REQUIRES_SEPARATE_SLICE
NAUI2_COMPARISON_INCONCLUSIVE
NAUI2_CLEANUP_UNCONFIRMED
```

`NAUI2_SOFTWARE_RENDERING_SELECTED` requires inherited blank, software-rendering
nonblank, restored inherited blank, exact equal identities, zero dropped process
observations, positive cleanup and no competing specific failure family. It selects
only operation-scoped `--disable-gpu` for this application generation.

A sandbox/child result may select a later NAUI3 comparison, but NAUI2 never admits or
installs `--no-sandbox`.

## Review amendment: admission and submission authority

The installed Python persisted-spec route independently enforces the exact NAUI1
resource census, observation/seal, managed environment/root, historical installation
receipt, current software record, operation reservation and report location. Invalid
admission writes no result. Source-owned fixtures use a sealed internal entry with
`naui2-source-owned` identity; there is no installed fixture/test-mode selector.

Submission has three outcomes: `definitely_not_submitted`, `submitted_or_live`, and
`acknowledgment_uncertain`. The manager and supervisor share an operation writer
lock, acquired before reservation publication and held throughout the supervisor.
Unknown systemd state retains service suspension and exact Stop/reconcile authority.
Positive absent-unit/cgroup plus exclusive writer custody permits a no-replace
terminal refusal. A delayed unit cannot pass that terminal tombstone. A loaded unit,
even inactive, requires explicit synchronous Stop before recovery. Exact private
unit observations are retained separately from public receipts. Recovery preserves
an interrupted writer's prior result, including earlier renderer failure, in place
and adds `recovery-result.json` rather than rewriting the original evidence.

The four amended Windows sessions exercise the Rust reservation/submission/Stop core
with a source-owned application. The operator dispatch tests separately exercise
service-resume ownership and restoration through the same production helpers while
substituting system I/O. They do not claim a real bridge-service restart or GUI Focus.
Bounded ANGLE/D3D/OpenGL initialization facts are retained without selecting a remedy.
The public binding contains only schema, operation, epoch, token/artifact hashes,
size, status/reason and root ordinal. Windows PIDs/creation times stay private.
