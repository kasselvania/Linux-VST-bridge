# No active implementation slice

AP17 is accepted for integration. Its product result is the exact Steam Deck / Bitwig / Arturia operating envelope described in [docs/AP17.md](docs/AP17.md) and [docs/AP17_CLOSURE.md](docs/AP17_CLOSURE.md).

Accepted exact-fixture envelope:

- 16 bounded service workers;
- 6 simultaneous DSP instances globally;
- at most 3 Pure LoFi instances;
- at most 4 Efx FRAGMENTS instances;
- native hard capacity of 4 per loaded class image;
- 3 qualified parallel tracks;
- serial bridged depth 3;
- 2 simultaneous direct vendor editors exercised;
- 7 instances remain unqualified;
- 8 were excluded for the tested workload.

Revision 10 is the accepted ordinary profile generation in source. Revision 9 and revision 8 remain immutable candidate history. Revision 7 is the exact immediate rollback generation, with revision 3 retained beneath it. The Steam Deck was deliberately left on revision 7 after the final investigation; activating revision 10 is a bounded installation step, not a request to replay AP17.

The one historical Bitwig quit hang remains retained and unexplained. It did not recur across the focused differential matrix or two complete repetitions of the triggering sequence. No causal repair is claimed. SteamOS `foreground_booster` was identified as the writer of the observed quit-time CPUWeight overrides; that is a performance-comparison confound, not an AP17 product blocker.

The lease-enumeration/retirement race is tracked separately as follow-up and does not invalidate the conservative capacity envelope.

Final accepted posture remains 512 added frames selected/supported/recommended; 256 available but unqualified. Residual delivery classes remain in issue #90. The repository remains publicly readable but proprietary under `COPYRIGHT.md` and `CONTRIBUTING.md`.

The next selected product slice is AP18: the official Arturia Software Center to Pigments acquisition, installation, discovery, publication, and qualification vertical. Its implementation authority begins only from the AP18 preparation branch and issue.
