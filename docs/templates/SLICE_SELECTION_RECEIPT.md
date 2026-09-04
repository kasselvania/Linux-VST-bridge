# Product Slice Selection Receipt

Use this after technical-lead analysis and explicit operator selection of one product contract.

```yaml
schema: linux-vst-bridge-product-slice-selection/v2
repository: kasselvania/Linux-VST-bridge
basis_commit: <COMMIT>
basis_tree: <TREE>
selected_slice: <SLICE_ID>
selected_title: <TITLE>
change_class: PRODUCT_CONTRACT_CHANGE
primary_claim: >-
  <ONE BOUNDED PRODUCT CLAIM>
claim_ceiling:
  - <NONCLAIM>
exact_fixture:
  <EXACT FIXTURE AND VERSION IDENTITIES>
authority_phase: reconnaissance_and_design | implementation
product_implementation_authorized: false | true
live_execution_authorized: false | true
permitted_execution_class: none | diagnostic_non_authoritative | acceptance_candidate
design_gate: required | waived
design_gate_reason: <REASON>
design_card_path: docs/slices/<SLICE_ID>/IMPLEMENTATION_DESIGN.md | null
allowed_changed_paths:
  - <PATH>
permitted_external_mutation:
  - <MUTATION OR none>
protected_state:
  - <IDENTITY>
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
failure_closure:
  bounded_diagnostic_pair_only: true
  success_evidence_on_failure: false
  full_audit_on_failure: false
proof_harness_maintenance_without_product_reapproval: true
return_to_product_design_conditions:
  - claim or claim ceiling change
  - VST3 roster/order/interpretation change
  - product owner/lifecycle change
  - Windows product behavior change
  - fixture meaning change
  - product normalization change
  - claimed containment/shutdown change
  - protocol/trust/security change
  - product proof or compatibility claim change
successor_selection_authorized: false
operator_approval_text: >-
  <EXACT OPERATOR MESSAGE>
approved_at: <ISO-8601>
```

## Selection meaning

A selected product slice authorizes product reconnaissance and design. It does not make diagnostic runs authoritative and does not combine diagnostic and acceptance budgets.

For a design-gated slice, the receipt must initially state:

```text
product_implementation_authorized: false
live_execution_authorized: false
permitted_execution_class: none
```

## Required operator sentence

```text
I explicitly approve selecting <SLICE_ID> — <TITLE> and replacing the
no-active-product-slice card with its bounded reconnaissance-and-design
authority. This approval does not authorize implementation or live execution.
Implementation requires a separately reviewed and approved product-design
revision.
```

A design-gate waiver must separately identify why the work changes no product owner, lifecycle, VST3 contract, fixture semantics, process/security boundary, or compatibility claim.

A proof-harness-only change must not use this template merely because it is high risk. Use `PROOF_HARNESS_MAINTENANCE_RECEIPT.md`.