# Current slice — fresh r7 Native Access → Kontakt (#482)

Goal: a new CachyOS user installs one signed, release-shaped package, acquires r7,
installs and authorizes Native Access/Kontakt, plays in native REAPER and recalls
changed state after full exit and reboot, without terminal recovery or blocking
an unrelated setup. Architecture and beta scope remain in `docs/ARCHITECTURE.md`.

Delivered reference comparison:
- Test225 / PR 484 delivers exact signed supervisor source, isolated launch and
  CPython 3.13/3.14, tested on both (next package: >=3.13,<3.16, 3.15 in CI).
  Ordinary old bytecode → source → exact old restore → source passed Reference
  playback, saved-instance recall, full exit and clean retirement on CachyOS.
  The original long-home account also launches and retires with the repaired
  home-independent keeper endpoint. Historical generation records stay exact.
- #468 recovery and setup isolation are complete through #474–#480. Test223
  admits new Reference instances during NI maintenance and preserves mixed recall.
- Fresh NI installation passed on r5; r7 migration did not prove fresh r7 install.
  Deck/Test216 remains earlier cross-generation qualification.

Remaining journey, using the same signed Test225 package:
1. In a fresh CachyOS account and normal graphical login, acquire r7 through the
   product, install Native Access and reach its managed usable window/sign-in.
2. Install Kontakt + Factory Selection, discover/publish, play MIDI/audio and use
   the editor in REAPER; recall changed instrument state after full exit and reboot.
3. Keep unrelated Reference audio working, admit a new instance during NI
   maintenance and follow-up, and preserve its saved state and ownership.

Working completion:
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
Old Download and update cannot read source-only bundles; use the signed installer
and Open Setup. The retained Test223 no-prefix fixture remains unrecovered.
Confirm Storage2 is mounted before homelab writes under /mnt/Storage2/LinuxPluginBridge.
