# PC0 Design Approval Receipt

```yaml
schema: linux-vst-bridge-design-approval/v1
repository: kasselvania/Linux-VST-bridge
slice_id: PC0
slice_title: Windows VST3 Pre-Setup Processing Contract Census
design_revision: pc0-design-v3
design_commit: c2349780f9ed1aa6077b118be000cbab5aba698a
design_tree: c1a71eb2afb9c71f62ddc2cc3bd9a5b5baa4e4bd
design_parent: 3abad98911792ece3320d0ec56fd526663db5f28
design_repair_basis_commit: 1c0c31c4ab69a40303cd00b155ca30626323c451
design_repair_basis_tree: 192d2af4b5d83d94264510eab7c7729b5de1a9b5
design_card_path: docs/slices/PC0/IMPLEMENTATION_DESIGN.md
design_card_git_blob: b991e204681e56869a0977cad091c7de7345cbeb
design_card_sha256: 4dcdce46f5f7d478fe2687c3d685418c4dd4b940804cb7a6744b890a6acff3cc
adversarial_review_path: docs/slices/PC0/ADVERSARIAL_DESIGN_REVIEW.md
adversarial_review_git_blob: 3498f6e0cf342c33c7c5023a4c193ab3c9dc7c79
adversarial_review_id: 5108043079
adversarial_review_result: PC0_DESIGN_V3_CLEAR
prior_v3_repair_review_id: 5107795355
prior_v3_repair_review_result: PC0_DESIGN_V3_REPAIR_REQUIRED
v2_design_commit: 996ee557d33ea55d6acf7ef2242f703c2c63f262
v2_design_tree: bb1fe157bef42854812ebeb0b73b1a5332c93cf6
v2_design_blob: ec0683fc66028239d7481640ba72e2dd9a060a2c
v2_design_sha256: 20e653a6b1fad720ea5fc888a8d531bda44c338840f5eb96e488612d197de992
v2_technical_lead_review: 5105712590 / PC0_DESIGN_V2_CLEAR
runtime_discovery: PR #37 comment 5533164226 / PC0_RUNTIME_DISCOVERY
failed_source_commit: 7ac6095a488d0077fcc78fedc5abd870b5ffb1cb
failed_source_tree: ad4230a713a2bb644476be8be9d374a1feb36b06
failed_transaction_id: a14bcc65d15e9fbdf15810a658b2ee87
failed_transaction_journal_sha256: 06473755eb7ccfa2529522bb29e3fb44a5a4a67ea38fd0e797d198939e1ed966
operator_recovery_receipt_sha256: 870039310c0f6ee4d0bf4044d629e6f6e7d92d2f901b7b40a60f9724bb84f9e5
primary_claim: >-
  On the exact accepted AGain lifecycle, while the processor remains in the
  Initialized state, the supervised Windows host performs one bounded read-only
  census of all audio/event buses, all existing BusInfo records, current audio
  speaker arrangements, and kSample32/kSample64 support, retains one exact
  normalized pre-setup processing contract, and completes the accepted
  interface/component/factory/module shutdown without mutating processing state.
exact_fixture:
  - accepted AGain VST3 module SHA-256 60aa9ff6b9918d4330449e7b3ab34b588dd93cba09f37413a3cd91f6e7d2e18f
  - accepted AGain bundle-manifest SHA-256 bfaa1dce4d2e189f89cee41493838824efe647e361e81436676b7b3a86ff5164
  - accepted AGain fixture identity SHA-256 6c87be964d26a7ad06e7a4c69c5c5261d1046e9cfb0b17a225fd24c3e40d0ba6
  - accepted Runtime 4 / Proton 11 identity 2d64df1d36786ca2d0e955c553005423dc2b5bdd714bd0a17872622e33912547
  - Steam Deck Galileo / SteamOS 3.8.16 / x86_64 / read-only enabled
approved_product_envelope:
  product_owner_count: 1
  product_owner: PreSetupProcessingContractCensus
  stable_product_states: 9
  vst3_operation_types: 4
  positive_pc0_calls: 11
  focused_proof_rows: 16
  runtime_blockers: 10
  source_configuration_paths: 14
  tracked_evidence_paths: 5
approved_repair_paths:
  - tools/host-proof.py
  - tools/wf0-factory-census/run.py
  - tools/wf0-factory-census/evidence.py
  - tools/wf0-factory-census/negative_tests.py
byte_identical_source_paths_relative_to_failed_source:
  - .github/workflows/wf0-windows-msvc-build.yml
  - tools/wf0-factory-census/artifacts.py
  - tools/wf0-factory-census/build.py
  - tools/wf0-factory-census/common.py
  - tools/wf0-factory-census/normalize.py
  - tools/wf0-factory-census/supervise.py
  - tools/wf0-factory-census/verify.py
  - windows-factory-probe/source/component_instance_session.cpp
  - windows-factory-probe/source/component_instance_session.h
  - windows-factory-probe/source/main.cpp
windows_build_input_identity: 575d3bd9183be1ec0fe0311cff48bc0107c4299a3355748c470e3d195d284849
retained_windows_producer_source: 7ac6095a488d0077fcc78fedc5abd870b5ffb1cb
retained_windows_producer_run: 33812659869
retained_windows_producer_attempt: 1
retained_windows_artifact: 9915439437
failure_diagnostic_schema: linux-vst-bridge-pc0-failure-diagnostic/v1
failure_diagnostic_json: PC0_FAILURE_DIAGNOSTIC.json
failure_diagnostic_sidecar: PC0_FAILURE_DIAGNOSTIC.json.sha256
pre_evidence_snapshot_json: PC0_CORRECTIVE_PRE_EVIDENCE_STATE.json
pre_evidence_snapshot_sidecar: PC0_CORRECTIVE_PRE_EVIDENCE_STATE.json.sha256
corrective_history_snapshot_key: pre_evidence_journal_sha256
failed_v2_classification: unresolved_v2_no_diagnostic
implementation_cost_ceiling:
  additional_windows_builds: 0
  additional_workflow_dispatches: 0
  additional_artifact_downloads: 0
  additional_custody_operations: 0
  additional_host_artifact_transfers: 0
  additional_fixture_seeds: 0
  additional_again_builds: 0
  repaired_source_handoffs_or_transfers_maximum: 1
  corrective_deck_reservations_maximum: 1
  corrective_positive_deck_batches_maximum: 1
  total_pc0_deck_executions_after_success: 2
  live_negative_exercises: 0
  bitwig_launches: 0
  serum_launches: 0
  third_pc0_deck_batch_authorized: false
archive_ref: refs/heads/codex/archive/pc0-v2-failed-7ac6095a488d
implementation_ref: refs/heads/codex/pc0-windows-vst3-pre-setup-processing-contract
implementation_ref_force_with_lease_expected_old_tip: 7ac6095a488d0077fcc78fedc5abd870b5ffb1cb
material_discovery_requires_stop: true
implementation_authorized: true
successor_selection_authorized: false
operator_approval_source: >-
  The operator explicitly approved exact pc0-design-v3 blob
  b991e204681e56869a0977cad091c7de7345cbeb and raw SHA-256
  4dcdce46f5f7d478fe2687c3d685418c4dd4b940804cb7a6744b890a6acff3cc
  against technical-lead review 5108043079 / PC0_DESIGN_V3_CLEAR. The approval
  authorizes only the four-path non-Windows repair, one read-only corrective
  Deck preflight, and one predicate-bound corrective positive Deck batch after
  merged authority, with every stated historical, cost, identity, archive,
  evidence, and stop law retained.
approved_at: 2026-09-03T18:23:01-07:00
```

