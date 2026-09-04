# PX2 — Focused Proof-Harness Maintenance Review

```yaml
schema: linux-vst-bridge-proof-harness-maintenance-review/v1
repository: kasselvania/Linux-VST-bridge
maintenance_id: PX2
reviewed_head: 665f98b507b779e8bbada97984ede37b674d48ba
reviewed_basis: 98110016c1d50d578b2cf44d439830442f917b68
reviewed_receipt: docs/maintenance/PX2_CLASS_AWARE_PROOF_BACKEND.md
review_result: PROOF_HARNESS_MAINTENANCE_CLEAR
product_contract_changed: false
live_execution_authorized: false
windows_workloads_observed: 0
deck_workloads_observed: 0
```

## Authority

The reviewed head is a repository-only proof-harness-maintenance authority. It
changes `CURRENT_SLICE.md`, the PX2 receipt, and the factual repository README.
It does not change PC0 product design, VST3 operations or order, product
ownership or lifecycle, Windows product/ABI source, fixture semantics, product
normalization, containment or shutdown claims, security/privacy boundaries, or
accepted evidence.

The accepted product frontier remains WA0. PC0 remains suspended and
unaccepted. No diagnostic campaign, acceptance candidate, Windows producer, or
Steam Deck workload is authorized by PX2.

## Maintenance assessment

The maintenance claim is necessary and bounded. PX1 deliberately blocks live
execution because the existing DX0 transaction backend does not durably own
execution class, acceptance eligibility, independent diagnostic/acceptance
budgets, or a structural separation between diagnostic output and product
evidence. PX2 supplies only that generic transaction core; it does not add a
live PC0 adapter.

The tightened receipt correctly requires:

- a finite legal authority/delegation table rather than a policy language;
- exact product-contract and canonical plan-content binding;
- cheap and read-only validation before workload reservation;
- monotonic durable transaction state;
- real cross-process single-writer budget ownership;
- consumed reservation after crash, timeout, lost acknowledgement, or unknown
  outcome;
- reconciliation without duplicate physical launch;
- stable diagnostic campaign budgets across source revisions;
- one-source/one-batch acceptance candidates;
- structural removal of product-evidence capability from diagnostic paths;
- bounded failure closure with unknown effects retained as unknown;
- explicit direct `run` and `seed-fixture` circuit-breaker subprocess tests;
- multiprocess reservation, crash, lost-acknowledgement, retained-result, and
  renderer-separation tests;
- one Linux-only policy/core CI check;
- main-branch protection after the check exists;
- no Windows or Deck execution during PX2.

## Findings

No product-design return is required.

The prior activation prematurely stated that implementation was authorized
before its required maintenance review existed. The reviewed head corrects that
posture and keeps implementation disabled pending this review and authority
finalization.

The repository README was materially stale: it stopped the accepted boundary at
WR0 and described the retired single acceptance-only process. The reviewed head
corrects that factual projection without creating another authority owner.

Main is presently reported as unprotected. PX2 must add the cheap named check
before protection can require it. Repository protection is therefore a PX2
completion requirement, or an exact administration blocker if the available
GitHub authority cannot configure it.

## Verdict

```text
PROOF_HARNESS_MAINTENANCE_CLEAR
```

The reviewed maintenance authority may be finalized and merged. Engineering
must begin from that merged authority basis. It must remain local-only, must not
modify the prohibited product/runtime paths, and must not enable a live adapter
or workload.
