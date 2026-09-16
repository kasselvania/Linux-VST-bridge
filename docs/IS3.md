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
