# AP4-A2 — Lead-approved remaining fresh acceptance candidate

The technical lead approves this exact replacement after reviewing the GUI-window repair at `871e074e9ec3d3b6652bbcb7278b5812fdc6d12d`, conditional on a complete AP4-D5 diagnostic pass at that same source. This uses the remaining candidate in the existing two-candidate allowance. It is permission to measure, not product acceptance; AP3 remains accepted pending AP4 review and merge.

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
authorized_source_commit: 871e074e9ec3d3b6652bbcb7278b5812fdc6d12d
authorized_source_tree: 10cfdea375dcbe203657831176ce36de90062fa3
authorized_plan_id: ap4-state-recall-acceptance-v1
authorized_product_contract_identity: ap4-plugin-state-recall-v1
authorized_product_contract_sha256: f8d45e3ae48d573e520a9b03fb1b81de08378feb1c84a25f17b12ac9ddd8d89f
authorized_plan_content_sha256: 1917c0475dd109e4a3430fee1321ee7d676c57f1b33a0c7c2c42a13ad9c38068
acceptance_candidate_identity: fa7dced3d29e9253a1579016af8ff2eb7f952ec47592108f4f01ec42d67697a8
acceptance_batch_budget: 1
accepted_product_frontier: AP3
current_product_target: AP4
```

## Conditions and execution

First reconcile actual spending and confirm that earlier A2 bindings have no consumed or unknown reservation. The operator reports 2/6 Windows producers, 4/10 diagnostics and 1/2 acceptance candidates consumed: the first acceptance stays historical. This replaces only an unreserved A2 binding; it does not alter any transaction. If another A2 reservation exists or its outcome is unknown, reconcile it and do not launch this replacement. Prior receipt revisions remain preserved in Git and in any original delegations.

Run only after AP4-D5's complete real-Windows SDK and Bitwig diagnostic succeeds at this exact source, including normal Quit, fresh-process gain and mute recall, retained observations and final cleanup. The same repaired source must pass ordinary local and read-only preflight checks. An SDK-only pass, a missing GUI result or a forced exit is insufficient. Once all conditions pass, proceed directly without an additional planning or lead-approval handoff.

Use the normal `proof-run.py accept` route from a clean checkout of the bound source, reading this file from the newer authority checkout. The closed operation remains `ap4-state-three-sdk-and-three-bitwig`, now explicitly binding the 180-second GUI post-gate window. Automated-stage, plug-in-call, readiness and cleanup limits remain unchanged. Reuse native manifest `0add784f5f8c996768909792243f7a18ea8d9e08bf148db166fb57d9455fb017`, Windows artifact `9978188887`, and the descriptor's exact AGain/runtime identities. No native or Windows rebuild is needed solely for this timing/authority change.

Obtain fresh actual state bytes, independent restored-audio comparisons, controller readback and the disposable Bitwig project's gain/mute recall across fresh processes. Observe restored values and playback before editing; do not re-enter saved values through a driver. Preserve the same saved project between the batch's reopen stages. Have Moonlight and the action sequence ready before the timed GUI work, perform normal Save/Quit promptly after the required observations, and inspect extended logs after exit. Preserve callback safety, useful failure retention, ownership, preferences/publication restoration and protected state.

This candidate may launch once within the existing task ceiling, not once per segment. Lost acknowledgement or renderer failure requires retained-result recovery, never re-execution. No failed run, diagnostic, consumed count or old candidate is relabelled. Changing the source or descriptor invalidates this exact permission; preserve historical records rather than editing an executed candidate.

Publish one non-draft PR closing #57 only when the full result passes; leave it unmerged. If it does not pass, report the specific failed stage and retained evidence without claiming project recall. Tool, sandbox and administrator approvals still apply; use their normal approval flow rather than working around a denial.
