# Current Slice: WF0 — Bitwig 6.1 Protected-Fixture Amendment

## Status

```text
status: active_design_amendment
authority_phase: reconnaissance_and_design
implementation_authorized: false
slice: WF0
target: main
amendment_basis_commit: 15523c69567d24b256cb3c65cb6f06bfa07854be
amendment_basis_tree: a6e2fc8c7a564f50e9033094fde847387478de62
amendment_branch: codex/wf0-bitwig-6-1-fixture-amendment
amendment_authorized: true
amendment_authorization_text: I authorize an ammendment.
repair_basis_commit: 62cbf973f2e107f91191fc0dee90188d73b50423
repair_basis_tree: 8142bc714ac7b757fe17444d7c76c1242afec1ce
design_revision: wf0-design-v5
design_status: proposed_for_adversarial_review
design_card: docs/slices/WF0/IMPLEMENTATION_DESIGN_V5.md
design_card_git_blob: pending_external_identity_from_design_commit
design_card_sha256: pending_external_identity_from_design_commit
adversarial_review: pending_fresh_independent_v5_review
design_approval: pending_exact_operator_approval
implementation_branch: codex/wf0-windows-vst3-factory-census
implementation_branch_state: paused_at_superseded_v2_basis
successor_selection_authorized: false
```

The operator authorized the narrow amendment proposed immediately after live
WF0 preflight found that the Deck had intentionally moved from the former
Bitwig `6.0.11` beta fixture to Bitwig `6.1` on the stable Flatpak branch.
That authorization returns WF0 to the design gate. It authorizes bounded
read-only reconciliation, a complete successor design, fresh independent adversarial
review, and preparation of an exact approval request.

It does **not** approve the draft, reauthorize implementation, authorize a
toolchain installation, permit a build or Windows workload, or authorize a
merge. Implementation may resume only after the exact V5 card is independently
reviewed, explicitly approved by the operator, retained in a new approval
receipt, merged to `main`, and read back into this authority card.

## Amendment trigger

The implementation preflight reached the protected-fixture gate without a
protected-fixture or implementation-source mutation and returned:

```text
WF0_PROTECTED_FIXTURE_DRIFT
```

The approved V2-era protected snapshot expected the historical Bitwig
`6.0.11` application identity. Live readback found the intentional stable
update below:

```text
application ref: app/com.bitwig.BitwigStudio/x86_64/stable
version: 6.1
application commit: 8a048e733e74dda8b897339436153a2d5df952f29d362dfcaca9d8e0d6f6c231
scope: system
origin: flathub
runtime ref: org.freedesktop.Platform/x86_64/25.08
runtime commit: bd44a6230581917d04f89812a4c21090c304d390edb73995af1c2f9fd8abf4e8
user shadow: absent
user override SHA-256: 1b4a6a6ed688f69dd3c36dcac8db008c5a41ed52170ea3e23dee984b0aae6a1e
system override SHA-256: e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
```

The application version and application commit changed. The runtime identity,
override-byte identities, installation scope, Steam Deck identity, SteamOS
read-only posture, Runtime/Proton digest, accepted WR0 environment, VST3 SDK,
and implementation basis remained exact. The reconciliation record is
[`docs/slices/WF0/BITWIG_6_1_FIXTURE_RECONCILIATION.md`](docs/slices/WF0/BITWIG_6_1_FIXTURE_RECONCILIATION.md).

## Prior approved authority

The following V2 artifacts remain immutable historical authority:

```text
design:
  path: docs/slices/WF0/IMPLEMENTATION_DESIGN.md
  revision: wf0-design-v2
  Git blob: d618cbf6b397f10947d50fd4824cd3e06ef55726
  SHA-256: f323e2b429c4f91c1821488d980b249901b8d82c3cceaa7bf9be9d874a455e24

review:
  path: docs/slices/WF0/ADVERSARIAL_DESIGN_REVIEW.md
  Git blob: 7ee98c4c0fb0d28b11d43d33b46e2ab5fa57c299
  GitHub review: 5082923871
  result: DESIGN_CLEAR

approval:
  path: docs/slices/WF0/DESIGN_APPROVAL.md
  Git blob: a84df2044b8a78b44b7c004b53b620d474994221

design-authority merge:
  PR: #16
  commit: df102033292e815e26103e08929c1183fb1c260a
  tree: f1ed5493b51518705c759c1b8f7dab7407414038

exact former implementation basis:
  commit: 15523c69567d24b256cb3c65cb6f06bfa07854be
  tree: a6e2fc8c7a564f50e9033094fde847387478de62
```

