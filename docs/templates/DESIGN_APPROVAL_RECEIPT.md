# Design Approval Receipt

A design-gated slice may not enter implementation until this receipt, the exact approved design card, and `CURRENT_SLICE.md` agree.

```yaml
schema: linux-vst-bridge-design-approval/v1
repository: kasselvania/Linux-VST-bridge
slice_id: <SLICE_ID>
slice_title: <TITLE>
design_revision: <REVISION>
design_card_path: docs/slices/<SLICE_ID>/IMPLEMENTATION_DESIGN.md
design_card_git_blob: <40-HEX GIT BLOB>
design_card_sha256: <64-HEX SHA-256>
adversarial_review_path: docs/slices/<SLICE_ID>/ADVERSARIAL_DESIGN_REVIEW.md
adversarial_review_identity: <BLOB OR SHA-256>
adversarial_review_result: DESIGN_CLEAR
basis_commit: <EXACT DESIGN BASIS COMMIT>
basis_tree: <EXACT DESIGN BASIS TREE>
primary_claim: >-
  <EXACT PRIMARY CLAIM>
exact_fixture:
  <EXACT FIXTURE>
approved_owner_model: true
approved_state_machine: true
approved_mutation_fault_ledger: true
approved_identity_ledger: true
approved_proof_matrix: true
approved_changed_path_envelope: true
approved_external_mutation_envelope: true
material_discovery_requires_stop: true
implementation_authorized: true
successor_selection_authorized: false
technical_lead_approval_text: >-
  <EXACT TECHNICAL-LEAD APPROVAL>
operator_approval_text: >-
  <EXACT OPERATOR APPROVAL>
approved_at: <ISO-8601>
```

## Exact approval sentence

```text
I explicitly approve implementation design revision <REVISION> for
<SLICE_ID> — <TITLE>.

Approved design path:
<PATH>

Approved design SHA-256 or Git blob:
<IDENTITY>

Implementation is authorized only for the exact primary claim, owner map,
state machine, mutation/fault ledger, identity ledger, proof matrix,
changed-path envelope, fixture, and nonclaims contained in that revision.

A material design discovery requires implementation to stop and return to the
design gate. No successor slice or design amendment is authorized by this
approval.
```

Approval binds an immutable design revision. A material amendment creates a new revision, new adversarial review, and new approval receipt.
