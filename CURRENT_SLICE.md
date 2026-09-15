# MF2 — Managed Windows installer onboarding

Base: reviewed UIR2 merge `7364fdb9b6f02eb1bec88cbd43fe81697c5fe9d5`.
Work only on `codex/mf2-managed-installer-onboarding` and its one draft PR.
SV1 / Serum qualification is deferred, not cancelled. Resume it only after MF2
produces an exact managed Xfer installation and inventory.

## Selected outcome

A human operator can use **Linux Audio Compatibility Manager** to add a normal
Windows installer without an agent or terminal:

```text
Add Windows installer
→ choose a local .exe or .msi
→ import exact bytes into manager custody
→ create a new isolated managed environment
→ run the real installer under supervision
→ operate installer/account screens personally
→ finish or stop the owned installer session
→ scan the resulting environment
→ see exact installed modules/classes
→ leave unknown products Installed — unqualified and unpublished
```

The first real acceptance case is the operator's existing Xfer / Serum installer.
This is a generic manager workflow, not a Serum button or an Xfer-specific code
path.

## Product correction

The proposed operator flow is accepted with two important boundaries.

1. **A selected pathname is not manager authority.** The frontend may present a
   native file picker and read the operator-selected file, but the typed operator
   model must not gain a generic path, command, PID or argument escape hatch.
   Installer bytes enter through one fixed manager-owned import capability; the
   manager computes the digest, stores an immutable private copy, and returns an
   installation-source identity. The original pathname is not retained. A bounded
   sanitized basename may be a display hint only.

2. **Installation does not authorize a Bitwig preview.** MF2 ends with exact
   discovery and, where safe, isolated component inspection. It does not infer a
   compatibility profile, generate a native proxy, activate a review candidate or
   publish an unknown class merely because installation succeeded. A later SV1
   resumes from MF2's exact Serum evidence and owns candidate preparation and the
   first Bitwig product check.

For an installer with no prior lineage, MF2 offers only a **new isolated
managed environment**. Arbitrary reuse of the working Arturia environment, a
historical `.wine` prefix, a yabridge prefix or another product's environment is
not an operator choice. A future update/import slice may reuse an environment only
through exact installation lineage and compatibility evidence.

## Existing owners to reuse

Reuse rather than replace:

- the native MF1 frontend and versioned operator projection;
- manager-owned asynchronous operation receipts and systemd units;
- authoritative LVC1 inactivity and cleanup readback;
- operation-bound service/keeper suspension and recovery;
- the existing supervised `--install` runtime route;
- exact pinned `Runner` verification;
- bounded environment scan and factory/class inventory;
- isolated inspection admission and cleanup;
- unknown-product, stale-inventory and quarantine presentation;
- immutable software generations and rollback;
- Luna/Astra GUI custody rules in `AGENTS.md`.

The current CLI `install ENV_ID INSTALLER SHA256` proves underlying installer
supervision exists. MF2 must stop requiring a person or agent to manufacture the
environment, installer path and hash by command.

## Operator model v2

Introduce a new exact manager/frontend generation with a versioned operator model
for installer onboarding. Retain the previous MF1 manager/frontend as an immutable
rollback generation; do not claim that an old frontend can parse the new snapshot.

The v2 read model should add bounded projections for:

- imported installer artifacts;
- onboarding environments;
- installation sessions and durable outcomes;
- scan state and discovered module/class counts;
- exact user-action requirements;
- isolated inspection outcomes where available.

Suggested states include:

```text
installer: importing | imported | refused
onboarding: environment_ready | installer_running | awaiting_user
            install_completed | install_failed | cleanup_unconfirmed
            scan_ready | scanning | installed_unqualified | quarantined
```

Human labels and discovered vendor/product names are projections. Exact artifact,
environment, runner, module and class identities remain authority.

## Installer import capability

Add a native **Add Windows installer** action to the manager frontend.

The frontend may use the simplest reliable native Deck file picker, without a
browser or WebView. The picker should filter for `.exe` and `.msi` for convenience,
but suffix and filename do not establish file type, vendor or trust.

