# Completed external-stop handoff

This is the narrow amendment requested by the PR #121 rereview at `7e2ae5b`.
It preserves the earlier cleanup, pipe/privacy, pending-stop and diagnostic repair
results. It does not explain the real failed stop with both listeners present.

Executed source: `d42b245a495c79f847894600acabf4e38deca8bd`.
Tree: `1bd0aa66c3973fe92d38881af27aac672e5592b6`.
The source seal hashes the relevant production and generated inputs. The Git tree
binds the full source; hosted checks and local outcomes are in `validation.json`.

## Handoff law

A Windows process object can disappear after its last open handle is closed;
retaining only its PID and creation time cannot keep that object available.
[Microsoft process termination contract](https://learn.microsoft.com/en-us/windows/win32/procthread/terminating-a-process).

The start adapter now retains the handle acquired during its initial verified
RUNNING observation instead of closing it and reopening it in the anchor loop.
If startup is still pending, it retains the handle at the first admitted RUNNING
observation. Startup is acknowledged only after that ownership is established;
a pending startup is bounded to 25 seconds, while normal use has no deadline.
It never adopts a later service generation.

On SCM STOPPED, the anchor requires that original handle to signal exit, rechecks
its creation identity, and requires zero listeners owned by its Windows PID.
It writes and flushes an operation/nonce/PID/creation-bound private receipt, then
publishes it by a non-replacing rename before releasing the handle. A missing
receipt during publication leaves the original handle available. A publication
error fails the anchor. Cleanup waits remain bounded.

The stop helper uses the receipt only when its initial exact SCM query reports
STOPPED and it has the owner's previously admitted generation. The receipt must
have the exact closed bytes and extent, be a regular single-link non-reparse file,
and match the operation nonce and generation. The helper still queries SCM and
checks current listener absence through the normal bounded retirement law. The
independent Linux process census remains required by the Python owner.

Without that receipt, the original handle-based validation remains required.
Missing, inaccessible or malformed data does not prove exit. Failed OpenProcess
remains a failure and retains its actual error; it is never interpreted as absence.
A valid receipt is an independent original-handle observation, not an inference
from a failed lookup. No duplicate stop control is sent for already-stopped state.

The supervisor independently matches the returned retirement generation to its
admitted generation and validates the optional handoff frame. Public stages name
`retained_anchor_exit`, `retained_process_handle`, or
`exact_already_inactive` for the existing no-generation inactive case; raw generation and nonce
material remain private, including after the login raw-output cutoff. Receipts
remain retained with the operation after retirement acknowledgment.

## Generated cases

The Windows SCM fixture starts the exact source-owned service, observes RUNNING,
retains that generation, requests an external stop, and waits for the anchor to
exit successfully. Only afterward does it invoke the owner stop helper with the
old generation. The test requires the retained exit-receipt authority, initial
STOPPED, successful retirement, and zero additional stop controls.

The same fixture rejects wrong nonce, operation and generation, truncated/extended
records, a hard-linked receipt, and a missing receipt with a failed process lookup.
Its final accepted invocation verifies the receipt remains unchanged. These are
mutations of generated scratch evidence only.

Python generated-helper regressions exercise successful handoff under raw privacy
suppression and reject changed generation, changed nonce and duplicate handoff
frames. Existing pending/delayed-stop, cleanup, pipe and Arturia coverage remains.

## Preservation and limits

All previously tracked evidence is unchanged from the reviewed checkpoint; the
comparison and manifest digest are in `preservation.json`. No Deck connection,
physical census, production installation, real daemon transition or Native Access
launch occurred. Existing live-state readbacks remain historical.

PR #121 remains draft and unmerged. These generated results qualify the handoff
mechanism. Reliable real NTKDaemon shutdown still requires a separately authorized
check after review; Native Access remains closed during this amendment.

The first generated candidate `fa65e4b` failed the fast repeated-start/stop anchor
exit assertion in AP8 run `35299668099`. Its source seal is retained as
`development-source-seal.json`. Inspection identified the remaining START_PENDING
acknowledgment window before anchor ownership; `f6e0f6a` closes that window.
This development failure is not a vendor or Deck observation.
