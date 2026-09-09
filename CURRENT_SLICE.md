# AP14 — Profile-driven managed publication and rollback

## Outcome and authority

Turn the installed **Pure LoFi → Efx FRAGMENTS** engineering vertical into the first profile-driven managed publication workflow.

From the existing bridge-owned Arturia environment, one ordinary manager operation must discover or consume a current exact module/class census, select one unambiguous reviewed declarative compatibility profile for each supported class, derive the registration from product-owned observations and installed artifacts, and atomically publish the native Linux VST3 bundles. The operator must not have to author class IDs, module hashes, registration JSON, native build-tree paths, Wine/Proton commands, or per-product compatibility flags.

The same management surface must reconcile interruption and return to an exact prior known-good publication revision without reinstalling vendor software, changing vendor state, changing stable Linux-facing processor/controller identity, or damaging an existing Bitwig project.

[Issue #83](https://github.com/kasselvania/Linux-VST-bridge/issues/83), branch `codex/ap14-profile-publication`, starts from integrated AP13 `main` at `ca0e2f7d5c85515c8ec22b67d434c6288ca611f7`. AP13 [PR #82](https://github.com/kasselvania/Linux-VST-bridge/pull/82) was reviewed at exact implementation head `5620c56660651c0624deb3935c91c91eb0653aa6` and merged. Preserve its protocol-12 state-capture delivery repair, fault/refusal semantics, installed service, exact Arturia environment, stable projects and **512-frame supported/recommended setting**. The 256-frame option remains tested but unqualified.

Implementation, necessary builds, focused tests, reversible bridge-only deployment and proportionate current-status documentation are authorized under [AGENTS.md](AGENTS.md). Preserve dirty work and coordinate control of the operator's open desktop/project. Continue on the prepared branch and one implementation PR against `main`; leave it unmerged for technical review.

## Current R1 repair status

[technical review 5158330660](https://github.com/kasselvania/Linux-VST-bridge/pull/85#pullrequestreview-5158330660) supports the AP14 architecture and exact fixture evidence and selects a narrow claim-lifecycle correction. Continue the same branch and PR #85. Ordinary new preview/publication and current installed-host policy require exactly one `VerifiedExactFixture` profile. Explicit candidate qualification remains nonactivating. Retained revision-2 candidate publications and older rollback targets remain immutable and usable under the existing history laws. Both shipped profiles advance to revision 3 for the exact AP14 fixture only. Preserve every technical constraint, limitation, installed binding and 512 recommendation; 256 stays unqualified. The repaired head requires focused rereview, not a new slice or final merge-acceptance claim. See [docs/AP14.md](docs/AP14.md) for results.

## Starting-state gap addressed by AP14

At the prepared AP14 base, the difficult bridge mechanics existed. The Rust manager already owns exact environment/runner records, module and software digests, supervised inspection, immutable native bundle copying, atomic discovery links, registry reconciliation, installed service startup and independent instance admission. But the current ordinary path still exposes engineering inputs:

- `inspect INSPECTION.json` requires an environment ID, exact module path/hash, class ID and compatibility selection;
- `register REGISTRATION.json` requires a complete externally assembled registration and generated native artifact;
- `Compatibility` currently carries only a directly supplied `disable_windows_accessibility` boolean;
- `Manager::register` correctly refuses a changed binding because an explicit update transaction does not yet exist;
- `compatibility/` contains no accepted or verified profile.

AP14 connects these exact primitives into a product workflow. It is not another installation campaign, transport proof, performance matrix or vendor expansion.

## Product boundary and ownership

The Rust management plane owns:

- the profile schema and parser;
- profile identity, revision and lifecycle/claim state;
- exact matching and ambiguity refusal;
- scanner/inspection orchestration;
- derivation of a local registration from observed facts plus reviewed profile capabilities;
- selection or generation of the exact installed native proxy artifact;
- durable publication/update/rollback transactions;
- canonical status/readback and refusal categories.

Profiles supply reviewed declarative compatibility facts. They do not execute work. The Windows host remains the VST3/vendor-object boundary; the native proxy remains the DAW/VST3 boundary; the transport remains typed mechanism. No product or profile name may dispatch DSP, SDK methods, process code or arbitrary commands.

Machine-local environment IDs, absolute paths, publication locations, process IDs, credentials and license state remain local state. They are not portable profile identity. Friendly names are presentation; exact module digest, class identity, observed SDK metadata, runner/environment revision and profile revision govern selection.

## Closed profile contract

Implement the smallest explicitly versioned, bounded, `deny_unknown_fields`-style schema needed for the two exact Arturia fixtures. Private Rust types and file organization belong to the engineer. The accepted meaning must distinguish at least:

- stable profile ID and immutable profile revision;
- schema version and profile lifecycle/claim state appropriate to this cut;
- exact vendor/product/build/module match conditions;
- exact VST3 class ID and required observed SDK metadata/role conditions;
- compatible runner, environment-family/revision constraints and installed host/proxy requirements;
- reviewed capability selections, including the existing process-scoped Windows-accessibility choice, detached-editor posture, supported performance setting and any AP13 protocol/state behavior that must not be assumed for an unknown plug-in;
- known limitations and non-authoritative evidence references.

Use a closed enum or equivalent for every capability. Unsupported values fail. Avoid generic key/value capability bags, templated commands, expression languages, hooks or an `extra` escape hatch.

Profiles must not contain or trigger:

- shell, PowerShell, Python, JavaScript or another arbitrary executable command;
- credentials, tokens, cookies, serials or license data;
- proprietary binaries, presets, opaque vendor state or patches;
- DRM/authorization bypasses;
- unreviewed remote download URLs;
- machine-local mutable paths as portable identity;
- destructive repair actions or silent vendor/environment mutation.

Bound file size, string size, collection count and nesting sufficiently to make hostile or corrupted local profile data fail before expensive work. Unknown fields, duplicate profile identities/revisions, unsupported schema versions, malformed hashes/IDs and unsupported capabilities fail closed. Repository profile files are data, not permission to redistribute a matched vendor module.

## Discovery, inspection and exact matching

Use existing bridge-owned environment and scanner/inspection results as authority. Do not trust filenames, directories or display strings alone. The normal operation must:

1. verify the installed manager/host/native software and selected environment/runner revision;
2. discover or consume a current exact module/factory/class census through supervised product code;
3. normalize only the observed facts needed for matching and registration;
4. compare every eligible profile deterministically;
5. yield exactly one match per selected class or refuse without mutation;
6. run any deeper class inspection needed to produce the current registration metadata;
7. derive compatibility and supported performance from the profile rather than a user-authored request;
8. bind the result to a durable local receipt.

Explicitly test and categorize zero match, multiple matches, changed module digest, absent class, wrong class role, conflicting SDK metadata, runner/environment/installed-host mismatch, stale census and unsupported capability. A profile matching one class in a multi-class module does not silently publish another class. Discovery facts and profile claims remain distinguishable.

The ordinary operator may choose an environment or a supported product when real ambiguity exists, but must not type identities or duplicated technical facts. A dry-run/preview is useful if it reads from the same planning result that the commit operation validates; it must not become a separate source of truth.

## Product-owned native artifact

The current registration accepts a generated native artifact from an external path. AP14 must close that ordinary-user gap. Determine the smallest product-owned route consistent with the existing build and installation:

- install a verified generic or generated proxy artifact with the immutable software package;
- invoke an existing deterministic product-owned generation/build step outside the DAW and audio path;
- or select another bounded route that produces an exact retained artifact and provenance.

The ordinary user must not supply a source-checkout/build-directory path. Stable processor/controller IDs and the native request identity must remain derived by the established law, not regenerated from a profile filename, display name or publication revision. Do not compile, copy, hash or perform filesystem work in a DAW audio callback.

## Publication revision and transaction law

Do not relax the existing changed-binding refusal or overwrite the current registration in place. Add an explicit revisioned update/rollback transaction around the existing immutable bundle and atomic link posture.

A completed publication revision must durably bind:

- class and stable Linux-facing identities;
- profile ID/revision and claim state;
- exact local module/class observation or census identity;
- environment/runner revision;
- installed Windows host/source identity;
- native proxy artifact identity;
- compatibility and supported performance selection;
- publication target and parent/prior revision;
- transaction/result identity.

The engineer may choose the exact state representation, but every externally visible intermediate state belongs to recovery. The following physical laws are mandatory:

- the prior known-good publication remains active until the complete candidate is validated and commit-ready;
- candidate files and provenance are immutable and exact before pointer activation;
- the active discovery pointer changes atomically;
- durable records identify both prior and candidate targets before an operation can make either externally relevant;
- `reconcile` determines truth from durable records plus physical target/link readback, never an in-memory boolean or guessed intent;
- interruption before activation leaves the prior revision active and may clean or retain an exact candidate safely;
- interruption during/after pointer activation deterministically completes the candidate or restores the recorded prior revision;
- a failed candidate never erases, rewrites or reconstructs the prior target from mutable current files;
- rollback activates an exact retained known-good revision, not a newly derived approximation;
- active DSP instance leases block unsafe publication change or rollback; the environment keeper alone is not a DSP lease;
- foreign files/links are refused and preserved;
- one class can update, unpublish or roll back without changing its healthy sibling;
- vendor modules, environment state, account/license state and user projects are never deleted or rewritten by this transaction.

Use fallible-stage tests around durable writes, candidate publication, pointer exchange, registry commit and cleanup. Merely testing a happy atomic rename is insufficient.

This is bridge profile/registration/native-publication rollback. It is not a general vendor installer/update/backup system.

## Ordinary management surface and readback

Provide one coherent typed CLI workflow suitable for a later UI. Exact command names and private subcommands are implementation choices. The normal operator path may be conceptually:

```text
scan/select supported environment
preview exact supported matches
publish or update selected supported products
status
rollback exact publication revision
```

It must not require the operator to chain internal inspection, hand-edit JSON, locate a native build, then call raw registration.

Canonical `status` or an adjacent readback command must expose, per class:

- friendly name and exact instrument/effect role;
- selected profile ID/revision and lifecycle/claim state;
- local module/build and runner/environment validity;
- active publication revision, physical target and prior rollback target when available;
- supported/recommended and currently selected performance posture;
- editor/accessibility posture and known limitations;
- pending/recovery state and a stable refusal category when action is unsafe.

Structured output is canonical. Human-readable output may summarize it. A future GUI must be able to call the same Rust-owned operation/readback rather than reproduce policy.

## Verification and completion

### Deterministic tests

Cover at least:

- strict profile parsing, schema/version/count/size bounds and unknown/prohibited shapes;
- exact one-match selection and zero/ambiguous matches;
- changed digest, missing/wrong class, role/metadata mismatch, stale census, wrong runner/environment/host and unsupported capability;
- registration derivation without operator-authored class ID, digest, compatibility or native build path;
- stable external identities across profile/publication revision;
- idempotent repeated managed publication;
- prior publication preservation when candidate validation or generation fails;
- injected interruption/failure at every durable transaction boundary followed by deterministic `reconcile`;
- exact rollback and rollback refusal while an instance lease is active;
- foreign publication ownership refusal;
- independent LoFi and FRAGMENTS update/unpublish/rollback;
- vendor files/state untouched;
- status/readback matching the physical link/target and exact durable records.

Reuse current publication and manager fixtures rather than inventing a second registry implementation. Add focused scanner/native-generation tests only where the managed route actually changes those components.

### Exact Steam Deck check

Use the existing lawful AP13 Arturia environment and a protected copy of the existing Bitwig project.

1. Record current installed software, environment, runner, module/class, registration, profile, performance and publication identities.
2. Close bridged devices before publication mutation. Preserve the environment keeper, vendor files, account/license state and original project.
3. Remove or deactivate only bridge-owned discovery publications as required by the test.
4. Run the new normal managed workflow without hand-authored class/hash/registration/build-path inputs.
5. Confirm Bitwig discovers Pure LoFi as an instrument and Efx FRAGMENTS as an effect with the same stable Linux-facing identities.
6. Reopen the saved chain and confirm its edited state/automation and same-instance vendor controls remain available.
7. Present a deliberately incompatible profile/module/runner candidate and prove it cannot replace the working publication.
8. Exercise interruption/reconcile or an equivalent safe injected transaction fault on disposable bridge-owned publication state.
9. Demonstrate exact rollback to the recorded known-good revision.
10. Finish with a bounded 512-frame playback, editor, explicit save/recall and independent FRAGMENTS-removal smoke check.

Reuse AP13 for unchanged audio/state/performance claims. Do not replay its latency matrix or claim the AP14 smoke check is new universal reliability evidence. A user-service restart and normal Bitwig relaunch are proportionate. Perform a full Deck reboot only if startup/persistence code changes or physical readback leaves a genuine unresolved persistence question.

Retain exact commands, source/artifact/profile/transaction identities, failures and claim limits. Do not retain proprietary binaries, opaque state, presets, credentials, license material or sensitive environment exports.

## Scope and non-goals

AP14 does not include:

- another commercial vendor or plug-in build;
- Serum authorization or vendor reinstall;
- speculative Wine/Proton migration;
- promotion or further qualification of the 256-frame option;
- a universal compatibility schema for every VST3 feature;
- a GUI toolkit, consumer installer or general remote profile service;
- general vendor software download/update/repair;
- sidechains, arbitrary buses, float64, broad MIDI/MPE, CLAP, Wayland embedding or another DAW;
- a transport/protocol rewrite or another performance campaign;
- elimination of every remaining startup/preparation/publication timing outlier;
- FRAGMENTS Advanced-panel/redraw work in [#80](https://github.com/kasselvania/Linux-VST-bridge/issues/80);
- removal of the visible native editor open/close intermediary in [#84](https://github.com/kasselvania/Linux-VST-bridge/issues/84).

Issue #84 records the desired later UX: opening a plug-in editor should directly show/focus the vendor window, and closing that vendor window should retire only the editor session without a separate bridge-owned open/close panel. Preserve current editor behavior during AP14 unless a strictly mechanical compatibility-field move is required.

## Completion report

Update the single implementation PR with:

- exact source head and parent;
- implemented profile and transaction contracts;
- changed paths and why;
- focused local/CI/Deck checks actually run;
- retained profile/publication/recovery evidence;
- exact installed state left behind;
- failed attempts and remaining limits;
- explicit statement that 512 remains supported/recommended and 256 remains unqualified.

Leave the PR open, non-draft only when implementation and evidence are ready, and unmerged for independent technical review.