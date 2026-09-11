# AP17 closure — capacity and recovery envelope

AP17 is accepted from PR #92.

## Accepted result

On the exact Steam Deck / SteamOS 3.8.16, Bitwig Studio 6.1 Flatpak, pinned Proton-SLR, Pure LoFi 1.0.0.6121, and Efx FRAGMENTS 1.0.0.2925 fixture, the bridge established and enforced these distinct capacities:

- service-worker ceiling: 16;
- global DSP envelope: 6;
- Pure LoFi class envelope: 3;
- Efx FRAGMENTS class envelope: 4;
- native-image hard ceiling: 4 per loaded class image;
- qualified parallel topology: 3 tracks;
- qualified serial bridged depth: 3;
- simultaneous direct editors exercised: 2.

Seven instances remain unqualified. Eight were excluded for the tested workload. These are exact-fixture operating limits, not universal Arturia, Bitwig, Linux, or hardware limits.

AP17 also established typed pre-ownership refusal; exact one-unit release and replacement; distinct same-class project state; six-device save/reopen; one-host failure containment; service restart; bounded unexpected service loss; and normal reboot recovery. Revision 10 preserves the independently accepted revision-9 native behavior as a new ordinary `verified_exact_fixture` generation. Revision 7 is its immediate rollback generation; revision 3 remains earlier retained ancestry.

## Retained anomaly

One final ordinary smoke left the Bitwig frontend and audio-engine process alive after all six bridge owners had retired positively. The event was retained rather than disguised. It did not recur in the focused matrix: revision 7, revision 10 at 1+1 and 2+4, refusal, removal/replacement, editors, and two complete repetitions of the triggering sequence all exited without manual containment.

No causal source repair is claimed. The prior surviving owner could not be classified from the retained capture. SteamOS `foreground_booster` was separately proven to write the observed quit-time `CPUWeight=10000` overrides. That makes those runs unsuitable for controlled performance comparison but does not erase their successful functional exits or make disabling normal SteamOS behavior a product requirement.

The anomaly remains a truthful known host/lifecycle observation. It is not a reason to continue repeating the same AP17 campaign without a reproducer.

## Follow-through

A narrow fail-closed race remains possible when capacity enumeration overlaps durable lease deletion. It is tracked separately and can be repaired with a deterministic source test; it does not permit over-admission or invalidate the qualified six-instance envelope.

Future committed evidence should prefer stable aliases for transient local process/session identifiers. Existing AP17 evidence remains historical and is not rewritten as a condition of product progress.

## Final disposition

```text
AP17_ACCEPTED
SIX_INSTANCE_EXACT_FIXTURE_ENVELOPE_ACCEPTED
REVISION_10_SOURCE_AUTHORITY_ACCEPTED
HISTORICAL_QUIT_HANG_RETAINED_NONREPRODUCED
NO_CAUSAL_QUIT_REPAIR_CLAIMED
512_RECOMMENDED
256_UNQUALIFIED
```