## Binding implementation authority

Implementation is authorized only for exact reviewed `pc0-design-v3`. The product claim, owner, nine-state machine, four VST3 operation types, eleven-call positive AGain sequence, sixteen proof rows, ten runtime blockers, fourteen-path source/configuration envelope, five-path tracked evidence envelope, and all nonclaims remain unchanged.

Relative to failed source `7ac6095a488d0077fcc78fedc5abd870b5ffb1cb`, only `tools/host-proof.py`, `tools/wf0-factory-census/run.py`, `tools/wf0-factory-census/evidence.py`, and `tools/wf0-factory-census/negative_tests.py` may differ. `common.py`, `artifacts.py`, `supervise.py`, workflow/build/verify/CMake/C++ inputs, SDK locks, and the exact seventeen-record Windows roster remain frozen. The repaired source must reproduce WindowsBuildInputIdentity `575d3bd9183be1ec0fe0311cff48bc0107c4299a3355748c470e3d195d284849` and reuse producer run `33812659869` attempt 1 and artifact `9915439437`. No Windows producer, dispatch, build, download, custody, host-artifact transfer, fixture seed, or AGain build fallback is authorized.

The V3-only read-only Deck preflight occurs after the repaired source handoff and detached-clean worktree admission and before current intent publication, any Mac or Deck execution lock, execute-phase preparation, corrective reservation, Deck-effect increment, environment/stage creation, result/diagnostic publication, or Runtime/Proton launch. A failed preflight consumes no corrective reservation and performs no execution or protected-state mutation. Any later invocation remains part of truthful implementation history.

Only after that preflight and all twelve corrective-authority predicates pass may one corrective positive Deck reservation be persisted. That reservation is consumed once made. A failed corrective execution may retain only the exact bounded private diagnostic and sidecar in its execution lock, publish no success result, and authorize no retry. No second corrective batch or third total PC0 Deck batch is authorized. The historical V2 failure remains `unresolved_v2_no_diagnostic`.

On corrective success, the exact canonical transaction journal is frozen privately as `PC0_CORRECTIVE_PRE_EVIDENCE_STATE.json` plus `PC0_CORRECTIVE_PRE_EVIDENCE_STATE.json.sha256` after strict result admission and completed `retrieve_and_retain_result`, but before tracked evidence rendering, render-phase publication, close, or final completion. Tracked corrective history binds `pre_evidence_journal_sha256` to that immutable snapshot and never represents it as the digest of the subsequently mutated live journal.

Before replacing the implementation ref, preserve failed source at `refs/heads/codex/archive/pc0-v2-failed-7ac6095a488d`, read it back exactly, and use force-with-lease expecting old implementation tip `7ac6095a488d0077fcc78fedc5abd870b5ffb1cb`. Final topology is the merged V3 authority, one fourteen-path repaired source commit, and one five-path evidence-only child.

No `setupProcessing`, `getLatencySamples`, `getTailSamples`, activation, processing, buffers, controller, state, parameters, proxy, C ABI, IPC, Bitwig, Serum, packaging, signing, new runner, generalized retry mechanism, live negative family, broader compatibility claim, or successor slice is authorized. A material discovery, Windows identity drift, fifth repair path, added blocker, seventeenth proof row, sixth tracked evidence path, required Windows operation, or need for another corrective batch returns PC0 to the design gate.
