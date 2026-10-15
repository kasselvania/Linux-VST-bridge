# Current slice — learned fixes become shared tooling

Goal: one build that adapts to the distro it runs on, where every fix learned on
one machine ships as shared tooling and applies wherever that vendor or plug-in
appears. No machine keeps a fix that a fresh install cannot get. First case: a
musician on any distro installs Native Access, signs in, installs Kontakt
through it, plays it in the DAW, and gets it back after save and reopen.

State after 2026-10-15 (main = #343–#351; Test83 delivered for the tester):
- Fresh CachyOS install: Native Access installs past its close check, opens from
  the manager, starts NI's service, returns browser sign-in; Native Access installs
  Kontakt 8 on runtime r5 (MSI-reader fix, release runtime-msi-r5); Kontakt 8 is
  Available for REAPER.
- The tester's own failed state (Test69) recovers after updating: Retry moves the
  setup onto r5 and installs Native Access again.
- Failures explain themselves in plain English; CHOW counts once.
- Open: Recover after a host freeze/power loss leaves the vendor app unopenable;
  Native Access's download location starts empty; sign-in return is slow/flaky;
  existing Deck setups still use hand-built runners; Kontakt sound and
  save/reopen untested (bridge keeps component/controller state separate, so the
  known yabridge Kontakt recall bug does not apply; one residual case unproven).

Next changes, in order:
1. Recover re-verifies an interrupted vendor session and releases it (Test84),
   then finish Kontakt in the recovered tester-state setup.
2. Kontakt in REAPER and Bitwig: sound, editor, save and reopen (CachyOS, Deck),
   plus the single-component controller-state fixture test.
3. Native Access first-run gaps: download location, sign-in return timing.
4. One-click problem report; Serum renderer choice and transparency.
5. Convert remaining Deck-only fixes; Flatpak DAW permission; one installer.

Done for this slice: step 2 passes on the Deck and on a clean CachyOS install
from the same Test build, with no machine-specific records or runners.

Working limits: start from the failing musician action and judge the fix by the
same action. Three failed attempts or 15 minutes without progress on one
outcome means stop and reassess; changing agent or tool does not reset it.
