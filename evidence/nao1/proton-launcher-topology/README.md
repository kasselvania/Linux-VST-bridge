# NAO1 Proton launcher topology repair

Executable source: `e4a6bfa471308cf50917d361f02c8fe4d594ff18`

Executable tree: `65a88be6f260efbd6d2125ee5d38ac2854b027d4`

Package seal: `0f3d2d6e22f8263c3f4b082d1e9c6eb2f8ce304624acfd7e2866875d97d057e7`

The retained real failures established that Native Access never launched and that
the manager observed service absence. They did not establish that the vendor
installation had lost its service registration. A read-only exact-Proton query
subsequently observed the fixed service in the intended managed prefix while the
old manager path still observed absence. The repaired path therefore keeps the
operation-owned Windows anchor and all later SCM helpers in one verified exact
Proton `runinprefix` command context instead of mixing that context with bare Wine.

The anchor publishes an exact private command endpoint. Later helper requests use
that endpoint and retain the admitted environment, prefix, runner, operation, and
nonce authority. A source-owned hold object keeps the command context alive and is
validated by exact identity, type, link count, extent, and content. Replacing or
aliasing the hold refuses; deleting the exact hold requests ordinary retirement.
The ordinary Proton `run` action is deliberately excluded because this pinned
runner routes it through Steam's Windows shim rather than the required source-owned
command path.

The final source-owned campaign ran 16 generated application sessions. Ten reached
their expected completed result, five retained expected failure, and one retained
operator cancellation. Every session used the qualified-recovery origin. A new
cold interoperability case installed and queried the fixture service through a
separate exact Proton invocation before constructing the production manager
runtime. The production runtime's first query then observed the already-registered
service as stopped, performed no installation, established readiness, launched the
fixture application, issued one stop request, cleaned up, and permitted a later
fresh session.

All 714 evidence files present at the merged base remain unchanged. Failed
generated candidates remain preserved privately and are not relabelled as success.
The generated campaign preserved protected state and ended with the bridge active,
two keepers, zero leases, zero pending transactions, zero stale transports, and
capture off.

This repair was not installed. No real Native Access application, NTKDaemon,
installer, DAW, plug-in, update, or other production operation ran. Generated
interoperability does not establish graceful or reliable real NTKDaemon shutdown.
After independent review, merge, and installation, the next physical step is one
user-present Native Access launch attempt.
