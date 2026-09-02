# Current Slice: WF0 — Supervised Windows VST3 Factory Census Probe

## Status

```text
status: active_design_amendment
authority_phase: reconnaissance_and_design
implementation_authorized: false
slice: WF0
target: main
selection_basis_commit: 745ca63bdd8641ade85cb9a024c1dc842681192d
selection_basis_tree: 275521adff574f165bb2b3e883c2ec909bae4c66
v5_amendment_basis_commit: 15523c69567d24b256cb3c65cb6f06bfa07854be
v5_amendment_basis_tree: a6e2fc8c7a564f50e9033094fde847387478de62
v5_design_commit: 6fc3632a04fdba9b8fc20794205e97881c4313f9
v5_design_tree: 0e6b666b28edf6be510d9d29dfa501b5460312bc
v5_design_card: docs/slices/WF0/IMPLEMENTATION_DESIGN_V5.md
v5_design_blob: 74fd8dc5616bf6a5bca45753503947681b35cfaf
v5_review_blob: 8abbbdaf60fc00e00eb0a036fe9463ab4da35984
v5_approval_blob: aabf50ae545a4a4a3d7efddc4abec00d04dfc452
stopped_implementation_basis_commit: 67026ad7160a584cbf9cdb4bf0db7b8dbfa60136
stopped_implementation_basis_tree: 2021dfcbf000d934a462d40efef6bbe7b54e0862
mandatory_stop_result: RETURN_TO_DESIGN_GATE
v6_design_commit: 1e14eb8c318263b7307fe8e07aae13628402e0c1
v6_design_tree: 63f9ce1edfe663848e2a4832c7facad881b83aac
v6_design_card: docs/slices/WF0/IMPLEMENTATION_DESIGN_V6.md
v6_design_blob: e153943a302e3f9b38c74c9048f70a1d65c7b238
v6_design_sha256: c6835b31ac8b2cf2f0fb1dee7634ef16f133387c0bffba8bf6671447175cd737
v6_review_path: docs/slices/WF0/ADVERSARIAL_DESIGN_REVIEW_V6.md
v6_github_review_id: 5084539680
v6_review_result: DESIGN_REPAIR_REQUIRED
design_revision: wf0-design-v7
design_status: proposed_for_adversarial_review
design_card: docs/slices/WF0/IMPLEMENTATION_DESIGN_V7.md
reconciliation: docs/slices/WF0/WINDOWS_BUILD_PLANE_RECONCILIATION.md
implementation_authorized: false
successor_selection_authorized: false
```

WF0 returned to the design gate after the exact V5 MinGW/AGain route exposed a
material build-owner mismatch. V5 remains immutable historical authority for
the stopped attempt; it does not authorize the supported Windows/MSVC build
plane.

The exact immutable V6 design received `DESIGN_REPAIR_REQUIRED` in GitHub
technical-lead review `5084539680`. V7 repairs only its two P1 custody gaps:
mandatory attestation unavailable for the actual private/user repository and
the missing exact implementation-source handoff to the Deck. No implementation
resumes until the exact V7 design receives fresh independent review, separate
operator approval, an approval receipt, authority merge, and exact `main`
commit/tree readback.

## Primary claim

> On the exact accepted Steam Deck fixture, a repository-owned supervised
> Windows x86_64 factory probe, built against the pinned official VST3 SDK and
> executed through the accepted Runtime 4 / Proton 11 lane in a disposable
> WF0-owned scan environment, loads the exact pinned AGain VST3 bundle, obtains
> its plug-in factory, retains deterministic factory metadata and the complete
> expected three-class census, unloads cleanly, and leaves the accepted WR0
> environment and every protected fixture unchanged.

The claim and ceiling are unchanged. V7 repairs only private Actions artifact
custody and exact execution-source custody.

## Valid mandatory stop

The pinned upstream source establishes:

```text
SMTG VSTGUI integration forces VSTGUI_STANDALONE=ON
    -> pinned standalone target selects Win32 sources only under if(MSVC)
    -> MinGW selects no applicable source branch
    -> CMake: No SOURCES given to target: vstgui_standalone
```

Pinned AGain returns when VSTGUI support is disabled and links
`vstgui_support` when enabled. Patching the SDK/VSTGUI, copying upstream source,
inventing a replacement target, or rewriting AGain would change the approved
fixture/build owner. The stop was correct.

The exact installed MinGW posture is:

```text
installed_exact_but_not_selected_for_wf0_again
```

This does not claim that MinGW is generally unusable for every future Windows
utility.

## Preserved provisional source

