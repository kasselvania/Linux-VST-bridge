# Current Work: Complete PC0 Acceptance

**One task: connect the repaired, working AGain execution to acceptance, test the complete path locally, run one fresh acceptance workload, retain its evidence, and publish one implementation PR.** Do not stop after adapter preparation when the authorized run can proceed.

Branch: `codex/pc0-acceptance-completion`.
Accepted starting main: `c33c23267fcc1c59606bdad25a5cf1b10a947a54`.
The scoped ruling in `AGENTS.md` authorizes implementation, exact candidate/plan binding by the implementation agent, and one acceptance reservation. No intermediate approval or main-branch merge is required. The previous inactive acceptance notice and historical four-path/topology restrictions do not block this task.

## Starting facts

PR #46 merged as harness maintenance at `3b556ea2712da7694272b8b32720ae3eb62ec654`. Reported diagnostic source `9c2413154b4e4b87a3071d76aafad81ea32e30ee` obtained audio buses 1 in / 1 out, each stereo; event buses 1 in / 0 out; main/default-active BusInfo; and supported returns for kSample32 and kSample64. It exited zero with clean shutdown, process containment, stage retirement and unchanged protected state. The relevant CI suite passed 65 tests.

That private observation is permanently acceptance-ineligible. WA0 remains accepted. PC0 is observed diagnostically, not accepted. PC0-D1 consumed three of eight reservations; its five unused reservations need not be spent. Preserve every old observation, unknown result, intent, lock and campaign record.

`tools/proof-run.py` currently registers only the PC0 diagnostic adapter. An `accept` parser is not an implemented acceptance path. Completing that path is part of THIS task, not another prerequisite project.

## Implementation

Read the existing PC0 selection, design and approval for the product behavior and proof obligations, plus the repaired worker/adapter/runtime helpers and existing renderer. Do not reread unrelated historical process dossiers. Preserve local work and use the prepared branch; do not reopen merged PR #46.

1. Add the smallest PC0 acceptance adapter and closed plan through the existing classified backend and `proof-run.py`. Reuse the existing acceptance interfaces, result admission and renderer where compatible. No second transaction framework, new supervisor copy or new Windows host.
2. Share the repaired supervision/retention/normalization code where needed; thin class-specific entrypoints and storage boundaries remain distinct. Keep the diagnostic command diagnostic. Its old results and failed runs cannot become acceptance evidence.
3. Identify every actual execution-source dependency: the new worker/helpers and any retained frozen Deck source they import. Reuse pinned historical code where practical; do not silently substitute a new source or report only the old stopped source while executing changed helpers. Import required existing product implementation files only where necessary and preserve Windows build-input equivalence; do not merge an archived branch wholesale or restore stale authority.
4. Preserve bounded checkpoints before normalization/retirement and independent error reporting. Reuse the corrected normalizer; do not reintroduce the undefined `expected_calls` reference. Do not weaken product assertions to make acceptance pass.
5. Bind the unchanged deployed runtime/installed selection using the reviewed PR #46 verifier semantics, with actual full observed metadata and declared-input digests recorded separately. Historical WR0 evidence stays unchanged. New executable/selection drift or active runtime staging is not covered.

Keep the existing host artifact **9915439437**, AGain module/bundle, VST3 call sequence, ownership, timeouts, cleanup and protected-state checks. No Windows rebuild/download, fixture reseed, runtime replacement, live negative plug-in tests, Bitwig/Serum launch, processing setup or audio processing.

## Local tests before live spending

Exercise the real acceptance command → adapter → worker → normalizer → result admission → renderer chain with effectful platform/process operations substituted. An empty fabricated success dictionary is insufficient.

Prove a complete synthetic AGain result can be retained and rendered; diagnostic/checkpoint/failure/wrong-class records cannot satisfy acceptance; wrong source/artifact/runtime/plan and malformed or incomplete census/shutdown records are rejected; lost acknowledgement/reinvocation does not duplicate a launch; and renderer-only failure can use the same immutable acceptance result without rerunning the plug-in. Keep the existing 65 relevant tests passing. Use focused tests, not a new test framework or broad build matrix.

