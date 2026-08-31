# HP0 negative-test ledger

All synthetic fixtures and detached Git worktrees were created beneath the
canonical user cache and removed after the run. No private test path is
retained here.

| Case | Expected refusal or invariant | Result |
|---|---|---|
| Evidence-only source identity | evidence commit leaves canonical build-source manifest byte-identical | `passed` |
| Evidence-only receipt verification | historical commit/tree may differ while exact manifest and build identities remain valid | `passed` |
| Native source change | manifest changes and stale receipt is refused | `passed` |
| Historical-field edit | changing only stale receipt commit/tree cannot authorize changed source | `passed` |
| Top-level CMake change | manifest changes | `passed` |
| CMake-directory change | manifest changes | `passed` |
| Dependency-lock documentation change | manifest changes | `passed` |
| Unexpected tracked native source | canonical manifest generation refuses undeclared path | `passed` |
| Wrong VST3 SDK commit | dependency verifier rejects before configure | `passed` |
| Dirty exact SDK checkout | dependency verifier rejects untracked marker | `passed`; marker removed and clean lock reverified |
| Missing user Freedesktop SDK | build check refuses instead of using host tools | `passed` |
| Unrelated ordinary receipt | publisher refuses unknown exact-path receipt before destination mutation | `passed`; bytes and mode/size/timestamps unchanged |
| Alternate ordinary receipt | publisher refuses every non-project receipt path | `passed`; absent path remained absent |
| Accepted prior HP0 receipt | strict prior-ownership check still permits transactional replacement | `passed` |
| Test receipt outside exact test root | canonical containment rejects it | `passed`; path remained absent |
| Test build outside exact test root | sandbox probe rejects before validator/sandbox execution | `passed` |
| Symlinked test-root ancestor | root validation rejects lexical cache descendant that escapes | `passed`; escaped sentinel unchanged |
| Symlinked publication ancestor | publication root validation rejects escape | `passed`; escaped sentinel unchanged |
| Symlinked build ancestor | sandbox build-root validation rejects escape | `passed`; escaped sentinel unchanged |
| Unknown publication destination | publisher refuses overwrite | `passed`; unknown content unchanged |
| Interrupted verified staging | prior complete roster/hashes and receipt survive | `passed` |
| Failure after destination swap | replacement removed; exact prior complete roster/hashes and receipt survive | `passed`; no stage/backup sibling |
| Receipt-stage failure with prior publication | exact prior complete roster/hashes and receipt survive | `passed`; no stage/backup sibling |
| Receipt-stage failure without prior publication | prior absence restored; no receipt remains | `passed`; no stage/backup sibling |
| Failure after atomic receipt commit with prior publication | transaction receipt/replacement removed; prior bundle/receipt restored | `passed` |
| Failure after atomic receipt commit without prior publication | exact prior absence restored | `passed` |
| Symlink receipt target | publisher refuses before publication state changes | `passed`; target bytes unchanged |
| Wrong Bitwig app commit | exact fixture guard refuses before sandbox entry | `passed`; real app unchanged |
| Wrong Bitwig runtime commit | exact fixture guard refuses before sandbox entry | `passed`; real runtime unchanged |
| Modified/substituted validator | receipt hash guard refuses before execution | `passed` |
| Missing published bundle | sandbox probe fails before validator | `passed` |
| Modified published module | manifest/hash check fails before validator success | `passed` |
| Wrong ELF architecture | receipt-bound module identity rejects the manifest-consistent wrong-machine fixture | `passed` |
| Bitwig-running precondition | publisher refuses while exact synthetic process name exists | `passed` |
| Current overrides | exact expected hashes and actual pre/post bytes must match | `passed` for user and system scopes |

An unresolved dependency is governed by the same acceptance gate: build and
app-sandbox inspection run `ldd` and fail on any `not found` line. The real
module had no unresolved dependency; the deterministic architecture branch was
used for the synthetic combined architecture/dependency negative requirement.

The run emitted 35 deterministic passed assertions plus the exact
override-preservation invariant. Complete-roster comparisons included ownership
and source-manifest files, not only the module binary. Aggregate result:
`passed`.
