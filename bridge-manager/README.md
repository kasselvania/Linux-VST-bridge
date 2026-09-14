# Registered bridge manager

The Rust manager owns exact environments, runners, installer/application records, registration, publication, capacity, rollback, and canonical readback. The installed Python supervisor owns each launched Windows process tree. Neither runs in an audio callback.

## Installed setup

`setup PACKAGE` installs an immutable private software revision containing the manager, `session.py`, `ownership.py`, the exact Windows host, its source manifest, and the native catalogue. It updates the stable command link and enables `linux-vst-bridge.service`. Updating software requires closed devices and a stopped service.

The typed CLI currently includes:

- `environment-create RUNNER.json` — create a private persistent environment;
- `environment-import ENVIRONMENT.json` — adopt an existing exact bridge-owned environment without changing vendor/account state;
- `install ENV_ID INSTALLER_PATH SHA256` — verify and run one normal foreground vendor installer in a bounded user unit;
- `inspect INSPECTION.json` — inspect one exact module/class in an inactive environment;
- `vendor-editor INSPECTION.json` — open one exact unpublished vendor editor without DAW audio;
- managed preview, publication, status, rollback, reconciliation, and unpublish operations;
- sealed engineering qualification and accepted-transition routes.

Unknown fields, changed artifacts, ambiguous class roles, foreign links, active mutation conflicts, and unresolved ownership are refused.

## Playback and ownership

A native bundle under `~/.vst3/LVB_<class-id>.vst3` points atomically to one immutable managed publication. The proxy requests an exact class/module/environment binding outside the audio callback.

The service validates the registered class and connecting DAW, reserves capacity, creates a private tmpfs session under `/run/user/<uid>/linux-vst-bridge`, and returns a typed startup binding. The supervisor preserves the Windows host's durable `C:\bridge\sessions\...` readiness contract while exposing only the fixed high-frequency transport files as verified views into the RAM-backed session.

One environment keeper owns shared Proton/Wine infrastructure and loads no plug-in. Each DSP instance has its own supervisor, Windows process cohort, transport, state owner, controller, and editor.

## Accepted AP17 capacity

The exact Steam Deck / Bitwig / pinned Arturia fixture supports the following conservative envelope:

- 16 service workers;
- 6 live DSP instances globally;
- Pure LoFi maximum 3;
- Efx FRAGMENTS maximum 4;
- native hard capacity 4 per loaded class image;
- three qualified parallel tracks;
- serial bridged depth three;
- two simultaneous direct editors exercised.

Seven remains unqualified. Eight was excluded for the tested workload. Global, class, native-image, maintenance, and service-worker limits remain distinct.

Startup uses `LVB3`/`LVR3` to return a nonce-correlated exact binding or typed service/global/class/native/maintenance/cleanup refusal before partial ownership. Capacity is retained by durable owner leases and released only after positive native/Windows retirement. Unknown cleanup remains fail-closed.

Revision 10 is the accepted ordinary source generation for AP17. The Deck was left on revision 7 after the final investigation; AP18 may activate revision 10 once through the existing bounded transition. No AP17 campaign replay is required.

The narrow capacity-enumeration/lease-deletion race is tracked in issue #93 as a focused manager follow-up.

## Volatile transport and durable state

High-frequency mappings are private per-session tmpfs files:

- `ap1.control`;
- `ap1.audio`;
- `ap10.delivery`;
- `ap11.ui`;
- `ap12.status`.

They disappear at user-session or machine restart. Environment identity, owner specifications, readiness/gate files, reports, receipts, software, profiles, publications, vendor state, and projects remain durable.

The manager verifies tmpfs type, ownership, permissions, marker, device/inode, and the connecting process's mount namespace. There is no disk-backed fallback. Failed physical retirement cannot produce a successful receipt.

## AP18 installer and vendor-application path

The existing `install` route is a one-shot installer supervisor. It verifies exact bytes, obtains exclusive environment access, launches the installer through the pinned runner, tracks descendants, bounds diagnostics and time, contains owned processes, and records launcher exit plus cleanup.

AP18 extends this only as required by the real Arturia Software Center workflow. ASC is a persistent companion application rather than a one-shot installer. The manager needs a reusable exact application identity, launch/focus/status ownership, continuing helper/download/install classification, bounded before/after manifests, and post-install product discovery.

The intended AP18 flow is:

```text
private exact ASC installer
→ installed ASC application in the existing Arturia environment
→ user-owned Arturia sign-in/activation
→ Pigments download and installation
→ exact VST3/resource discovery
→ immutable Pigments ReviewCandidate
→ reversible Bitwig qualification
```

The operator has placed unopened ASC installer file(s) in the Steam Deck user's Downloads directory. The implementation must hash and privately admit one exact file before execution. Installer/product bytes, account data, authorization payloads, presets, and sensitive vendor logs never enter Git.

The real ASC UI remains the account/catalogue/download surface. MF1 adds the native operator frontend for launch, focus, rescan, capture and canonical status. Manager state remains canonical; no frontend may automate credentials or licensing.

## Performance posture

