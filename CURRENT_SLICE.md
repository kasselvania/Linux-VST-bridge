# CA1 — per-instance crash attribution

## Selected outcome

Give an ordinary plug-in-use failure an actionable, private incident report:
which process failed, the available exception/signal/exit, faulting module and
module-relative offset, available stack, relevant recent context, and the
existing IF1/IF2 failure and cleanup facts. Do not stop at `endpoint disconnected`.
The user also reported a crash when changing a wavetable waveform. That is an
operator observation, not a captured cause or proof of a rendering/resource bug.

IF2 was accepted by review 5188874997 at
`1cb75f16dd4249a6f51ef9f5438071b4fd5ddb66` and merged through PR #99 as
`383185ccb3fc8ff63624f64a2f97f89c881e54e4`. Its controlled Windows-child failure
proved native-host survival, terminal presentation, dead-forwarding refusal,
and positive cleanup. Preserve that result; do not repeat its campaign.

Continue on `codex/ca1-per-instance-crash-attribution` from that merge. The
preparation changes only this file. Implement in this same branch and its draft
PR; no separate planning review or closure PR is needed. Read AGENTS.md and the
relevant source, not the entire historical corpus. Normal implementation,
focused verification and in-scope debugging are authorized; use engineering
judgment rather than a universal retry count.

## Reuse the existing owners

- `bridge-manager/runtime/session.py`: `run`, `FaultStatus`, `TerminalStatus`,
  `vendor_diagnostic_environment`, `PrivateCapture`, process supervision and
  receipt finalization. Extend this path; do not launch a second process owner.
- `bridge-manager/runtime/ownership.py`: existing identity/containment helpers.
  Borrow observation logic without changing cohort ownership or killing a keeper.
- `bridge-manager/src/main.rs` and existing manager observation/CLI owners:
  select, arm and report one admitted instance through canonical manager state.
- `tools/ap18-tests/crash_capture.cpp` and the AP18 validated attribution work:
  the existing fixture deliberately faults through known functions but catches
  the exception and exits zero. It proves unwinding, NOT terminal-crash handling.
- `docs/AP18.md`, sections "Validated attribution follow-up" and "Exact
  attribution and scoped mitigation candidate", and
  `evidence/ap18/post-login/uia-attribution.json`: actual ASC precedent.
- IF1/IF2 terminal records, native reports, and optional existing UIO1 records:
  correlate them; do not duplicate their ownership or add callback logging.

The current plug-in supervisor keeps only the first 65,536 bytes of vendor
stdout and stderr. ASC's PrivateCapture is also first-N/time-limited. Simply
turning on Wine logs through either unchanged sink can lose a late crash.
Separate the validated host protocol from diagnostic streams: observing extra
logs must not consume, corrupt, reorder or relax protocol admission.

## Implementation contract

**Opt-in, exact-instance capture.** Provide a usable way to arm the next admitted
instance, see whether capture is active, obtain its incident summary, and disarm.
The CLI spelling and private types are the engineer's choice. Resolve installed
or explicitly selected qualification identity, not an arbitrary executable or
PID supplied as authority. Arm before launch so loaded bases and early module
facts exist. Do not silently restart an already-running instance to enable it.
Keep diagnostics off for unrelated instances, ASC, keeper and siblings. Disable
capture on session completion/cancellation; an unused arm must be cancellable.

**Surviving bounded evidence.** Reuse the proven process/SEH/unwind/loader capture
configuration through separate bounded private pipes; leave Proton's unbounded
log redirection off. Retain module/load/unload identity separately from a recent
context ring and reserve capacity for terminal exception/exit material. Preserve
actual module bases and the matching binary identity, not preferred PE bases.
Choose and document finite memory, disk, record, line-length, retention and
final-drain limits. Continue draining after quota exhaustion; count dropped
bytes/records and report incomplete attribution rather than blocking the child
or allowing logging to exhaust storage. Do not stop all retention merely because
a generic first fault arrived. Do not add a per-audio-block disk journal.

**One incident, separate facts.** Keep the original IF1 first-failure record
immutable. Attach later attribution to the same session/incident. Distinguish
Windows-host exit, outer Proton exit, native plug-in-host outcome, bridge-owned
termination/timeout and ordinary retirement. Identify processes by PID plus
start identity and actual mapped images. Preserve available exception code,
thread, access kind, fault module/offset, stack, module identity and relevant
recent bridge/loader messages. Resolve symbols against exact artifacts when
available; proprietary frames may truthfully remain module plus offset.
Do not obtain nonexistent child wait status by relabeling a launcher return.
An unavailable native exit/stack is explicit; do not change system-wide core
policy or attach a debugger to unrelated processes to fill it.

A first-chance/handled exception is not automatically fatal. A signal such as
SIGKILL has no catchable Windows exception stack. A faulting DLL, the last
missing-file message, or temporal proximity to a UI action is not automatically
the cause. Separate observation, likely failure path and established cause.
Keep original clock domains; do not subtract unsynchronized clocks.

