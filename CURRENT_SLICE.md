# Current slice — from working Kontakt to explainable plug-in failures

Goal: a musician on any distro installs Native Access and Kontakt, plays Kontakt
in their DAW and gets it back after save and reopen (done, see below); next, any
plug-in that fails tells them what kind of problem it is in plain words and, where
safe, offers a fix. One build adapts to the distro; learned fixes ship as shared
tooling, never per machine.

State after 2026-10-15 (main = #343–#357; Test88 delivered for the tester):
- Native Access + Kontakt 8 install through the manager on CachyOS (fresh and the
  tester's recovered state); a host freeze no longer blocks Recover or updates.
- Kontakt in REAPER 7.82: editor inside REAPER's FX window, DAW MIDI plays (start
  notes held ≤250 ms, #357), save/full restart/reopen restores the instrument
  (Deck, Test88). MIDI verified on the CachyOS fixture; CachyOS reopen untested.
- Native Access's download folder is filled on new setups (#355; first live check
  pending). The Deck is on Test88, yabridge moved aside.
- Open defects: #365 Xwayland abort when the DAW dies with an embedded editor;
  #366 no reload after a host death and the manager is silent; #364 crash-
  interrupted continuation; #367 confirm processing start on the audio thread.

Next changes, in order:
1. Valhalla (#358) and CHOW (#359): reproduce with free plug-ins, fix the causes.
2. Failure types from that evidence (#360): plain sentence per type, safe self-help.
3. #365/#366 host-death recovery; #367 start confirmation.
4. Problem report button (#361); per-plug-in renderer choice, Serum (#362, #330).
5. Backlog as issues: #363 #364 #368–#371.

Done for this slice: Valhalla Supermassive and the free CHOW plug-ins play in
REAPER on the CachyOS fixture, or fail with a correct plain-English type.

Working limits: start from the failing musician action and judge the fix by the
same action. Three failed attempts or 15 minutes without progress on one
outcome means stop and reassess; changing agent or tool does not reset it.
