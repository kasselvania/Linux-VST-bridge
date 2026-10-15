# Current slice — learned fixes become shared tooling

Goal: one build that adapts to the distro it runs on, where every fix learned on
one machine ships as shared tooling and applies wherever that vendor or plug-in
appears. No machine keeps a fix that a fresh install cannot get. First case: a
musician on any distro installs Native Access, signs in, installs Kontakt
through it, plays it in the DAW, and gets it back after save and reopen.

State after 2026-10-15 (main = #343–#345, fresh CachyOS VM, Test80):
- Works on a fresh install: Native Access installer (close-check advice now uses
  the kit's launch component), Retry after a failed installer, Open Native Access
  (icon-derived app location), NTKDaemon preparation, browser sign-in return,
  clean close, automatic Kontakt 8 discovery and REAPER publication.
- Kontakt's installer still fails on the downloaded runtime (Wine MSI reader).
  A prefix-only experiment with the Deck's patched msi.dll installed it; the fix
  exists only as a hand-built Deck runner.
- The Deck still works only through hand-built, Deck-bound records and runners.
- A failed vendor-app session can leave the bridge stopped with Recover refused.
- Failures show internal codes; Valhalla/CHOW reasons are retained but hidden.

Next changes, in order:
1. Kontakt MSI fix as a hash-pinned layer on the managed runtime, built from the
   delivered runtime's own Wine source and selected by the NI profile.
2. Kontakt in REAPER and Bitwig: sound, editor, save and reopen (CachyOS, Deck).
3. Recovery after a failed vendor-app session must resume the bridge.
4. Problems explain themselves: plain-English reasons from retained records and
   one "Create problem report" file (plan ready); fixes CHOW's double count.
5. Serum: find the setting that fixes transparency, ship it in Serum's profile,
   and give users a renderer choice they can try and revert.
6. Convert remaining Deck-only fixes and move to one installer for every distro.

Done for this slice: step 2 passes on the Deck and on a clean CachyOS install
from the same Test build, with no machine-specific records or runners.

Working limits: start from the failing musician action and judge the fix by the
same action. Three failed attempts or 15 minutes without progress on one
outcome means stop and reassess; changing agent or tool does not reset it.
