# Current slice — restore Native Access → Kontakt on r5 (#465)

Goal: restore the ordinary managed Native Access → Kontakt journey on current
`main` before further runtime consolidation or beta qualification.

Architecture: [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) is fixed. Runtime code
is shared, mutable vendor installations live in setups, and each live plug-in
instance has its own Windows host. This slice cannot redefine those boundaries.

Reference: build one normal current-main package that selects managed GE-Proton
r5 for a fresh disposable CachyOS account with native REAPER 7.82. Compare with
the retained working r5 package/fixture at the first differing behavior, using
Test88 package behavior and Test126's accepted CachyOS musical result as retained
references where applicable. Do not assume current main + r5 still works.

Method:
1. Reproduce ordinary Native Access installation and managed launch.
2. At the first difference, hold application build, setup/dependencies, profile
   choices and graphical context constant and compare the exact package/runtime/
   launch behavior.
3. Change one causal variable. Keep the correction or revert it.
4. Keep installer completion, usable application window, authorization, Kontakt
   installation, plug-in discovery and musical use as separate outcomes.

All relevant layers remain suspects: manager, profile, setup/dependency handling,
launcher and runtime. Existing implementation structure is not a reason to protect
a broken mechanism. Preserve exact target identity, bounded custody, setup
isolation, user state and safe cleanup.

Do not rebuild or salvage r6, start the DirectComposition track, change the
supervisor execution boundary, redesign the UI, create another profile/runtime
framework, or touch licensed Deck installations in this slice. PR464's structured
installer result remains useful, but completion does not establish Native Access
usability.

Done: the ordinary product installs Native Access, opens a usable managed window,
returns user-controlled sign-in, installs Kontakt, discovers and publishes it
without manual prefix repair, plays MIDI/audio in REAPER, recalls changed state
after full exit/reopen, and retires the affected owned processes/services cleanly.
Record the causal correction and a focused regression. A diagnosis, copied app,
green tests or completed installer is not completion while that journey is broken.

Next after #465: run the architecture's mixed-use regression—two distinct live
instances from one shared setup plus a plug-in from another setup, simultaneous
playback, independent instance removal/reload and fresh-process project recall—
with corresponding REAPER and Bitwig projects.