The accepted products use 512 added frames per proxy. 256 remains available but unqualified. AP16 repaired one journaled-storage stall by moving hot mappings to private tmpfs; it did not claim gap-free or hard-real-time operation. Remaining delivery classes are tracked in issue #90.

## Focused checks

```text
cargo test --manifest-path bridge-manager/Cargo.toml --locked
cargo clippy --manifest-path bridge-manager/Cargo.toml --locked --all-targets -- -D warnings
python3 -m unittest discover -s bridge-manager/runtime -v
```

Run native and Windows lanes only when their owners or shared contracts change. Synthetic tests do not substitute for real ASC, Pigments, and normally launched Bitwig evidence.

AP18 adds `vendor-product scan pigments` for nonpublishing discovery from the
installed ASC environment. The engineering-only `qualify-pigments stage
EXACT_PRODUCT_PACKAGE`, `qualify-pigments publish`, and `qualify-pigments restore`
use a compiled exact candidate; they accept no caller-authored registration or
profile. The sealed package path supplies bytes for verification, never policy.
The candidate remains unverified and cannot use ordinary managed activation.

## Accepted Pigments software

`linux-vst-bridge accept-pigments` is the argument-free, exact-review AP18
software transition. Close devices and stop the service first. It requires the
retained accepted revision-10 candidate inactive, exact prior software and
completed evidence, and unchanged LoFi/FRAGMENTS baseline. It installs immutable
software using the existing command-pointer law; `managed preview` and
`managed publish` then select ordinary Pigments revision 11. The candidate
remains historical provenance, never ordinary Activation authority.

Catalogue schema 2 adds exact supplemental host/manifest artifacts. Each profile
selects its exact host, so Pigments does not replace the Windows host used by
LoFi, FRAGMENTS or the environment keeper. No command accepts an arbitrary host
path, review ID, profile or hash as acceptance authority.

The AP18 acceptance setup uses a single content digest for each supplemental
host/source-manifest directory and refuses executable paths outside the pinned
Windows launch extent before copying software. Before ordinary publication, a
software installation retry is permitted only from the same immutable compiled
AP18 review receipt and exact retained artifact set, with the accepted candidate
still inactive and all original history, baseline and inactivity checks intact.
It does not accept another candidate, review, path or hash. The ordinary revision-11
installation and one audio/editor/retirement smoke are retained under
`evidence/ap18/acceptance/`; candidate history remains nonactivating.

## Private per-instance incident capture (CA1)

`capture arm SELECTION` arms the next exact ordinary admitted instance; `capture
status`, `capture summary ID`, `capture report ID`, `capture export ID`, and
`capture disarm` manage it without restarting devices. Raw reports are private;
only `export` selects sanitized fields. The sealed engineering candidate uses
`capture arm-failure`. See [CA1](../docs/CA1.md) for finite retention, exit
classification, and the distinction between reporting readiness and a proven
crash cause.

## Accepted UIO2 Pigments ordinary transition

`linux-vst-bridge accept-pigments-ui` installs the reviewed candidate-17 artifact
set as ordinary revision-18 software authority. It accepts no profile, review ID,
or artifact arguments. Close devices and stop the service before this immutable
software transition; use `managed preview` and `managed publish` afterward.
The compiled seal requires the exact inactive candidate publication, the current
software generation, ordinary11 parent, and unchanged LoFi/FRAGMENTS10 siblings.

Revision18 preserves candidate17's exact module, native proxy, Windows host,
descriptor, runner, environment, capabilities and limitations. Only revision,
claim and accepted evidence change. Catalogue schema3 retains both exact native
builds for the same class and selects by complete profile identity; schemas1/2
retain their single-image uniqueness law. Ordinary11 remains the publication
parent and rollback, while candidate17 stays immutable reviewed provenance.

The accepted operator resize and continued use are not an automated numeric
geometry waterfall or a universal stability claim. Windows accessibility remains
disabled only for the exact vendor process. Delivery gaps remain; 512 is
recommended and 256 unqualified. This change adds no ASC integration or frontend.

## MF1 native desktop frontend

Launch **Linux Audio Compatibility Manager** from the desktop Applications menu.
Expand the Arturia section to open/focus/stop the registered ASC application, then
rescan after ASC has closed and its operation has retired. The manager resumes
the bridge service and keeper after this exclusive environment work. Unknown
products remain installed/unqualified and cannot be activated by the ordinary UI.

Product cards show current ordinary revision, recommended revision, exact
rollback targets and inactive history. Capture arms the next exact admitted
launch without restarting a running device. Incident display/export reads only
CA1's sanitized projection. Closing the frontend leaves product/session ownership
with the manager. Disabled controls explain active-instance or unresolved-state
refusals; the operation worker revalidates before mutation.

Development build: `sh tools/mf1/build-linux.sh` from the repository root. Add
`linux-audio-compatibility-manager` beside the existing package files for `setup`.
Setup retains prior ordinary rollback images in catalogue schema3 and creates the
native application-menu entry. Operator readback is `operator snapshot` or
`operator activity`; closed requests use schema1 JSON on `operator request` stdin.
No arbitrary command, executable, PID or path operation is exposed.
