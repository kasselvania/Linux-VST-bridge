# Runtime and SCM shutdown repair

This amendment repairs the four findings in the tech-lead review of `ddbbd54`.
It does not assert that the real daemon's previous exit 149 is explained or fixed.
The successful signed-in library session and subsequent failed preparation remain
separate, unchanged historical results.

## Source and authority

Executed source: `f3487e5616a8a7141ce8ae113562d18d1c14da02`.
Tree: `44120a7f90081892e825e3546c09537e695ef07c`.
`source-seal.json` hashes the principal runtime, adapter, fixtures and workflow
inputs; the Git tree binds the full source. Hosted PR checks execute GitHub's test
merge `4510260bb938d21cdf0a1681108477b4a58fd203`, independently verified to have
that same tree. No installed generation or real prefix is part of this execution.

The small Arturia normal-completion repair is separate commit `a8e9d75`.
Runtime and SCM changes are `1e5a868`; `f3487e5` retains the precise runtime-close
error when SCM retirement itself succeeded. The subsequent evidence commit does
not change those executing inputs.

## Repaired behavior

- Resource closure attempts all remaining steps and retains closed error categories.
  Both dependency and application owners still attempt process cleanup and publish
  a terminal report after a generated `ENOTEMPTY`. An earlier readiness/application
  failure keeps precedence. A close error alone also produces failure.
- One operation-owned reader drains runtime stdout and stderr continuously, including
  while commands, application waits, census and retirement are underway. Capture
  saturation discards bytes without ceasing pipe reads. A synchronized login cutoff
  stops raw capture, while drainage continues. A reader error is a retained failure.
- Stop admission retains the previously observed Windows running generation. This
  is a private supervisor-to-adapter request, not an operator argument. Pending
  state cannot supply a new process identity. Clean stop requires SCM STOPPED,
  the exact retained process handle signaling exit, no owned listeners, and the
  independent Linux daemon census retiring. STOPPED's invalid SCM PID is not tested.
- Already STOP_PENDING is observed without another control request. A race yielding
  `ERROR_SERVICE_CANNOT_ACCEPT_CTRL` is observed without retrying. The existing
  12-second observation budget includes ControlService time. The supervisor retains
  its separate bounded helper/cleanup limits; no timeout increase was introduced.
- A fixed numeric result retains initial/final SCM state, query errors, checkpoint,
  wait hint, process wait/error, endpoint mask, control result/timing, total elapsed
  time and progress/query counts. These fields remain available after raw login
  diagnostics are suppressed. If the helper dies before its final frame, the result
  explicitly records that unavailability and any observed control-dispatch marker.

Microsoft documents the state-dependent PID contract in
[QueryServiceStatusEx](https://learn.microsoft.com/en-us/windows/win32/api/winsvc/nf-winsvc-queryservicestatusex)
and the existing-pending-stop observation in
[Stopping a Service](https://learn.microsoft.com/en-us/windows/win32/services/stopping-a-service).

## Generated qualification

Local and hosted Linux tests run generated Python writers through real pipes,
exceed pipe/capture capacity both before and after privacy cutoff, and exercise
both production finalizers with a real nonempty temporary directory. A generated
nonzero stop helper demonstrates that structured facts survive raw suppression.

The portable C++ test compiles the same stop-state law included by the Windows
adapter. It covers pending and racing stop, delayed progress, stalled checkpoints,
query/control failures, invalid handle/identity, live process/listeners and a
control call consuming the observation budget. This is generated state-machine
proof, not a real Windows timing measurement.

AP8 additionally runs the source-owned service through Windows SCM: normal stop,
repeat start, a three-second checkpointed shutdown, and an externally initiated
pending stop. The last case checks no second control was sent and exactly one
fixture stop request occurred. The real application and daemon are never involved.

`validation.json` retains local limitations and hosted workflow identities. The
first four local manager failures were sandbox Unix-socket refusals; all affected
binary tests passed with local socket access, and the full Linux manager suite
passed in AP12. No failed physical vendor attempt was repeated or reclassified.

## Preservation and remaining boundary

`preservation.json` records equality of every previously tracked evidence file
against the reviewed checkpoint. No device connection, production installation,
real daemon operation, Native Access launch, registry mutation, product operation
or DAW operation occurred during this repair. Accordingly, this receipt makes no
new live-state/health claim about the Deck.

Draft PR #121 carries the continuation beyond merged PR #120. Native Access stays
closed for review. Generated success establishes the repaired paths; only a later
explicitly authorized real check can establish reliable shutdown for NTKDaemon.