The live fixture change does not erase or rewrite those facts. It means the V2
approval no longer authorizes implementation against the current fixture.

## V3/V4 reviews and bounded V5 repair

The first complete amendment card was retained immutably as:

```text
V3 design:
  commit: 158e1229d1e3b6e84fbb05fc472ebf7f75887965
  tree: 2c312d069cc54cefb3b382dc87638d3fa1dc82fa
  path: docs/slices/WF0/IMPLEMENTATION_DESIGN_V3.md
  Git blob: 523d81f4b449ff96251dd1e52abb741b20033f95
  SHA-256: 948fbc249e399de82db8f7cd50f46a357e32be67ccdb7598b4d69183b921fc78

V3 independent review:
  commit: b6a697d71ea1cf81974199cdd35d4b7dcede6d45
  tree: 0182c76f39567147da1c6767e3f0e2ac8eafc7e1
  path: docs/slices/WF0/ADVERSARIAL_DESIGN_REVIEW_V3.md
  Git blob: 62c0b953ed6c9eda7724de53f7dd87f9a55d613f
  SHA-256: 08284b2f8f5f7b11b15dbf8eec3b30494e64539e0f5bc756b7a6e5c71f387a36
  result: DESIGN_REPAIR_REQUIRED
  unresolved findings: 1

V4 design:
  commit: 566625445c1b3dca3a253da54756513b0c5862e3
  tree: 9e03cc47caf044237310a9ffd155c4d908142894
  path: docs/slices/WF0/IMPLEMENTATION_DESIGN_V4.md
  Git blob: 6701ddf068dc1853ad746b15df413fc12fe095fd
  SHA-256: 138f545ff28f9d1f4f76e049d4d0fa30186bfcebae627f18140b0be0eb55ad09

V4 independent review:
  commit: 62cbf973f2e107f91191fc0dee90188d73b50423
  tree: 8142bc714ac7b757fe17444d7c76c1242afec1ce
  path: docs/slices/WF0/ADVERSARIAL_DESIGN_REVIEW_V4.md
  Git blob: cdb3bdbc743586dd8fba9b693330a28a177bdac0
  SHA-256: 3404119a3076070216506004e4ab89c51d23fef59285d519626dda29d023bd73
  result: DESIGN_REPAIR_REQUIRED
  unresolved findings: 1
```

The V3 reviewer found that V3 correctly separated current Bitwig `6.1` from the
historical `6.0.11` proof, but did not unambiguously restate both binding V2
implementation clarifications in the successor approval contract. V4 restated
both, but its independent reviewer found one internal ordering conflict: the
transition law published `factory_export_found` before optional entry while
the retained 23-state lifecycle published the entry result first.

V5 repairs only that conflict while preserving the binding laws:

1. every ordinary return from all 15 closed call operations must be followed
   immediately by the paired synchronously flushed `call_completed`, before
   any later lifecycle event or call attempt; and
2. required `GetPluginFactory` must be resolved and checked before optional
   `InitDll` may be invoked, and a missing required export must not execute
   `InitDll`; and
3. on a present export, successful resolution/check is retained internally,
   optional entry handling completes, the entry result is published, then
   `factory_export_found` is published, and only an entry-absent or
   entry-succeeded branch may attempt the already-resolved export.

The future V5 approval receipt and implementation handoff must bind both
inherited clauses and the resolution/check-versus-publication distinction. The
repair creates no owner, state, call operation, implementation path, proof row,
blocker, mutation, or reconnaissance requirement.

## Amendment scope

The fixture amendment may change only the protected-fixture interpretation
needed to distinguish:

1. immutable HP0, HP1, WR0, and WR0A evidence that truthfully records the
   historical Bitwig `6.0.11` fixture; and
