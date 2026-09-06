# Product Design Dossier

**Document identity:** `linux-audio-compatibility-bridge.soft-design-dossier.v1`  
**Purpose:** Human-facing product direction, not a release promise or implementation checklist.

## Product

A native Linux proxy represents a Windows plug-in class to the DAW. A supervised Windows host loads the actual module under a deliberately selected compatible runner. Explicit control/state and real-time audio/event interfaces connect them. The intended manager covers installation, vendor-authorized activation, scanning, publication, organization, updates, diagnostics and rollback.

The user should not need to administer Wine prefixes, Flatpak paths, synchronization commands or runner archaeology to make music. State and project recall are product behavior, not optional reporting polish.

## Dossier set

- [Product and user experience](design-dossier/01-product-and-user-experience.md): product declaration, user laws and installation-to-recall workflows.
- [Identity, presets and visuals](design-dossier/02-identity-presets-and-visual-system.md): persistent identity, DAW state, content and editor presentation.
- [Activation, Flatpak, runtime and recovery](design-dossier/03-activation-flatpak-runtime-and-recovery.md): vendor-owned authorization, publication and environment lifetime.
- [Fixtures, development dependencies and pre-mortem](design-dossier/04-fixtures-proof-sequence-and-next-decision.md): Serum/Kontakt pressure, technical dependencies, risks and scenarios.

These retain the original product reasoning. Historical status statements and suggested ordering in them are not current progress or an obligatory task sequence. Consult [current work](../CURRENT_SLICE.md) and reviewed results. Development follows [AGENTS.md](../AGENTS.md) and [the simplified process](DEVELOPMENT_PROCESS.md), not the retired selection/receipt/acceptance ceremony formerly reproduced here.

## Accepted capability

AP5 is accepted through PR #61, merge `6694f6e410cb1ee7395879b570ae817b7ceed38f`, at reviewed head `24cae167c4b3e61fc59f07fa51ec6cce9e638996`. Two real Windows AGain effects operate independently in one native Bitwig host, save and recall distinct state, and permit one instance's removal while its sibling continues. The normal desktop recall compared 61,495,808 samples at zero error. The owner reporting/cleanup repair was separately verified. See [AP5 results](AP5_RESULT.md). **An earlier isolated transport loss remains unexplained; this is not a stable-everyday-use guarantee.**

AP4's normal application launch, complete component-state recall and non-expiring interactive lifetime remain accepted prerequisites. See [AP4 result](AP4_RESULT.md) and [preview setup](AP4_PREVIEW.md); AP5 supersedes that setup document's historical one-instance limit. Earlier accepted source/evidence and failed observations remain retained without relabeling.

The fixture still needs prepared artifacts/runtime and a private owner. Native and owner capacity is four; desktop evidence covers two. It uses float32 stereo at 48 kHz, 1–256-frame callbacks and 1024 samples added latency. Instruments, vendor editors, commercial compatibility, automated setup, reboot persistence and low-latency suitability remain unproved.

## Next product step

**AP6: recover an individual failed Windows instance without restarting Bitwig or disturbing healthy siblings**, tracked in #62 and authorized by the operator. Preserve useful first-fault evidence, retain confirmed complete component state, and provide explicit bounded recovery in the existing two-track project. The concise work order is [CURRENT_SLICE.md](../CURRENT_SLICE.md). Recovery restores an identified snapshot, not uncaptured edits, buffer history or tails; a missing/failed restore is explicit. Unknown historical failures remain unknown unless evidence identifies their cause.

The reusable part is instance isolation, lifecycle ownership, opaque snapshot transport/restoration and host synchronization. Current AGain class/format validation and numerical oracles are reference-specific; new recovery logic must not add dependence on those values. Commercial plug-ins can have separate controller state, content dependencies and authorization; passing an AGain check cannot certify them. Preserve VST3's separate processor/controller responsibilities and test non-gain state so a remembered slider is not mistaken for recovery.

Serum 2 remains the first intended commercial fixture. AGain is controllable engineering instrumentation, not its replacement. After this concrete recovery gap, lead selection should return to the actual commercial-input/interface needs rather than invent an endless reference-only hardening ladder. MIDI/instrument behavior, general plug-in interfaces, editor/authorization support and product setup still need deliberate implementation. Kontakt remains a later vendor-manager/content/recovery challenge.

## Using this material

Leads use the dossier and current code to choose a coherent result. Implementers receive that result, relevant boundaries and a few targeted references, with room to choose private implementation details. Additional design is for consequential unresolved behavior; ordinary debugging and test-helper repairs stay within the implementation task. A successful, relevant observation is reviewed on its merits without a compulsory second run under another label.
