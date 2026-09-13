# Project value and strategy

## Thesis

Linux Audio Compatibility Bridge is not only a transport between a Linux DAW and a Windows VST3. Its durable asset is an accumulating body of **versioned compatibility knowledge**:

- exact runtime and environment bindings;
- reusable Windows/VST3 behavior fixtures;
- deterministic regressions for generic failure classes;
- product-specific compatibility profiles;
- real DAW qualification evidence and explicit limits;
- private crash attribution and positive cleanup records;
- immutable candidate, ordinary activation and rollback history.

The product opportunity is to make existing Windows audio software available on Linux with less friction than asking every musician, support engineer or plug-in manufacturer to rediscover the same Wine, lifecycle, UI and failure behavior independently.

## The problem is larger than loading a VST3

A useful commercial plug-in is a stack, not a DLL:

```text
installer and updater
account and authorization
content and presets
VST3 factory/classes
DSP and events
opaque state and project recall
editor, focus, popups and resizing
process lifetime and cleanup
release updates and support incidents
```

A generic bridge can solve part of the module-loading problem while leaving the rest to manual Wine-prefix administration and ad hoc troubleshooting. This project treats those surrounding concerns as first-class product responsibilities.

The end-user goal is deliberately simple:

```text
install
→ open in the Linux DAW
→ make music
```

The system absorbs the compatibility machinery beneath that experience.

## The compounding asset

The project is designed so that a real failure becomes a reusable platform capability.

```text
real product incident
→ durable evidence
→ generic concept
→ source-owned reproducer
→ shared compatibility law
→ focused product confirmation
```

The distinction matters. A product-name exception such as “special-case this Pigments window” helps one build. A generic transient-surface model can help any Windows framework that represents a menu as an ownerless group of content, border and shadow windows.

The same pattern has already produced reusable work around:

- VST3 lifecycle restart and same-instance reconfiguration;
- fair Win32 input retrieval under sustained posted work;
- editor creation, focus, close and reopen;
- exact state capture and save/reopen;
- process-scoped retirement where SDK teardown is unsafe;
- terminal-failure custody;
- native-host survival and controlled silence after Windows-peer loss;
- bounded crash attribution after long-running diagnostic output;
- canonical owned popups and ongoing generalized transient-surface work.

The desired economic effect is straightforward:

> Each generic repair should reduce the engineering cost and uncertainty of qualifying the next plug-in.

That hypothesis should be measured rather than assumed. Useful measures include time to first audio, time to normal installer/authorization, number of new generic failure classes, number of product-specific policies, test reuse, incident-to-diagnosis time, and maintenance cost after vendor updates.

## Three durable layers

### 1. Shared compatibility kernel

The shared kernel owns behavior that should not depend on a product name:

- native/Windows process supervision;
- transport and real-time boundaries;
- VST3 lifecycle, state, events and buses;
- editor and input behavior;
- terminal failure and cleanup;
- crash attribution;
- immutable publication and rollback.

Fixes here benefit every admitted product.

### 2. Declarative compatibility profiles

Profiles bind exact product facts and closed policy choices:

- module and class identity;
- native proxy, Windows host and descriptor;
- runner and environment revision;
- qualified buses and precision;
- event interpretation;
- editor/accessibility/retirement policy;
- known limitations and evidence.

Profiles do not contain arbitrary code or licensing bypasses. They select reviewed capabilities already implemented by the shared system.

### 3. Evidence and incidents

Evidence separates what is implemented from what has actually been demonstrated:

- generated source-owned fixtures;
- exact real-product checks;
- private incidents;
- sanitized shareable reports;
- installed-state readback;
- limitations and nonclaims.

This allows the project to say “the generic mechanism works,” “this exact product passed,” or “this failure remains unattributed” without collapsing those statements into one marketing claim.

## Why Proton/Wine is useful

Proton/Wine provides a mature Windows-shaped execution layer beneath the project-owned bridge. The value comes from making that layer exact and managed:

```text
product profile
→ exact runner generation
→ exact environment/prefix family
→ exact Windows host
→ exact plug-in module
```

A future catalogue may bind different products to different validated runner generations. A modern plug-in, a legacy instrument and a vendor installer do not necessarily need the same runtime posture.

This should not become a user-facing “try every Proton version” control. Runtime selection belongs to reviewed compatibility profiles with evidence and rollback. Prefix/environment migration, authorization persistence and simultaneous multi-runner isolation require their own qualification.

## Value to musicians

The intended musician value is not “more Wine configuration.” It is less:

