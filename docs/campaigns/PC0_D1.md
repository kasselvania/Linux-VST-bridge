# PC0-D1 — Repair Result Retention and Obtain the AGain Observation

## Authority

```yaml
status: active_diagnostic_campaign
authority_phase: proof_harness_maintenance
change_class: PROOF_HARNESS_MAINTENANCE
campaign_id: PC0-D1
repository: kasselvania/Linux-VST-bridge
product_implementation_authorized: false
maintenance_implementation_authorized: true
live_execution_authorized: true
permitted_execution_class: DIAGNOSTIC_NON_AUTHORITATIVE
classified_backend_core_ready: true
classified_backend_ready: true
acceptance_eligible: false
authorized_source_commit: 2ff23fe74b5501b633b575f7a903289ec2db1866
authorized_source_tree: c82270184beab47bcfd4e9676a701bede8e9df14
authorized_plan_id: pc0-pre-setup-processing-contract-diagnostic-v1
authorized_product_contract_identity: pc0-selection-v2
authorized_product_contract_sha256: daa042e4184cb5fffdf1ff08d59cc51f7755b4635c85e02597adc2584c4b4c1d
authorized_plan_content_sha256: a5303e12d644fefba2ca4003555ebe30f578e60c0a334e5f0bb96b7498decc92
diagnostic_campaign_identity: be62c45243de799f2474b26d4b20ac3593f16fc0392f58f42a10ce922500281b
diagnostic_batch_budget: 8
diagnostic_campaign_authorized: true
acceptance_candidate_authorized: false
windows_workload_authorized: false
deck_workload_authorized: true
accepted_product_frontier: WA0
current_product_target: PC0
```

The source fields above identify the last executed source, NOT a direction to rerun it. Before the next live reservation, commit and test the retention/budget repair, then update only these source fields to that exact commit/tree. This source rebinding is explicitly delegated to the implementation agent; it requires no new approval. Use separate clean source and authority checkouts if needed by the existing command. Do not change campaign, fixture, plan, product-contract identities or ceiling while rebinding source.

## Technical-lead decision

The operator directed the technical lead to get this stalled task completed. The recovery ruling in `AGENTS.md` and `GOVERNANCE.md` authorizes one engineer to repair, test and continue on **the existing PR #46 / `codex/pc0-d1-observation` branch**. No new infrastructure slice, product-design card, pre-run independent-review handoff, or additional permission request is needed for work inside this scope. The branch is an authorized development lane before merge; `main` remains the accepted baseline, not the required source for these repairs.

The SAME campaign's ceiling is increased from two to **eight total diagnostic reservations**. Two are already consumed, so at most six additional reservations are available, less any later consumption. This is not a fresh campaign, not eight new attempts, and not acceptance authority. Do not reset the ledger or make inconclusive history disappear. The higher limit is unusable until the actual policy and stored-budget handling are repaired and tested below; do not bypass those controls.

Immediate goal: the retained AGain host must either report its existing pre-setup bus/sample-format contract or leave the exact useful error explaining why it could not. Success is a plug-in observation, not a larger test count or another repaired launcher.

## 1. Preserve current work and reconcile only what is necessary

Fetch the existing branch and preserve unknown local modifications. Do not reset, force-push, recreate the branch, discard local work, rewrite old receipts or remove the unknown reservation's intent/lock.

Reuse host artifact **9915439437**, the existing accepted AGain fixture and the retained deployed source/artifact stores. No Windows producer, artifact download, fixture reseed, custody reconstruction, Steam-file restoration or runtime replacement is authorized. Read-only access diagnosis and current process/stage/protected-state readback are authorized. Existing failure-pair inspection is permitted to diagnose the retention defect; do not repeat the workload to recover the old result.

The prior readback reported the exact stage absent, the process guard clear, and protected state matching the first batch. Establish these CURRENT containment facts again before any new reservation. They do not establish what the old run returned. If they cannot be established, repair/investigate locally or read-only; do not launch a new workload.

