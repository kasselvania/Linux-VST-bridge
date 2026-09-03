# PC0 Design Approval Receipt

```yaml
schema: linux-vst-bridge-design-approval/v1
repository: kasselvania/Linux-VST-bridge
slice_id: PC0
slice_title: Windows VST3 Pre-Setup Processing Contract Census
selection_basis_commit: 858c240b104e090aaed8bd23ace04fd9a0dfd20e
selection_basis_tree: 8a560faf07b793de8faab91952ee6f34f15a1e73
revised_selection_commit: f00824e196d2a71667d80a9cc622c03ea2cef403
revised_selection_tree: d895a44b98264f47549d4187303b00e9e9f85982
design_revision: pc0-design-v2
design_commit: 996ee557d33ea55d6acf7ef2242f703c2c63f262
design_tree: bb1fe157bef42854812ebeb0b73b1a5332c93cf6
design_card_path: docs/slices/PC0/IMPLEMENTATION_DESIGN.md
design_card_git_blob: ec0683fc66028239d7481640ba72e2dd9a060a2c
design_card_sha256: 20e653a6b1fad720ea5fc888a8d531bda44c338840f5eb96e488612d197de992
adversarial_review_path: docs/slices/PC0/ADVERSARIAL_DESIGN_REVIEW.md
adversarial_review_git_blob: 7444c490e620c16903e5a59b052376d8dcaef3e9
adversarial_review_id: 5105712590
adversarial_review_result: PC0_DESIGN_V2_CLEAR
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
  - accepted Runtime 4 / Proton 11 identity 2d64df1d36786ca2d0e955c553005423dc2b5bdd714bd0a17872622e33912547
  - accepted WA0 initialized IComponent and IAudioProcessor lifecycle
  - accepted DX0 one-command transaction, split identities, fixture reuse, retained-result admission, and recovery
approved_owner_model: true
approved_state_machine: true
approved_mutation_fault_ledger: true
approved_identity_ledger: true
approved_proof_matrix: true
approved_changed_path_envelope: true
approved_external_mutation_envelope: true
binding_implementation_clarifications:
  - COST_AND_INVALIDATION.json uses schema linux-vst-bridge-pc0-cost-and-invalidation/v1; incompatible PC0 keys are not published under the DX0 v1 label.
  - A failed durable call_started publication means no VST3 call occurred. With no earlier blocker it maps to PC0_EVIDENCE_BLOCKED and permits only accepted physical containment; with an earlier blocker the writer failure is secondary.
implementation_cost_ceiling:
  ordinary_mac_driver_commands: 1
  manually_copied_identifiers: 0
  windows_acceptance_producers_maximum: 1
  positive_deck_batches_maximum: 1
  live_negative_exercises: 0
  accepted_again_rebuilds: 0
  accepted_fixture_reseeds: 0
material_discovery_requires_stop: true
implementation_authorized: true
successor_selection_authorized: false
operator_approval_source: >-
  The operator conveyed approval of exact pc0-design-v2 to the implementation
  agent, which reported that approval back in the current conversation as
  verified against the exact design hashes and technical-lead review 5105712590,
  with both binding implementation clarifications accepted. No additional
  operator approval is required for repository authority finalization.
approved_at: 2026-09-03T12:30:00-07:00
```

## Binding implementation authority

Implementation is authorized only for the exact reviewed `pc0-design-v2` primary claim and envelope: one `PreSetupProcessingContractCensus` owner, nine states, four selected operation types, the canonical eleven-call positive AGain sequence, sixteen focused proof rows, ten blocked outcomes, exactly fourteen source/configuration paths, exactly five evidence paths, one Windows producer maximum, one positive Deck batch maximum, zero live negative exercises, zero accepted AGain rebuilds, zero fixture reseeds, one ordinary Mac command, and zero manually copied identifiers.

The private `linux-vst-bridge-pc0-transaction-result/v1` object binds the true artifact producer P and original Deck execution E without containing or requiring a later consumer C. The tracked `linux-vst-bridge-pc0-evidence-packet/v1` in `TRANSACTION.json` separately binds P/E/C, strict private-result admission, observation disposition, the nested immutable pre-setup processing contract, call facts, quiescence, shutdown, cleanup, protected-state comparison, all sixteen proof dispositions, renderer identity, and acyclic integrity.

`COST_AND_INVALIDATION.json` uses the distinct schema `linux-vst-bridge-pc0-cost-and-invalidation/v1` and owns only external-effect, phase-reuse, invalidation, command-count, copied-identifier, renderer-reuse, and fixture-accounting facts.

A failed durable `call_started` publication means no VST3 call occurred. With no earlier PC0 blocker it is `PC0_EVIDENCE_BLOCKED`; because later release and shutdown cannot be durably evidenced, only accepted physical containment may be claimed. A failed durable `call_completed` publication after ordinary return makes that return/output unconsumable, leaves the exact operation unmatched, and permits no claimed clean in-process retirement.

No `getLatencySamples`, `getTailSamples`, `setupProcessing`, `setBusArrangements`, activation, processing, audio/event buffers, controller, state, parameters, native proxy, C ABI, IPC, Bitwig, Serum, packaging, signing, runner selection, or general compatibility work is authorized.

A material design discovery returns PC0 to the design gate. No successor or design amendment is authorized by this receipt.
