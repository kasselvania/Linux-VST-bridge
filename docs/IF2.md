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

Generated verification and the one controlled product check are in progress.
No new live result, resize result, Windows exit cause or product acceptance is
claimed here. Candidate 16 will retain the exact revision-15 Windows host,
descriptor, module, runner, environment and compatibility policies, with the
changed native artifact bound immutably. Ordinary Pigments 11 remains rollback;
LoFi/FRAGMENTS 10 remain unchanged.
