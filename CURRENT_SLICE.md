# Current slice — approved Console application (#504)

Goal: deliver the agreed Console interface and service/client structure on the
installed SteamOS and CachyOS routes. The musician gets the actual approved
screens and journeys, with working controls, touch and scrolling.
`docs/ARCHITECTURE.md` remains the product and runtime authority.

Reference comparison:
- `docs/design-dossier/console-journeys.html` is the recovered approved 27-screen
  proof: install, readiness, repair, update, recovery, presets and sample locations.
  Its outer journey selector and illustrative records are prototype scaffolding.
- Test242 exposed wrong screen composition, idle flashing, missing Linux touch,
  slow scrolling and Activity/Support reads stuck loading (Makepad issue #10).
- Colours and shared widgets alone do not satisfy the approved design.

Next changes:
1. Compose the compact header, library rows, exact plug-in detail, readiness
   ladder, contextual actions and journey screens from actual owner records.
2. Make passive refresh quiet; keep failed reads actionable; complete native
   input, file selection, forms and exact retained-operation feedback.
3. Complete service-owned command execution and setup-scoped maintenance.
   UI/CLI consume the same owner view and offers; no frontend policy fallback.
4. Complete the missing owner facts/actions needed by the agreed journeys,
   then qualify and land the native window as the product interface.

Working completion:
- Installed screens match the approved hierarchy and layout at Deck and desktop
  sizes; ordinary and failure states are navigable without development chrome.
- Activity and Support settle; background observation does not flash or reset
  controls; physical touch, mouse, keyboard and scrolling work on target hosts.
- Each problem has its exact next action; offers execute through the service,
  preserve confirmations and survive refresh without replay or wrong identity.
- Setup maintenance leaves unrelated playback and new-instance admission working.
- Native installed journey checks and independent code review pass before merge.
- Existing projects, installed plug-ins, vendor accounts and activation survive.

The closed-beta targets and runtime/saved-state obligations are unchanged.
