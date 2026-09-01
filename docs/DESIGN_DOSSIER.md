# Soft Design Dossier

**Document identity:** `linux-audio-compatibility-bridge.soft-design-dossier.v1`  
**Document class:** `HUMAN_NORTH_STAR__REPOSITORY_OWNED__NON_IMPLEMENTATION_CLAIM`  
**Status:** Product north star; implementation authority remains slice- and design-revision-specific.  
**Prepared:** 2026-08-29 America/Los_Angeles; process posture updated after accepted WR0.

This dossier states the intended functionality, product laws, end-user experience, technical pressure, failure posture, fixture strategy, and dependency-ordered proof shape for a managed Windows-audio compatibility platform serving native Linux DAWs.

It is deliberately **not** a roadmap, schedule, release promise, compatibility matrix, or implementation task card. `CURRENT_SLICE.md`, the selection receipt, and—where required—the exact approved implementation design and approval receipt own active work.

## Central ruling

The product is not “Proton loads Windows plug-ins in Bitwig.” A native Linux proxy represents an exact Windows plug-in class to the DAW. A supervised Windows host loads the proprietary module under a pinned audio-oriented runner. A versioned, reentrant, real-time-bounded transport connects them. A manager owns installation, authorization handoff, scanning, organization, host publication, diagnostics, update, repair, and rollback.

The ordinary user should not need to understand Wine prefixes, bridge synchronization, Flatpak mount paths, runner archaeology, or hand-written compatibility files in order to make music.

## Dossier set

1. [`01-product-and-user-experience.md`](design-dossier/01-product-and-user-experience.md) — product declaration, users, laws, nonclaims, installation-to-recall end-user flow, updates, repair, and diagnostics.
2. [`02-identity-presets-and-visual-system.md`](design-dossier/02-identity-presets-and-visual-system.md) — canonical identities, managed environments, presets, DAW state, content roots, organization, manager visuals, chain view, and editor presentation.
3. [`03-activation-flatpak-runtime-and-recovery.md`](design-dossier/03-activation-flatpak-runtime-and-recovery.md) — browser/deep-link/localhost activation, phone-home behavior, stable machine identity, Flatpak publication choices, runner/environment lifecycles, and failure ownership.
4. [`04-fixtures-proof-sequence-and-next-decision.md`](design-dossier/04-fixtures-proof-sequence-and-next-decision.md) — Serum 2 and Kontakt pressure, dependency-ordered development proof sequence, acceptance scenarios, pre-mortem, prohibited designs, and bounded next-decision alternatives.

## Accepted implementation frontier

The repository now retains these exact proof layers:

```text
SR0
  Steam Deck / Bitwig / existing Serum-artifact reconnaissance

HP0
  repository-owned native Linux VST3 build
  -> official validator
  -> exact user-space publication
  -> explicit load inside the exact Bitwig Flatpak sandbox

HP1
  normal Bitwig discovery
  -> exact native class registration
  -> operator instance admission
  -> exact module mapping by a Bitwig descendant

WR0
  exact installed Runtime 4 / Proton 11 process route
  -> nonce/run/source/workload-bound Windows command execution
  -> isolated owned environment
  -> transactional reuse/replacement
  -> exact process cleanup
```

The first unproved product region begins on the Windows VST3 side. No Windows module factory census, Windows plug-in host, native proxy/factory crossing, IPC, shared-memory audio, Serum operation, activation, or compatibility claim exists yet.

## Fixture declaration

- **Serum 2 VST3 is the first real commercial plug-in fixture.** An open SDK/reference plug-in remains necessary test instrumentation but does not replace the Serum product proof.
- **Kontakt is a later hostile systems fixture.** It pressures Native Access, account/network state, large relocatable libraries, Player/full behavior, licensed and legacy content, locate/repair flows, and project recall. It must not block the first Windows VST3 protocol proof.
- The Steam Deck + Bitwig Flatpak is the first exact host fixture because it is available. It is not the definition of the universal product.

## How implementation consumes this dossier

The operator and technical lead use the full dossier to reason about the product. Implementation does not consume it as one enormous task.

The process is governed by [`DEVELOPMENT_PROCESS.md`](DEVELOPMENT_PROCESS.md):

```text
successor analysis
    -> explicit operator selection
    -> bounded reconnaissance
    -> implementation design card
    -> independent adversarial design review
    -> exact design approval
    -> implementation
    -> independent pre-PR audit
    -> technical-lead exact-head review
```

An implementation agent receives only:

- exact repository basis;
- `AGENTS.md` and `GOVERNANCE.md`;
- the active slice and authority phase;
- exact approved design revision when required;
- applicable architecture headings;
- applicable fixture requirements;
- exact interfaces, tests, evidence obligations, and nonclaims.

A material discovery returns work to the design gate. It is not patched forward merely because implementation has begun.

## Current next-decision posture

DG0 establishes the process by which the next product slice will be selected and designed. It does not select that slice.

After DG0 and its status closure, use [`prompts/CHOOSE_NEXT_SLICE.md`](prompts/CHOOSE_NEXT_SLICE.md) against the exact accepted main boundary. The likely frontier is a bounded Windows VST3 reconnaissance/scanning or factory-host proof, but the technical lead must compare candidates from current evidence rather than treating the old dependency sequence as an automatic roadmap.
