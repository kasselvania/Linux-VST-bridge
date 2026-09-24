# Current work selection

Canonical base: `9f37338140a61e838e83d757628b6cb914b83bb5`, tree
`3bb22f1d9971c97f01dc65a8227f1a7d7f727f54`. This normal merge of the
WD0 architecture PR includes the accepted Serum candidate-D authority. It records
the operator's parallel product direction selected on 2026-09-24. It does not
supersede another agent's branch-local task or grant ownership of its checkout
or live experiment.

## Native-Linux product: current sequence

The native-Linux DAW bridge remains the first release-driving product. Serum 2
candidate D and the shared per-window X11 touch route are now merged and
physically accepted for the exact Deck waveform-popup interaction. Current
support and remaining shared failures are maintained in
[docs/SUPPORT_MATRIX.md](docs/SUPPORT_MATRIX.md) and
[docs/FAILURE_CLASSES.md](docs/FAILURE_CLASSES.md).

The selected near-term sequence is:

1. the operator uses the current six-product Deck fleet in Desktop Mode and
   records only useful real-use failures or friction;
2. a focused manager-frontend product-design slice improves product readiness,
   live-session, incident and recovery presentation without inventing new
   backend authority;
3. after that Desktop-mode soak, GM0 validates one known-good Serum instance
   through the Desktop-to-Gaming-Mode graphical/audio/session transition.

These are product-development steps, not instructions to replay the historical
qualification campaigns.

## Parallel Windows-DAW outcome

The selected Windows-DAW implementation is
[WD0 — managed FL Studio stock-project workflow](docs/WD0.md), governed by
[docs/WINDOWS_DAW_WORKSPACES.md](docs/WINDOWS_DAW_WORKSPACES.md). It proceeds in
parallel with the native bridge and the separately owned ARM appliance.
Coordination and integration rules are in
[docs/WORKSTREAMS.md](docs/WORKSTREAMS.md).

WD0 must deliver a real managed install, normal launch, stock-project playback,
export, clean relaunch and licensed project recall in a new FL workspace. It is
not a Linux VST proxy path and not a generic Windows-program launcher.

The active WD0 implementation branch is `codex/wd0-fl-studio-workspace`. Source
and Deck execution now have the operator's explicit physical handoff. The
official FL installer was imported into a fresh private workspace and completed
its visible UI, while its outer worker returned nonzero; the installed x64 FL
application was verified separately. Its first managed launch stopped at FL's
program-validity dialog. An isolated, immutable FL-only Wine `crypt32` successor
is being selected to correct that exact signature-verification boundary. This
does not change the installed native bridge, Serum or the other five products.
A source build or successful unlicensed runner smoke is not WD0 acceptance;
audible stock playback, project save/export, clean relaunch and licensed recall
remain the connected completion gate.

The exact operator-supplied FL Studio 26.1.6 installer was fingerprinted and
imported without modifying the original download. Installer bytes, account
state, license material and private projects never enter Git.

## Shared Deck ownership

Source work and isolated builds may proceed concurrently. Only one task custodian
may mutate or physically exercise the Deck at a time. Installing software,
replacing the manager/service, changing a runner or environment, changing audio
or Steam launch settings, and running a physical acceptance session require an
explicit handoff from the current custodian.

The FL agent may complete source, tests, installer admission and private workspace
preparation while the operator uses Bitwig. It must not replace the installed
manager generation, stop another task's session or execute the FL installer until
it receives the physical-work window.

Every common product package must preserve the now-canonical
`x11_touch_routing_v2` policy, Serum candidate-D authority, all required exact
Windows host/source pairs and the other five selected publications.

## Current implementation boundary

The WD0 branch adds a typed FL workspace beside the native DSP lane, with
separate package and process ownership. The FL-only crypt32 correction retains
signature verification and changes no native-bridge catalogue, publication,
capacity or audio path. FL Studio remains unqualified until the actual stock
musical workflow and retirement gates pass.

## Retained completed slice

The completed Serum touchscreen slice remains available at the exact pre-WD0
main pointer and in
[evidence/serum-x11-touch-routing](evidence/serum-x11-touch-routing/). Its
accepted result is current product authority; it is not an instruction to rerun
Serum before WD0 or GM0.
