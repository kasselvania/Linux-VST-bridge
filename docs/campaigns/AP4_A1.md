# AP4-A1 — Lead-issued fresh acceptance candidate

Issued alongside AP4-D1 by the technical lead under the operator's existing AP4 task and explicit request to unblock execution. This authorizes one fresh full acceptance batch at the exact source below after the diagnostic and readiness conditions pass. It records permission to measure, not a successful measurement or product acceptance. AP3 remains accepted until AP4 review and merge.

## Authority

```yaml
status: active_acceptance_candidate
authority_phase: implementation
change_class: PRODUCT_CONTRACT_CHANGE
repository: kasselvania/Linux-VST-bridge
product_implementation_authorized: true
maintenance_implementation_authorized: false
live_execution_authorized: true
permitted_execution_class: ACCEPTANCE_CANDIDATE
classified_backend_core_ready: true
classified_backend_ready: true
acceptance_eligible: true
authorized_source_commit: da3e0f3c1b0afbaa405abdf97451614969480531
authorized_source_tree: e5a130579f2513e1d2f90d8acfaad657ffd40c5d
authorized_plan_id: ap4-state-recall-acceptance-v1
authorized_product_contract_identity: ap4-plugin-state-recall-v1
authorized_product_contract_sha256: f8d45e3ae48d573e520a9b03fb1b81de08378feb1c84a25f17b12ac9ddd8d89f
authorized_plan_content_sha256: a2f7cc3edb79da2ae7a671dfd0313fa0eca5ab2ac8b245726d31742b10e7ad64
acceptance_candidate_identity: dcf680a1ccdeb4cbaafff60181a8a28126e793779c8629b8a801dc570c42cbcb
acceptance_batch_budget: 1
accepted_product_frontier: AP3
current_product_target: AP4
```

## Prerequisites and bounded work

Use only after the AP4-D1 real-Windows diagnostic at this same source succeeds, local state/controller/failure checks pass, the intended disposable Bitwig project and Moonlight/SSH path are ready, and ordinary read-only preflight verifies actual source, artifact, fixture, runtime, containment and remaining allowance. Do not treat the already reported substituted-peer sample comparisons as Windows or Bitwig evidence. A failed diagnostic or source-changing repair makes this candidate unsuitable; it is not permission to try acceptance as a substitute for debugging.

Run the existing `proof-run.py accept` route from the clean exact source checkout, reading this authority from a separate checkout as needed. Its closed operation is `ap4-state-three-sdk-and-three-bitwig`. Windows artifact `9978188887`, native manifest `24f40e2509a464188ed53b89d86c5a44c6c879a8cb425ffe385c0935e12c110c`, the exact AGain module and the selected Proton 11.0-2c / Runtime 4 composition remain bound by the source descriptor. No rebuild is required merely because these two authority files were added.

Obtain the actual state bytes, restored numerical audio/controller behavior and the same saved disposable Bitwig project's gain and mute recall across fully fresh processes. Do not re-enter saved values through a driver or sidecar. Preserve callback safety, fail explicitly on uncertain restoration, and retain evidence before cleanup/reporting can lose it. Restore changed preferences and remove only temporary publication; preserve the private saved test project and existing installations.

The unchanged task ceiling is 6 Windows producers, 10 diagnostic batches and 2 acceptance candidates. This is one of those two candidates, not two executions of one candidate. The batch remains within the work order's eight serial Windows instances, ten minutes of active audio and bounded waits. Do not spend the second candidate without the required concrete tested repair and properly approved exact binding. Retain every consumed reservation and prior outcome; a lost acknowledgement requires reconciliation, never replay.

Open one non-draft PR closing #57 only after full verification passes; leave it unmerged. If reporting alone fails after a valid retained result, recover that result without another workload. If code or bindings change, do not edit an executed candidate or silently run new source under this receipt.

These repository permissions do not override tool/sandbox/administrator policy. Any remaining automatic-approval refusal requires the normal specific user approval, not a safeguard bypass or another governance project.
