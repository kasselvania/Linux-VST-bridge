# Current slice — five bounded pre-beta fixes

Goal: truthful completion, usable passive monitoring, reliable inspection and
faster connection/update work, preserving the accepted managed flow.

Baseline: Test66 is accepted on Deck and CachyOS. App/bootstrap continuation,
Native Access and missing-sibling connection work; protected projects/history,
saved-state recall and status/handoff polling pass. Detailed accepted facts remain
in SUPPORT_MATRIX and the existing Deck/Cachy evidence.

Deck hold: the user is actively using it and has not handed it back. No Deck
access, scans, service calls, input, installs or experiments until explicit release.
The owned Cachy VM remains off; restart only for useful installed acceptance.

1. Completion state (current): Home's permanent “Next: open your DAW” fallback
   mistakes a completed connection for an unfinished task. Show observed native
   availability and the exactly bound DAW when known; keep usage help optional.
   Preserve real unfinished, unavailable and empty states. No usage confirmation,
   activation claim or audio-success claim follows from publication alone.
2. Passive refresh: retain coherent last-observed task content and unrelated
   browsing/choices. Validate actual mutations against unchanged targets and
   consequences; do not remove freshness/ownership checks.
3. Idle monitoring: measure the owned/source-harness path, remove demonstrated
   unnecessary reconstruction and retain external-change/service-failure detection.
   No arbitrary longer timer or broad watcher/cache framework.
4. Reliability: recover the Kontakt editor-interface allocator failure and exact
   successful comparison; run the smallest causal A/B only after Deck release.
   No retry wrapper or suppressed required check substitutes for a crash fix.
5. Connection/update cost: measure phase costs and remove proven repeated work
   without weakening checks or preservation. Keep the user's untouched Lunacy
   connection baseline distinct from package-update and VM timings.

Land one reviewed, green increment at a time; start the next slice only after
coordination. Source/UI acceptance and installed Deck acceptance stay distinct.

Done: completed setup has no permanent next-task reminder; refresh remains usable,
idle overhead is measured/reduced, the crash is causally fixed and slow work is
improved with original licensed state, projects, settings and history intact.