## 2. Fix the existing worker's loss of diagnostic information

Relevant files are `tools/pc0_diagnostic_worker.py`, `tools/pc0_diagnostic_primitives.py`, `tools/pc0_proof_adapter.py` and their existing tests. Change only what the repair needs. Keep the current host artifact and VST3 calls unchanged; do not start a generic telemetry or execution framework, and do not add another copy of the supervisor.

The reviewed failure path obtains an observation, attempts normalization, swallows exception details, retires the stage and only then builds/validates a failure projection. A failure in that projection can return `deck_workloads: 1, diagnostic_publications: 0` without retaining the observation. Fix that ordering and exception handling.

- Retain a bounded privacy-safe checkpoint outside the disposable stage BEFORE interpretation or retirement can lose it. Where necessary capture the existing supervisor's bounded event/tail information incrementally; product call behavior and timeout/containment laws do not change.
- Record the actual stage reached, last available call/event, original failure class and sanitized error detail. For a Python error, retain exception type and repository-relative module/function/line information and a safe cause chain. No frame locals, credentials, environment dumps, private host paths, account identities or arbitrary unsanitized exception representations.
- Treat operation result, interpretation/validation result, process containment, stage retirement and protected-state readback as separate facts. Preserve the primary failure when cleanup or reporting also fails.
- The fallback error record must not depend on the same normalizer or validator that just failed. A structured record can remain inconclusive and acceptance-ineligible while still being useful.
- Validate before accepting claims, not before saving diagnostic information. Keep the final observation/sidecar and bounded checkpoint consistent. Use existing file helpers or ordinary file operations, not a new transaction framework.
- If a write fails, preserve any earlier checkpoint and return a bounded explicit persistence error through the existing response/stderr path. Do not claim durable publication when it failed or claim certainty after an unobservable process death.
- Always perform existing owned-process cleanup; retire stage files only after current process containment is established and available observations have been retained. Do not keep live children merely to save a log. Never relaunch the same reservation to make a missing receipt appear.

The diagnostic-copy structural tests may be narrowed/updated for necessary diagnostic capture hooks, but must continue checking that the product operation roster, ordering and containment guarantees are unchanged. Do not bypass failing product assertions.

## 3. Repair both budget enforcement seams

`tools/proof_execution_policy.py` currently rejects diagnostic maxima outside `{1, 2}`. `tools/classified_proof_backend.py::_load_or_initialize_budget` separately treats a changed maximum as stable-binding drift. Correct BOTH using the explicit authority above.

The implementation must validate a positive finite integer ceiling from explicit current authority, retain the normal default where no higher ruling exists, and keep acceptance budget/eligibility unchanged. It must not introduce an unbounded retry mode.

Within the existing budget lock and atomic-file path, support this authorized maximum increase from 2 to 8 for the unchanged campaign. Validate all other stable identity fields and the existing reservation/count consistency. Retain a small before/after adjustment record tied to the authority; do not build a budget-management subsystem. Preserve old transactions, delegations, nonces, intents, locks, classifications and consumed count byte-for-byte where applicable. Reject unrelated identity changes, corruption and an attempted reset/decrease. Do not hand-edit or delete `budget.json` to get a fresh budget.

Read old transactions against their original recorded delegation when reconciling them. Do not overwrite historical delegations with the new authority. A new tested source gets a new reservation; reconnecting, re-reading or reconciling an old reservation must never launch again.

## 4. Test the real failure path locally before another live run

Extend the EXISTING local worker/adapter/backend tests. Exercise the actual worker control flow with only effectful platform/process primitives substituted:

