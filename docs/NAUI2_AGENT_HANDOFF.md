# NAUI2 implementation-agent handoff

The operator authorized and the implementation completed the source/generated cut
in PR #117. This document retains the original bounded assignment; current results
are in `evidence/naui2/README.md`. No additional implementation handoff is needed.
A real Native Access comparison still requires review and deliberate installation.

## Exact basis

Work only from this prepared branch and read, in order:

1. `CURRENT_SLICE.md`
2. `docs/NAUI2.md`
3. `docs/NAUI1.md`
4. `evidence/naui1/README.md`
5. the existing `vendor_application`, `vendor_cli`, operator and runtime owners
6. the IS2 exact launch adapter and IS1/IS4 process/effective-policy custody

This branch is rebased directly onto the exact NAUI1 merge:

```text
merge: eeffae76d0e34e9a2a09cddd05707afac665c4de
accepted PR #116 head: 5259d0069297e074501dda292c40239c1da04000
tree: 93b015a54211fc3df35f0ce05ea815ed64673fc9
```

The old planning commit `e4062c50f5564024006866c284db333ae1fd94e6` is superseded.
Do not start from its pre-repair parent. The corrected branch includes the accepted
parser/classifier completeness law and all original evidence unchanged.

Read `docs/NAUI2.md` → “Inherited trace-completeness law” before implementation.
Both parser and classifier must block positive Chromium/GDI PID and retained
role/self-exit attribution with Windows trace drops. Ordinary runner-tail loss is
separate. Preserve all 47 accepted NAUI1 tests and their authority distinctions.

Do not modify or rewrite NAUI1 evidence. Its physical source remains
`f37d7aebe7bb86722aac1349ba79cb80d1439ab3`; the repaired repository source is not
claimed to have produced that historical observation.

## Integration verification

Merged-main checks passed against exact merge
`eeffae76d0e34e9a2a09cddd05707afac665c4de`:

- [AP12 registered owner](https://github.com/kasselvania/Linux-VST-bridge/actions/runs/35174673738): manager and frontend jobs passed.
- [PX2 proof policy](https://github.com/kasselvania/Linux-VST-bridge/actions/runs/35174661711): passed.

The merge tree equals accepted tree `93b015a54211fc3df35f0ce05ea815ed64673fc9`.
The rebased preparation changes only the three planning documents. All accepted
NAUI1 executable source and historical evidence remain unchanged. Its 47 generated
NAUI1 tests also pass locally on the preparation. No Deck connection, installation
or application process is needed or performed by this integration.

## Mission

Implement the source and generated proof needed for a later exact Native Access
renderer comparison. Do **not** launch Native Access in this PR.

The implementation must make the following future experiment possible without an
arbitrary launcher:

```text
inherited -> software_rendering (--disable-gpu only) -> restored inherited
```

## Required implementation

### 1. Exact Native Access application owner

Add a closed Native Access application identity alongside the existing Arturia
Software Center owner. It must derive and verify the exact NAUI1 application from
manager state and committed evidence:

```text
environment: 627d2cba97edbecf113c22504eb4c81b
Native Access.exe:
  sha256: 7b2413e90db79538cd1edc7c6169ff384aea181a85635c43a363f4aa18b9e841
  size: 225757168
app.asar:
  sha256: 2df87bef2a7c7113b56374d4f5620519d268496bba9414db2e3c101328783ea5
version: 3.26.0
PE version: 3.26.0.962
architecture: x64
```

Do not accept caller paths, digests, versions, environments, commands or executable
names. Preserve Arturia behavior and existing schemas unless a version bump is
required and migrated explicitly.

### 2. Closed renderer policy

Add only:

```text
Inherited
SoftwareRendering
```

`SoftwareRendering` means exactly one `--disable-gpu` argument. It does not mean
`--no-sandbox`, `--in-process-gpu`, DXVK, ANGLE selection, registry overrides,
application patching or a generic Chromium argument list.

The manager/operator request contains the exact application identity and enum only.
Unknown fields and arbitrary args/environment/path/PID/window inputs must fail
closed. Retain requested versus effective policy separately; effective authority
starts only after exact owned target creation.

### 3. Exact application launch-root binding

Do not use first-process, name or ordering inference. Reuse or factor the IS2 adapter
pattern to bind operation, token, exact artifact, child handle/image generation,
epoch, root ordinal and renderer policy. Keep the adapter contract closed to the
exact manager-generated application request.

### 4. Large-image process custody

Support the exact 225,757,168-byte application without turning every process hash
into an unbounded read. Use stable open-file identity and bounded digest caching for
the admitted application generation. Keep existing unrelated limits unchanged.

Retain exact Windows generations and parentage for main/GPU/renderer/utility roles.
No Linux/Windows PID equality. No process-name identity. A selected causal result
must require complete process observation; loss weakens to inconclusive.

### 5. Bounded diagnostics and presentation

Retain allowlisted Chromium/Electron/Wine categories privately and project only
closed facts. Add the human-only, operation-bound presentation enum:

```text
blank_white
rendered_nonblank
unavailable
```

No free-text note, screenshot payload, account value, URL, command line or raw path
may enter the public result. Exact Focus and Stop must remain usable.

### 6. Generated fixtures

Exercise the production owners, not a parallel toy classifier. Build a generated
x64 PE larger than 64 MiB, launch it under the pinned runner through the exact
adapter, self-spawn role children, and cover inherited/software/restored argv,
process generations, diagnostics, abnormal exits, PID reuse, saturation, competing
causes and positive cleanup.

The binary must be generated, not committed. No vendor bytes or copied community
source enter the repository.

## Negative coverage

At minimum prove refusal or unresolved behavior for:

- another environment or similar application name;
- changed/missing/aliased executable, ASAR or required resources;
- missing/conflicting launch binding;
- arbitrary argument/environment/path/PID fields;
- `--no-sandbox` or any unrecognized renderer mode;
- duplicate/reused PID without retirement;
- parent retired before child creation;
- image larger than the closed admitted application grant;
- application replacement between verification and creation;
- dropped Windows-generation observations for each Chromium, GDI and role/self-exit
  attribution route, including bypassing the parser with a well-formed fact;
- malformed completeness counters or process rows;
- complete Windows generations with ordinary runner-tail loss (positive authority
  survives under the accepted NAUI1 law);
- diagnostic prose without exact source grammar/generation;
- competing GPU/sandbox/GDI causes;
- lost acknowledgment and interrupted owner recovery;
- Stop against the wrong operation;
- any regression in the Arturia application path or ordinary installer/product
  behavior.

## Validation

Run focused suites plus all affected repository tests and strict checks. Required CI:

```text
AP8 Windows host
AP12 registered owner
PX2 proof policy
```

AP10 is out of scope unless you change native/audio behavior.

## Evidence and PR

Commit implementation source before generated proof. Retain exact source head/tree,
build inputs, fixture identities, results and cleanup. Separate generated evidence
from later documentation-only commits.

After implementation is authorized and completed, open one **draft** PR against
main. PR #116 is already merged. The implementation PR must remain unmerged and
uninstalled for independent review; this planning reconciliation creates no PR.

Do not perform any real Native Access launch, reinstall, updater run, account input,
registry mutation, `app.asar` modification, graphics/runtime dependency install,
Wine/runner replacement, scan, publication, Bitwig session or product interaction.

Stop and return the exact head/tree, changed scope, generated result, validation and
preservation state. A physical comparison requires a later explicit authorization.
