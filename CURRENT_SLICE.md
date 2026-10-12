# Interaction prototype before manager implementation

## Goal

Compare three arrangements of the agreed model: collection first, next action
first and a goal workspace. Which makes what I have, my next action and returning
to music obvious? Production manager implementation stays paused.

## Best explanation

The agreed model separates current access, observed use, optional reports and
maintenance history. Navigation and interaction hierarchy are still unproven;
the existing manager UI has no design authority. Test concrete choices before
implementing another manager shape.

## Current design work

The [standalone prototype](docs/design-dossier/prototype/index.html?variant=A&scenario=kontakt)
uses `?variant=A/B/C` and `?scenario=reaper/asc/kontakt`. It simulates first REAPER
installation, another selected Arturia product through existing ASC, and Kontakt
already available in Bitwig despite missing historical installer evidence.
Separate controls cover interrupted vendor work, drive reconnection and restoration
after a product update fails in actual use. No real operations or persistence.
Sol implements; Astra reviews. The operator's usability verdict is pending.
The [model](docs/design-dossier/01-product-and-user-experience.md), five selected
policies and [diagrams](docs/design-dossier/flows/README.md) remain binding.

## What done looks like

The operator can identify the current situation, next meaningful action and return
to music without narration, then choose or combine structures. Record that answer
before rewriting the manager; absorb the decision and remove the losing prototype.
Prototype comprehension is separate from installed behavior and beta acceptance.

## Preserved delivery state

Test53 delivery and earlier physical results remain in SUPPORT_MATRIX.md and
historical delivery records. Open audio, startup, update, recovery and editor
failures remain in FAILURE_CLASSES.md. Do not restart those campaigns from this
slice or alter user projects, vendor installations or activation state for design.
