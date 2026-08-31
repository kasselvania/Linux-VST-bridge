# HP0 basis and pre-flight

Classification: `observed`

Second-repair exercise completed at `2026-08-31T22:31:18Z` on the maintainer
Steam Deck. The real hostname is intentionally not retained.

| Check | Result |
|---|---|
| Host | Steam Deck model `Galileo`, SteamOS `3.8.16`, x86_64 |
| Kernel | `6.16.12-valve24.5-1-neptune-616-gb2f7cfe85e45` |
| Repository | `<HOME>/code/Linux-VST-bridge`; origin is `kasselvania/Linux-VST-bridge` |
| Pinned basis commit | `5614da2d769931f0aa26a8f902d14e911b39b085` |
| Pinned basis tree | `6d3cea1638e12e65298d9496755d8cc4d0f2eeba` |
| `origin/main` after fetch | exact pinned commit and tree |
| Worktree before branch creation | clean |
| Branch | `codex/hp0-native-vst3-bitwig-sandbox-probe`, created directly from the pinned basis |
| Bitwig process before mutation | absent for both exact process-name checks |
| Bitwig process after acceptance | absent for both exact process-name checks |

The second repair began from reviewed head
`e86dc7b26c052e753a3cb9b050440ea7c795f1cc`, whose parent remained the pinned
basis. The two retained clean builds used historical implementation commit
`e2d87997dc200cd49ec0e6fc2a0e7646b7cfed62`, tree
`3c60d7f48b78677f65c9f2879aa49a2c0d913db6`. Their v2 receipts bind the exact
build-affecting source manifest, SDK, bundle, module, validator, class IDs, and
parameter IDs. The historical commit/tree are provenance, not the sole
acceptance identity.

An evidence-only amendment follows those runs. At that resulting clean PR
head, the canonical build-source manifest and its digest were regenerated,
found byte-identical to the receipt-bound manifest, and both retained build
receipts verified without rebuilding. The exact final head/tree are retained
in the PR topology record because a tracked commit cannot contain its own Git
object identity.

The Deck had no usable GitHub credential. Origin state was fetched through a
temporary, checksum-verified Git transport populated from the maintainer's
authenticated checkout. The persisted `origin` URL was not changed, and the
Deck's resulting `origin/main` was independently read back at the exact pinned
commit and tree before branch creation.

Pre-flight result: `PRE-FLIGHT_CLEAR`.
