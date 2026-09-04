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
maintenance_review_status: pending
maintenance_implementation_authorized: false
product_implementation_authorized: false
live_execution_authorized: false
permitted_execution_class: none
classified_backend_core_ready: false
classified_backend_ready: false
diagnostic_campaign_authorized: false
acceptance_candidate_authorized: false
windows_workload_authorized: false
deck_workload_authorized: false
policy_core_ci_required: true
main_protection_required: true
accepted_product_frontier: WA0
current_product_target: PC0
pc0_status: suspended_after_inconclusive_acceptance_attempt
stopped_pc0_source: 309b8918c128c0b9e6701d0453dc841a111d5ac5
stopped_pc0_source_tree: a6a123b554eb220cbdfb9bf9afab3d08d4a8ac92
stopped_pc0_archive_ref: refs/heads/codex/archive/pc0-v3-stopped-309b8918
successor_selection_authorized: false
```

## Operator ruling

The operator directed the technical leads to complete the process correction,
inspect the detailed PX2 handoff, fix safe authority and documentation defects,
and provide bounded engineering prompts. This authorizes repository-only PX2
maintenance preparation. It authorizes no product implementation and no live
workload.

## Maintenance claim

PX2 will install one class-aware proof transaction core around closed plan
adapters. The core must durably retain execution class, product contract, exact
plan digest, execution identity, exact per-run source, acceptance eligibility,
and the independently owned budget before workload launch. It must use real
cross-process single-writer reservation, monotonic crash-recoverable state,
structurally separate diagnostic and acceptance outputs, and bounded failure
closure.

PX2 does not add a live PC0 adapter and does not make any live plan executable.

## Required sequence

```text
exact PX2 receipt and factual README correction
→ one focused fresh-context maintenance review
→ merge maintenance authority
→ implement locally from the merged authority basis
→ deterministic production-helper and multiprocess tests
→ cheap Linux-only policy/core CI
→ one non-draft maintenance PR
→ one exact-head technical-lead review
→ merge
→ configure main protection against direct/force/deletion bypass
```

There is no product-slice selection, product-design revision, product approval,
Windows producer, artifact transaction, Steam Deck contact, diagnostic batch,
or acceptance batch in PX2.

## Completion posture

After PX2 merges, the class-aware core may be marked ready, but live execution
remains disabled until a later exact authority supplies one closed PC0 adapter,
one `DiagnosticCampaignIdentity` or `AcceptanceCandidateIdentity`, and its
separate budget.

The accepted product frontier remains WA0.

## Operator frontier

```text
classification: derived_non_authoritative_summary
last_accepted_capability: WA0 — exact IAudioProcessor acquired and retired without invoking a method
current_target: PC0 — initialized-state bus and sample-format contract
current_state: PX2 authority awaiting focused maintenance review; no implementation or live execution authorized
next_step: review and merge PX2 maintenance authority, then implement the local-only class-aware transaction core
next_product_step_after_px2: add one closed PC0 adapter and use a bounded non-authoritative diagnostic campaign before one fresh acceptance candidate
explicit_nonclaim: PX2 adds no VST3 capability and PC0 has not advanced the accepted frontier
```
