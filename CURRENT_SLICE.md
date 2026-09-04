# Current Work: PX2 — Class-Aware Proof Transaction Core

## Authority

```yaml
status: active_proof_harness_maintenance
authority_phase: proof_harness_maintenance
change_class: PROOF_HARNESS_MAINTENANCE
maintenance_id: PX2
maintenance_title: Class-Aware Proof Transaction Core
repository: kasselvania/Linux-VST-bridge
basis_commit: 98110016c1d50d578b2cf44d439830442f917b68
basis_tree: 6e0964a6c506a743ce4fc229940f1e6ed982acc5
maintenance_branch: codex/px2-class-aware-proof-backend
maintenance_receipt: docs/maintenance/PX2_CLASS_AWARE_PROOF_BACKEND.md
maintenance_review: docs/maintenance/PX2_MAINTENANCE_REVIEW.md
maintenance_implementation_authorized: true
product_implementation_authorized: false
live_execution_authorized: false
permitted_execution_class: none
classified_backend_core_ready: false
classified_backend_ready: false
diagnostic_campaign_authorized: false
acceptance_candidate_authorized: false
windows_workload_authorized: false
deck_workload_authorized: false
accepted_product_frontier: WA0
current_product_target: PC0
pc0_status: suspended_after_inconclusive_acceptance_attempt
stopped_pc0_source: 309b8918c128c0b9e6701d0453dc841a111d5ac5
stopped_pc0_source_tree: a6a123b554eb220cbdfb9bf9afab3d08d4a8ac92
stopped_pc0_archive_ref: refs/heads/codex/archive/pc0-v3-stopped-309b8918
successor_selection_authorized: false
```

## Operator ruling

The operator directed the technical leads to take the next process-repair step,
prepare the repository, and provide the implementation prompt. That direction
authorizes the local-only PX2 maintenance work described by the exact receipt.
It authorizes no product implementation and no live workload.

## Maintenance claim

PX2 will install one class-aware proof transaction core around closed plan
adapters. The core must durably retain execution class, execution identity,
exact per-run source, acceptance eligibility, and the independently owned
budget before workload launch. Diagnostic and acceptance outputs must be
structurally separate, and failure closure must remain bounded.

PX2 does not add a live PC0 adapter and does not make any live plan executable.

## Required sequence

```text
maintenance receipt already activated
→ one fresh-context focused maintenance review before code
→ local implementation
→ deterministic production-helper tests
→ one non-draft maintenance PR
→ one exact-head technical-lead review
→ merge
```

There is no product-slice selection, product-design revision, product approval,
Windows producer, artifact transaction, Steam Deck contact, diagnostic batch,
or acceptance batch in PX2.

## Completion posture

After PX2 merges, the class-aware core may be marked ready, but live execution
remains disabled until a later exact authority supplies one closed plan adapter,
one DiagnosticCampaignIdentity or AcceptanceCandidateIdentity, and its separate
budget. The accepted product frontier remains WA0.

## Operator frontier

```text
classification: derived_non_authoritative_summary
last_accepted_capability: WA0 — exact IAudioProcessor acquired and retired without invoking a method
current_target: PC0 — initialized-state bus and sample-format contract
current_state: PX2 local-only proof-harness maintenance active; no live execution authorized
next_step: implement and review the class-aware transaction core, then resume PC0 through a bounded diagnostic campaign
explicit_nonclaim: PX2 adds no VST3 capability and PC0 has not advanced the accepted frontier
```
