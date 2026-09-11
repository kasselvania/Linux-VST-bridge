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

The real ASC UI remains the account/catalogue/download surface. A minimal manager frontend is added only if a demonstrated launch, focus, lifecycle, rescan, or status problem blocks the workflow. Manager state remains canonical; no frontend may automate credentials or licensing.

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
