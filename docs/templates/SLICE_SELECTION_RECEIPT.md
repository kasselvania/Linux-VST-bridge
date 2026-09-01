# Slice Selection Receipt

Use this template after technical-lead next-slice analysis and explicit operator selection.

```yaml
schema: linux-vst-bridge-slice-selection/v1
repository: kasselvania/Linux-VST-bridge
basis_commit: <EXACT MAIN COMMIT>
basis_tree: <EXACT MAIN TREE>
selected_slice: <SLICE_ID>
selected_title: <TITLE>
primary_claim: >-
  <ONE BOUNDED CLAIM>
exact_fixture:
  <EXACT FIXTURE AND VERSION IDENTITIES>
authority_phase: reconnaissance_and_design | implementation
implementation_authorized: false | true
design_gate: required | waived
design_gate_reason:
  <TRIGGERS OR EXACT WAIVER REASON>
design_card_path: docs/slices/<SLICE_ID>/IMPLEMENTATION_DESIGN.md | null
design_approval_path: docs/slices/<SLICE_ID>/DESIGN_APPROVAL.md | null
allowed_changed_paths:
  - <PATH OR PREFIX>
permitted_external_mutation:
  - <EXACT MUTATION OR none>
protected_state:
  - <EXACT ACCEPTED IDENTITY>
explicit_nonclaims:
  - <NONCLAIM>
material_discovery_requires_stop: true
successor_selection_authorized: false
operator_approval_text: >-
  <EXACT OPERATOR MESSAGE>
approved_at: <ISO-8601>
```

## Required operator sentence — design-gated slice

```text
I explicitly approve selecting <SLICE_ID> — <TITLE> and replacing the
no-active-slice card with its bounded reconnaissance-and-design authority.
This approval does not authorize implementation. Implementation requires a
separate approved design revision.
```

## Required operator sentence — design-gate waiver

```text
I explicitly approve activating <SLICE_ID> — <TITLE>, replacing the
no-active-slice card, and implementing only its exact bounded claim and
changed-path/mutation envelope.
```

The generated sentence is a draft until the operator sends it. An implementation prompt cannot substitute for operator approval.