**Practical report.** Produce a local readable summary and machine-readable
record with incident ID, exact build/session, outcome, available attribution,
limits/dropped-data counts, first-failure and cleanup facts, and optional user
note such as "changed wavetable waveform". A note is not measured telemetry.
Include a bounded relevant-error excerpt, not arbitrary filesystem/network
tracing or an entire log as the only result. A sanitizer must explicitly select
what is safe to share. Raw diagnostics are sensitive: private directories/files,
no automatic upload, no public paths/PIDs, credentials, authorization payloads,
proprietary binary/preset/state bytes, screenshots, arguments or locals.

**Do not disturb the product.** Preserve IF2's native-host survival, successful
contained-silence callbacks, truthful state-unavailable result, terminal view,
local Note Off cleanup and positive ownership acknowledgement. Diagnostic I/O or
parsing failure must not prevent containment/removal. Final capture is bounded,
not a new indefinite wait. Normal launch with diagnostics off stays unchanged.
Do not change graphics drivers, runner, DLL overrides, accessibility policy,
resources, scheduling, protocol, musical behavior or saved projects on a theory.
Full dumps, new debugger infrastructure, general UI automation, popup/resize
qualification, recovery UI and second-vendor work are outside CA1.

## Proportionate verification

Use the same production collector/finalizer for the proof, not only parser mocks.
Reuse the AP18 fixture and known fault/caller symbols under the exact pinned
runner; retain its handled/zero-exit case and add an unhandled-fault case so a
terminal report proves process/thread/module-relative attribution and available
call frames after the child exits. A small explicit-exit/normal-exit case must
not invent an exception or crash. Existing failure cleanup tests supply their
accepted proof; extend only where the collector changes it.

Focused tests cover a crash after early log capacity is exhausted, a malformed
or oversized record, absent symbols/stack, a first-chance exception followed by
normal continuation, finalization/write failure, identity mixing, and unrelated
process safety. Preserve IF1 first-winner data when later details arrive.
Use existing fixtures/retained safe examples; do not create a broad matrix.
Run affected tests and Clippy plus applicable CI. Reuse accepted product binaries
when their inputs are unchanged; no Windows/native rebuild for report-only edits.

After the generated end-to-end capture works, exercise the reporter on a
protected, reversible candidate-16 Pigments session without mouse/focus takeover.
The selected normal-use case may be a waveform change; the engineer may use
existing permitted tooling, or leave a verified armed handoff for the user's
ordinary interaction. Do not require popup capture or re-run menu/resize first.
Do not modify user projects or dismiss a save/permission dialog without authority.
Inspect an existing spontaneous incident if useful facts are already retained.
If a natural crash occurs, finalize and interpret that incident before another
launch. Do not make repeated blind attempts merely to obtain a crash. A healthy
session is not proof that the historical crash is fixed.

The reporter can be reviewed as ready when its real collector captures a known
fatal failure and works in the plug-in launch path, even if a spontaneous
Pigments crash does not recur. Report the two results separately: reporter
readiness versus historical-crash attribution. No fabricated cause or indefinite
stress run to manufacture completion. A real actionable Pigments report is the
preferred diagnostic result, not a precondition for every source change.

## Installation and handoff

The last verified baseline is ordinary Pigments 11 active; revisions 12–16
inactive; LoFi/FRAGMENTS 10 unchanged. This preparation did not access the Deck.
Candidate 16 already has accepted containment and the repaired input pump; reuse
its exact host/native bytes for CA1 if they are unchanged. Diagnostic manager
changes use existing immutable setup. No new Pigments revision just for logs.
If host/native product bytes genuinely change, bind a new immutable candidate
through the existing route without mutating prior history or promoting it.

Restore ordinary 11 after an agent-owned session, turn diagnostics off and leave
no held input, owned helper, DSP lease, transaction or stale transport. Preserve
projects, authorization and SteamOS policy; 512 remains recommended and 256 is
unqualified. A user-requested armed handoff is reported explicitly rather than
misrepresented as idle cleanup. Finish this PR with code, focused proof, a
sample sanitized report, exact installed/armed state, unresolved attribution and
next evidence-backed repair direction. No ordinary Pigments acceptance is implied.

## Implementation result

CA1_REPORTER_READY_FOR_REVIEW. Implementation source
`ffbe640a447f8767282966aad86017cb63cc847d` and the final installed reporter
passed the focused exception/cleanup proof. [CA1](docs/CA1.md) and
[evidence/ca1](evidence/ca1/) retain the source-owned fatal/handled distinction,
one protected candidate-16 waveform interaction, the historical module-lookup
correction, and final physical readback.

The waveform selection completed and the session retired normally. This does
not reproduce, attribute or repair the historical crash. Ordinary Pigments 11
is restored; capture is off, with no user handoff armed. No product revision or
ordinary promotion was created. PR #100 remains open for independent review.
