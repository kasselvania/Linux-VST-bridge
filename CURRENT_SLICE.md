# Interaction prototype before manager implementation

## Goal

Return to Kontakt 8 after reported play/save/reopen in Bitwig, without an
installation-completion task for missing old history. Does the calm collection
and one missing-library-drive message make sense? Production work stays paused.

## Best explanation

The user accepted both first installation with suggested storage and returning
ASC installation with its shared-use decision. Preserve those flows. The original
Kontakt complaint concerns missing installer history despite independently valid
present use; successful play/recall remains attributed to the user's report.

## Current design work

The [prototype](docs/design-dossier/prototype/index.html?journey=kontakt) has first,
returning ASC and working Kontakt starts. Kontakt shows Native Instruments and
instrument availability, with optional finding instructions and no history chore.
An outside demo disconnects Music SSD; only sounds from that drive need reconnecting.
The same-drive return clears the issue while installation and old history remain.
All activity is simulated and offline-capable. Sol implements; Astra reviews.
The first two interactions are accepted; the Kontakt walkthrough is pending.
The [model](docs/design-dossier/01-product-and-user-experience.md), five selected
policies and [diagrams](docs/design-dossier/flows/README.md) remain binding.

## What done looks like

The operator can return to music without an installation review, recognize the
specific missing-drive issue, and reconnect without reinstalling Kontakt or
inventing history. Broader design/beta approval remains separate; REAPER, storage
and update/recovery needs remain. Production implementation stays paused.

## Preserved delivery state

Test53 delivery and earlier physical results remain in SUPPORT_MATRIX.md and
historical delivery records. Open audio, startup, update, recovery and editor
failures remain in FAILURE_CLASSES.md. Do not restart those campaigns from this
slice or alter user projects, vendor installations or activation state for design.
