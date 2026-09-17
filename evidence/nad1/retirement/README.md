# NAD1 exact service retirement and integrated application qualification

Executed source: `5c44eff081b6ffde207799e10db39ca5504f5d36`.
Tree: `4c3870ddd6e5a29ece4267517c6dd54775ce2fe5`.
Sealed package SHA-256:
`778cac742fa5380c805327508411eb9514da6857e331ce0e1b79bfeeff763d62`.

This is a new source-owned fixture campaign. It does not replace the corrected
physical absence observation, lead proof, initial six-case campaign, historical
refusals or NAUI2 A/B/C. No proprietary application or daemon was launched.

| Case | Application outcome | SCM retirement | Forced dependency cleanup |
| --- | --- | --- | --- |
| Normal close | outer 0 | confirmed | no |
| Repeated start in the same scratch prefix | outer 0 | confirmed | no |
| Exact manager Stop | cancelled | confirmed | no |
| Readiness missing after start | target never created | confirmed | no |
| Application exit 7 plus SCM stop refusal | first application failure retained | unconfirmed | yes |
| Application exit 7 | failure retained | confirmed | no |

Every integrated run freshly started and verified the exact prepared service before
application creation, except the readiness refusal which never created a target.
Each made exactly one stop request and no installer call. The adapter's private
retirement frame binds the pre-stop Windows handle generation, SCM STOPPED, process
exit and zero owned endpoint mask. A fresh SCM query and complete Linux daemon
absence independently follow. Windows and Linux PID domains remain separate.

The cancellation goes through the compiled production `renderer_session::stop`
reservation/writer-gate owner. KillMode=mixed lets the supervisor retire the exact
application and stop its Windows dependency before cgroup cleanup. All six exact
units and cgroups are absent, with no owned survivors. The shared normal/repeat
prefix is retained only between those two operations and is absent at final readback.

The physical fixture does not stop the real bridge service. Production manager tests
exercise exact resume-owner matching, wrong-operation refusal, uncertain state,
interrupted writer recovery and one-time bridge restoration through its callbacks.
These results complement the physical application/SCM/reservation/Stop proof; they
are not a claim that real Native Access or real bridge recovery was exercised.

`custody.json` retains 270 private-file hashes, both full private readbacks, the six
exact unit/cgroup/prefix absence checks, and Linux test-output hashes. No raw command
line, account data, Windows PID or service creation time is projected publicly.

Preservation: installed software record unchanged; real daemon still absent; real
Windows-prefix and private-HOME metadata unchanged; registry, products, publication
links, environments, application records, 307 retained witnesses and five projects
unchanged. The accepted shared-runtime law records 6,701 hardlinked inode ctime
changes separately; exact runtime contents and all other metadata match. Service
active, two keepers, zero leases/transactions/stale transports, no pending resume,
capture off. No installation of production software occurred.

Development facts: four initial local manager socket tests were sandbox-denied and
passed with socket permission; the first Linux runtime staging omitted committed
compatibility test data, causing one test error. Its output is retained separately.
After adding that test data, all 185 runtime tests passed (one platform skip).
Neither event launched a proprietary process or discarded a failed Windows campaign.

The real NTKDaemon installation, service functionality and Native Access integration
remain untested. The PR remains draft/unmerged and real dependency mutation remains
gated for independent review.