- normal observation survives normalization and final publication;
- supervisor exception retains useful stage/error context and still performs valid cleanup;
- normalization failure preserves the original observation;
- failure-projection validation failure retains an independent diagnostic;
- retirement failure does not replace the primary error;
- publication/sidecar failure and lost acknowledgement cannot trigger a duplicate launch;
- the 2-to-8 extension preserves two consumed reservations and the unknown history, permits only the remaining six, rejects a ninth total, and is idempotent;
- stale authority, wrong campaign/plan/fixture, malformed budgets and acceptance eligibility remain rejected as before.

Use temporary local state. Local synthetic tests and read-only inspection consume no live reservation. Existing relevant policy, worker, adapter, runtime and backend suites must pass. No Windows build or Steam Deck workload is needed to reproduce the result-retention branch.

## 5. Continue through the actual AGain diagnostic

Do not stop after passing the tests. Commit the source repair, update the source binding in this receipt, and use the existing command:

```text
python3 tools/proof-run.py diagnose --authority <current-PC0_D1-receipt> --source <tested-source-commit> --plan pc0-pre-setup-processing-contract-diagnostic-v1 --campaign be62c45243de799f2474b26d4b20ac3593f16fc0392f58f42a10ce922500281b
```

`--preflight-only` remains read-only and reserves nothing. Do not invoke the retired downloaded launcher or legacy acceptance driver. Resolve routine access/configuration defects under this same task without requesting another permission round; do not weaken SSH or share credentials.

After the first new diagnostic, read the retained output immediately. An ordinary in-scope harness error is repaired locally, regression-tested, source-bound and retried within the remaining allowance. Each new live reservation needs a concrete tested fix or documented new hypothesis; never repeat unchanged input hoping for a different result. If a post-repair run again has no usable diagnostic, stop live consumption and repair locally. Two identical failures without new information are a reason to change the diagnosis, not spend more reservations.

The previous runtime metadata repair may remain: distinguish observed Steam bookkeeping from declared executable/build/depot/runtime selection. Do not silently accept executable drift or modify Steam's managed files.

## Completion and stop conditions

A successful diagnostic reports the actual audio/event bus counts and BusInfo, speaker arrangements, kSample32/kSample64 returns and clean shutdown/containment. Keep it private and acceptance-ineligible. The accepted frontier stays WA0. Leave PR #46 open for technical review; update its title/body to describe the complete recovery and the ACTUAL result. Do not open another maintenance PR merely because this task had intermediate repairs.

Stop for a genuine product-call/lifecycle change, changed protected/runtime state, unresolved current containment, a required new producer/deployment/commercial fixture, exhausted total allowance, or a physical/credential action only the operator can perform. Do not call ordinary code defects new product-design decisions.

Return a short report: the literal plug-in result or precise error stage/message, what was fixed, actual cumulative reservations, cleanup/protected-state result, PR and source head. Do not hide a failure behind `INCONCLUSIVE`, hash lists or test counts alone. There is no acceptance execution, evidence promotion, audio-processing expansion or successor selection in this task.

## Retained history — not rewritten by the recovery ruling

The first reservation `4008dc6b0628dea1157628a3e04108caaba478e6954c56b900a37bb0c3d65a6f` at source `1e4be8a1386385380b7d240241d3cabd6793cea2` reached host readiness but stopped before plug-in calls because the ready-gate verifier was missing its runtime-identity argument. Containment and retirement completed. It remains consumed.

Source `2ff23fe74b5501b633b575f7a903289ec2db1866` repaired that call site. The second reservation `b22bc4c5898093dc9784dc08545dc1e5fa4b3bf1354548de15df6db0cb489ddc` reported one workload and zero publications. One reconciliation found no observation/sidecar and did not relaunch. Its private failure pair was validated. It remains consumed and `OUTCOME_UNKNOWN`; no AGain contract result is inferred.

PR #46 at pre-recovery head `c3cb7ef36ff409f197e78585481ae1c81266e371` retains the diagnostic runtime-identity repair and 54 reported passing tests. Readback found the stage absent, process guard clear, stopped worktree clean and protected snapshot equal to the first batch. Those readbacks do not reconstruct the second run's missing execution receipt.
