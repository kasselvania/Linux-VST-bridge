# Product decisions, ownership and completion

`AGENTS.md` defines the engineering method. `CURRENT_SLICE.md` is the short current
task pointer. The architecture defines shared product contracts; failure records
and the support matrix describe their implementation and observed coverage.
Historical task selections are retained context, not current commands.

## Responsibility and authority

The operator sets the product goal and decides consequential scope, spending,
security and destructive changes. The engineer owns the architectural reasoning,
implementation, integration and verification needed to deliver that goal. Routine
choices do not require another operator instruction. Current operator direction
takes precedence over repository process text; genuine conflicts must be surfaced.

Work is organized around reusable capabilities and complete user journeys. A
plug-in incident can expose a shared design problem; it does not automatically
become a vendor-specific task or dictate the architecture. Review across owners
when the failure or requirement crosses them. A small commit can be part of a
larger coherent capability; narrowing the commit must not quietly narrow the goal.

Architecture is revisable through explicit, evidence-based decisions. Existing
code and previous proofs have no exemption from the product contract. Review
whether to retain, repair, replace or retire an implementation without defaulting
to either endless local workarounds or a new engine. Record consequential decisions
in the existing architecture/decision documents, not a parallel authority system.

## Evidence and completion

Report source correctness, built artifacts, installed behavior, platform coverage
and release readiness separately. Reference fixtures demonstrate their contracts;
commercial and physical results demonstrate their declared conditions. Unknown
compatibility remains distinguishable from missing capability or actual failure.
A partial result is useful, but required unfinished integration remains unfinished.

Qualify the measurement path and preserve original observations. Correct reporting
errors without rewriting history, erasing failures or reclassifying a failed test
as a pass. Reuse unaffected results; test affected boundaries and the complete
promised workflow. Only change agreed acceptance scope through an explicit product
decision. A green test suite or reviewed PR cannot substitute for that workflow.

## Delivery and cost

Keep one current task pointer and one support/failure record system. Commit and
push coherent work with accurate remaining gaps. Review before merge and publish
only supported claims. A policy change must be independently reviewable from an
unfinished runtime or beta implementation when their acceptance differs.

Respect existing machine custody, resource and spending limits. Use inexpensive
readback and supported tooling; reserve physical runs for questions that need them.
Tool approvals, privacy, licensed state and ownership protections remain in force.
New process text cannot grant a prohibited operation or require pointless reruns.
