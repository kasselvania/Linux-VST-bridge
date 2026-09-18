# NAD2 generated stop-response characterization

Executable source: `0ea735e471b2d34a68d9ce817506fe89fdd1ca23`  
Tree: `cd567a0b113924b87b15213d7cd8a4baf3213dbb`  
Selected authority: `CURRENT_SLICE.md` and `docs/NAD2.md` at
`bea752e5b78631bafcc118b4cc8502141b1b7d88`.

The production Windows adapter now retains the initial and final controls-accepted
masks, the complete `SERVICE_STATUS` returned by a successful `ControlService`,
one-control count, control and observation timing, exact-generation process-wait
class, listener masks, and a fixed 12-entry distinct-transition ledger with total
and dropped counts. A successful control return means only that submission occurred.
It is not service acceptance.

The production runtime parser independently validates and projects the six bounded
NAD2 classifications. Manager success construction is unchanged: only SCM stopped,
exact admitted generation exited, and both owned listeners absent confirm retirement.
Forced cleanup, Linux process disappearance, or bridge/keeper recovery do not become
retirement or preparation success.

## Generated cases

| Case | Result |
| --- | --- |
| `RUNNING -> STOP_PENDING -> STOPPED` | `NAD2_STOP_CONFIRMED` |
| Submitted request, unchanged `RUNNING` | `NAD2_STOP_SUBMITTED_NO_TRANSITION` |
| Already pending, completes without another control | `NAD2_STOP_CONFIRMED` |
| Pending and progressing, but incomplete at the bound | `NAD2_STOP_SUBMITTED_PROGRESSING` |
| Control refusal | `NAD2_STOP_NOT_SUBMITTED` |
| Query failure | `NAD2_STOP_OBSERVATION_UNAVAILABLE` |
| `STOPPED` with the exact process alive | `NAD2_STOPPED_PROCESS_OR_LISTENER_REMAINS` |
| `STOPPED` with owned listeners present | `NAD2_STOPPED_PROCESS_OR_LISTENER_REMAINS` |

The same source-owned Windows fixture also passes ledger saturation and dropped-count
accounting, and each adverse case is followed by a fresh normal operation. Production
parser tests cover all six classifications, public/private projection, and forced
cleanup remaining unsuccessful.

Local qualification passed 24 NAD1/NAD2 adapter tests, 244 runtime tests with 44
platform skips, 144 manager-library tests, 84 manager-binary tests, five manager
report tests, and 27 frontend tests. Both strict Clippy checks and Python compilation
passed. The production adapter and source-owned Windows fixture cross-compile for
Windows x86-64.

Exact-source hosted validation:

- [AP8](https://github.com/kasselvania/Linux-VST-bridge/actions/runs/35317428659) — passed
- [AP12](https://github.com/kasselvania/Linux-VST-bridge/actions/runs/35317428498) — passed
- [PX2](https://github.com/kasselvania/Linux-VST-bridge/actions/runs/35317428504) — passed

## Retained development failures

AP8 run `35316474493` at `f6e0cf0…` exposed a stale generated release marker.
AP8 run `35316924618` at `38139c3…` then showed that Windows can release the
generated service dispatcher immediately after the fixture publishes stopped, so
the intended stopped-process residue did not remain observable. Both failures remain
preserved. The final fixture keeps only the source-owned test image alive after that
dispatcher release; it does not alter the production stop adapter or observation law.

## Boundaries and preservation

All 685 evidence files tracked at the starting head are unchanged; their exact Git
manifest digest is in `preservation.json`. The candidate was not installed. No real
NTKDaemon, Native Access, DAW, plug-in, installer, updater, or production operation
was started. This generated qualification does not qualify reliable real-daemon
shutdown.
