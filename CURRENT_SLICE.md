# Current Slice: PC0 — Durable Failed-Batch Diagnostic Design Repair

## Status

```text
status: active_design_slice
authority_phase: reconnaissance_and_design
implementation_authorized: false
slice: PC0
product_claim_revision: pc0-selection-v2
design_revision: pc0-design-v3
design_status: proposed_for_adversarial_review
target: main
design_repair_basis_commit: 1c0c31c4ab69a40303cd00b155ca30626323c451
design_repair_basis_tree: 192d2af4b5d83d94264510eab7c7729b5de1a9b5
design_branch: codex/pc0-durable-failure-diagnostic-design-v3
implementation_branch: codex/pc0-windows-vst3-pre-setup-processing-contract
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
successor_selection_authorized: false
```

## Primary claim

The PC0 product claim is unchanged:

> On the exact accepted AGain lifecycle, while the processor remains in the
> Initialized state, the supervised Windows host performs one bounded read-only
> census of all audio/event buses, all existing `BusInfo` records, current
> audio speaker arrangements, and `kSample32`/`kSample64` support, retains
> one exact normalized pre-setup processing contract, and completes the accepted
> interface/component/factory/module shutdown without mutating processing state.

V3 does not reopen the product design. It repairs only the proof-transaction
failure-evidence boundary discovered by the first implementation attempt and
designs one exact corrective authority. No corrective authority exists until
an exact V3 design is independently cleared, explicitly approved by the
operator, finalized as repository authority, and merged.

## Retained V2 authority and runtime discovery

The immutable V2 design remains blob
`ec0683fc66028239d7481640ba72e2dd9a060a2c`, SHA-256
`20e653a6b1fad720ea5fc888a8d531bda44c338840f5eb96e488612d197de992`.
Technical-lead review `5105712590 / PC0_DESIGN_V2_CLEAR` and the existing
V2 approval remain historical authority for failed source
`7ac6095a488d0077fcc78fedc5abd870b5ffb1cb`; they do not authorize a second
Deck batch.

Merged PR #37 comment `5533164226` records the material runtime discovery:
`supervise.py` retained a structured failed run in memory, `run.py`
collapsed it to `PC0_EVIDENCE_BLOCKED`, and no durable failure diagnostic or
success result was published. The originally authorized positive Deck batch is
spent.

## Frozen failed-execution facts

Read-only reconnaissance admitted the canonical local transaction journal and
operator-recovery receipt by exact bytes:

- transaction journal SHA-256
  `06473755eb7ccfa2529522bb29e3fb44a5a4a67ea38fd0e797d198939e1ed966`;
- operator-recovery receipt SHA-256
  `870039310c0f6ee4d0bf4044d629e6f6e7d92d2f901b7b40a60f9724bb84f9e5`;
- one recorded ordinary driver invocation and one separately recorded
  operator-authorized recovery continuation;
- actual effects: one Windows build, artifact download, custody operation,
  host-artifact transfer, source transfer, and Deck execution; zero evidence
  renders;
- execute phase `failed` with `PC0_EVIDENCE_BLOCKED`, while the monotonic
  transaction state remains `deck_batch_in_flight`;
- no local retained success-result directory for DeckExecutionInputIdentity
  `ae89ee61636074feac5c875bab9b8a9621e3a0c6f7fa83b6d33bc11b94e631a3`.

The local files do not identify the failed scanner/supervision class and do not
prove a current remote-lock state. V3 preserves the historical classification
as `unresolved_v2_no_diagnostic`; a later authorized corrective preflight
must strictly read and admit the expected remote stores and locks before any
reservation.

## Exact design ceiling

The V3 design preserves:

```text
new product owners:                 1
product owner:                      PreSetupProcessingContractCensus
stable product states:              9
VST3 operation types:               4
positive PC0 calls:                11
focused proof rows:                16
blocked outcome taxonomy:          10
source/configuration envelope:     14 paths
tracked evidence envelope:          5 paths
live negative exercises:            0
AGain rebuilds:                      0
fixture reseeds:                     0
Bitwig launches:                     0
Serum launches:                      0
additional Windows builds:           0
additional workflow dispatches:      0
additional corrective Deck batches:  1 maximum, only after later authority
```

The durable diagnostic remains private proof-transaction data owned by the
existing Deck execution lock. It is not a product owner, product state,
VST3 operation, proof row, blocker, tracked evidence file, live negative
fixture, protocol framework, or Windows build plane.

## Design-only external-effect ceiling

This design slice may edit and push only its three authority paths and open one
draft design PR. It authorizes no Windows workflow, artifact download, custody
operation, fixture operation, source handoff, SSH session, Deck read, Deck
write, Proton/Wine execution, plug-in execution, DAW execution, result
retrieval, or evidence render.

## Next action

Submit exact pc0-design-v3 to independent adversarial review. A clear review
must name the exact design head, tree, design blob, and raw SHA-256. Only a
later explicit operator approval may authorize authority finalization. Until
that merge and readback, implementation and the corrective Deck batch remain
forbidden.
