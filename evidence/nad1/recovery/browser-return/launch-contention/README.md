# Launch preflight contention repair

The first repair waited for the service capacity read, but the action's following inactivity recheck and bridge suspension still acquired `registry.lock` fail-fast. Both share that registry with UI polling. Two real human reopen attempts refused after snapshot validation, before a renderer reservation or resume obligation. Native Access was already retired cleanly; the error did not mean another Native Access process existed.

The retained generic error does not distinguish which of those two sites lost the lock. The amendment fixes both: use the existing bounded operator acquisition, record its wait facts, and recheck durable owners after acquisition. It does not bypass inactivity, reuse stale readiness, retry application dispatch, change Windows service handling, or alter rendering/login behavior.

Generated manager tests execute the production inactivity and suspension owners under separate contended locks, confirm one service-stop obligation and exactly-once restoration, prove timeout occurs before suspension/reservation, and prove a durable active lease refuses even when the capacity input says idle. Only external bridge service I/O is replaced in these tests. No Windows or vendor launch is part of this proof.

Validation before installation: all 84 manager-binary tests and strict all-target manager Clippy passed. The Linux x86-64 release manager built. The three new tests cover both preflight acquisition and suspension, timeout without mutation, and durable-owner revalidation. Windows adapter, supervisor, frontend and renderer policy are byte-unchanged by this amendment.
