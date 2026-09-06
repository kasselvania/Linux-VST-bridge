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

AP7 is accepted through PR #65, merge `cdfd05be9af2576768d8f2ccc55d3064e9e72a36`, reviewed at `5232870e6ce024efe52c2d6be0af7f62b7f021c3`. The bridge distinguishes a transient presentation underrun from a failed connection, retains the live instance through a counted gap, and resumes at the correct absolute position. Optional numerical observation cannot hold up audio delivery. Actual two-instance Bitwig playback passed the scoped check without gaps, terminal failures or sample errors. [Results](AP7_PLAYBACK_REPAIR.md) identify the different tested sources and coverage.

AP4's normal launch and complete-state recall, AP5's independent instances and AP6's explicit last-confirmed-state recovery remain accepted. See [AP4](AP4_RESULT.md), [AP5](AP5_RESULT.md) and [AP6](AP6_RESULT.md). Historical transport/timing losses remain unresolved; the repaired policy and a quiet later run are not a long-run reliability claim. Original evidence and failed observations remain unchanged. AP7 N1's bounded JSONL emission repair is included as ordinary AP8 housekeeping, not another playback campaign.

The accepted fixture is still the prepared AGain effect: native/owner capacity four with real evidence for two, float32 stereo/48 kHz, 1–256-frame callbacks and 1024 samples of bridge delay. The vendor-generic host, instruments, editors, commercial activation/content, installation, reboot persistence and low-latency suitability have not yet been demonstrated.

## Next product step: actual Serum 2 first sound

The lead selects **AP8, issue #66: play a note clip through actual Windows Serum 2 in normally launched Bitwig, expose a real control and save/reopen the sound**. This returns to the intended commercial input rather than extending the AGain-only reliability ladder. Necessary module inspection, interface adaptation, ordinary vendor setup, note/parameter transport and opaque state belong to this one connected implementation task. The work order is [CURRENT_SLICE.md](../CURRENT_SLICE.md).

SR0 recorded an installed Windows Serum2.vst3 candidate and a distinct yabridge wrapper; neither proves the present license, module or content works under our selected runner. Inspect the actual input early. Preserve the user's existing .wine/yabridge environment; use a bridge-owned persistent vendor environment and separately owned sessions. Normal activation may need a small detached Windows editor and an operator login. Do not forbid that prerequisite while pretending headless first sound is unconditional; full native editor embedding and a product installer remain later work.

The concrete transfer work is to remove AGain-only class/parameter/audio-input/state assumptions from the new path. Query real SDK metadata, transport note events and parameter points with their identities and offsets, and let the real Windows component/controller own sound and serialized state. Keep the existing AGain publication/state compatible. Use stable separate native class identities so experimenting with our Serum proxy does not replace or collide with the user's other wrapper.

An SDK instrument may support local interface debugging, but it is not a substitute commercial milestone. A module census or a successful initializer alone does not complete AP8. Conversely, first sound does not certify every Serum engine, preset, editor action, license channel, sample rate, MPE path or long-term workload. Additional compatibility claims follow observed requirements and actual results, not inherited ceremony.

## Using this material

Leads use the dossier and current code to choose a coherent result. Implementers receive that result, relevant boundaries and a few targeted references, with room to choose private implementation details. Additional design is for consequential unresolved behavior; ordinary debugging and test-helper repairs stay within the implementation task. A successful, relevant observation is reviewed on its merits without a compulsory second run under another label.
