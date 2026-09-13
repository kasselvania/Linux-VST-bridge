# IF2 — contained terminal state and native-host survival

IF1 proved durable terminal custody and host-visible failure, but candidate 15
lost the native Bitwig plug-in-host. Its Windows exit cause is unresolved; resize
was never selected. These observations remain unchanged in the IF1 evidence.

Two native policies conflicted with terminal presentation: immediate
`kReloadComponent` asks the DAW to unload both processor and controller, and
subsequent silent callbacks returned failure. IF2 keeps the current native
instance as the presentation and cleanup owner after a classified peer loss.

The queued worker and bounded UI query latch only a complete session/generation
bound IF1 terminal record. The audio callback reads an atomic Boolean. A new
native C ABI entry point retains the full existing input validation and returns
a distinct contained-terminal result before queue admission. The SDK edge then
writes exact stereo silence, retains separate terminal-silence counters, and
returns `kResultOk`. It flushes locally owned Note Offs through the current host
sink and never asks the dead Windows endpoint for output. Invalid extents,
buffers, events, values and context remain errors. No audio/wire layout changes.

The processor sends the existing one-shot terminal notification on `AP10.poll`.
The controller ends gestures, cancels focus and editor ownership, refuses dead
forwarding, and exposes the existing native terminal failure view. It does not
request automatic component reload. State requests fail explicitly; an old
cached blob is not silently returned. This is not in-place recovery.

Removal acknowledges contained cleanup only after worker join, exact terminal
custody, no outstanding control request, and the existing positive supervisor
cohort/transport retirement acknowledgement. That result is separately reported
from normal vendor retirement. A missing acknowledgement remains failure.

## Verification

The host fixture honors a full reload by actually terminating and releasing its
controller and processor after notification returns. It fails against the IF1
source because that removes the terminal-view owner. The corrected fixture also
covers the first failing callback before UI polling, repeated terminal silence,
malformed callbacks, zero-frame flush, stale-state refusal and positive removal.
The result-lifetime fixture covers exact local Note Off cleanup, a temporarily
absent result sink and callback allocation/blocking/I/O audit.

Rust tests use all three terminal producer classes, including the production
worker's unexpected peer EOF, preserve coherent custody after mapping unlink,
and verify that contained calls do not advance position/epoch or publish new
requests. Existing AP10/AP11/AP13 regressions remain applicable.

Generated verification passed, including all four hosted lanes at `b3006a4`.
The exact candidate-16 native artifact was built from `1da2983`; it reuses the
revision-15 Windows host, descriptor, module, runner, environment and policies.
The manager installed a new immutable software revision and published candidate
16 with exact ordinary 11 as parent. See [artifacts](../evidence/if2/artifacts.json)
and [validation](../evidence/if2/validation.json).

An earlier no-fault [preflight](../evidence/if2/product-check.json) paused at a
private capture approval boundary and restored ordinary 11. That original
record and its [readback](../evidence/if2/installed-final.json) remain unchanged.
After permission, the single [controlled failure check](../evidence/if2/live-check.json)
completed on candidate 16. No menu or resize action was attempted.

## Controlled child failure

The exact candidate was loaded through the normal Applications desktop entry in
a protected project with transport stopped. Its real editor opened once. The
source-owned helper verified profile, artifacts, session, supervisor ancestry,
and fresh process identities before sending one pidfd-bound SIGKILL to the
Windows child. It never signalled Bitwig or the native host.

The first complete record retained generation 1, epoch 1, sequence 25,044,
completed position 6,410,240 and confirmed snapshot revision 3. Native transport
failure class/status 3 won first-complete custody. The last Windows delivery
row was stage 3; no Collector rejection or normal vendor-retirement record was
present. This run has a known injected cause. It does not explain the historical
spontaneous Windows exits. The outer Proton launcher reported exit 1, which is
distinct from the deliberately signalled Windows child.

The same native PID/start identity survived custody, editor reopen and a later
host edit. The controller acknowledged exactly one terminal notification and
requested no automatic reload. Reopening displayed the 640×210 native terminal
failure view. On-Deck RGB comparison against a pixmap rendered with the same
source-owned text, colors, coordinates and font matched every byte, including
the retained snapshot identity. Only hashes and bounded facts are public.

A host Macro 1 edit changed its displayed value from 0.0600 to 0.1200 while the
exact-session native-to-Windows GUI counter remained 5. No new vendor command
was published. The live method return was not sampled; generated SDK tests
separately establish refusal return codes. After failure, 71,844 callbacks
produced 36,784,128 contained silent frames. These are terminal-silence counters,
not an audio-performance result or a relabelling of delivery gaps.

## Quit and restoration

One normal window-manager close requested Bitwig quit. The test-only Macro edit
was discarded with operator permission. Bitwig then warned that current
Pigments state was unavailable; proceeding was acknowledged without saving or
substituting the older confirmed snapshot. The frontend and native host exited
normally. The supervisor retained `terminal_instance_failure`, positive cohort
cleanup and transport retirement. Native `if2_terminal_cleanup` confirmed the
retirement acknowledgement and explicitly reported `normal_sdk_retirement=false`.
There was no forced native/Bitwig termination and no claim of in-place recovery.

The final native row retains two delivery-gap groups, 11,264 underrun frames,
512 expired frames and 6,409,728 delivered frames before contained terminal
silence. Earlier fault-status counters cover a shorter interval and are retained
separately. No musical, latency or performance comparison was made; SteamOS
CPUWeight 10000 was unchanged.

The [completed readback](../evidence/if2/installed-completed.json) proves exact
ordinary Pigments 11 restored, candidate 16 inactive, LoFi/FRAGMENTS 10 and
protected projects unchanged. Service/keeper remain active; DSP leases,
transactions and stale transports are zero, Bitwig/debuggers/diagnostics are
absent and tracing is off. Revisions 12–15 remain immutable inactive history.
512 stays selected/recommended and 256 unqualified. PR #99 remains draft and
unmerged for independent product review. Resize, UIO2 and the cause of prior
Windows exits remain outside this completed IF2 check.
