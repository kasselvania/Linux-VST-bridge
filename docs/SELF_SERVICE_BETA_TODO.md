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
Task 1 passed on the stated Ubuntu fixture: normal upstream acquisition,
official vendor installation and SDK discovery on the application-owned -r3
runtime, without preinstalled Wine, Proton, Steam or development tools. This
does not qualify every platform or plug-in. Task 2
has implementation and independently verified Linux builds of three exact
proxy entries. Normal compatibility checking prepared the official trial
module by copying matching packaged bytes on a machine without development
tools; normal Make available for testing completed exact managed publication.
It remains explicitly unqualified and experimental until task 3 establishes use.
Task 3 completed official vendor installation and discovery on the corrected
runtime in a fresh application account. Product controls exposed a further
readback timeout; its installed correction passed in 2.173 seconds. The
actual official module differs from the retained Deck bytes; prebuilt matching
must preserve that distinction. Exact test publication completed, then exposed
a separate catalogue ownership gap in normal status; its installed internal22
correction passed normal Home and product controls. Bitwig's separate agreement was confirmed and accepted; guest demo mode opened
without an account or paid license. Native scanning recognized FRAGMENTS, but
its first DAW load failed before a Windows session was created. Internal23 installed fresh per-admission full-byte verification and reached the
exact Windows module and shared transport. DAW activation still failed after
70.481 seconds: the vendor refused initial state capture (SDK result 1), and
Bitwig actually requested 1024 samples at 48 kHz against the selected 512-frame
bridge limit. Entering 512 in Audio settings reverted to 1024; no effective
configuration change is claimed. No audio callbacks or editor opens occurred;
Windows host and transport retired cleanly. The normal exact product controls
have no vendor-editor action before native activation, so the official trial
choice could not be reached. These are delivery/integration gaps, not a demand
for paid activation. Demo mode disables save/export; an eligible official trial
can test persistence without a paid license. Trial usability, sound, editor and
persistence remain open. Tasks 3–5 remain open for their complete stated outcomes.

The next task-3 work is concrete: expose the existing supervised vendor-editor
journey in normal product controls for the exact installed trial; identify and
resolve its state-capture refusal without inventing state; and deliver or guide
an effective supported DAW block configuration. Retest activation, trial editor
and processing before proceeding to persistence. Paid licenses are not required
for every disposable test. Do not count an internal command-line editor as the
self-service journey.
