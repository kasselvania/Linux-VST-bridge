# Self-service beta delivery

Selected by the operator on 2026-09-29. Source basis: main
`13ed1d85e830d581ec297760e04e9f9433bfd671`.

The operator selected the [audio recovery roadmap](AUDIO_RECOVERY_ROADMAP.md) on
2026-10-02. It now orders this work. Tasks 1–2 retain their bounded delivery
results; tasks 3–5 remain open under the current
[beta acceptance contract](INTEGRATED_BETA_DELIVERY.md#acceptance-method).

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

The operator expanded the assignment on 2026-09-30 to the
[integrated beta delivery effort](INTEGRATED_BETA_DELIVERY.md). Tasks 3–5 are
part of the same delivery outcome, including the implementation needed to
finish them. Tasks 1–2 retain their bounded Ubuntu results. A source test,
package launch or SDK example does not
complete the commercial journey. Preserve partial observations and failures;
check a task only when its stated outcome has actually been demonstrated.

Current progress is retained in the [Ubuntu delivery test](../evidence/self-service-delivery/ubuntu-fragments-trial-2026-09-29.json).
The [internal31 development observations](../evidence/self-service-delivery/ubuntu-internal31-development.json)
retain same-account normal package selection and unchanged FRAGMENTS
registration. Its headless hold ended naturally before GUI Stop was successfully
exercised. That result does not close recovery. The next frontend correction
moves the selected Setup card above acquisition controls and refuses expired
offers while allowing fresh exact controls without capacity readiness. Its 81
frontend tests pass; installed recovery remains open. First-party stateful
instrument/effect installers are being assembled for repeatable delivered tests.
Commercial recall uses the existing licensed Deck installation after safe
staging, as selected by the operator. Internal42 selected exact paired successors
through normal product controls while retaining predecessors. The original
project survived reboot/reopen; a separate updated project copy saved and reopened.
Both editors completed three playback close/reopen cycles, but both plug-ins'
processing lifetimes contain unexplained missing frames. Populated rollback and
the remaining acceptance repetitions are still open. See the
[internal42 receipt](../evidence/self-service-delivery/internal42-populated-recall-and-deadline-failure.json).
Task 1 passed on the declared Ubuntu fixture: normal verified upstream
acquisition, official vendor installation and discovery on the application-owned
-r3 runtime, without preinstalled Wine, Proton, Steam or development tools.
Task 2 passed exact prebuilt preparation and experimental publication through
normal product controls. Three exact proxy entries were independently rebuilt
and compared; the official trial module remains distinct from the Deck bytes.
Neither result qualifies another platform or arbitrary plug-ins.

Internal43 completed the ordinary Ubuntu reference-effect preliminary inspection
without the earlier action-lock refusal, then failed the service-restoration
reply deadline. Its completed inspection and old publication remained retained;
later finalization restored the service. The source successor shares service
startup's exact runtime preparation and rechecks registration/components before
readiness. Delivered recovery acceptance remains open. Internal43 also completed
normal Deck application version selection with unchanged registry and fixture
project digests. That manager update does not qualify audio, recall or rollback.
See the [internal43 receipt](../evidence/self-service-delivery/internal43-recovery-handoff.json).

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
this is not a normal recovery pass. Internal28 includes the shared readback
correction and was selected through normal stop/select/start controls on the
new account, retaining its environment and runtime. Normal discovery found
the exact trial module with no inspection error or quarantine. Its compatibility
controls loaded and normal preparation supplied the exact prebuilt proxy.
Internal28 completed normal experimental publication and 1024-frame selection.
Bitwig guest demo rendered the vendor editor and captured processed stereo
output. Closing the editor reproduced the crash twice with the corrected host;
frame detach did not resolve it. The fault is a null provider dereference in
Wine UI Automation, with the vendor caller unproved. The
[exact preparation policy and isolated Wine correction](RUNTIME_UIA_GUARD.md)
retain separate evidence. The delivered default DLL crashes the null-provider
reference; two isolated corrected builds pass it. Commercial testing of the
new preparation policy had not yet run in that generation. Normal live
installer recovery remains open.
Preparation still took several minutes and is not a responsiveness qualification.

Internal29 selected the exact accessibility policy through managed preparation
and normal replacement, preserving the environment, runtime and predecessor
publication. Its replacement snapshot retained 1024 frames; the old admission
verifier still required 512 and refused before a Windows DSP session existed.
The bounded publication repair now verifies the larger snapshot against exact
prebuilt capacity. Its 37 preparation tests, all-target Clippy and eight CI
checks pass. Internal30 selected the exact successor through normal package
controls and admitted the retained publication. Cold live preview refused
`admission_service_busy`; the ordinary inserted instance then loaded without
manual Reload Plug-in. Its editor completed three normal close/reopen cycles
and instance retirement. A fresh demo instance completed another close/reopen,
captured altered stereo audio afterwards, and retired with host/transport
cleanup confirmed and no editor exception. The earlier demo had reached its
vendor time limit before its audio capture, so that capture is not a processed
audio pass. A full homelab disk also paused the VM during later removal; resuming
the VM after space became available is infrastructure recovery, not a bridge
recovery pass.

The internal30 first-party installer hold test exposed the remaining recovery
gap: the manager returned exact Focus/Stop offers, but frontend controls remained
disabled while capacity status was unavailable. The process completed naturally
with confirmed cleanup; normal GUI Stop was not demonstrated. That observation
has no established failure attribution and is not a vendor-installer or window
focus qualification. Existing-product reinspection/replacement also still
required expert controls. Preparation/startup responsiveness and missing-frame
audio behavior remain unqualified.

State capture remains refused (SDK result 1, zero bytes). [Arturia's demo
policy](https://support.arturia.com/hc/en-us/articles/5671785160732-Demo-versions-What-should-I-know)
disables save/load features; the refusal is consistent with that restriction,
but its exact cause is unproven. Guest demo disables
project save/export; an eligible official Bitwig trial can exercise persistence
without a paid license, but vendor state must also work. No account, paid
activation, fabricated state or licensing bypass was used. Save/reopen,
reboot/recall and ordinary recovery remain task-3
gaps. Tasks 3–5 remain open for their complete stated outcomes.
