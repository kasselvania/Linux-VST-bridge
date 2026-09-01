# WR0A final-repair reconciliation

`reconcile.py` is the single, fixed owner for adopting the accepted WR0
final-repair archive into current repository truth. It is deliberately not a
general Git migration or fixture-management utility.

It accepts exactly one of seven modes:

```text
verify-authority
plan
stage-adoption
verify-candidate
inspect-live
render-evidence
negative-tests
```

The implementation basis, archive and historical objects, 25-path comparison
envelope, 13 imported blobs, 11 preserved blobs, excluded archived
`CURRENT_SLICE.md`, authority and DG0 blobs, canonical WR0 packet, contract
source, workload, live environment, and evidence destination are compiled into
the tool. Normal modes accept no caller-selected ref, path, blob, live target,
call target, or output location.

`verify-authority` proves the implementation topology and immutable inputs.
`plan` emits the exact 14-difference/13-import/11-preserved reconciliation
ledger without mutation. `stage-adoption` reads the thirteen accepted archive
objects, writes and stages only their literal destinations, and restores the
exact old objects on a partial failure. The operator creates the separate
adoption commit after its staged receipt passes.

`verify-candidate` accepts the staged adoption or its exact clean commit and
proves the WR0, WR0A, canonical-packet, authority, and governance identities.
`inspect-live` performs bounded diagnostic-only readback through the frozen
allowlisted call graph. `render-evidence` performs a separate retained
readback, renders and validates the fixed eight-file WR0A packet, and stages
only that packet. Neither mode may mutate the live WR0 environment.

`negative-tests` exercises the production comparison, exact-object adoption,
partial-failure recovery, topology, packet, and read-only-dispatch guards in
owned synthetic state. It never substitutes synthetic output for the real
candidate or Deck readback.

Every failure is fail-closed and begins with a stable WR0A blocker
classification. A blocked result is not acceptance. This slice does not run a
Windows workload, alter Proton/Wine/Steam/Bitwig/Serum state, merge its own pull
request, close the active slice, or expand WR0's claim ceiling.
