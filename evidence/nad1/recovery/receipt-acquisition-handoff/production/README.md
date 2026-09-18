# PR #121 production installation and one-shot qualification

PR #121 final head `92bb809801e773b6f86726c9a2657e012b5db224` was reviewed with
executable source `0b2ca827b5a10e36027c9d7286c887c5683fa520`. It merged as
`710b0f3be642cca342f2a5915d37158246023aed`. The reviewed PR tree and merged-main
tree are both `77efd70fac7985a051343407d4ec13ca183422a8`; the merge changed no source bytes.
Merged-main AP8, AP12 and PX2 passed:

- AP8: <https://github.com/kasselvania/Linux-VST-bridge/actions/runs/35304856700>
- AP12: <https://github.com/kasselvania/Linux-VST-bridge/actions/runs/35304857556>
- PX2: <https://github.com/kasselvania/Linux-VST-bridge/actions/runs/35304826401>

Before merge, all eleven sealed inputs matched and all 677 evidence files predating
the final correction were unchanged. The approved branch was clean and synchronized.
This release record adds new files only under this `production/` directory and the
corresponding current-slice disposition; it does not rewrite prior evidence.

One immutable installation selected generation
`67e3004b087fe1dbb9c1bbd2c0b667a3eab1d1ac8cf14f7931f5ac37bb38880f` and
software record `bd831fb3659ea27bc52d07bd63327206e46efb87e76af29c604c6ee281a96553`.
The manager and frontend agreed on operator model 7. Installation did not select a
dependency action, start NTKDaemon, or create a preparation receipt. Its sanitized
receipt is `installation.json`; private inputs and logs remain in the immutable
fixture receipt directory. The first release-driver invocation refused before an
installation attempt marker, setup, service stop, or package selection; that
non-installation refusal is retained separately in `driver-preflight-refusal.json`.

Exactly one parameter-free dependency-preparation request was accepted as operation
`f4a2b703d8ea518d14c893c480fd93ed`. It selected `recover_installed`, reused the
qualified existing payload and registration, and did not replay the installer or
launch Native Access. Fresh readiness passed for the exact owned Windows generation
and both expected loopback listeners (mask 3).

The one stop request did not establish retirement. The helper initially and finally
observed SCM state 4 (`RUNNING`), with query/control errors zero. It sent one control
request, spent 670 ms in that call and 12,045 ms in total observation, made 209
queries, and returned 149. Exact-process wait returned 258 (`WAIT_TIMEOUT`); both
listeners remained (mask 3). Process acquisition had succeeded, so the
receipt/acquisition fallback was neither considered nor used. Service retirement
remained unconfirmed and bounded forced cleanup was required. Process cleanup then
confirmed, the dependency unit/cgroup disappeared, and the bridge plus both keepers
recovered to idle with no resume owner. The manager reported
`dependency_command_nonzero`; the specific retained release disposition is
`REAL_DAEMON_SHUTDOWN_RETIREMENT_UNCONFIRMED`. No successful preparation receipt
points to the failed operation.

The daemon payload, software record, products, publications, imports, environments,
projects, retained evidence and prior operation history remained unchanged. Native
Access remains closed and a postcondition census found no Native Access or NTKDaemon
process. `qualification.json` is the sanitized derived terminal receipt. Its process
acquisition classification follows the production stop law: a non-default process
wait value with identity error zero is reachable only after opening and verifying
the admitted generation. Raw process identity material remains private and is bound
by the hashes in that receipt.

This is one physical failure result. It does not qualify reliable or general
NTKDaemon shutdown, and it does not authorize another daemon check or an application
session.
