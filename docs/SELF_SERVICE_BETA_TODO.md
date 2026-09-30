# Self-service beta delivery

Selected by the operator on 2026-09-29. Source basis: main
`13ed1d85e830d581ec297760e04e9f9433bfd671`.

- [x] **1. Deliver an application-owned runtime.** Supply a versioned, verified
  Wine/Proton-derived runtime with its required support components through
  explicit verified upstream acquisition. Preserve upstream notices. Install
  it through the normal product route without
  a pre-existing Wine, Proton or Steam installation. Retain bound predecessors.
- [x] **2. Deliver prebuilt native proxies.** Ship exact supported plug-in
  proxies and the Windows host as compatible release components. Installation
  verifies module/class/metadata identities and publishes the matching proxy;
  the customer does not install compilers, a VST3 SDK or a Flatpak development
  SDK. Preserve stable external class IDs and project identity.
- [ ] **3. Test the complete delivered journey.** On a clean supported x86
  Linux fixture: install bridge, prepare runtime, install/authorize a lawful
  commercial plug-in through the normal UI, publish, use sound/editor, save and
  reopen, reboot and reopen, and exercise failure/recovery. No manually seeded
  runtime, development cache, copied authorization state or maintainer repair.
- [ ] **4. Deliver platform releases.** Complete authenticated customer
  releases with required dependencies, signing and notices. SteamOS delivery
  must use writable user storage without disabling its protected system.
- [ ] **5. Complete update and recovery.** Verify compatible manager/runtime/
  proxy updates and exact rollback while preserving projects, state and vendor
  authorization. Recovery must be available in the application.

The current assignment implements 1 and 2 and tests 3. Tasks 4 and 5 remain
tracked follow-on work. A source test, package launch or SDK example does not
complete the commercial journey. Preserve partial observations and failures;
check a task only when its stated outcome has actually been demonstrated.

Current progress is retained in the [Ubuntu delivery test](../evidence/self-service-delivery/ubuntu-fragments-trial-2026-09-29.json).
Task 1 passed on the declared Ubuntu fixture: normal verified upstream
acquisition, official vendor installation and discovery on the application-owned
-r3 runtime, without preinstalled Wine, Proton, Steam or development tools.
Task 2 passed exact prebuilt preparation and experimental publication through
normal product controls. Three exact proxy entries were independently rebuilt
and compared; the official trial module remains distinct from the Deck bytes.
Neither result qualifies another platform or arbitrary plug-ins.

Task 3 reached real installed use in internal26: Bitwig 6.1.1 guest demo loaded
FRAGMENTS from an idle bridge without a manual reload, rendered the vendor DEMO
editor, and captured stereo output through the active effect at 48 kHz. The
explicit 1024-frame testing configuration reports 1216 total frames including
192 vendor frames; historical support/default settings are unchanged. Startup
took 72.440 seconds and the session had missing output frames, so responsive or
dropout-free use is not claimed. Normal editor close crashed in removed(), with
confirmed host/transport cleanup. The shared host's frame-detach order is now
corrected in source and requires its exact built commercial retest.

Internal27 includes that Windows host correction. Its Windows CI and three
independent native proxy comparisons passed, and the system package installed.
Normal selection on the populated account refused
`package_existing_product_host_pair_changed`, preserving internal26 and its
publication. The existing account with no registry or native catalogue selected
internal27 through normal package controls for a focused host retest. That
account retains earlier failed onboarding attempts; it is not another clean
account qualification. Safe migration of a changed host remains a task-5 gap.

A new clean user account then adopted internal27 and acquired its runtime
through normal Setup, without copied environments or authorization. The
official installer reached Finish but did not retire its process cohort.
Manager progress-file stamp checks also hid live recovery controls. An exact
supervisor cancellation preserved installed files and confirmed cleanup;
this is not a normal recovery pass. The shared readback correction passes
focused source checks; its built/installed test and the corrected host's
commercial editor test remain pending.

State capture remains refused (SDK result 1, zero bytes). [Arturia's demo
policy](https://support.arturia.com/hc/en-us/articles/5671785160732-Demo-versions-What-should-I-know)
disables save/load features; the refusal is consistent with that restriction,
but its exact cause is unproven. Guest demo disables
project save/export; an eligible official Bitwig trial can exercise persistence
without a paid license, but vendor state must also work. No account, paid
activation, fabricated state or licensing bypass was used. Save/reopen,
reboot/recall, normal editor retirement and ordinary recovery remain task-3
gaps. Tasks 3–5 remain open for their complete stated outcomes.
