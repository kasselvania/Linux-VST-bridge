# Current Slice: PC0 — Durable Failed-Batch Diagnostic and Corrective Execution Repair

## Status

```text
status: active_implementation_slice
authority_phase: implementation
implementation_authorized: true
slice: PC0
product_claim_revision: pc0-selection-v2
design_revision: pc0-design-v3
design_status: approved_for_implementation
design_commit: c2349780f9ed1aa6077b118be000cbab5aba698a
design_tree: c1a71eb2afb9c71f62ddc2cc3bd9a5b5baa4e4bd
design_card: docs/slices/PC0/IMPLEMENTATION_DESIGN.md
design_card_git_blob: b991e204681e56869a0977cad091c7de7345cbeb
design_card_sha256: 4dcdce46f5f7d478fe2687c3d685418c4dd4b940804cb7a6744b890a6acff3cc
design_review_record: docs/slices/PC0/ADVERSARIAL_DESIGN_REVIEW.md
design_review_blob: 3498f6e0cf342c33c7c5023a4c193ab3c9dc7c79
technical_lead_review_id: 5108043079
technical_lead_review_result: PC0_DESIGN_V3_CLEAR
prior_v3_repair_review: 5107795355 / PC0_DESIGN_V3_REPAIR_REQUIRED
design_approval: docs/slices/PC0/DESIGN_APPROVAL.md
design_approval_blob: 32cec9c63d908687cf5e8071656413db48c049cb
design_repair_basis_commit: 1c0c31c4ab69a40303cd00b155ca30626323c451
design_repair_basis_tree: 192d2af4b5d83d94264510eab7c7729b5de1a9b5
design_branch: codex/pc0-durable-failure-diagnostic-design-v3
implementation_branch: codex/pc0-windows-vst3-pre-setup-processing-contract
implementation_basis: exact merged PR #38 V3 authority commit and tree, established by main readback before source creation
failed_archive_ref: refs/heads/codex/archive/pc0-v2-failed-7ac6095a488d
implementation_force_with_lease_expected_old_tip: 7ac6095a488d0077fcc78fedc5abd870b5ffb1cb
v2_design_commit: 996ee557d33ea55d6acf7ef2242f703c2c63f262
v2_design_tree: bb1fe157bef42854812ebeb0b73b1a5332c93cf6
v2_design_blob: ec0683fc66028239d7481640ba72e2dd9a060a2c
v2_design_sha256: 20e653a6b1fad720ea5fc888a8d531bda44c338840f5eb96e488612d197de992
v2_technical_lead_review: 5105712590 / PC0_DESIGN_V2_CLEAR
runtime_discovery: PR #37 comment 5533164226 / PC0_RUNTIME_DISCOVERY — RETURN_TO_DESIGN_GATE
failed_source_commit: 7ac6095a488d0077fcc78fedc5abd870b5ffb1cb
failed_source_tree: ad4230a713a2bb644476be8be9d374a1feb36b06
failed_transaction_id: a14bcc65d15e9fbdf15810a658b2ee87
failed_transaction_journal_sha256: 06473755eb7ccfa2529522bb29e3fb44a5a4a67ea38fd0e797d198939e1ed966
operator_recovery_receipt_sha256: 870039310c0f6ee4d0bf4044d629e6f6e7d92d2f901b7b40a60f9724bb84f9e5
windows_build_input_identity: 575d3bd9183be1ec0fe0311cff48bc0107c4299a3355748c470e3d195d284849
retained_producer_run_attempt: 33812659869 / 1
retained_artifact: 9915439437
corrective_positive_deck_batches_maximum: 1
successor_selection_authorized: false
```

## Primary claim

> On the exact accepted AGain lifecycle, while the processor remains in the
> Initialized state, the supervised Windows host performs one bounded read-only
> census of all audio/event buses, all existing `BusInfo` records, current
> audio speaker arrangements, and `kSample32`/`kSample64` support, retains one
> exact normalized pre-setup processing contract, and completes the accepted
> interface/component/factory/module shutdown without mutating processing state.

## Exact implementation authority

The immutable reviewed [pc0-design-v3](docs/slices/PC0/IMPLEMENTATION_DESIGN.md), technical-lead review `5108043079 / PC0_DESIGN_V3_CLEAR`, retained [adversarial review history](docs/slices/PC0/ADVERSARIAL_DESIGN_REVIEW.md), and [operator approval receipt](docs/slices/PC0/DESIGN_APPROVAL.md) authorize only the bounded V3 repair. The design card remains byte-identical to reviewed Git blob `b991e204681e56869a0977cad091c7de7345cbeb`, raw SHA-256 `4dcdce46f5f7d478fe2687c3d685418c4dd4b940804cb7a6744b890a6acff3cc`.

