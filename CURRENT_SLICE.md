# AP18 — Arturia Software Center to Pigments vendor pathway

## Authority and basis

AP18 is the active implementation slice.

- Integrated AP17 main: `2329706a6e797137e68d719edbbbe5cc1e0cdbf1`
- Active issue: [#94 — AP18: Arturia Software Center to Pigments vendor pathway](https://github.com/kasselvania/Linux-VST-bridge/issues/94)
- Branch: `codex/ap18-asc-pigments-vertical`
- Product setting: 512 added frames per proxy, selected/supported/recommended
- 256-frame setting: available but unqualified and outside AP18

Continue in one implementation PR against `main`. Work to the product outcome under [AGENTS.md](AGENTS.md); do not recreate the long AP17 campaign or add process ceremony that does not change the result.

AP17 is accepted and integrated. Its exact Steam Deck / Bitwig / Arturia envelope is six live DSP instances globally, at most three Pure LoFi and four Efx FRAGMENTS, four per loaded native class image, three qualified parallel tracks, serial bridged depth three, and two simultaneous direct editors exercised. Seven remains unqualified and eight was excluded for the tested workload. See [docs/AP17_CLOSURE.md](docs/AP17_CLOSURE.md).

Revision 10 is the accepted ordinary AP17 profile generation in source. The Deck was deliberately left on exact revision 7 after the final investigation. Activate revision 10 once through the existing bounded transition and verify physical readback before ASC work; this is installation housekeeping, not authority to replay AP17.

The repository remains publicly readable but proprietary. Preserve [COPYRIGHT.md](COPYRIGHT.md), [CONTRIBUTING.md](CONTRIBUTING.md), and the all-rights-reserved README notice.

## Private installer handoff

The operator has already downloaded one or more Windows Arturia Software Center installer files into the Steam Deck user's `$HOME/Downloads` directory. They have not been opened or executed.

Before first execution:

- identify the plausible regular files without scanning unrelated Downloads content;
- retain original filename, size, SHA-256, file type, and available Authenticode publisher/signature facts;
- let the operator resolve ambiguity if multiple distinct plausible installers remain;
- copy the selected exact bytes into a private manager-owned, content-addressed artifact location;
- leave the original file unchanged;
- never commit, upload, extract into the repository, or publish the installer bytes.

The operator states that the installer came from Arturia's official site. The exact selected bytes and later installed readback—not a remembered website version—are authority for this fixture.

## Product outcome

Complete one official vendor-managed acquisition-to-use path:

```text
exact ASC installer
→ ASC installed in the existing managed Arturia environment
→ user signs in through the real Arturia UI
→ Pigments is selected, downloaded, activated and installed through ASC
→ the manager discovers the exact installed module, classes and resource roots
→ one immutable Pigments ReviewCandidate and native proxy are produced
→ Pigments is published reversibly to Bitwig
→ its editor, preset, notes, automation, state, save/reopen and cleanup work
```

The goal is not merely to make `Arturia Software Center.exe` launch. The slice closes the gap from the official vendor application to a usable, exactly identified Pigments device in Bitwig.

## Reuse before invention

The repository already provides:

- exact persistent environments and pinned runners;
- `install ENV_ID INSTALLER_PATH SHA256`, which verifies and supervises one normal foreground installer;
- exclusive environment operation locking;
- supervised process ownership and cleanup;
- exact Windows VST3 module/class inspection;
- profile construction, native proxy generation, publication, rollback and readback;
- standalone vendor-editor access;
- the accepted AP17 capacity and failure laws.

Extend those owners only where the real ASC workflow demonstrates a missing capability. Do not start by designing a general package manager, vendor abstraction hierarchy, or full GUI framework.

## Account and authorization boundary

Arturia owns authentication and licensing; the operator owns the account interaction.

- The operator enters credentials directly in ASC.
- Pause automation at sign-in, license selection, purchase, consent, or other secret-bearing steps.
- Do not inspect, capture, log, store, transmit, or commit account email, password, cookies, tokens, serials, unlock codes, license payloads, or activation files.
- Do not intercept TLS, scrape undocumented Arturia APIs, replay vendor network calls, patch licensing, or manufacture authorization.
- Do not clone or run a copied authorized environment as another device.
- Do not update Pure LoFi or Efx FRAGMENTS merely because ASC offers an update.

The bridge may report non-secret posture such as `signed_in`, `activation_required`, `installed`, `update_available`, or `unknown` only when that state is directly observable without retaining secret material.

## Work sequence

### 1. Establish the safe baseline

With Bitwig closed, require zero DSP leases, no pending publication transaction, no stale tmpfs session, and a healthy service/keeper. Record exact software, environment, runner, profiles, publications, modules, resource roots, authorization posture, free space, and protected-project hashes.

Activate ordinary revision 10 through the accepted AP17 transition and verify it physically. Do not rerun capacity, reboot, or failure matrices.

Create a private non-running rollback snapshot or an equivalent exact backup of every environment location ASC may mutate. Do not launch a cloned authorized prefix. Stop before installation if safe rollback and sufficient disk headroom cannot be established.

### 2. Install and register ASC

Run only the exact admitted installer through the managed Arturia environment. Retain a bounded before/after transaction covering allowlisted filesystem and safe registry roots, process ownership, exit/cleanup, installed application identity, helpers/services/startup entries, and discovered download/resource/VST locations.

Do not infer successful installation from exit code alone. Confirm the installed ASC application exists, has an exact identity, and can launch again in the same environment.

### 3. Give ASC a reusable product-owned lifecycle

The manager must be able to launch or focus the exact registered ASC application without accepting an arbitrary executable path. It should report installed/running/needs-attention/completed/failed state, distinguish the visible launcher from continuing download/install helpers, preserve required vendor infrastructure, and clean up only owned operation processes.

The current one-hour installer owner is not automatically appropriate for a long-running software center or a large product download. Change its lifecycle only in response to actual ASC behavior.

Attempt the real ASC UI first. Build a minimal manager frontend only if a demonstrated focus, navigation, lifetime, or integration problem prevents the user from completing the flow. Any such surface remains a thin view over Rust manager truth: launch/focus, current operation, detected products, rescan, Pigments status, and errors. It must not reproduce Arturia authentication, licensing, catalogue, or download APIs.

### 4. Install Pigments through ASC

Let the operator sign in. Select only Pigments, use the normal licensed or vendor-provided demo pathway, and allow ASC to download and install it. Do not update unrelated Arturia products.

Prefer stable Windows `C:` locations inside the existing managed environment. Observe rather than assume the actual VST3, standalone application, shared Resources, presets/samples, app-data, program-data, preferences, databases, caches, and authorization locations. Avoid arbitrary `Z:` or external Linux paths unless real evidence requires them and rollback remains clear.

Close and relaunch ASC once after installation to confirm its own installed-product state persists.

### 5. Discover Pigments automatically

The manager must derive the candidate from observed installation facts:

```text
allowlisted installation delta
→ exact new/changed VST3 module candidate
→ supervised factory/class census
→ exact Pigments class and metadata
→ exact runner/environment/module/host/native identities
→ immutable ReviewCandidate profile
```

The operator must not transcribe hashes or class IDs. Product selection may use exact Arturia factory/class metadata after the module is identified; filename or display-name substring alone is not authority.

Build the native proxy through the existing source-owned descriptor route. Derive stable Linux processor/controller IDs from the exact VST3 class identity. Begin with a conservative Pigments class limit of one unless the actual work justifies more. Extend AP17's capacity policy explicitly rather than treating all Arturia products as identical.

Preserve LoFi and FRAGMENTS revision 10 unchanged. Ordinary activation remains verified-only. Exercise Pigments through one exact sealed AP18 engineering publication with revision 10 as the stable existing-product baseline.

### 6. Qualify the real Pigments result

At 48 kHz, float32, actual host maximum 512 and 512 added bridge frames:

- load one Pigments candidate in normally launched Bitwig;
- open and navigate the real vendor editor, including ordinary resize;
- load one factory preset from Pigments' own browser;
- play notes and retain nonzero local output;
- move one real control and verify Begin/Value/End plus Bitwig automation;
- replay the automation and confirm UI/sound follow;
- close and reopen the editor on the same DSP instance;
- save a protected project, quit normally, relaunch through Applications and reopen it;
- confirm intended preset/visible state/opaque state behavior and audio;
- exercise Pigments alongside one existing Arturia sibling;
- remove Pigments without disturbing the sibling;
- quit normally and require positive ownership retirement.

Retain all gaps and failures truthfully. Do not turn this into exhaustive Pigments, preset, MPE, multibus, rendering, or performance qualification.

## Existing-product preservation

ASC may modify shared Arturia resources or offer updates. Verify proportionately that Pure LoFi and Efx FRAGMENTS retain their accepted exact modules/profiles, direct editors, audio/automation, saved-project behavior, and cleanup.

If ASC changes an existing module, resource tree, or authorization state unexpectedly, do not silently bless or overwrite history. Retain the change privately, restore the safe baseline when necessary, and classify the update as a distinct exact transition.

## Evidence and implementation discipline

- Rust owns installer/application/product records, environment revisions, discovery, profile, publication, rollback, and canonical readback.
- C++ remains limited to VST3/Win32 SDK boundaries.
- Nothing involving installation, hashing, process census, files, registry, logging, or UI enters an audio callback.
- Public evidence uses stable aliases and excludes private absolute paths, raw local process/session IDs, account data, authorization material, installer/product bytes, presets, opaque state payloads, and sensitive vendor logs.
- Private diagnostics may retain what is necessary for recovery and debugging outside Git.
- Reuse accepted AP15–AP17 evidence when its owners do not change. Test changed owners and the new Pigments path; do not replay unrelated historical campaigns.

## Completion threshold

AP18 is complete when the single implementation PR demonstrates:

1. exact private ASC installer admission and provenance;
2. reversible installation into the managed Arturia environment;
3. reusable product-owned ASC launch/status ownership;
4. user-completed real sign-in/activation without credential capture;
5. Pigments installed through ASC;
6. exact installed application/module/resource/preset roots discovered;
7. automatic Pigments class census and immutable candidate construction;
8. exact native proxy build and reversible publication;
9. real Bitwig preset, notes/audio, automation, editor reopen, save/recall, sibling, removal, and cleanup behavior;
10. existing LoFi/FRAGMENTS preserved or any vendor update classified honestly;
11. exact final environment/software/profile/publication state and rollback;
12. no active DSP lease, pending transaction, orphan vendor operation, stale tmpfs session, debugger, or heavy trace at handoff.

512 remains selected/supported/recommended. 256 remains unqualified.

## Explicit non-goals

AP18 does not include Serum, another vendor, broad support for every software center, automatic purchasing/account creation, credential/license automation, updating all Arturia products, exhaustive Pigments/MPE/multibus/offline-render coverage, 256 qualification, residual delivery issue #90, lease-race issue #93 unless it directly blocks this operation, process pooling, another DAW, Wayland-native embedding, CLAP, consumer release packaging, or a speculative full manager UI.

## Handoff

Update one AP18 implementation PR with exact source and artifact identities, installer/ASC lifecycle, user-owned authorization boundary, installation changes, Pigments discovery/profile/publication, actual Bitwig result, preservation/rollback, focused tests, final physical state, and explicit nonclaims. Leave it open and unmerged for independent technical review.

### Current checkpoint

ASC 2.12.0.3157 was installed through the managed environment. Three registered
launches reached operator-owned sign-in but no subsequent library window; all
ended with launcher exit 5 and positive cleanup. Matching the vendor shortcut's
working directory did not resolve the failure. The failure owner remains
unknown, and further login retries are stopped pending a focused diagnostic
handoff. See [the bounded record](evidence/ap18/asc-login-blocker.json) and
[AP18's post-login blocker](docs/AP18.md#post-login-blocker-bounded-stopping-point).
Pigments and the end-to-end vertical remain incomplete. Ordinary revision-10
publications are unchanged and valid; PR #95 remains draft, open and unmerged.
