# Interaction prototype before manager implementation

## Goal

Add Analog Lab V through the already installed Arturia Software Center while
eFX FRAGMENTS works in Bitwig. Can the shared-use decision be understood without
repeating setup? Production manager implementation stays paused.

## Best explanation

The user explicitly accepted the first-install interaction, including suggested
device-neutral storage. Keep that baseline. Returning to a vendor application
must reuse the existing installation and location, explain the named plug-ins
temporarily unavailable, and distinguish an open project from stopped playback.

## Current design work

The [prototype](docs/design-dossier/prototype/index.html?journey=returning) has first
and returning starting situations. First completion naturally exposes installed
ASC. Returning use can postpone, or close the FRAGMENTS project before opening it.
Analog Lab selection remains in the vendor stand-in; safe completion automatically
shows each plug-in's result. Existing download pause/resume and storage are retained.
All activity is simulated and offline-capable. Sol implements; Astra reviews.
The first interaction is accepted; the returning walkthrough verdict is pending.
The [model](docs/design-dossier/01-product-and-user-experience.md), five selected
policies and [diagrams](docs/design-dossier/flows/README.md) remain binding.

## What done looks like

The operator can find the installed ASC, understand why FRAGMENTS must close,
postpone without losing use, and add Analog Lab without choosing source/storage
again. Broader design/beta approval remains separate; REAPER, working Kontakt,
storage and update/recovery needs remain. Production implementation stays paused.

## Preserved delivery state

Test53 delivery and earlier physical results remain in SUPPORT_MATRIX.md and
historical delivery records. Open audio, startup, update, recovery and editor
failures remain in FAILURE_CLASSES.md. Do not restart those campaigns from this
slice or alter user projects, vendor installations or activation state for design.
