# Project decisions and evidence

`AGENTS.md` contains the working rules. `CURRENT_SLICE.md` names the current outcome. This document explains how decisions and evidence are used; it does not add another execution gate.

## Decisions

The operator sets goals and approves consequential scope, spending and security changes. The technical lead selects useful work, closes consequential public-behavior decisions and reviews results. The engineer implements and tests within that scope, including routine repairs. The operator's latest direction takes precedence over older repository process instructions.

Normal work is: agree on an outcome, implement and test, then review one PR. There is no mandatory sequence of selection receipts, adversarial design approvals, diagnostic campaigns, frozen acceptance candidates and separate closure PRs. Use a short design note only when a real decision needs it. Do not make the number of owners crossed determine the number of tasks.

## What evidence means

Record the tested code/build, relevant environment, action, observed result and limitations. Source/build identifiers identify evidence; ordinary changes within an approved task do not require renewed permission for every commit.

A successful development test may support a bounded claim directly. An incomplete or failed test cannot. A mock establishes local behavior, not live Windows or DAW compatibility. A reviewer may ask for an additional test for a specific unresolved risk, not just to move an observation between procedural categories.

Historical receipts and acceptance packets remain unchanged, including their original labels, failed attempts and consumption counts. Do not change a legacy diagnostic's `acceptance_eligible` field or claim that its old acceptance protocol passed. Under the present process, review can consider its actual observed facts alongside code and other evidence, without pretending the historical record is something else.

## Cost and safety

Reuse unchanged artifacts. Keep explicit user spending limits and useful run accounting; do not invent a universal two-attempt debugging limit. A retry should answer a question or verify a repair. Preserve partial results so a failed GUI/reporting step does not erase correct sample measurements.

Resource ownership, real-time behavior, data protection and useful errors remain engineering requirements. Tool, sandbox and administrator permissions remain separate from project decisions. Updating a repository document cannot grant a prohibited tool operation.

## Review and completion

Review the actual diff and supporting results against the requested outcome. State what was not independently reproduced. Merge only within the user's authority. Update accepted capability and remaining limitations in the same PR where possible; a separate status-closure cycle is unnecessary.

Legacy transaction tooling remains available for exact old receipts and reporting. Its private state is not migrated or reset by this policy change. Retiring a development ceremony neither certifies unfinished product behavior nor removes the product's integrity and cleanup checks.