The historical V2 batch is spent and remains `unresolved_v2_no_diagnostic`. It must not be retroactively assigned a diagnostic classification. Its transaction journal, separately authorized recovery continuation, failed source, producer run/artifact, and actual effects remain part of final cumulative history.

## Exact implementation envelope

```text
product owners:                               1
product owner:                                PreSetupProcessingContractCensus
stable product states:                        9
VST3 operation types:                         4
positive PC0 calls:                          11
focused proof rows:                          16
runtime blockers:                            10
source/configuration paths:                   14
tracked evidence paths:                       5
implementation repair paths versus failed:    4
additional Windows builds:                    0
additional workflow dispatches:               0
additional artifact downloads:                0
additional custody operations:                0
additional host-artifact transfers:           0
additional fixture seeds:                     0
additional AGain builds:                      0
repaired-source handoffs/transfers:            1 maximum
corrective positive Deck reservations:         1 maximum
corrective positive Deck batches:              1 maximum
third total PC0 Deck batch:                    0
deterministic negative tests:                  required
live negative exercises:                       0
Bitwig launches:                               0
Serum launches:                                0
```

The ordinary clean steady-state interface remains one Mac driver command. Every actual driver invocation, including any preflight-only invocation, must be retained truthfully in private transaction and final cumulative cost history.

## Exact source and Windows identity law

The repaired implementation commit is one direct fourteen-path child of the merged V3 authority. Relative to failed source `7ac6095a488d0077fcc78fedc5abd870b5ffb1cb`, only these four paths may differ:

```text
tools/host-proof.py
tools/wf0-factory-census/run.py
tools/wf0-factory-census/evidence.py
tools/wf0-factory-census/negative_tests.py
```

All other members of the fourteen-path source envelope remain byte-identical to failed source. The complete seventeen-record Windows roster must reproduce WindowsBuildInputIdentity `575d3bd9183be1ec0fe0311cff48bc0107c4299a3355748c470e3d195d284849`. Producer P remains failed source `7ac6095a...`, run `33812659869` attempt 1, artifact `9915439437`; no producer fallback is selectable.

Before implementation-ref replacement, preserve and read back failed source at `refs/heads/codex/archive/pc0-v2-failed-7ac6095a488d`. Replace the implementation ref only with force-with-lease expecting old tip `7ac6095a488d0077fcc78fedc5abd870b5ffb1cb`.

## Corrective execution authority

One V3-only read-only Deck preflight must complete after repaired-source handoff and detached-clean worktree admission, but before current intent publication, any Mac or Deck execution lock, execute-phase preparation, corrective reservation, Deck-effect increment, environment/stage creation, diagnostic/result publication, or Runtime/Proton launch. It validates exact source/handoff/worktree, process/stage guard, Galileo/SteamOS/read-only/authority posture, Runtime/Proton identity, protected state, existing host/fixture stores, historical intent/locks/result absence, and current corrective absence. Failure consumes no corrective reservation, mutates no execution or protected state, and authorizes no automatic launch.

Only after that preflight and all twelve corrective predicates pass may one reservation be persisted. The reservation is consumed once made. A failed corrective batch retains its lock and may publish only the exact bounded private `linux-vst-bridge-pc0-failure-diagnostic/v1` pair; it publishes no success result and authorizes no retry. No second corrective or third total PC0 Deck batch is authorized.

On success, freeze exact canonical `DX0_TRANSACTION_STATE.json` bytes privately as `PC0_CORRECTIVE_PRE_EVIDENCE_STATE.json` plus `PC0_CORRECTIVE_PRE_EVIDENCE_STATE.json.sha256` after strict result admission and completed `retrieve_and_retain_result`, but before tracked render, render-phase publication, close, or final completion. Final corrective history uses `pre_evidence_journal_sha256` only for that immutable snapshot and never labels it as the digest of the later mutable journal.

## Absolute claim ceiling

PC0 remains Initialized-state read-only observation. It must not call or prepare `setupProcessing`, `getLatencySamples`, `getTailSamples`, activation, processing, audio/event buffers, controller, state, parameters, proxy, C ABI, IPC, Bitwig, Serum, packaging, signing, a new runner, generalized retry machinery, a live negative family, or a broader compatibility claim.

A material discovery, WindowsBuildInputIdentity drift, fifth repair path, additional runtime blocker, seventeenth proof row, sixth tracked evidence path, required Windows operation, or need for another corrective Deck batch returns PC0 to the design gate. No successor slice is authorized.

## Next action

Merge this exact authority finalization through PR #38 and read back the resulting `main` merge commit/tree. Then create and verify the failed-source archive ref, create one fourteen-path repaired source commit directly above that merged authority, prove the frozen Windows identity and deterministic tests, replace the implementation ref under the exact lease, and perform the authorized read-only Deck preflight. The corrective reservation remains unavailable until every predicate passes.
