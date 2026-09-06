# Development: outcome, implementation, review

The process serves a working bridge. `AGENTS.md` owns the rules; this page is practical guidance, not another approval checklist.

## Prepare useful work

Describe the user-visible outcome, important constraints, relevant starting code and an observable completion check. Keep the agent prompt short. Settle consequential interface, ownership or failure behavior, but leave private algorithms and organization to the engineer. Select work that advances the product, not merely the easiest fact to certify.

Before handing off, remove obsolete task restrictions that would prevent the requested work. Necessary builds, focused tests, ordinary debugging and integration repairs are part of implementation. Do not require the engineer to author a document that appears to grant itself permission. Repository decisions do not override platform/tool permissions; use the normal approval flow for those.

## Iterate normally

Run the smallest useful test, inspect its result, repair the relevant code and continue. Record enough source/build/environment information to reproduce a result. Reuse unchanged artifacts; no mandatory clean rebuild or whole-suite replay after each edit. Run broader regression coverage before review when affected interfaces warrant it.

Existing plain compiler, Cargo, CMake, Python and supervised device-test commands are legitimate development paths. Use the existing supervision/cleanup helpers for Windows and DAW sessions; do not introduce another orchestration framework just to avoid the older one. No receipt file is required by current policy for each revision. `tools/proof-run.py` retains its exact historical protocol for users who deliberately choose it; its closed default does not govern ordinary work.

Examples of local checks from the repository root (choose the ones affected):

```sh
python3 -m unittest tools.test_proof_cli tools.test_proof_execution_policy
cargo test --manifest-path native-audio-client/Cargo.toml --locked --offline
cargo test --manifest-path native-vst3-proxy/backend/Cargo.toml --locked --offline
```

These commands test existing code; they do not by themselves prove Deck or Bitwig compatibility. Use the task's known-good platform build/test invocation for live checks and retain the actual inputs and result. Do not route around a platform denial by choosing a different helper.

## Make failures useful

Retain the original error, the completed portions of the test and cleanup disposition. A report-generation failure is not evidence that DSP failed. A GUI deadline missed by the operator/agent is not a plug-in performance measurement. Separate plug-in-call timeouts, application shutdown and interactive pacing. Use a reasonable bounded window or readiness coordination for GUI work; do not turn Save/Quit into a speed test.

Fix the failed part and reuse still-valid observations. Repeat an entire test only when the change or uncertainty affects that entire result. Do not spend a run merely to turn a diagnostic label into an acceptance label. Never claim a missing result was observed, reset consumed counts, or automatically replay an operation whose remote outcome is unknown.

There is no default two-try doctrine. Keep explicit resource limits, avoid blind retries, and ask before substantial unapproved spending. A bug in a local checker or receipt does not require a new product-design cycle.

## Review once against the goal

A normal PR should explain what changed, what actually worked, what was tested and what is still unsupported. A successful development observation can be reviewed directly if it covers the delivered code and intended claim. More testing needs a reason. Mocked, SDK-host and actual DAW results should be distinguished, not forced into separate mandatory campaigns.

The reviewer examines correctness, ownership, callback safety, failures and evidence relevant to the change. Use an additional independent review for a consequential risk when useful, not as an automatic chain of approvals for every patch. Status closure belongs in the same PR where practical. Do not advance a claim beyond the actual demonstrated behavior.

## While another agent is working

Keep unrelated cleanup on a separate branch. Do not replace the working agent's files, rebase it or mutate its device session. Communicate a short policy change without requiring a restart. Reconcile documentation at merge; do not overwrite implementation to resolve a process-document conflict.

## Historical records

Campaigns, old design/maintenance receipts and prior evidence remain at their paths so references and recovery continue to work. Their original execution identities, labels and limits are historical facts, not templates for new development. Never edit an executed record to make a new run look old or successful. The original workflow remains available in Git history; no new archive-copying project is needed.
