# NAD2 — Exact NTKDaemon stop-response characterization

## 1. Selected basis

NAD2 begins from retained evidence commit
`b175fdf5317a6afb512b17dc02cd40fdd907a201`, whose parent is merged main
`710b0f3be642cca342f2a5915d37158246023aed` with tree
`77efd70fac7985a051343407d4ec13ca183422a8`.

The retained physical result is operation
`f4a2b703d8ea518d14c893c480fd93ed`:

```text
mode                         recover_installed
readiness                    exact generation + listener mask 3
stop controls submitted      1
ControlService error         0
SCM state                    RUNNING -> RUNNING
process wait                 WAIT_TIMEOUT (258)
listener mask                3 -> 3
queries                      209
control call                 670 ms
complete observation         12,045 ms
exit-receipt fallback        not considered; process acquisition succeeded
forced cleanup               used
preparation receipt          not created
```

This proves neither the cause of the non-transition nor a suitable workaround.
Native Access remains closed.

## 2. Purpose

Implement a closed, bounded characterization of the Windows service stop response so
one later, separately authorized physical run can distinguish these outcomes without
changing retirement policy:

```text
NAD2_STOP_CONFIRMED
NAD2_STOP_SUBMITTED_PROGRESSING
NAD2_STOP_SUBMITTED_NO_TRANSITION
NAD2_STOP_NOT_SUBMITTED
NAD2_STOPPED_PROCESS_OR_LISTENER_REMAINS
NAD2_STOP_OBSERVATION_UNAVAILABLE
```

Internal names may differ, but the report must preserve these semantic distinctions.
A successful `ControlService` API return proves only that the request was submitted;
it does not prove that the service accepted, began, or completed retirement.

NAD2 is diagnostic characterization. It does not authorize a second stop control, a
longer production wait, direct daemon termination as success, an alternative service
command, or a Native Access session.

## 3. Required implementation

### 3.1 Exact control-return observation

The fixed Windows adapter must retain, in addition to existing facts:

- initial `dwControlsAccepted`;
- the complete `SERVICE_STATUS` returned by `ControlService` when the call succeeds;
- final `dwControlsAccepted`;
- exact state, checkpoint, wait hint, Win32 exit, service-specific exit and query
  error values already observed;
- one-stop count;
- control-call and total-observation timing.

Do not use the PID field from `SERVICE_STOPPED` as identity authority. Existing exact
process-generation and listener requirements remain unchanged.

### 3.2 Bounded transition ledger

Retain a small structured ledger of distinct observations rather than every poll. Add
a record only when one of these changes:

```text
state
controls accepted
checkpoint
wait hint
Win32 exit
service-specific exit
query error
process-wait class
listener mask
```

The ledger must have a fixed maximum, a retained total observation count, and an
explicit dropped-transition count. Initial, control-return and final observations
must remain representable even when the cap is reached. Raw Windows identity remains
private.

### 3.3 Truthful classification

Classification must derive from observed facts:

- `STOPPED` plus exact process exit plus listener absence is confirmed retirement.
- `STOP_PENDING` or checkpoint/state progress without complete retirement is
  submitted/progressing only when the observations support that description.
- A successful control call followed by unchanged `RUNNING`, no checkpoint progress,
  a live exact process and live listeners is submitted/no-transition only as a
  bounded observation label; it remains unsuccessful retirement.
- A control or query failure remains not submitted or unavailable according to its
  exact error.
- `STOPPED` with a live exact process or listener remains failure.
- Forced cleanup, cgroup disappearance, bridge recovery, or keeper recovery cannot
  convert unsuccessful service retirement into success.
- No unsuccessful classification may produce a successful dependency-preparation
  receipt.

Do not infer vendor intent or causality from controls accepted, API success, timing,
logs, or generated fixtures.

### 3.4 Generated qualification

Use source-owned Windows service fixtures and the production parser to cover at
least:

1. `RUNNING -> STOP_PENDING -> STOPPED` with process/listener retirement.
2. `ControlService` succeeds but state remains `RUNNING` with no progress through the
   observation bound.
3. `STOP_PENDING` already exists and completes without a second control request.
4. `STOP_PENDING` progresses but does not complete before the bound.
5. Control refusal.
6. Query unavailability.
7. `STOPPED` while the exact process remains alive.
8. `STOPPED` while one or both listeners remain.
9. Transition-ledger saturation and dropped-transition accounting.
10. Exact manager/runtime parsing and public/private projection.
11. Forced cleanup remaining unsuccessful retirement.
12. A later generated operation remaining usable after every adverse case.

The generated fixtures must cross the production adapter and production parser.
Tests that merely duplicate the intended law are insufficient by themselves.

### 3.5 Operator presentation

The manager/frontend may explain the bounded classification, but must not claim:

- that NTKDaemon accepted the stop internally;
- that Wine lost the control;
- that more time would have succeeded;
- that a particular Native Instruments component blocked shutdown;
- that general Native Access or product operation is qualified.

Public results retain bounded numeric service facts and classification. Raw PID,
creation time, operation nonce, callback material, credentials, and private vendor
logs remain private.

## 4. Preservation and compatibility

The slice must not alter:

- Native Access's selected exact `--disable-gpu` rendering policy;
- browser-return handling;
- Native Access application lifetime;
- daemon installation/recovery admission;
- successful preparation requirements;
- completed-stop and acquisition-handoff behavior;
- Arturia application behavior;
- VST proxy, audio, state, editor, product publication, or capacity behavior;
- operator model 7 unless a schema change is independently required and reviewed.

All earlier evidence remains immutable. New generated evidence is additive.

## 5. Hard boundaries

During implementation and PR qualification:

- do not install a candidate;
- do not start or stop the real NTKDaemon;
- do not launch Native Access, a DAW, a plug-in, an updater, or a product installer;
- do not replay the daemon installer;
- do not use `net stop`, `sc stop`, `wineserver -k`, direct daemon execution, or
  name-based kill as a new production path;
- do not send more than one stop control;
- do not increase the 12-second production observation bound;
- do not convert forced cleanup into successful retirement;
- do not expose arbitrary service, path, port, command, registry, or environment
  inputs;
- do not rewrite prior evidence;
- do not claim a vendor fix from generated fixtures.

If characterization reveals a separate source defect, repair it only when it remains
within these boundaries. A new retirement strategy belongs to a later slice.

## 6. Required validation

Run:

- focused NAD2/NAD1 adapter and parser tests;
- the affected runtime suite;
- manager and frontend tests plus strict Clippy;
- Windows cross-compilation;
- AP8, AP12 and PX2 at the exact executable source;
- final-head checks when the final commit is evidence-only.

AP10 remains out of scope unless native/audio source changes.

## 7. Delivery

Return one draft PR, uninstalled and unmerged, containing:

- implementation and focused regressions;
- generated qualification evidence;
- exact executable-source and final-head identities;
- AP8/AP12/PX2 results;
- explicit preservation of prior Native Access, Arturia, VST, product and project
  behavior;
- an explicit statement that reliable real-daemon shutdown remains unqualified.

Do not perform a physical vendor check in the implementation PR. A later explicit
release authorization may permit one installation and one real characterization.
