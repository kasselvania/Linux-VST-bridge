# IS3 — Truthful Windows scripting capability

IS3 begins from integrated IS2, main
`8d306d8f211a66ff5f140705b8c7bc1f5bcd1de9`. The accepted stack and immutable
installation are retained in `evidence/is2/integration-installation.json`.
Source-owned tests cannot confer vendor compatibility or publication authority.

## Capability contract

A successful executable launch, zero exit, requested script side effect, requested
exit status, working process query and exact close are distinct facts. No
individual one proves the others. A basic script probe requires its requested
side effect **and** exit code. Full scripting/CIM/close capability requires the
additional query and exact-close observations; this initial probe does not claim
that complete capability.

The fixed source-owned consumer calls the pinned system PowerShell with three
scripts: file-write/exit-37, Get-CimInstance capability predicate, and an exact
known-absent process predicate requiring exit 1. Its declared fallback runs a
separate source-owned child requiring exit 43. It never kills by name or executes
vendor script text. Each helper has a 10-second wait; timeout cleanup uses only
its returned process handle. Missing records and timeout remain unavailable.

## Initial A/B/control result

The 32-bit payload ran once through the newly installed production installer
supervisor and adapter, pinned runner, private HOME, new disposable prefix and
exact dedicated cgroup. The imported source-owned root bound positively. Outer
exit was 0; every owned process retired and the scratch prefix was removed.

| Requested mode | Observed script behavior | Consumer fallback |
| --- | --- | --- |
| Baseline pinned stub | No side effect; all three helpers exit 0 | Not selected |
| Child environment requests `powershell.exe=` | Same false-success behavior | Not selected |
| Normal inherited environment again | Same false-success behavior | Not selected |

The `unavailable` label in the raw oracle names the **requested mode**, not a
proved unavailable capability. This proposed child-environment mechanism did not
work. The parser reports false success in all three modes rather than turning the
configuration intent into an observation. The experiment does not establish why
that loader setting was ineffective, nor whether another supported scoped route
would work. It does not establish vendor fallback behavior.

No genuine interpreter was installed or tested. Comparison C, a working managed
interpreter with script/query/exact-close proof, remains unperformed. No product
policy, prefix registry, pinned runner or vendor installer was changed. This is
initial IS3 evidence, not completion or installation clearance for an IS3 repair.

## Source basis and next boundary

