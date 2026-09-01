# WF0 Design Approval Receipt

```yaml
schema: linux-vst-bridge-design-approval/v1
repository: kasselvania/Linux-VST-bridge
slice_id: WF0
slice_title: Supervised Windows VST3 Factory Census Probe
design_revision: wf0-design-v2
design_card_path: docs/slices/WF0/IMPLEMENTATION_DESIGN.md
design_card_git_blob: d618cbf6b397f10947d50fd4824cd3e06ef55726
design_card_sha256: f323e2b429c4f91c1821488d980b249901b8d82c3cceaa7bf9be9d874a455e24
adversarial_review_path: docs/slices/WF0/ADVERSARIAL_DESIGN_REVIEW.md
adversarial_review_identity: 7ee98c4c0fb0d28b11d43d33b46e2ab5fa57c299
adversarial_review_github_id: 5082923871
adversarial_review_result: DESIGN_CLEAR
basis_commit: 745ca63bdd8641ade85cb9a024c1dc842681192d
basis_tree: 275521adff574f165bb2b3e883c2ec909bae4c66
primary_claim: >-
  On the exact accepted Steam Deck fixture, a repository-owned supervised
  Windows x86_64 factory probe, built against the pinned official VST3 SDK and
  executed through the accepted Runtime 4/Proton 11 lane in a disposable
  WF0-owned scan environment, loads the exact pinned AGain VST3 bundle,
  obtains its plug-in factory, retains deterministic factory metadata and the
  complete expected three-class census, unloads cleanly, and leaves the
  accepted WR0 environment and every protected fixture unchanged.
exact_fixture:
  host: Steam Deck Galileo / SteamOS 3.8.16 / x86_64 / KDE Wayland
  runner_runtime_digest: 2d64df1d36786ca2d0e955c553005423dc2b5bdd714bd0a17872622e33912547
  accepted_wr0_environment_identity: d3ed38d7ed53e9a5973cbf22fc504f2479dbdd15616fe1bfc1d5e1cd0bf5f4c4
  accepted_wr0_posture: protected_read_only
  freedesktop_sdk_commit: b90ed309cc1d505dea48b6a2121c5dcfac22868120eee643b0596d31f96b9bb8
  mingw_extension_ref: org.freedesktop.Sdk.Extension.mingw-w64/x86_64/25.08
  mingw_extension_commit: f15a5a88eb09557f645860bfcb3c4a6bc267683fce06cb68436bd76376fca694
  vst3_sdk_root_commit: 3cdf9ca5d1f5b1b21e0a86832aa4abe55607bd96
  positive_fixture: official pinned AGain Windows VST3
approved_owner_model: true
approved_state_machine: true
approved_mutation_fault_ledger: true
approved_identity_ledger: true
approved_proof_matrix: true
approved_changed_path_envelope: true
approved_external_mutation_envelope: true
approved_implementation_path_count: 40
approved_implementation_source_path_count: 26
approved_evidence_path_count: 14
approved_proof_matrix_rows: 35
approved_blocked_results: 23
approved_material_operations: 14
binding_implementation_clarifications:
  - >-
    Emit and synchronously flush each call_completed record immediately after
    capturing the bounded return value and before any later lifecycle event or
    call attempt. In-flight means no validated completion was observed.
  - >-
    Resolve and check the required GetPluginFactory export before invoking
    optional InitDll. A missing required export must not execute InitDll.
material_discovery_requires_stop: true
implementation_authorized: true
successor_selection_authorized: false
technical_lead_approval_text: >-
  I explicitly approve wf0-design-v2 as DESIGN_CLEAR under GitHub review
  5082923871. The two implementation clarifications retained by that review
  are binding. Implementation is limited to the exact approved WF0 claim,
  owners, lifecycle, proof matrix, 40-path envelope, external mutation
  envelope, and nonclaims.
operator_approval_text: |-
  I explicitly approve implementation design revision wf0-design-v2 for
  WF0 — Supervised Windows VST3 Factory Census Probe.

  Approved design path:
  docs/slices/WF0/IMPLEMENTATION\_DESIGN.md

  Approved design SHA-256 or Git blob:
  Git blob d618cbf6b397f10947d50fd4824cd3e06ef55726 and SHA-256 f323e2b429c4f91c1821488d980b249901b8d82c3cceaa7bf9be9d874a455e24

  Implementation is authorized only for the exact primary claim, owner map,
  state machine, mutation/fault ledger, identity ledger, proof matrix,
  changed-path envelope, fixture, and nonclaims contained in that revision and
  the implementation clarifications retained by GitHub review 5082923871.

  A material design discovery requires implementation to stop and return to the
  design gate. No successor slice or design amendment is authorized by this
  approval.
approved_at: 2026-09-01T13:47:12-07:00
```

## Authority effect

This receipt approves the immutable `wf0-design-v2` card and its binding review clarifications. It does not itself make the WF0 technical claim true. Implementation may begin only from the exact merged design-authority basis and must remain inside the approved 40-path tracked envelope and declared external mutations.

The exact MinGW extension installation is authorized only as part of WF0 implementation, at the approved user-scope ref and commit. No other package, runner, SDK, system, Bitwig, Serum, WR0-environment, `.wine`, or Steam-compatdata mutation is authorized.

```text
implementation_authorized=true
successor_selection_authorized=false
```