```text
commit: 5b166f4902af5e1e3b9c2287512f1f8f099c51d0
parent: 67026ad7160a584cbf9cdb4bf0db7b8dbfa60136
tree: fb6a14a78bcab6dd11b5720a725e9df841714743
source/configuration delta: exactly 26 paths
evidence delta: 0 paths
status: salvage material only

archive ref:
  refs/archive/wf0-minwg-provisional-5b166f

bundle:
  <DECK_HOME>/.local/share/linux-vst-bridge/handoffs/
    wf0-minwg-provisional-5b166f.bundle
  SHA-256: c442d429ff20447d616c62c4273bbefaedbe4b28cd4631e3fc5683c4bb2d1143
  git bundle verify: passed
  advertised ref/commit: exact

Mac custody:
  same bundle imported under the same local archive ref
  SHA-256, commit, parent, tree, and path delta: exact
  archive ref pushed: no
```

The future V7 implementation does not merge, rebase, or wholesale cherry-pick
this commit. It begins from exact merged V7 authority and reconstructs each
audited reusable path under the V7 card.

## Selected three-plane correction

```text
MacControlPlane
    -> private GitHub repository, branches, PRs, exact run/artifact-ID API,
       raw wrapper/hash custody, exact source Git bundle,
       SSH transfer, evidence publication

WindowsBuildPlane
    -> GitHub-hosted `windows-2022`
    -> Visual Studio 2022 / v143 / x64
    -> Windows SDK 10.0.19041.0
    -> Visual Studio 17 2022 generator
    -> exact SDK acquisition, two clean builds, PE checks,
       canonical manifest, three-file envelope and one artifact upload

SteamDeckExecutionPlane
    -> exact source-bundle verification, fixed local ref,
       clean detached execution worktree and per-run source manifest
    -> local content-addressed artifact verification/import
    -> Runtime 4 / Proton 11 execution
    -> held-gate, negative and positive proofs
    -> cleanup, sanitized evidence, SSH return
```

The Deck has no normal WF0 GitHub login, API, artifact-download, push, or PR
dependency. The hosted label is not durable build identity; every accepted run
must bind the actual runner image, Windows, Visual Studio, MSVC, Windows SDK,
CMake, generator, workflow, source, and artifact identities.

The exact repository posture is `kasselvania/Linux-VST-bridge`, visibility
`private`, owner type `User`. GitHub artifact attestation, Enterprise Cloud,
repository visibility/ownership change, organizational migration, replacement
signing, release signing, and code signing are not WF0 prerequisites or claims.

## Proposed V7 implementation boundary

```text
tracked paths: 40
implementation/configuration paths: 26
evidence paths: 14
implementation-source schema: linux-vst-bridge-wf0-implementation-source/v1
windows-build schema: linux-vst-bridge-wf0-windows-build/v1
artifact-manifest schema: linux-vst-bridge-wf0-artifact-manifest/v1
Mac artifact-custody schema: linux-vst-bridge-wf0-mac-artifact-custody/v1
source-handoff schema: linux-vst-bridge-wf0-source-handoff/v1
evidence-handoff schema: linux-vst-bridge-wf0-evidence-handoff/v1
proof-matrix rows: 62
blocked results: 29
material operations: 25
owners: 14
```

These are proposed design values, not implementation permission. The exact
paths, schemas, proofs, blockers, handoff contracts, and stop law are owned by
`docs/slices/WF0/IMPLEMENTATION_DESIGN_V7.md`.

## Protected state

The accepted WR0 environment identity
`d3ed38d7ed53e9a5973cbf22fc504f2479dbdd15616fe1bfc1d5e1cd0bf5f4c4`,
Runtime 4 / Proton 11 identity
`2d64df1d36786ca2d0e955c553005423dc2b5bdd714bd0a17872622e33912547`,
SteamOS read-only posture, `.wine`, Steam compatdata, historical SR0/HP0/HP1/
WR0/WR0A source/evidence, current Bitwig `6.1` installation and configuration,
Serum/vendor material, unrelated worktrees, and every path outside the design
envelope are protected.

Bitwig remains unlaunched protected state only:

```text
application commit: 8a048e733e74dda8b897339436153a2d5df952f29d362dfcaca9d8e0d6f6c231
runtime commit: bd44a6230581917d04f89812a4c21090c304d390edb73995af1c2f9fd8abf4e8
user override SHA-256: 1b4a6a6ed688f69dd3c36dcac8db008c5a41ed52170ea3e23dee984b0aae6a1e
system override SHA-256: e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
permission-output SHA-256: c7f5a34dce104cc3d347dcaf89e135cc5ad73b891101d7587f78423858ad5c73
```

Historical Bitwig `6.0.11` evidence stays truthful for its original fixture and
is not promoted to current `6.1` behavioral acceptance.

## Current no-implementation posture

This phase authorizes only the completed source preservation, bounded read-only
failure investigation, supplied V6 review record, V7 design repair, one local
design-repair commit, and draft design PR update through the Mac. It does not
authorize a workflow run, Windows build, another MinGW attempt, SDK/VSTGUI
patch, artifact implementation, Proton/Wine/
scanner/validator/Bitwig/Serum workload, WR0 mutation, V7 approval receipt,
merge, or successor slice.

```text
RETURN_TO_DESIGN_GATE retained
design_status=proposed_for_adversarial_review
implementation_authorized=false
successor_selection_authorized=false
```