Valve's [PowerShell implementation](https://github.com/ValveSoftware/wine/blob/b8fdff8e1f855b5276ec4ddca0f31b2792554322/programs/powershell/main.c)
corroborates the limited stub behavior; [loader override parsing](https://github.com/ValveSoftware/wine/blob/b8fdff8e1f855b5276ec4ddca0f31b2792554322/dlls/ntdll/unix/loadorder.c)
documents the environment override mechanism considered for the first probe.
These source references are comparison material, not a claim that this source
commit built the installed runner. The actual runner/image hashes and behavior
are recorded in the machine receipt.

The next work is to establish why the explicit child environment did not enforce
unavailability on this launch route, and select a supported, bounded mechanism
before comparing a genuine dependency. Never turn a dummy helper, returned zero,
or invented result into scripting capability. No Native Access confirmation is
authorized by this source-owned result.

## Windows delivery and Unix target-launch comparison

The continuation verifies canonical Windows environment blocks: one
case-insensitive override key, ordinal case-insensitive sorting, exact double NUL,
and preserved drive-current-directory entries. Malformed, duplicate and unsorted
blocks refuse in the Windows self-test. A source-owned child returns only presence,
UTF-16 length, SHA-256 of the UTF-16LE value, and duplicate count. Parent expected
and child observed hashes match; normal inherited values return in the restored
child. No inherited environment text is published. This proves delivery only.

The three new independent sessions use identical x86 payload bytes and the same
pinned runner, with fresh prefixes and dedicated production installer cgroups:

| Target-runner Unix environment | PowerShell result | Source-owned fallback |
| --- | --- | --- |
| Baseline | Launched, exit 0, sentinel absent | Not selected |
| Fixed `powershell.exe=` after prefix initialization | CreateProcessW refused, Win32 last-error 126; no helper exit or sentinel | Launched, exit 43 |
| Restored baseline in another fresh prefix | Launched, exit 0, sentinel absent | Not selected |

Each session retains the earlier three Windows-child modes. The Unix setting
makes all nine script/helper calls refuse; its three fallback calls each exit 43.
The middle session retains Wine's `get_load_order_value` diagnostic reporting an
empty environment load order for `powershell.exe`. Baseline/restored diagnostics
retain builtin loads. Combined with exact launch refusal and fallback receipts,
this establishes **operation-scoped honest absence** on this source-owned route.
The Windows-only injected block remains ineffective in the two baseline sessions.
No inference from the requested mode label is used as loader authority.

The supervisor seam is keyword-only and source-owned. Normal `--install` specs,
manager requests and frontend actions cannot enable it. The staged driver verifies
its payload/source manifest and binds the operation and artifact; the production
owner independently verifies those identities. It applies the single fixed Unix
setting immediately before target launch, after successful prefix initialization.
The installer adapter/root-token and existing cgroup cleanup remain unchanged.
The changed supervisor is staged diagnostic source, **not installed software**.

The first continuation baseline had an incomplete oracle because module tracing
filled the rotating diagnostic tail. It was refused and cleaned up. A separate
18-record/256-byte-per-record oracle sink preserves complete source-owned rows;
a separate 64-record bounded loader sink retains private diagnostics. The successful
baseline and restored observations dropped 170 supplemental loader rows each;
the override observation dropped none. All 18 oracle rows were retained in each
complete session. The earlier failed observation is retained, not overwritten.

`evidence/is3/loader-authority.json` records all exact source, payload, installed
artifact, private-record hashes and scalar receipts. The optional registry route
was not needed. No genuine interpreter was installed or tested: honest absence is
sufficient for the declared source-owned fallback contract. Whether it is sufficient
for Native Access is **not established**; its actual fallback and any remaining
scripting requirement need separate review/authorization. No commercial retry,
installed policy, runner replacement, product or publication mutation occurred.

Final readback retained all 307 checked files, five protected projects and existing
publications. Service active, two keepers healthy, zero DSP/maintenance leases,
transactions and stale transports, capture off. All four development sessions
(including the incomplete observation) retired with zero survivors and their
scratch prefixes removed. AP8 builds and self-tests both x86 and x64; the real
pinned comparison is x86. AP10 is out of scope.

## Sealed experiment custody (rereview repair)

The prior observation in `loader-authority.json` remains credible but is superseded
for comparison custody. `package.py` now builds the fixed x86 consumer from clean,
committed source and stages an exact file set. `fixture-manifest.json` is the
resulting canonical, retained manifest: source head/tree, payload architecture and
bytes, complete runner identity, both PowerShell images, installed owners, baseline
override receipt, compiler recipe and every driver/owner source digest.

The manifest does not hash itself. Its SHA-256 is a detached seal supplied to the
campaign and each supervised invocation, and retained in every proof and the public
result. The exact package directory includes the manifest plus the closed hashed
file set. Extra/missing files, unknown manifest fields, duplicate JSON keys,
noncanonical manifest bytes, writable or altered files, a changed seal, and source
generation mismatch refuse before entering the production installer owner.

The package and runner APIs here are source-owned development tools, not operator
capabilities. The runner is selected by its sealed expected identity. Repeated
registrations of the identical runner are one identity; conflicting generations
with the same runner ID refuse. Every declared runner file, entry-point and Proton
artifact is verified, and paths are retained publicly only as hashes. Installed
manager, frontend, supervisor, ownership helper, adapter and the complete software
record are bound separately from the staged session/ownership sources.

`campaign.py` verifies the package before each session, invokes baseline → Unix
override → restored, rereads the three immutable private proofs, and calls the
committed comparator. That comparator requires canonical byte equality of the
complete identity envelope, including the sealed manifest digest and every source
owner. Baseline Windows override identities must match exactly across both outer
sessions; the middle session's changed override is the declared independent
variable, not silently normalized away. The campaign projects bounded private
loader diagnostics, then writes `loader-authority.json` using a complete fsynced
temporary file, atomic no-replace link and parent fsync. Failures stop the sequence;
no result or historical evidence is overwritten.

The source commit recorded by a package precedes its generated manifest/evidence
commit, avoiding a Git/hash self-reference. All executable source bytes in the
final candidate must still match that committed manifest. Tests mutate every
identity leaf, require the exact file set/seal, reject ambiguous runners, exercise
campaign order/early stop, and protect atomic retained results. AP8 hashes all
binaries only after x64 and x86 build/self-test completion.
