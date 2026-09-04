# Current Work: PX0 — Diagnostic Development and Acceptance Separation

## Authority

```yaml
status: active_process_repair
authority_phase: documentation_and_governance
change_class: PROOF_HARNESS_MAINTENANCE
process_repair_authorized: true
product_implementation_authorized: false
live_execution_authorized: false
permitted_execution_class: READ_ONLY_RECONCILIATION
repository: kasselvania/Linux-VST-bridge
basis_commit: 7ed1fbf985b5bb717883e0cd2132620b960e3aa6
basis_tree: 07efd2d95c7f72205ac5c201361c4291bb8eaf6d
repair_branch: codex/px0-diagnostic-acceptance-process-repair
accepted_product_frontier: WA0
current_product_target: PC0
pc0_status: suspended_after_inconclusive_acceptance_attempt
pc0_product_claim_changed: false
stopped_pc0_source: 309b8918c128c0b9e6701d0453dc841a111d5ac5
stopped_pc0_source_tree: a6a123b554eb220cbdfb9bf9afab3d08d4a8ac92
stopped_pc0_archive_ref: refs/heads/codex/archive/pc0-v3-stopped-309b8918
prior_failed_pc0_source: 7ac6095a488d0077fcc78fedc5abd870b5ffb1cb
prior_failed_pc0_archive_ref: refs/heads/codex/archive/pc0-v2-failed-7ac6095a488d
successor_selection_authorized: false
```

## Operator ruling

The current product implementation was stopped after an extended failed attempt because the repository treated proof-harness development and failure reporting as acceptance-grade work. The operator directed the technical lead to repair the repository process and prevent recurrence.

No PC0 source from the stopped attempt is accepted. No PC0 evidence exists. The accepted product frontier remains WA0.

## Process repair claim

The repository will distinguish:

```text
change class:
  PRODUCT_CONTRACT_CHANGE
  PROOF_HARNESS_MAINTENANCE
  MECHANICAL_MAINTENANCE

execution class:
  READ_ONLY_RECONCILIATION
  DIAGNOSTIC_NON_AUTHORITATIVE
  ACCEPTANCE_CANDIDATE
```

A proof-harness-only defect will no longer require a new product constitution. Diagnostic runs will be permanently acceptance-ineligible and separately budgeted. Only one exact frozen acceptance candidate may produce product evidence.

## Mandatory reduced sequence

```text
approve product contract once
→ implement and validate locally
→ run at most two bounded AGain diagnostic batches when needed
→ repair harness under maintenance authority
→ freeze one acceptance candidate
→ run one strict acceptance transaction
→ audit successful evidence only
→ merge or return to product design only for a product-contract change
```

## Failure closure

A failed diagnostic or inconclusive acceptance attempt may produce only:

1. one bounded diagnostic JSON object and sidecar;
2. cleanup and protected-state disposition;
3. actual effect counts;
4. one concise failure report.

It must not trigger a success evidence packet, full proof-matrix replay, full pre-PR audit, or product-design amendment merely to document failure.

## PC0 disposition

PC0 is suspended, not rejected.

The pre-setup processing-contract product claim remains a sensible next capability. It may resume only after:

1. this process repair is accepted;
2. the proof driver enforces distinct diagnostic and acceptance classes;
3. the stopped source is treated only as archived development history;
4. a new bounded DiagnosticCampaignIdentity is created if real-fixture debugging remains necessary;
5. a fresh AcceptanceCandidateIdentity is frozen only after diagnostic development succeeds.

No old diagnostic or failed acceptance observation may be promoted.

## Operator frontier

```text
classification: derived_non_authoritative_summary
last_accepted_capability: WA0 — exact IAudioProcessor acquired and retired without invoking a method
current_target: PC0 — read initialized-state buses, BusInfo, speaker arrangements, and kSample32/kSample64 support
current_state: PC0 implementation stopped after an inconclusive harness/development failure; process repair active; no live execution authorized
next_accepted_capability_if_pc0_later_succeeds: exact pre-setup audio/event bus and sample-format contract
explicit_nonclaim: PC0 has not advanced the accepted product frontier
```

## Prohibited during PX0

- Windows workflow or build;
- artifact download or custody;
- source or artifact transfer to the Deck;
- Runtime, Proton, Wine, Bitwig, Serum, scanner, diagnostic, or acceptance workload;
- PC0 product implementation;
- promotion of stopped PC0 work;
- successor selection.

Read-only Git and repository-state inspection remain permitted.