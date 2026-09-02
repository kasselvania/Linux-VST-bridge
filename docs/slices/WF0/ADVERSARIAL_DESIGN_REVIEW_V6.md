# WF0 V6 Adversarial Design Review

## Review identity

```yaml
slice: WF0
review_kind: supplied_github_technical_lead_review
reviewed_design_commit: 1e14eb8c318263b7307fe8e07aae13628402e0c1
reviewed_design_tree: 63f9ce1edfe663848e2a4832c7facad881b83aac
reviewed_design_path: docs/slices/WF0/IMPLEMENTATION_DESIGN_V6.md
reviewed_design_blob: e153943a302e3f9b38c74c9048f70a1d65c7b238
reviewed_design_sha256: c6835b31ac8b2cf2f0fb1dee7634ef16f133387c0bffba8bf6671447175cd737
reviewed_revision: wf0-design-v6
github_review_id: 5084539680
review_result: DESIGN_REPAIR_REQUIRED
finding_count: 2
finding_severity: P1
additional_live_reconnaissance_required: false
implementation_authorized: false
```

This file materializes the supplied GitHub technical-lead review. The
design-authoring context did not independently review itself. The review found
exactly two P1 custody defects and found no other P0, P1, or P2 design issue.
The sound V6 scanner, process, factory, census, fixture, runner, claim, and
provisional-source decisions remain review inputs to the repair rather than
subjects of a new design.

## P1 finding 1 — Mandatory GitHub artifact attestation is unavailable for the actual repository posture

### Exact finding

```yaml
repository:
  full_name: kasselvania/Linux-VST-bridge
  visibility: private
  owner_type: User
v6_assumption: public_repository_with_mandatory_github_artifact_attestation
result: DESIGN_REPAIR_REQUIRED
additional_live_reconnaissance_required: false
```

V6 incorrectly describes the repository as public and requires GitHub artifact
attestation generation and Mac verification. GitHub currently limits artifact
attestations for private or internal repositories to GitHub Enterprise Cloud.
Repository visibility, ownership, organization membership, and account-plan
changes are outside WF0 authority.

**Violated invariant:** A compatibility proof may depend only on capabilities
available within the exact accepted fixture and authority envelope. Repository
visibility, owner type, and service plan are distinct facts. WF0 may not repair
an unavailable infrastructure capability by changing repository ownership,
visibility, or account posture, and it may not claim provenance that the
selected posture cannot establish.

**Concrete blocked or false-acceptance state:** The otherwise valid Windows
build would deterministically stop at mandatory attestation because the exact
private, user-owned repository lacks the required service posture. Conversely,
omitting or failing open around that step would let V6 claim cryptographic
provenance without the required attestation. Both outcomes are false ownership:
one blocks on an unauthorized prerequisite and the other accepts an unproved
claim.

**Exact required repair:** Remove `actions/attest`, the attestation job and
permissions, attestation generation/download/verification, public-repository
Sigstore language, the attestation-specific blocker, and every proof requiring
cryptographic provenance. Do not change repository visibility or ownership and
do not add a signing system. Replace the dependency with exact private GitHub
Actions artifact custody: exact workflow/run/head/attempt plus exact artifact
ID, name, URL, size, digest, raw exact-ID download, three-file inner envelope,
build receipt, payload and manifest hashes, and a credential-free canonical Mac
custody receipt. No trusted-builder, SLSA, code-signing, or cryptographic
provenance claim follows.

**Adjacent operations receiving the same audit:** workflow trigger and
permissions; checkout identity; build receipt construction; upload; GitHub run
and artifact API selection; raw wrapper download; safe extraction; Mac custody
receipt; Mac-to-Deck artifact transfer; Deck artifact import; proof matrix;
blocked-result taxonomy; evidence return; final claim ceiling.

## P1 finding 2 — The exact implementation source has no Mac-to-Deck handoff

### Exact finding

```yaml
v6_gap: windows_artifact_transferred_but_execution_source_not_transferred
required_transport: existing_ordinary_mac_to_deck_ssh_lane
required_source_identity: exact_26_record_implementation_source_manifest
result: DESIGN_REPAIR_REQUIRED
additional_live_reconnaissance_required: false
```

V6 removes GitHub from the Deck and transfers the Windows artifact envelope,
but does not transfer the exact committed implementation source that performs
Deck import, supervision, normalization, and evidence generation. The preserved
MinGW worktree is salvage material, not the future source, and individually
copied scripts have no complete Git authority.

**Violated invariant:** Every execution and evidence-producing component must
be bound to the exact authorized source commit, tree, path/mode/blob manifest,
and authority basis before it can make a claim. Removing GitHub from the Deck
does not remove source-custody responsibility.

**Concrete blocked or false-acceptance state:** With no source handoff the Deck
either cannot execute the future implementation at all, must acquire it through
an undeclared GitHub dependency, or can accidentally execute the provisional,
dirty, stale, or individually copied source while accepting Windows artifacts
from another commit. The latter can produce apparently coherent evidence whose
build and execution sources never joined.

**Exact required repair:** After creating and verifying the clean exact
26-path implementation commit, the Mac creates a self-contained exact Git
bundle containing the required merged V7 authority history and implementation
commit under one fixed advertised ref. A canonical source-handoff receipt binds
the authority and implementation commit/tree/parent/ref, 26-record manifest,
bundle ref/hash/size, and `git bundle verify`. The Mac transfers bundle and
receipt over ordinary SSH without credential forwarding. The Deck verifies and
imports them under a fixed local non-GitHub ref, creates one clean detached
execution worktree at the exact commit, reproduces the source manifest before
every Deck run, and requires source/build/artifact/evidence commit agreement.
Unknown or dirty state is preserved and blocks.

**Adjacent operations receiving the same audit:** implementation commit and
manifest freeze; workflow source binding; source-bundle creation; SSH source
transfer; Deck handoff staging; bundle verification/import; detached worktree
creation; artifact import; every held-gate, fault, positive, normalization, and
evidence run; evidence return; final source equality; handoff retirement or
retention.

## Disposition

V6 is not implementation authority. A V7 repair may supersede it only by
closing both findings while preserving all other sound V6 decisions. The exact
V7 card requires fresh independent adversarial review and a later separate
operator approval before implementation can be authorized.

```text
DESIGN_REPAIR_REQUIRED
implementation_authorized=false
```
