# Final sealed service campaign

Executed source `d75d3085e2b55d5939cf56f628535d6d2297efa3`, tree
`04e381e39a64ef90d70b6b05e4d13e80b99427fa`, package seal
`a69a1c4839258a770388587c9bf42cc587b771398c409d57609fff5efe8f8b30`.

| Initial state | Result |
| --- | --- |
| Absent | Exact installer, registration, SCM start, verified readiness, retirement |
| File present / service unregistered | Exact installer and registration, verified readiness, retirement |
| Service stopped | SCM-only start, verified readiness, retirement |
| Already running and ready in the exact owned cohort | Verified readiness with no additional install/start, retirement |
| Running without admitted endpoints | Refused; no reinstall; positive cleanup |
| Cancellation while not ready | Exact manager lifecycle Stop; cancelled; positive cleanup |

All six cases have zero owned survivors, retired units, absent exact cgroups and
removed disposable prefixes. The Windows endpoint-owner mask is 3 only for the
verified pair. Readiness and helper exit remain independent. The production service
lifetime is bounded; no persistent unmanaged daemon is introduced.

`custody.json` retains exact private before/after and ledger hashes, all development
and final operation retirement, unchanged installed software, 23 predecessor files,
unchanged real registry, absent real daemon and retired A/B/C/dependency-failure
application operations. `state.json` confirms all 22 generated units/cgroups are gone,
the installed schema remains 6, service is active, both keepers are healthy, leases,
transactions and stale transports are zero, and capture is off.

All 307 retained witnesses and five projects match. Runtime-cache contents match;
6,701 shared inode ctime changes are retained separately from unchanged Windows
prefix and private-HOME metadata. No blanket whole-environment metadata-equality
claim is made. No commercial launch, real dependency mutation, production installation,
registry mutation or product/publication change occurred.

These are generated service/process facts, not proof that the proprietary daemon's
installation, endpoints, authorization or application integration work. The exact
real prefix remains mutation-gated for independent review.
