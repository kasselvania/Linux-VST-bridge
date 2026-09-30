# Self-service beta delivery

Selected by the operator on 2026-09-29. Source basis: main
`13ed1d85e830d581ec297760e04e9f9433bfd671`.

- [ ] **1. Deliver an application-owned runtime.** Supply a versioned, verified
  Wine/Proton-derived runtime with its required support components through
  explicit verified upstream acquisition. Preserve upstream notices. Install
  it through the normal product route without
  a pre-existing Wine, Proton or Steam installation. Retain bound predecessors.
- [ ] **2. Deliver prebuilt native proxies.** Ship exact supported plug-in
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
Task 1 has implementation, normal runtime acquisition and an observed official
vendor-installer window on the delivered runtime. Task 2
has implementation and verified Linux builds of both packaged proxies.
Task 3 completed official vendor installation but found upstream runtime mode
drift blocking discovery. The source correction awaits installed retest; trial
usability, publication, sound and persistence remain open. The checkboxes remain open for their complete stated outcomes.