- no manual proxy synchronization;
- no hand-selected loader on every launch;
- no need to understand process ownership or prefixes;
- normal DAW discovery and project identity;
- normal vendor installation and authorization where supported;
- visible failure rather than a silent zombie device;
- exact rollback when an update fails;
- actionable diagnostics that can be handed to support.

The bridge should eventually feel like infrastructure, not a hobbyist assembly exercise.

## Value to plug-in manufacturers

For some products, a managed compatibility layer may offer a commercially attractive intermediate or long-term Linux path compared with a full native port.

Potential manufacturer value includes:

- reuse of the shipping Windows VST3 and editor;
- preservation of normal account, licensing and content systems;
- a bounded Linux support matrix rather than arbitrary Wine setups;
- exact runtime and product profiles;
- deterministic regressions for Windows/VST3 behavior;
- private exception/module/RVA/stack reports;
- controlled candidate rollout and rollback;
- release qualification when plug-in or runtime versions change;
- a path to vendor-specific fixes becoming shared infrastructure where appropriate.

This is not free compatibility and it is not a promise that every product can avoid native engineering. Products with deep platform integration, drivers, unsupported graphics assumptions or strict support requirements may still need native work. The project’s value is reducing uncertainty and concentrating the remaining work into a managed layer.

## Commercial opportunity

The platform creates several non-exclusive commercial paths.

### Owner-operated product

The project owner can build and maintain a supported compatibility product, charge for access, support, profiles, updates or managed environments, and expand the working-plug-in catalogue over time.

### Paid compatibility engineering

A manufacturer or integrator can fund an exact proof, product qualification, release repair or portfolio assessment without licensing the complete platform initially.

### Vendor license or OEM integration

A manufacturer can license the bridge, profiles, runtime integration or a branded/embedded management experience for specific products or a broader catalogue.

### Ongoing compatibility maintenance

The project can provide release qualification, incident analysis, profile/runtime updates, regression maintenance and bounded Linux support as a recurring relationship.

### Joint upstream work

Where a demonstrated problem belongs in Wine, Proton or another dependency, the project’s source-owned reproducer and exact incident can support a high-quality upstream report or patch while product-specific mitigations remain controlled.

No commercial terms or rights are granted by this document or repository visibility. See [Vendor integration and commercial pathways](VENDOR_INTEGRATION.md) and [COPYRIGHT.md](../COPYRIGHT.md).

## Why the Arturia work matters

The current Arturia vertical spans more than module loading:

```text
official software center
→ account sign-in and authorization
→ download and installation
→ exact VST3 discovery
→ immutable publication
→ notes, sidechain, presets, automation and state
→ editor behavior
→ failure containment and attribution
```

That makes it a useful model for vendor collaboration. It demonstrates how the bridge can coexist with a vendor’s normal commercial systems rather than copying authorization state or replacing the vendor catalogue.

Arturia products are current engineering fixtures. No partnership, endorsement, support agreement or commercial relationship is claimed.

## Differentiation from a generic bridge

The differentiator is not that this project invokes Proton. It is the combination of:

- managed vendor acquisition;
- exact product/runtime identity;
- per-instance supervision;
- immutable product profiles and rollback;
- deterministic Windows compatibility fixtures;
- real DAW evidence;
- failure containment;
- actionable crash attribution;
- a maintained compatibility catalogue.

Another bridge could theoretically add the same capabilities. At that point it would also be becoming a compatibility platform rather than only a proxy mechanism.

## Roadmap logic

The roadmap should prioritize work that increases reusable capability and product credibility:

1. close the remaining Pigments transient-surface/resize and stability boundaries;
2. capture naturally occurring failures instead of forcing long reproduction campaigns;
3. qualify a second vendor and identify which Arturia assumptions are truly generic;
4. measure the marginal effort of each additional product;
5. turn engineering commands into a user-facing installation/status/repair experience;
6. establish evidence-backed runner selection and update policy;
7. define a supportable commercial catalogue rather than claiming universal compatibility.

Maintenance work remains important, but it should be selected by demonstrated risk or product value rather than ceremonial retesting.

## Guardrails

The commercial thesis does not weaken the engineering standard.

- No authorization or licensing bypass.
- No proprietary vendor binaries, presets, credentials or opaque state committed as evidence.
- No universal support claim from one clean session.
- No relabeling a generated fixture as real-product proof.
- No hard-coded product-name dispatch where a generic law can be expressed.
- No diagnostic I/O, blocking or allocation in real-time callbacks.
- No publication or promotion without exact retained identity and rollback.
- No automatic sharing of private incidents.

## Success condition

The project succeeds when supported Windows plug-ins can be delivered on Linux as controlled products rather than fragile local experiments—and when each qualification improves the platform enough that the next one becomes faster, cheaper and easier to maintain.
