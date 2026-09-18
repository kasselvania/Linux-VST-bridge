# Recovery repair installed

PR #120 head `80f9862bd135a054c2128677a8b2de432bb9665c` merged as
`a6ea9a57358dec1b188f1b02cf47c532e70182c3` with exact tree
`dca72e2076e23fd75f7a3dd0c190c8769fdea915`. Merged-main AP8, AP12 and PX2 passed.
The adapter was selected from that AP8 run and verified against its inventory.

One immutable generation was installed through the idle stop/setup/restart route.
Operator model remains 7; the frontend and manager agree. The dependency persisted
spec uses schema 2, a separate version axis, to retain the no-reinstall recovery mode.
Both previous immutable generations remain unchanged. The daemon executable, offline
service registry, Native Access registration, products, publications, imports,
environments, prior operations and protected projects remained unchanged during installation.
Service is active with two keepers, zero leases/transactions/stale transports, capture
disarmed and no resume obligation. Native Access remains closed.

This installation receipt makes no real daemon-readiness or application-connectivity
claim. A subsequent explicitly authorized recovery test has its own operation and receipt.

Local release building initially selected the wrong Rust compiler and then a denied
default Zig cache. Neither failure reached the Deck. Explicit Rust 1.95.0 and a
workspace-writable Zig cache produced the installed release; these were build
configuration corrections with no source changes.
