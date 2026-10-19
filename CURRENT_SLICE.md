# Current slice — move the Deck beta setups to r5 (#469)

Goal: use the maintained managed runtime on the Deck without losing installed
plug-ins, activation or saved projects, then prove the Bitwig mixed-use journey.
The completed Native Access recovery (#465) is the reference for that vendor
journey. `docs/ARCHITECTURE.md` fixes the boundaries and beta scope.

Reference: preserve the existing Deck setups and their previous runtimes. Compare
each saved project before and after its product-managed runtime move. Test215 is
installed; the Reference setup already plays and recalls independent instance
settings on r5 in REAPER, with clean retirement. This is not Bitwig qualification.

Next changes and checks:
1. Serum's retained runtime is restored; the operator confirms touch works again.
   Carry the released-contact and per-window routing corrections into the
   maintained GE line, then repeat playback/recall and physical popup checks.
   r5 playback and rendering passed; finger menu use stalled or lost the editor.
   Correct transparency remains an open, separate graphics defect (#330).
2. Move the Native Instruments setup through the same product journey. Preserve
   signed-in Native Access, licensed installations and content. Reopen the saved
   Kontakt Pad Noir project and verify the managed application still works.
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