The frontend opens the selected object read-only and no-follow, requires a regular
file, and sends its bytes through one fixed manager import command or inherited
read-only descriptor. Prefer a bounded stream / descriptor capability over a raw
pathname in JSON. The manager must:

- enforce an explicit maximum size and free-space preflight;
- reject directories, devices, FIFOs, sockets, symlinks and changing files;
- identify supported PE executable or MSI/compound-file input by bytes;
- stream to a private temporary file while computing SHA-256 and size;
- fsync, verify and atomically place the immutable artifact under manager custody;
- deduplicate exact bytes safely;
- create a durable import record;
- never persist the original path;
- retain only a sanitized display-name hint when useful;
- remove partial temporary bytes after refusal;
- expose no arbitrary file-write destination.

The durable record must begin with vendor/product unknown. Installer metadata,
filename, signature strings or UI text may later become observations; none grants
vendor identity or compatibility authority.

## Runner and environment creation

The UI presents only installed, manager-verified runner identities. It cannot
supply a runner path or runtime command.

For MF2, the operator chooses **Create new isolated environment** and one offered
runner. The recommended current pinned runner may be preselected. The manager
creates and owns the exact environment root, private HOME/cache/data/runtime
surfaces and operation lock. The operator may supply a bounded human label such as
"Serum test", but that label is not vendor family identity.

The onboarding record binds:

```text
installer artifact identity
runner identity
new environment identity
creation operation
service/keeper prior posture
installation and scan receipts
```

Do not import or mutate the historical `.wine` Serum installation. Do not touch
Arturia's managed environment. Do not make the new environment visible to ordinary
product admission until a later exact product publication selects it.

Environment mutation must remain explicit. This first generic flow permits one
initial installer transaction in the newly created, unpublished environment. A
future installer/update into an environment containing qualified products is out
of scope and must become a revision/rollback transaction rather than an in-place
surprise.

## Supervised installer session

Convert the imported installer and new environment into one manager-owned,
recoverable installation operation.

The operation must:

- require authoritative inactivity and positive prior cleanup;
- reserve one exact onboarding/environment operation;
- bind service suspension/recovery to that operation;
- invoke only the immutable imported artifact under the selected runner;
- use the existing supervisor/process containment and bounded private diagnostics;
- display the vendor's real installer on the Deck desktop;
- let the human operate credentials, license, destination and consent screens;
- retain exact process/cohort and installer outcome facts without secrets;
- support manager **Focus installer** and **Stop installer** actions bound to the
  exact current operation;
- survive manager-frontend close without abandoning or duplicating the installer;
- restore service/keeper only after exact positive retirement;
- retain failed, cancelled and cleanup-unconfirmed outcomes honestly;
- never infer success from exit code alone when owned processes remain.

No Luna or Astra credential entry. For the acceptance case, the operator supplies
all installer interaction personally. A GUI worker may later perform only a closed,
noncredential action capsule.

Do not retain credentials, cookies, serials, authorization files, raw installer
logs or proprietary payloads in public evidence. Private diagnostics remain
bounded and local.

## Scan and result presentation

After positive installer retirement, the manager offers or automatically begins
one exact environment scan using the existing bounded scanner owner.

The scan must:

- inspect declared VST3 roots inside only the new environment;
- retain exact module path inside that environment, size and digest;
- enumerate every factory class with exact class ID, role, vendor and version;
- distinguish factory discovery from component inspection;
- quarantine crash, hang, malformed or incomplete results;
- compare scanner host/source/environment currency;
- retain added, changed, removed and unchanged module locations;
- publish nothing to Bitwig.

The UI should produce honest cards such as:

```text
Serum 2
Xfer Records · Instrument
Installed — unqualified
Exact module and class discovered
Not published to Bitwig

or

Windows installation completed
No supported audio plug-in class discovered

or

Needs attention
Scanner failed during <exact stage>
```

Discovered factory metadata may organize the UI after the scan. It does not
rewrite the original installer record into a trusted vendor identity.

## Optional isolated inspection

