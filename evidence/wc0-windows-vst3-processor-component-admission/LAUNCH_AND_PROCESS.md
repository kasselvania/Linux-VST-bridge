# WC0 lifecycle and process result

The accepted Runtime
4 / Proton 11 route retained launch-critical digest `2d64df1d36786ca2d0e955c553005423dc2b5bdd714bd0a17872622e33912547`. The
positive scanner completed all 20 closed calls with no operation left in flight.
It created only the exact AGain processor as `IComponent`, verified the exact
controller CID before initialization, initialized and terminated once, released
the component to zero, retired the host reference sequence to zero, proved all
nine object-quiescence facts, then completed reverse factory releases,
`ExitDll`, and `FreeLibrary`.

The blocked release/leak/timeout/crash exercises suppressed later inherited
factory/module calls when object absence was unproved. Their zero-descendant and
environment-retirement results are physical containment, not a clean in-process
shutdown claim.
