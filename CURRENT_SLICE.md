# Current Work: PX0/PX1 — Diagnostic and Acceptance Process Enforcement

## Authority

```yaml
status: process_repair_enforced
authority_phase: no_active_slice
change_class: PROOF_HARNESS_MAINTENANCE
process_repair_authorized: true
product_implementation_authorized: false
live_execution_authorized: false
permitted_execution_class: none
classified_backend_ready: false
repository: kasselvania/Linux-VST-bridge
process_repair_merge: 39cad26ca914c9b89a9f79a3d2c25aabd4c06f30
maintenance_receipt: docs/maintenance/PX1_EXECUTION_CLASS_ENFORCEMENT.md
accepted_product_frontier: WA0
current_product_target: PC0
pc0_status: suspended_after_inconclusive_acceptance_attempt
stopped_pc0_source: 309b8918c128c0b9e6701d0453dc841a111d5ac5
stopped_pc0_source_tree: a6a123b554eb220cbdfb9bf9afab3d08d4a8ac92
stopped_pc0_archive_ref: refs/heads/codex/archive/pc0-v3-stopped-309b8918
successor_selection_authorized: false
```

## Enforced posture

The human process now distinguishes product-contract change, proof-harness
maintenance, and mechanical maintenance from read-only, diagnostic, and
acceptance execution classes.

The ordinary legacy live entry point is disabled. Cheap `plan` and `validate`
operations remain available. No live diagnostic or acceptance operation can run
until an exact authority names its class, source, closed plan, identity and
budget and a class-aware backend durably retains those facts.

## Mandatory development sequence

```text
approve the product contract once
→ implement and validate locally
→ use a bounded non-authoritative diagnostic campaign when needed
→ repair the harness without reopening product design
→ freeze one exact acceptance candidate
→ run one strict acceptance transaction
→ audit successful acceptance evidence only
```

## PC0 disposition

PC0 is suspended, not rejected. No stopped PC0 source or observation is
acceptance eligible. The accepted product frontier remains WA0.

## Operator frontier

```text
classification: derived_non_authoritative_summary
last_accepted_capability: WA0 — exact IAudioProcessor acquired and retired without invoking a method
current_target: PC0 — initialized-state bus and sample-format contract
current_state: process repaired and live execution hard-gated; PC0 suspended; no live authority
next_step: implement a class-aware diagnostic backend through proof-harness maintenance before resuming PC0
explicit_nonclaim: PC0 has not advanced the accepted product frontier
```
