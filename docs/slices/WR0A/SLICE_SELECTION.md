# WR0A slice selection receipt

```yaml
schema: linux-vst-bridge-slice-selection/v1
repository: kasselvania/Linux-VST-bridge
basis_commit: b517f96bafa388107ea28c9d2e91528fd37b00d9
basis_tree: 839cda6145f592642f06b5273855b04ffa0562ac
selected_slice: WR0A
selected_title: Post-Merge Final WR0 Repair Reconciliation and Adoption
primary_claim: >-
  Current main adopts the exact content-addressed final WR0 repair represented
  by archive commit 52be94316664f88a19df630e164105a0ca50b875 without
  re-running or mutating the live WR0 environment.
exact_fixture:
  platform: Steam Deck / SteamOS / x86_64
  environment_path: <HOME>/.local/share/linux-vst-bridge/environments/wr0-proton11
  transaction: wr0-20260901T053317Z-183afd5bbf0736e4
  environment_identity_sha256: d3ed38d7ed53e9a5973cbf22fc504f2479dbdd15616fe1bfc1d5e1cd0bf5f4c4
  runner_runtime_identity_sha256: 2d64df1d36786ca2d0e955c553005423dc2b5bdd714bd0a17872622e33912547
  contract_source_schema: linux-vst-bridge-wr0-contract-source/v1
  contract_source_sha256: c6543004fbcd70252393f0e0cab4f9ad7a31e85f433ce3e903e690244cc79541
  workload_sha256: 4518ca37b8d7e005f01b441de5273447b70fa7c8c9c3d2b9c194a91782cfecac
  retired_replacement_record_sha256: 917c939c605f457376b0bdd53e06d252d4aecee4b39728850b2e1a61aa102b3e
authority_phase: reconnaissance_and_design
implementation_authorized: false
design_gate: required
design_gate_reason:
  - historical and current repository truth have distinct owners
  - immutable archive source and live durable state must be reconciled exactly
  - content-addressed source identity must survive adoption
  - stale canonical evidence must be superseded without erasing history
  - DG0 governance and active authority must remain distinct from archive content
design_card_path: docs/slices/WR0A/IMPLEMENTATION_DESIGN.md
design_approval_path: null
allowed_changed_paths:
  - CURRENT_SLICE.md
  - docs/slices/WR0A/SLICE_SELECTION.md
  - docs/slices/WR0A/RECONNAISSANCE.md
  - docs/slices/WR0A/IMPLEMENTATION_DESIGN.md
permitted_external_mutation:
  - none
protected_state:
  - current main commit/tree b517f96bafa388107ea28c9d2e91528fd37b00d9 / 839cda6145f592642f06b5273855b04ffa0562ac
  - immutable archive commit/tree/parent 52be94316664f88a19df630e164105a0ca50b875 / 21601814bd352114fe2e55833c88a81e47e13e41 / 3deb414a54174cd95432c84e117a642f30c482fe
  - live WR0 environment d3ed38d7ed53e9a5973cbf22fc504f2479dbdd15616fe1bfc1d5e1cd0bf5f4c4
  - runner/runtime lock 2d64df1d36786ca2d0e955c553005423dc2b5bdd714bd0a17872622e33912547
  - archive contract source c6543004fbcd70252393f0e0cab4f9ad7a31e85f433ce3e903e690244cc79541
  - archive workload 4518ca37b8d7e005f01b441de5273447b70fa7c8c9c3d2b9c194a91782cfecac
  - DG0 governance at current main
explicit_nonclaims:
  - implementation or archive-blob adoption is not authorized
  - no Proton, Wine, Windows workload, Bitwig, Serum, or validator execution
  - no WR0 environment, receipt, runner, Steam, Flatpak, or .wine mutation
  - no Windows VST3 loading, scanning, hosting, authorization, audio, bridge, IPC, shared memory, manager, broker, or Rust work
  - no final product-runner or general compatibility claim
material_discovery_requires_stop: true
successor_selection_authorized: false
operator_approval_text: >-
  I explicitly approve selecting WR0A — Post-Merge Final WR0 Repair
  Reconciliation and Adoption and replacing the no-active-slice card with its
  bounded reconnaissance-and-design authority. This approval does not
  authorize implementation. Implementation requires a separate approved
  design revision.
approved_at: 2026-09-01T14:23:36Z
```

## Source authority note

The technical lead independently verified the exact GitHub main and immutable
archive identities. The Deck verified both exact local commit objects, trees,
and the archive parent. The public HTTPS fetch attempt could not authenticate;
network fetch is not required for source identity under the superseding
technical-lead receipt. No alternate commit, credential, token, origin change,
or SSH agent forwarding is authorized.

## Phase boundary

This receipt selects reconnaissance and design only. It is not an
implementation authorization receipt and cannot be used to import archive
blobs, replace evidence, mutate the live WR0 fixture, or merge any change.
