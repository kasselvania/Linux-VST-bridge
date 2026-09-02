# WF0 launch and process supervision

The exact Runtime
4 / Proton 11 launch-critical digest was `2d64df1d36786ca2d0e955c553005423dc2b5bdd714bd0a17872622e33912547`. The positive run
used neutral application ID `0`, the fixed `runinprefix` scanner vector,
bounded concurrent output, and a maximum of 256 observed identities.

Runtime, Proton, and Wine-hosted scanner roles were observed before gate
publication; Steam game-launch ancestry was absent. The positive scanner ended
at `scanner_completed`, raw exit `0`, with
`last_in_flight_operation=null`. Release, optional exit, and `FreeLibrary`
all completed before `scanner_completed`.

The dedicated gate was withheld for exactly 15 seconds. During that interval
there was no load attempt, AGain mapping, factory/class event, or scanner
departure. Scoped cleanup then reached zero descendants and retired the exact
environment. A live same-family sentinel outside the owned ancestry survived
that scoped cleanup and was retired only by its own verified owner afterward.
