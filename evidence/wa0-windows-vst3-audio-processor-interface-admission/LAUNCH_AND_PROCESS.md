# WA0 interface lease and process result

The accepted
Runtime 4 / Proton 11 route retained launch-critical digest `2d64df1d36786ca2d0e955c553005423dc2b5bdd714bd0a17872622e33912547`.
The positive scanner completed all 22 closed calls with no operation left in
flight. After accepted WC0 component initialization, it queried the exact
`IAudioProcessor` IID once, acquired one non-null lease, released that lease
once to component-owner reference baseline 1, cleared the interface pointer,
and proved audio-interface quiescence before WC0 termination.

The inherited host reference sequence remained 1 -> 2 -> 1 -> 0; final
`IComponent` release returned 0, followed by reverse factory releases,
`ExitDll`, and `FreeLibrary`. Query/release timeouts, crashes, and unexpected
release counts used physical containment and made no clean in-process shutdown
claim.
