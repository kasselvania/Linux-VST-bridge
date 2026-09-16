# Two retained Native Access attempts — offline result

Both attempts are **partial installations**. Environment initialization and the
real bootstrapper launch succeeded. The VC++ prerequisite packages completed;
the later outer route exited 2. The post-prerequisite failing child is unknown.

| Witness | Attempt 1 | Attempt 2 |
|---|---|---|
| Duration | 42.483 s | 90.390 s |
| Outer exit | 2 | 2 |
| VC++ runtime | 14.44.35211, retained success | Same exact packages, retained success |
| Native Access marker | 3.26.0 | 3.26.0 |
| Application executable in inspected surfaces | Not found | Not found |
| NTK service registration | Not found | Not found |
| VST output | Not found | Not found |
| Owned process cleanup | Positive | Positive |

The immutable import is a 179,877,344-byte x86 NSIS executable, SHA-256
`82d7b7977d4fbc19db32aee8778c75906f6ddf935bcf2958f2d8d45141b2b2cc`.
Its version resources identify Native Access 3.26.0.962 / product 3.26.0 and its
manifest requests administrator execution. This does not prove an elevation
problem.

The same prerequisite executables/MSI packages and uninstall entries survived
both attempts. The 24 observed default service configurations match. The three
prerequisite logs differ by attempt, while recording successful prerequisite
completion. There is no full pre-attempt prefix baseline, so a complete historical
file/registry delta cannot be reconstructed.

Xalia reported queries involving already exited processes. That is a retained
accessibility-observer lead, not an established installer cause. No fatal stack,
NTK failure, graphics failure, UAC failure or Proton defect is attributed.

[Sanitized metadata, hashes, scope and preserved-record identities](offline.json)
retain the detailed comparison. Original attempt records and environments were
not edited. No Native Access process was launched for this comparison.
