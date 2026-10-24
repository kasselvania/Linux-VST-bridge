# Current slice — beta feature integration (#504)

Goal: finish the approved Console on SteamOS and CachyOS. Current state is
Beta Feature Candidate in integration, not Beta RC. Freeze the product surface;
compatibility expansion continues through existing owners.
Architecture fixes boundaries; docs/INTEGRATED_BETA_DELIVERY.md defines release
acceptance. docs/SUPPORT_MATRIX.md holds exact installed claims.

Reference:
- Approved 27-screen Console proof: docs/design-dossier/console-journeys.html.
- Native Home, Plug-ins, Activity, Support, Setup, repair, update, recovery,
  presets and sample locations; no prototype chrome or UI ownership fallback.
- Test254 proves scoped first-party update/recovery/repair/recall. Bitwig reopen
  produced silence with an activation-limit message; that target remains open.

Next changes, in order:
1. Finish #536's failed check, final-head review and installed target acceptance;
   land the live state, recovery, display and standalone-window increment.
2. Rebase #538 on main; final-head review and land preset maintenance protection.
3. Repair #537's stable directory/mount custody and kernel-enforced access;
   run substitution/write controls, rebase, review and land. Do not merge as is.
4. Reconcile one operator schema. Complete physical touch, pointer, keyboard,
   scrolling, chooser/modal/cancel and failure/support paths on target hosts.
5. Cut the immutable signed candidate; qualify the core matrix and canaries,
   classify Musio, and prove installation/update/rollback/support delivery.
No additional feature goes above the current stack.

Working completion:
- Approved hierarchy at Deck/desktop sizes, no dead ends or known-bad offers.
- Quiet renewal; Activity/Support settle or offer an actionable failure.
- Service-owned actions preserve confirmation and exact target across refresh.
- Sample access is enforced; presets and external content survive maintenance.
- Unrelated playback and new-instance admission survive setup maintenance.
- Independent review and installed journeys pass before their increment lands.
- Projects, plug-ins, accounts and activation remain intact.

Beta RC requires the exact candidate on CachyOS/REAPER and SteamOS/Bitwig:
Reference, Tyrell, Kontakt, Serum, Valhalla/CHOW canaries, plus release delivery.
Musio is classified without adding a core support promise. Promotion reuses the
accepted artifact; changes invalidate affected gates, not history.
