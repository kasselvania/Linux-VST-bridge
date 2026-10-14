# Current slice — five bounded pre-beta fixes

Goal: truthful completion, usable passive monitoring, reliable inspection and
faster connection/update work, preserving the accepted managed flow.

Baseline: Test66 is accepted on Deck and CachyOS. App/bootstrap continuation,
Native Access and missing-sibling connection work; protected projects/history,
saved-state recall and status/handoff polling pass. Accepted facts remain in
SUPPORT_MATRIX and the existing Deck/Cachy evidence.

Deck hold: the user is actively using it and has not handed it back. No Deck
access, scans, service calls, input, installs or experiments until explicit release.
The owned Cachy VM and its evidence are retained for reviewed installed acceptance.

1. Completion state (source landed; installed acceptance awaits handoff): Home's
   permanent “Next: open your DAW” fallback is removed. Show observed native
   availability and the exactly bound DAW when known; keep usage help optional.
   Preserve real unfinished, unavailable and empty states. No usage confirmation,
   activation claim or audio-success claim follows from publication alone.
2. Passive refresh (source landed; installed check pending): retain last-observed
   tasks and browsing/choices.
   Capture the original clicked action/token through renewal; changed or missing
   tokens stop it locally. Same-token requests retain manager admission/refusal;
   never retarget or replay. Backend admission rechecks exact intent and owners.
3. Idle monitoring (source check): app cards wrongly used deep execution checks.
   Use bounded current record/binding observation; keep full checks at execution.
   Unselected CLI A/B on owned Cachy: six-class overview 5.90 → 0.48 s, unchanged
   status/token and 88 protected files. No timer, cache or polling-policy change.
4. Reliability: recover the Kontakt editor-interface allocator failure and exact
   successful comparison; run the smallest causal A/B only after Deck release.
   No retry wrapper or suppressed required check substitutes for a crash fix.
5. Connection/update cost: measure phase costs and remove proven repeated work
   without weakening checks or preservation. Keep the user's untouched Lunacy
   connection baseline distinct from package-update and VM timings.

Land one reviewed, green increment at a time. Source and installed acceptance stay distinct.

Done: completed setup has no permanent next-task reminder; refresh remains usable,
idle overhead is measured/reduced, the crash is causally fixed and slow work is
improved with original licensed state, projects, settings and history intact.