To make the beta manager useful without granting publication authority, MF2 may
expose **Inspect unqualified class** after one exact current module/class is
selected.

This action reuses the existing isolated inspection owner and can retain bounded
lifecycle, bus, parameter, state and editor-creation facts. It must:

- use the exact imported environment/module/class and installed host;
- remain a maintenance operation, never DSP admission;
- make no native proxy or DAW publication;
- keep compatibility settings conservative/default unless evidence selects one;
- stop after the first material refusal or terminal failure;
- show Ready for qualification, Needs attention or Quarantined as an engineering
  disposition, not a support claim.

Automatic review-candidate generation and **Try in Bitwig** are explicitly out of
scope. Those belong to resumed SV1, where product-specific facts can select the
profile and reversible candidate safely.

## Frontend experience

Add a clear top-level workflow, not an advanced JSON editor:

```text
Add Windows installer
  1. Select installer
  2. Review exact digest/size and runner
  3. Create isolated environment
  4. Run / focus / stop installer
  5. Scan installed products
  6. Review installed-unqualified results
```

The manager should reopen into the same durable step after frontend restart.
Buttons use canonical disabled reasons. In particular, importing may be allowed
while DSP is active if it is only a bounded read/copy operation, but creating,
running, stopping, scanning or inspecting an environment must revalidate the
appropriate inactivity, operation and cleanup owners inside the manager—not only
through disabled frontend controls.

A file-picker cancellation is not a failed installation and creates no partial
record. Closing the manager does not stop a running installer. A stale token,
changed importer artifact, replaced runner, foreign environment or mismatched
operation must fail closed.

## Generated verification

Add focused source-owned tests for at least:

### Import

- regular `.exe` and MSI/compound-file byte recognition;
- extension/name mismatch does not establish type or vendor;
- symlink, directory, FIFO/device/socket and non-owner refusal;
- file change/truncation during import;
- size and disk-space bounds;
- partial-stream cleanup;
- exact deduplication;
- path and credential privacy;
- no arbitrary path/command/PID field in the typed action model.

### Environment and operation ownership

- only exact installed runner identities are selectable;
- unknown installer defaults to a new isolated environment;
- Arturia and historical `.wine` environments cannot be selected;
- active DSP/maintenance, unresolved cleanup, vendor owner and pending transaction
  refusals;
- service stop/resume remains operation-bound;
- frontend close and reopen retain the operation;
- worker-launch failure and unexpected termination become terminal receipts;
- old cleanup cannot consume a newer onboarding recovery record;
- Focus/Stop target only the exact current installer cohort;
- no duplicate installer launch.

### Scan and disposition

- successful install with one unknown instrument becomes Installed — unqualified;
- multiple classes remain distinct;
- no module, malformed factory, scanner crash and timeout dispositions;
- stale scanner host/source/environment evidence requires rescan;
- unknown products never enter ordinary registry/publication;
- isolated inspection remains maintenance-only and cannot create a candidate;
- current Pigments18, LoFi10 and FRAGMENTS10 readbacks remain byte-identical.

### Frontend

- native file-picker cancellation;
- step recovery after restart;
- large-button/keyboard navigation;
- disabled reasons under active ownership;
- installer progress/focus/stop projection;
- no browser/WebView, shell command editor or competing state database.

## One real acceptance workflow

After generated verification passes, use the operator's existing Xfer / Serum
installer as the first real case.

1. Preserve canonical product, project, software, service and environment state.
2. Install the reviewed MF2 manager/frontend generation immutably.
3. Launch **Linux Audio Compatibility Manager** from Applications.
4. The operator clicks **Add Windows installer** and selects the existing Serum
   installer through the native picker.
5. Confirm the manager record contains exact digest/size but no trusted Xfer or
   Serum identity derived from the filename/path.
6. Create one new isolated environment under the current pinned runner.
7. Start the real installer from the manager.
8. The operator personally handles every installer, account, license and consent
   screen. The implementation agent sends no keyboard or mouse input.
9. Confirm the frontend can close/reopen, focus the exact installer and show the
   durable operation without duplicating it.
