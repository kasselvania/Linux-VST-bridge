# Current slice — Deck managed GE migration and mixed use (#469)

Goal: use the maintained runtime without losing installations, authorization or
saved projects, and prove the Bitwig mixed-use journey. `docs/ARCHITECTURE.md`
fixes boundaries and beta scope; the completed NI recovery (#465) is the reference.

Observed on Test216:
- Serum uses r7 (same GE line, UIA/MSI plus both retained X11 touch corrections).
  Instrument/FX playback and changed-state recall pass. The operator confirms
  finger-operated waveform selection/reopening/dismissal stays responsive.
- NI uses r7. Kontakt Pad Noir playback, save/reopen and visible editor pass.
  The operator confirms finger-operated menu reopening/dismissal and Cutoff
  adjustment remain responsive in the reopened Bitwig project.
  Native Access reaches signed-in Home; an initial Loading Products Failed screen
  cleared before the operator touched Retry. Installation/update is not retested.
- Bitwig 6.1 Desktop plays two distinct Reference chains from the r5 setup plus
  Kontakt from r7. Removing/reloading one chain leaves siblings playing. Save,
  full exit/reopen retain settings and Pad Noir, with matching capture peaks.
  Clean retirement after both full exits passes. See `docs/SUPPORT_MATRIX.md`.

Remaining:
1. #470 is merged. Finish #471 against main: verify the unchanged r7 driver
   against GE, complete Tyrell editor/audio/changed-state recall, then review/land.
   No new implementation PR until this runtime increment lands.
2. Resolve the live Native Access overview refusal hiding Stop controls, the
   empty editor offered for a fixture with no custom editor, and an older Bitwig
   Kontakt project's missing-device result. Preserve the old project and compare
   its known prior package before attributing that result to this runtime move.
   Serum transparency remains separate and open (#330).

Keep existing setups, projects, activation and old runtimes while any setup or
rollback uses them. Use the retained homelab backup. No synthetic Deck key events;
mouse and captures are allowed. Preserve BEAM, Blackhole, Arturia and FL Studio.
Game Mode secondary-window work, r6, DirectComposition and supervisor redesign
remain outside this slice. Do not turn these bounded checks into a beta claim.