2. the current Bitwig `6.1` stable installation, runtime, and override identities
   that WF0 must preserve without launching or modifying Bitwig.

The V5 review repair may only distinguish internal required-export
resolution/check from later lifecycle publication without changing either
already-binding V2 clarification.

V5 must assign current Bitwig readback to the existing
`ProtectedFixtureSnapshot` owner inside the already approved WF0 tooling paths.
It must not edit historical tools or evidence merely to make their old fixture
identity appear current. It must not promote HP1's `6.0.11` discovery and
instance-admission proof to a `6.1` claim.

The amendment and bounded V5 repair do not change the WF0 primary claim, AGain fixture,
Runtime/Proton route, ten-owner model, state machine, 40-path implementation
envelope, 26-path implementation-source roster, 14-file evidence packet,
35-row proof matrix, 23-result blocker taxonomy, external mutation envelope,
or explicit nonclaims.

## Primary claim

> On the exact accepted Steam Deck fixture, a repository-owned supervised
> Windows x86_64 factory probe, built against the pinned official VST3 SDK and
> executed through the accepted Runtime 4 / Proton 11 lane in a disposable
> WF0-owned scan environment, loads the exact pinned AGain VST3 bundle, obtains
> its plug-in factory, retains deterministic factory metadata and the complete
> expected three-class census, unloads cleanly, and leaves the accepted WR0
> environment and every protected fixture unchanged.

The claim remains unproved. Bitwig is protected state only and is not an
execution fixture for WF0.

## Current protected fixture

```text
host:
  Steam Deck Galileo
  SteamOS 3.8.16
  x86_64
  SteamOS read-only

current protected Bitwig installation:
  Bitwig 6.1
  app/com.bitwig.BitwigStudio/x86_64/stable
  commit 8a048e733e74dda8b897339436153a2d5df952f29d362dfcaca9d8e0d6f6c231
  system scope / flathub
  no user shadow

Bitwig runtime:
  org.freedesktop.Platform/x86_64/25.08
  commit bd44a6230581917d04f89812a4c21090c304d390edb73995af1c2f9fd8abf4e8

Bitwig configuration projections:
  user override SHA-256 1b4a6a6ed688f69dd3c36dcac8db008c5a41ed52170ea3e23dee984b0aae6a1e
  system override SHA-256 e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
  permission-output SHA-256 c7f5a34dce104cc3d347dcaf89e135cc5ad73b891101d7587f78423858ad5c73

runner/runtime digest:
  2d64df1d36786ca2d0e955c553005423dc2b5bdd714bd0a17872622e33912547

accepted WR0 environment:
  d3ed38d7ed53e9a5973cbf22fc504f2479dbdd15616fe1bfc1d5e1cd0bf5f4c4
  posture: protected_read_only

VST3 SDK root:
  3cdf9ca5d1f5b1b21e0a86832aa4abe55607bd96

positive WF0 fixture:
  official pinned AGain Windows VST3
```

Historical HP0/HP1/WR0 evidence remains protected byte-for-byte and retains
its original `6.0.11` fixture statements. No current Bitwig behavior is inferred
from those records.

## Phase restrictions

During this amendment phase:

- no implementation-source or evidence path may be edited;
- no MinGW extension may be installed;
- no build, validator, Bitwig, Wine, Proton, Runtime, scanner, or fixture
  workload may be launched;
- no WF0 scan environment may be created;
- no accepted WR0, HP0, HP1, Bitwig, Flatpak, Serum, `.wine`, Steam compatdata,
  vendor, or user state may be changed;
- the paused implementation branch may not be advanced, rebased, or used for
  evidence;
- no design approval or merge may be inferred from the amendment authorization.

## Required gate sequence

```text
bounded fixture reconciliation
    -> retain V3 DESIGN_REPAIR_REQUIRED review
    -> complete V4 design repair
    -> retain V4 DESIGN_REPAIR_REQUIRED review
    -> complete V5 design repair
    -> fresh independent V5 adversarial review
    -> repair and re-review again if required
    -> exact operator approval
    -> new approval receipt
    -> merge and exact main readback
    -> separately reissue the WF0 implementation handoff
```

No successor slice or adjacent feature is authorized.
