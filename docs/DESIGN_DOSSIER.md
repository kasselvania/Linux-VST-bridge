# Soft Design Dossier

**Document identity:** `linux-audio-compatibility-bridge.soft-design-dossier.v1`  
**Document class:** `HUMAN_NORTH_STAR__REPOSITORY_OWNED__NON_IMPLEMENTATION_CLAIM`  
**Status:** Initial operator-requested design seed.  
**Prepared:** 2026-08-29 America/Los_Angeles.

This dossier states the intended functionality, product laws, end-user experience, technical pressure, failure posture, fixture strategy, and dependency-ordered proof shape for a managed Windows-audio compatibility platform serving native Linux DAWs.

It is deliberately **not** a roadmap, schedule, release promise, compatibility matrix, or implementation task card. `CURRENT_SLICE.md` is the only active implementation authority.

## Central ruling

The product is not “Proton loads Windows plug-ins in Bitwig.” A native Linux proxy represents an exact Windows plug-in class to the DAW. A supervised Windows host loads the proprietary module under a pinned audio-oriented runner. A versioned, reentrant, real-time-bounded transport connects them. A manager owns installation, authorization handoff, scanning, organization, host publication, diagnostics, update, repair, and rollback.

The ordinary user should not need to understand Wine prefixes, bridge synchronization, Flatpak mount paths, runner archaeology, or hand-written compatibility files in order to make music.

## Dossier set

1. [`01-product-and-user-experience.md`](design-dossier/01-product-and-user-experience.md) — product declaration, users, laws, nonclaims, installation-to-recall end-user flow, updates, repair, and diagnostics.
2. [`02-identity-presets-and-visual-system.md`](design-dossier/02-identity-presets-and-visual-system.md) — canonical identities, managed environments, presets, DAW state, content roots, organization, manager visuals, chain view, and editor presentation.
3. [`03-activation-flatpak-runtime-and-recovery.md`](design-dossier/03-activation-flatpak-runtime-and-recovery.md) — browser/deep-link/localhost activation, phone-home behavior, stable machine identity, Flatpak publication choices, runner/environment lifecycles, and failure ownership.
4. [`04-fixtures-proof-sequence-and-next-decision.md`](design-dossier/04-fixtures-proof-sequence-and-next-decision.md) — Serum 2 and Kontakt pressure, actual dependency-ordered development proof sequence, acceptance scenarios, pre-mortem, prohibited designs, and the bounded next-decision alternatives.

## Fixture declaration

- **Serum 2 VST3 is the first real commercial plug-in fixture.** An open SDK/reference plug-in remains necessary test instrumentation, but does not replace the Serum product proof.
- **Kontakt is a later hostile systems fixture.** It pressures Native Access, account/network state, large relocatable libraries, Player/full behavior, licensed and legacy content, locate/repair flows, and project recall. It must not block the first VST3 protocol proof.
- The Steam Deck + Bitwig Flatpak is the first exact host fixture because it is available. It is not the definition of the universal product.

## How implementation consumes this dossier

The operator and technical lead may use the full set to reason about the product. An implementation agent receives only:

- exact repository basis;
- `AGENTS.md` and `GOVERNANCE.md`;
- one current slice with one claim;
- the applicable architecture headings;
- the applicable fixture requirements;
- exact tests, evidence obligations, and nonclaims.

No agent should turn the entire dossier into one implementation prompt.

## Current next-decision posture

The design seed selects no active implementation slice. The technical-lead recommendation is:

1. an exact Serum 2 fixture reconnaissance slice;
2. an independent native Bitwig VST3 probe slice;
3. then a much narrower mapped factory/proxy slice.

That sequence keeps Serum 2 as the first real target while preventing installer, activation, Flatpak, VST3 ABI, scanner, proxy, and runner uncertainty from collapsing into one uninterpretable failure.
