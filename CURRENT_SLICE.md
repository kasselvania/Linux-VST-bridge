# Current slice — fresh r7 Native Access → Kontakt (#482)

Goal: a new CachyOS user installs one signed, release-shaped package, acquires r7,
installs and authorizes Native Access/Kontakt, plays in native REAPER and recalls
changed state after full exit and reboot, without terminal recovery or blocking
an unrelated setup. Architecture and beta scope remain in `docs/ARCHITECTURE.md`.

Reference comparison:
- #468 recovery and setup isolation are complete through #474–#480. Test223
  admits new Reference instances during NI maintenance and preserves mixed recall.
- Fresh NI installation passed on r5; r7 migration did not prove fresh r7 install.
  Deck/Test216 remains earlier cross-generation qualification.

Next changes, in small reviewed increments:
1. New generations ship exact signed session.py/ownership.py source. Keep old
   bytecode generations truthful and runnable, including exact predecessor restore.
   Use a fixed isolated bootstrap loading both verified absolute paths; preserve
   arguments/display/scheduler handoff, reject ambient imports, write no bytecode.
   Declare/enforce only physically tested host-Python versions (test 3.13/3.14).
2. Prove old bytecode → source update → first-party launch/retire → exact old
   restore → old launch/retire → source update; cover package/bootstrap/recovery.
3. With that exact signed package, use a fresh CachyOS account for ordinary r7
   acquisition, Native Access install/sign-in, Kontakt + Factory Selection install,
   discovery/publication, REAPER MIDI/audio/editor and changed-state save/recall.

Working completion:
- Exact source packaging, interpreter range and isolated launch pass; rollback works.
- Kontakt identity and changed state survive full DAW exit/reopen and reboot.
- During NI maintenance, Reference plays, admits a new instance and recalls state.
- Progress and Stop/Cancel/Retry remain reachable; normal close/follow-up completes.
- Final retirement: zero DSP, setup maintenance leases, pending transactions,
  stale transports and cleanup uncertainty. Record phase timings and any Retry.

Repair the first demonstrated product-owned failure only. Slow bounded completion
may pass correctness while responsiveness stays open in #481; status invalidation
belongs to #382. Loading failure origin is unestablished; repeated Retry fails.
SteamOS, Serum transparency, Blackhole/BEAM, DirectComposition, old Bitwig project,
empty editor, Rust port, broad endurance and manager redesign are separate.
Preserve licensed setups, activation, original projects and retained predecessors.
Confirm Storage2 is mounted before homelab writes under /mnt/Storage2/LinuxPluginBridge.
