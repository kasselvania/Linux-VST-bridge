# Product model before manager implementation

## Goal

Establish tasks, user variations, responsibility, completion and recovery before
more manager implementation. The operator's 11 October instruction supersedes the
previous polish/tester next steps. Implementation stays paused; installed products remain available.

## Best explanation

The manager exposes internal operations without a coherent user task model.
The operator reports Kontakt working and recalling in Bitwig while Home asks
for installation review. Installer history, DAW availability, successful use and
maintenance authority need separate meanings. Relabeling controls cannot settle
those meanings or the full workflow. This is a product-design failure.

## Current design work

The [product model draft](docs/design-dossier/01-product-and-user-experience.md)
replaces the earlier interaction seed in its existing dossier location. Review
its inventory, shared and variable paths, control/observation boundaries,
interrupted flows and the [visual walkthrough](docs/design-dossier/flows/README.md).
All five policy choices are selected; walk the diagrams and derive the interaction design.
Then derive navigation and a disposable prototype and test comprehension with
the operator's Bitwig case and a REAPER user. Sol implements agreed slices;
Astra reviews. Existing screens and code have no presumptive authority.

## What done looks like

An agreed model explains what each person needs to do, what the product handles,
what belongs to vendors/DAWs, when a task is finished and how interruption recovers.
Design approval is separate from fixing the installed manager or qualifying beta.
Selected scope is recorded in the beta contract; its other acceptance requirements remain.

## Preserved delivery state

Test53 delivery and earlier physical results remain in SUPPORT_MATRIX.md and
historical delivery records. Open audio, startup, update, recovery and editor
failures remain in FAILURE_CLASSES.md. Do not restart those campaigns from this
slice or alter user projects, vendor installations or activation state for design.
