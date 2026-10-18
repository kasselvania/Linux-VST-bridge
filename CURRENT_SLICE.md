# Current slice — move Deck beta setups to the maintained GE line (#469)

Goal: use the maintained managed runtime on the Deck without losing installed
plug-ins, activation or saved projects, then prove the Bitwig mixed-use journey.
The completed Native Access recovery (#465) is the reference for that vendor
journey. `docs/ARCHITECTURE.md` fixes the boundaries and beta scope.

Reference: preserve the existing Deck setups and their previous runtimes. Compare
each saved project before and after its product-managed runtime move. Test216 is
installed; the Reference setup already plays and recalls independent instance
settings on r5 in REAPER, with clean retirement. This is not Bitwig qualification.

Next changes and checks:
1. Restoring Serum's retained runtime restored physical touch. Both touch
   corrections are now integrated into the maintained GE line as r7, retaining
   the existing UIA/MSI corrections. Serum's setup now uses r7.
   r7 is installed: Serum instrument/FX changed-state recall and playback pass.
   The operator confirms finger-menu selection and dismissal stay responsive.
   r5 playback/recall passed; finger menu use stalled or lost the editor.
   Correct transparency remains an open, separate graphics defect (#330).
2. NI now selects r7: Kontakt Pad Noir playback, save/reopen and visible editor
   pass. Native Access reached signed-in Home on r7 after a transient Loading
   Products Failed screen cleared before the operator touched Retry. Preserve
   installations and content; proceed to the Bitwig mixed-use check.
3. Connect Bitwig and run two distinct instances from one setup plus a plug-in
   from another: simultaneous playback, independent removal/reload, changed
   state, save, full exit and fresh-process recall, followed by clean retirement.

Use the existing homelab backup. Never rewrite licensed setup files outside
product operations. No synthetic Deck key events; mouse and captures are allowed.
Keep the old runtimes while any setup or rollback uses them. Preserve BEAM,
Blackhole, Arturia and the FL Studio workspace. Game Mode secondary-window work,
r6, DirectComposition integration and supervisor redesign are outside this slice.

Done: the named Deck beta setups use the managed runtime, retain authorization
and saved musical state, and complete the Bitwig mixed-use checks above. A
compatibility scan or publication alone does not establish working musical use.
Record only observed combinations in `docs/SUPPORT_MATRIX.md`; open shared defects
belong in the existing issue and failure-class map.
