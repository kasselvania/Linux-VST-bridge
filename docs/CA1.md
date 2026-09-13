# CA1 — per-instance crash attribution

CA1 adds optional private attribution to the existing session supervisor. It does
not change the native proxy, Windows product host, IF1 first-failure record,
IF2 contained-terminal behavior, runner, or Pigments compatibility policy.
Candidate 16 remains the exact product test binding. Ordinary Pigments 11 is the
rollback; LoFi and FRAGMENTS 10 remain independent.

## Operator route

Use the installed manager's ordinary product selection, then:

```text
linux-vst-bridge capture arm SELECTION
linux-vst-bridge capture status
linux-vst-bridge capture summary INCIDENT
linux-vst-bridge capture report INCIDENT
linux-vst-bridge capture export INCIDENT
linux-vst-bridge capture disarm
```

`arm` requires an exact physically published verified profile. `arm-failure`
is the existing sealed IF1/IF2 candidate-16 observation selection, for this
engineering check only. Neither accepts a PID, executable, command or binary path.
Arming takes effect only on the next matching DSP admission, never restarts an
existing instance, and does not arm a keeper, scanner, ASC or sibling class.
A changed registration or publication expires the arm. The request retains the
exact profile fingerprint, publication, registration and installed software.

`summary` and `report` are **private**. `export` explicitly selects numeric failure
facts, module hashes/offsets and cleanup/capacity facts; it excludes paths, process
and session identifiers, symbols, raw text, state and account material. There is
no upload. A user's description such as “changed waveform” belongs in their
incident note, not in measured telemetry.

Disarming cancels unused admission immediately or active retention at the next
supervisor turn (checked once per second). It never kills the instance. Already
launched Wine diagnostic flags cannot be removed without restarting that process;
the pipes continue draining until the normal session ends. No restart is automatic.

## Collection and limits

The existing ASC-proven `process`, `seh`, `unwind`, `loaddll` and module-error
channels use the existing private pipe owner. Unbounded Proton file logging and
DXVK logs remain disabled. Diagnostic output is separate from validated host
protocol records. No callback code changes.

The collector separates:

- up to 1,024 loaded-module rows, with actual Wine load base and Windows process/thread;
- 128 Linux PID/start identities from the existing ownership tracker, with an
  aggregate 512 KiB allowance for actual mapped paths;
- a 128 KiB / 512-row recent-error tail;
- a separate 256 KiB / 1,024-row exception/exit tail;
- the latest 16 scalar exceptions, up to 64 unwind frames each, plus four reserved
  exceptions correlated with a matching self-exit status;
- one unchanged session-matching IF1 terminal record and existing fault/UI status.

Lines are limited to 8 KiB. Oversized lines are skipped through newline while
pipes keep draining. Module, context, frame, process and line losses are counted.
Retention expires after two hours. Final drain is bounded to 250 ms after owned
cleanup; unresolved pipe data is reported. PE identity resolution admits work for
five seconds, with finite PE section/export/filename limits and a 512 MiB hash
budget. It does not fetch symbols or inspect arbitrary files: identities come from
actual mapped paths or the exact verified host/module. No preferred PE base or
basename match is address authority. Unloads match the exact full Windows path
and process. Load/unload history is resolved at the exception timestamp, not
at finalization: unrelated or later unloads cannot erase a prior frame. Unknown
unload syntax makes only subsequent bindings uncertain.

Private directories are 0700, files 0600. Incident JSON is capped at 4 MiB;
summary is capped at 64 KiB with omissions reported. At most 64 incident
directories may be retained before explicit operator housekeeping is required.
No automatic deletion of prior incidents is performed. Startup first-N output
cannot consume the independent terminal or module allowance.

Finalization follows positive process/transport cleanup and the minimal ownership
receipt. A report error is retained separately and cannot authorize cleanup or
prevent it. Native exit status is explicitly unavailable when this supervisor is
not its parent; PID/start survival at finalization is a separate observation.

## Interpretation

Wine trace time, Linux monotonic observation time and existing terminal identity
remain separate domains. The outer Proton wait result is never reused as a
Windows or native exit status. Wine self-termination records use **signed decimal**
NTSTATUS values on this runner; CA1 retains the original and unsigned 32-bit value.
A real process handle is not treated as a self-exit pseudo-handle.

A first-chance exception followed by Windows exit zero is recorded as normal
continuation, not a crash. Matching exception and self-exit status is a failure
path witness, not proof of the underlying cause. An access-violation frame in a
DLL, missing-file error, or UI action's temporal proximity does not establish that
DLL, resource or UI operation as root cause. SIGKILL alone supplies no Windows
exception stack. Missing frames/symbols remain unavailable.

Export symbols are labelled **nearest export; function extent unavailable**.
The exact fixture has deliberate exported fault/caller functions; proprietary
code may have only module SHA-256 plus relative offset. No vendor binary, state,
preset, register contents, locals or arguments enter public evidence.

## Focused proof

`tools/ca1/fixture.py` substitutes only the test command/registration. It invokes
actual production `session.run`: pinned runner, process tracking, pipe collection,
finalization and containment. The source-owned AP18 fixture has handled, normal,
explicit exit, fatal and late-fatal modes. The fatal modes really fault through
`ap18_fault_caller -> ap18_fault_site`; they are not SIGKILL or parser simulation.
Fixture-only `SEM_NOGPFAULTERRORBOX` avoids launching a crash dialog/debugger.

The final late-fatal fixture retains `0xc0000005`, fault RVA `0x1006`, caller RVA
`0x101f`, and available subsequent frames after roughly 489 KiB of output exceeds
the old 64 KiB retention. Windows status and outer Proton exit 5 remain distinct.
The first fatal proof exposed signed-decimal exit parsing; it was corrected from
that retained record, with a focused regression. No vendor retry selected that fix.

Manager tests cover exact one-shot binding, mismatches, unrelated class admission,
and disarm. Runtime tests cover saturation, malformed/oversized data, first-terminal
immutability, identity separation, missing symbols, disabled capture, actual
production cleanup with an unrelated sibling, and finalization failure.

Product-session and final installed results are retained separately under
`evidence/ca1/`; reporter readiness does not imply that the historical waveform
crash has been reproduced, attributed or repaired.

## Candidate-16 session

One protected Applications-launched Bitwig session exercised capture from launch
through normal quit. The editor switched Engine 1 to Wavetable, then selected
Basic Waveforms → Basics Clip once while the existing C3 loop ran. The local
editor image changed; the endpoint remained alive. No menu/resize campaign or
manual Mac input was used. The source-owned window driver briefly refused a
second write to its existing private preflight filename; the original was
preserved and a new readback filename was used, without repeating an action.

Pigments committed all seven process-scoped retirement milestones and retired
transport/cohort ownership. There was no IF1 terminal failure. Four observed
Windows exceptions remained first-chance/unclassified exception observations;
normal continuation and retirement do not turn them into a terminal crash.
No historical waveform-crash cause is claimed.

Inspecting this report exposed an over-conservative module-history bug: any
unload had invalidated all modules of that process, including unrelated images
and earlier exception frames. The path/time-specific correction has generated
coverage and a private reprojection of the already retained record. The original
incident is preserved. No second Pigments run was needed to correct the reporter.