10. Complete or explicitly stop the installer once. No blind retry campaign.
11. Require positive cohort cleanup and service/keeper restoration.
12. Scan the new environment once.
13. Confirm the current installed module/classes appear as exact
    **Installed — unqualified** results and remain unpublished.
14. Optionally run one isolated class inspection if installation and scanner
    evidence are complete. Do not create a Bitwig candidate.

If the installer or authorization path exposes a material new blocker, stop at
that exact boundary and preserve the managed environment/record for review. Do not
fall back to terminal creation or the historical `.wine` installation.

The successful new Xfer environment may remain installed and managed for resumed
SV1. It must not affect ordinary product admission until SV1 produces and reviews
a candidate.

## Preservation and scope

Preserve throughout:

- Pigments ordinary18 and rollback11;
- LoFi and FRAGMENTS ordinary10;
- every inactive candidate and publication history;
- Arturia environment, ASC and authorization;
- protected projects;
- CA1/IF1/IF2/UIO/UIR capabilities;
- 512 recommended, 256 unqualified.

Out of scope:

- Serum Bitwig publication or audio/editor qualification;
- automatic compatibility-profile synthesis;
- arbitrary existing-environment selection;
- environment update/rollback after qualified products exist;
- silent installers or vendor-specific command switches;
- automatic credential or authorization entry;
- downloading or purchasing software;
- MSI feature customization beyond normal vendor UI;
- universal installer compatibility;
- Wine/Proton replacement;
- audio, performance, buffer or long-run campaigns.

Run manager/frontend/runtime tests, strict Clippy, AP12 and PX2. Run AP8 when the
Windows scanner/installer fixture or host inputs change. AP10 is required only for
genuine native/audio source changes, which are not expected.

## Handoff

Return the same draft PR with:

- exact final source/head/tree;
- importer capability and privacy design;
- operator-model schema and action vocabulary;
- installation/environment record schemas;
- durable operation and recovery ownership;
- generated import/operation/scan/frontend results;
- exact imported Xfer installer digest and managed environment identity;
- real installer outcome and human-only interaction boundary;
- exact discovered modules/classes and dispositions;
- isolated inspection outcome if performed;
- validation and CI;
- final product/service/lease/transaction state;
- explicit limitations and nonclaims.

Keep the PR open and unmerged for independent review. Do not ordinary-publish or
prepare a Serum candidate in MF2.

## Implementation progress

MF2 adds operator model 2, inherited-file installer ingress, unpublished initial
installation records, operation-bound supervised installer units, native picker
and durable wizard, and existing-scanner reuse for onboarding environments.
See docs/MF2.md and evidence/mf2/result.json. Generated checks and AP12/PX2 passed;
the manager/frontend is immutably installed. Human file import passed. The
Create isolated environment action failed during validation on registry-lock
contention; no environment or installer launched. Retain this material boundary
and production-worker regression. No repeat occurred; onboarding remains incomplete.


## MF2-R1 — bounded operator lock coordination

Continue the same branch/PR from `62f8756df21ebedd5f5a2fa963112f773dd255f9`.
The historical `evidence/mf2/result.json` and imported installer remain immutable.
Repair only non-RT operator validation coordination and structured failure
presentation. `Manager::lock` admission/mutation behavior remains fail-fast.
See `docs/MF2-R1.md` for the lock order, generated regression and review boundary.

Independent source review is required before immutable installation and one
human resumed environment-create action. Until then no installation, file picker,
installer reimport, environment creation, vendor launch or product test is allowed.
PR108 remains draft and unmerged. No Serum candidate/publication belongs to MF2.

### MF2-R1 independent-review correction (44f251f)

Current cut: split registry capture from expensive projection with guarded token/
owner recheck; use one bounded, operation-bound environment-creation guard; retain
exact lock purposes. Preserve prior R1 custody and historical MF2 result bytes.
See `docs/MF2-R1.md`. Generated source verification and AP12/PX2 only. No
installation, reimport, real environment creation or vendor launch before
independent source rereview. Keep PR #108 draft/open/unmerged.
