# Design Approval Receipt

Use this receipt to approve one exact product-contract design. Proof-harness maintenance uses its own receipt and does not require a new product approval while the product contract remains unchanged.

```yaml
schema: linux-vst-bridge-design-approval/v2
repository: kasselvania/Linux-VST-bridge
slice_id: <SLICE_ID>
slice_title: <TITLE>
change_class: PRODUCT_CONTRACT_CHANGE
design_revision: <REVISION>
design_card_path: docs/slices/<SLICE_ID>/IMPLEMENTATION_DESIGN.md
design_card_git_blob: <40-HEX>
design_card_sha256: <64-HEX>
adversarial_review_path: docs/slices/<SLICE_ID>/ADVERSARIAL_DESIGN_REVIEW.md
adversarial_review_identity: <IDENTITY>
adversarial_review_result: DESIGN_CLEAR
basis_commit: <COMMIT>
basis_tree: <TREE>
primary_claim: >-
  <EXACT PRODUCT CLAIM>
claim_ceiling:
  - <NONCLAIM>
exact_fixture:
  <IDENTITY>
approved_product_owner_model: true
approved_product_lifecycle: true
approved_vst3_operation_roster: true
approved_product_normalization: true
approved_containment_shutdown_claim: true
approved_product_proof_matrix: true
approved_changed_path_envelope: true
diagnostic_campaign:
  required: true | false
  plan_id: <ID OR null>
  batch_budget: <0, 1, OR 2>
  acceptance_eligible: false
acceptance_candidate:
  plan_id: <ID>
  batch_budget: 1
windows_producer_budget:
  owner: WindowsBuildInputIdentity
  maximum: <COUNT>
live_negative_budget:
  owner: FaultPlanIdentity
  maximum: <COUNT>
proof_harness_maintenance_may_proceed_without_product_reapproval: true
return_to_product_design_conditions:
  - claim change
  - VST3 call/order/interpretation change
  - product owner/lifecycle change
  - Windows product behavior change
  - fixture meaning change
  - product normalization change
  - claimed containment/shutdown change
  - protocol/security boundary change
  - product proof or compatibility claim change
implementation_authorized: true
live_execution_authorized: false | true
permitted_execution_class: none | diagnostic_non_authoritative | acceptance_candidate
successor_selection_authorized: false
technical_lead_approval_text: >-
  <TEXT>
operator_approval_text: >-
  <TEXT>
approved_at: <ISO-8601>
```

## Required approval meaning

The approval authorizes the exact product contract. It does not mean every implementation or diagnostic run is acceptance evidence.

A proof-harness repair may proceed under `PROOF_HARNESS_MAINTENANCE_RECEIPT.md` without another product-design approval when every product-contract fact remains unchanged.

A diagnostic campaign is permanently non-authoritative:

```text
execution_class: diagnostic_non_authoritative
acceptance_eligible: false
```

Only a fresh `AcceptanceCandidateIdentity` may support product merge.

## Exact operator sentence

```text
I explicitly approve implementation design revision <REVISION> for
<SLICE_ID> — <TITLE>.

Approved design path:
<PATH>

Approved design identity:
<IDENTITY>

This approval binds the exact product claim, VST3 operation roster, product
owner/lifecycle, fixture semantics, product normalization, containment and
shutdown claims, proof matrix, changed-path envelope, diagnostic budget, and
acceptance budget in that revision.

Proof-harness maintenance that leaves those product facts unchanged may use
the focused maintenance process and does not require a new product-design
approval. A product-contract change returns to product design.

No successor is authorized by this approval.
```
