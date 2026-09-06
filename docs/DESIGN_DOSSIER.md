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

AP4 is accepted through PR #59, merge `a45a30b916f847d1cc683ef7e07121b57d52491c`. The retained single AGain instance now runs through a native VST3 proxy in normally launched Bitwig, processes real Windows audio and saves/recalls actual component state across fresh processes. Idle time and screen-sharing reconnect no longer determine its lifetime. See [AP4 result](AP4_RESULT.md) and [preview prerequisites/limits](AP4_PREVIEW.md).

The working fixture still needs prepared artifacts/runtime and a private owner. It uses float32 stereo at 48 kHz, 1–256-frame callbacks and 1024 samples added latency. Multi-instance use, instruments, vendor editors, commercial compatibility, automated setup, reboot persistence and low-latency suitability are not yet established. Earlier accepted source/evidence and failed observations remain retained.

## Next product step

The lead selects **AP5: two independent plug-in instances in one Bitwig project**, tracked in #60. This removes actual single-instance restrictions and tests separate audio/state/lifetime in a useful project shape. It is not the entire later commercial stress matrix. The concise work order is in [CURRENT_SLICE.md](../CURRENT_SLICE.md).

Serum 2 remains the first intended commercial fixture; AGain is controllable engineering instrumentation, not a replacement commercial milestone. MIDI/instrument behavior, general plug-in interfaces, editor/authorization support and product setup still need deliberate implementation. Kontakt remains a later vendor-manager/content/recovery challenge and must not define every early task.

## Using this material

Leads use the dossier and current code to choose a coherent result. Implementers receive that result, the relevant boundaries and a few targeted references, with room to choose private implementation details. Additional design is for consequential unresolved behavior; ordinary debugging and test-helper repairs stay within the implementation task. A successful, relevant observation is reviewed on its merits without a compulsory second run under another label.