## Freeze and run — already delegated

After local validation, commit the actual candidate source. Compute its exact worker/helper/build-input, artifact, fixture, runtime and closed-plan identities. Create `docs/campaigns/PC0_A1.md` as the executable authority with the existing policy's acceptance posture (`status: active_acceptance_candidate`, `authority_phase: implementation`, `change_class: PRODUCT_CONTRACT_CHANGE`, product/live authorization true, `permitted_execution_class: ACCEPTANCE_CANDIDATE`, backend readiness true), the actual bound fields, an `acceptance_candidate_identity`, and `acceptance_batch_budget: 1`.

The machine posture identifies the acceptance lane for the already approved PC0 contract; it does not authorize new VST3 behavior. Record the previously approved product-contract identity and the new plan's computed content hash. Do not invent a digest, copy the diagnostic plan's hash onto different content, or set readiness true before the real path exists and passes tests. Candidate source and authority may be separate commits/checkouts to avoid a self-referential source hash.

This mechanical freeze and binding is explicitly delegated by `AGENTS.md`; do not ask the lead to return and fill in identifiers. Read-only exact-runtime binding, necessary narrow source delivery and private result transfer are included. Credentials remain private and the Deck receives no GitHub token or forwarded SSH agent.

Run the registered acceptance path with explicit `--authority`, `--source`, `--plan` and `--candidate`. Use `--preflight-only` first. Preflight must not reserve or launch. Fix routine preflight/implementation errors locally and rebind source before reservation as needed; this requires no new approval.

Once reserved, the candidate is immutable and exactly one workload is permitted. Reconciliation never relaunches. A failed/unknown acceptance is retained honestly and does not authorize another candidate. Fix reporting-only defects without a new run when the original execution remains valid; record the renderer revision separately. Unknown containment or changed protected state stops live work, not read-only investigation.

## Result and publication

Read the actual fresh acceptance result and verify the complete expected pre-setup census and inherited shutdown. Use the existing PC0 proof obligations and appropriate renderer outputs, but ensure the implementation checks observed values rather than filling in expected constants. Default-active is a bus flag, not proof of activation. Sample-format support is not processed audio.

Retain private raw/checkpoint material privately and commit only sanitized, useful acceptance evidence with correct source/result bindings. Keep exploratory history unchanged. Update current status to `acceptance verified, awaiting technical-lead review` only when that is true; do not mark PC0 accepted or merge your own PR.

Open one non-draft implementation PR against main containing the necessary code, focused tests, exact candidate authority, evidence and concise usage/status notes. No process-only or separate status-closure PR. Return actual bus/arrangement/sample-format results, shutdown/cleanup, tests, acceptance reservation/result identity and PR/head. If blocked, give the literal stage/error and retained facts, not merely INCONCLUSIVE.

Known-buffer processing is the next proposed product milestone after PC0 review; it is not part of this task.

## Authority

The following is the unchanged disabled default CLI circuit breaker while candidate code is prepared. It is not a revocation of the active implementation ruling above. Live execution uses the separately frozen PC0_A1 authority only after the actual path is tested.

```yaml
status: no_active_slice
authority_phase: no_active_slice
change_class: PROOF_HARNESS_MAINTENANCE
maintenance_id: PC0-A1
maintenance_status: implementation_authorized_by_scoped_ruling
repository: kasselvania/Linux-VST-bridge
maintenance_implementation_authorized: true
product_implementation_authorized: false
live_execution_authorized: false
permitted_execution_class: none
classified_backend_core_ready: true
classified_backend_ready: true
production_adapter_registry: pc0_diagnostic_only
accepted_product_frontier: WA0
current_product_target: PC0
pc0_status: acceptance_completion_active_candidate_not_yet_frozen
successor_selection_authorized: false
```
