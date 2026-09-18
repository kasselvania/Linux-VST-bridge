# Receipt/acquisition handoff correction

This is the second narrow PR #121 shutdown-race amendment requested at
`18007a03bd9eb5463af4bb859c0e9304e0da62a4`. It preserves the completed-stop
handoff result while closing the transition in which the anchor publishes its
receipt and releases the original process handle after the helper's first receipt
lookup but before process acquisition.

Executed source: `0b2ca827b5a10e36027c9d7286c887c5683fa520`.
Tree: `7b79c24f8c77162e74c96bfad80320d3b440eeb4`.

## Admission law

The stop generation admission and the bounded stop observer now share the same
portable production header. A STOPPED observation still accepts an already exact
operation/nonce/PID/creation-bound exit receipt. Otherwise the helper attempts to
open and verify the admitted generation as before.

Only failed process acquisition opens the new handoff path. It performs exactly
one fresh SCM query. Only fresh `SERVICE_STOPPED`, one exact receipt read, and one
zero-listener census admit the retained-anchor exit witness. The fresh stopped
state becomes the observer's first state, so no duplicate control is sent. The
observer then independently rechecks SCM state and listener absence before it can
confirm retirement.

Initial STOP_PENDING and RUNNING observations use the same path when their exact
admitted process disappears before acquisition. A RUNNING PID must still match the
admitted PID. A successfully opened handle with unresolved image or creation
identity cannot switch to receipt authority. Missing, malformed, inaccessible,
aliased or mismatched receipts, a non-stopped fresh query, listener presence, and
failed acquisition without a receipt all refuse.

## Deterministic transition regression

The portable C++ regression compiles the exact production admission function and
the production stop observer. Its controlled acquisition callback publishes the
receipt and releases the modeled original anchor handle after the initial receipt
lookup and immediately before returning process-acquisition failure. The test
requires retirement confirmation for initial STOPPED, STOP_PENDING and RUNNING
states without a stop control.

Negative cases require refusal for missing or malformed evidence, a fresh pending
state, live listeners, and an opened process with unresolved identity. The direct
receipt-before-helper case remains covered. Hosted AP12 compiled and ran these
cases; hosted AP8 built the modified Windows adapter and passed the existing
source-owned SCM fixture.

## Preservation and limits

All 677 evidence files tracked at the reviewed `18007a0` checkpoint are unchanged
through the source correction. This amendment adds only its own separate receipt.
No Steam Deck or other device was contacted. No production generation was
installed, Native Access was not launched, and no real NTKDaemon transition was
performed. The preserved live listener failure is not explained by this source
race correction, and reliable real-daemon shutdown remains unqualified.
