# RPI1 headless factory-preset button preflight

## Bounded slice

- Base head: `ca07f08c27b3e06803e07edc3eedb504b425a4c8`.
- Base tree: `4e0cf427c48bdd320e4b84c226ee0fb753163aaf`.
- Basis: `RPI1.md` “Primary claim,” “Phase C — Pigments discovery and appliance qualification,” and “Hard boundaries”; repository `AGENTS.md` “Keep the engineering safeguards” and “Verification and review.”
- Primary claim: identify whether the exact Pigments VST3 controller exposes a host-selectable program list that can truthfully drive previous/next **factory** presets without an editor.
- Fixture: the accepted Pi 5/ShieldXL appliance, Pigments 7.0.1.6772, exact installed Proton/Box64 runtime, current private environment and saved 45% state. The Pi-side read-only probe runs under the existing bounded census supervisor.
- Source scope: add only VST3 `IUnitInfo`, program-list and program-change-parameter observation to the Windows census. The ordinary processing and editor paths remain unchanged. No vendor content is copied or modified.
- Negative result: if no usable program list is exposed, or its connection to the factory library cannot be established, buttons 2 and 3 remain unassigned. Parameter titles or files in private preset directories are insufficient authority.
- Evidence: retain the private raw census on the Pi; publish only sanitized interface/list counts, IDs, names if generic, and the resulting capability disposition. Do not publish preset names, preset bytes, account information, or opaque state.
- Cleanup: the census must retire its Windows host, owner and transport normally. Leave the existing Pigments executable, saved state, audio route and ShieldXL control service unchanged. A replacement census candidate is separately named so the prior executable remains available.

## Current status

The retained census reports sixteen `kIsProgramChange` parameters (IDs 2272–2287, each with 127 steps). Pigments' similarly named `Previous Preset` and `Next Preset` parameters are `kIsReadOnly`; they cannot be written by the host. The retained census omits `unitId` and `IUnitInfo`, so it does not establish what those program-change parameters select. The new observation closes that information gap before any physical button mapping or audio run.
