# Source-owned dependency qualification

These fixtures contain project-owned Windows service code only. No commercial
application, daemon, installer or registry was used or changed.

The fixed generated service can register only through its installer mode and can
become ready only through Windows SCM. Direct execution refuses service readiness.
The production adapter uses compile-time fixed fixture constants in this package;
installed production builds retain the exact Native Access bundle/service constants.
There is no production fixture flag or arbitrary operator input.

## Retained development outcomes

All failed runs are retained with their exact package seals, public result hashes,
positive cleanup, absent units and deleted disposable prefixes:

1. Installation and registration succeeded, but the short SCM start helper did not
   retain the service lifetime. Readiness refused.
2. Added exact SCM exit facts and source-owned service audit: the fixture reached
   running, while the next independent helper observed stopped/unstarted state.
3. A pipe-based lifetime anchor was ineffective through the pinned runner. Its exit
   was retained; readiness refused and cleanup passed.
4. SCM-state lifetime anchor worked. Linux process-FD association did not establish
   owned sockets; no readiness was claimed.
5. The Windows owner-PID endpoint table, tied to stable SCM process identity, provided
   exact listener authority. All five cases passed. This complete initial campaign
   is retained separately from the final six-case campaign.

The first package transfer included macOS metadata sidecars. Exact filename-set
validation refused before campaign creation or any Windows launch. A clean transfer
of the identical seal proceeded. No refused result was admitted into a success.

AP12 on the first implementation commit also caught accidental dependency-loop
references in ordinary installer/ASC paths. They were removed; those paths now match
accepted behavior and the existing runtime regressions pass. The failure is retained
in hosted run 35240179296; it was not reclassified as fixture success.

## Identity and readiness domains

The installer is verified through an open file and its exact created child image
before resume. Outer exit zero does not prove installation or readiness. The installed
image, exact own-process service path, SCM generation, Windows-owned loopback pair,
unique same-prefix Linux image generation and exact cgroup are independent witnesses.
Windows PIDs never become Linux process identities or signal targets.

A service runner anchor lasts at most 600 seconds and ends when the exact service
stops. All helper, installer and service descendants remain operation-owned. The
production renderer gate refuses before application creation unless fresh readiness
passes. The selected `--disable-gpu` argument remains unchanged.

Unit tests additionally cover changed images, foreign/deleted prefixes, ambiguous
candidates, false readiness, installer/start acknowledgment loss, wrong-operation
Stop, interrupted writer recovery, no-submission tombstones, one-time bridge-service
restoration, immutable receipts and closed operator inputs. They are distinct from
physical service proof. Native Access functionality remains untested by NAD1.

Campaign 6 passed all six service cases but refused its final preservation aggregate:
6,701 shared, 48-link runtime-cache inodes had changed only ctime. A separate retained
reconciliation against the read-only observation's complete metadata confirmed that
the actual Windows prefix and private HOME metadata were unchanged. No failed
aggregate was rewritten. The final driver retains before/after metadata and hashes
all bounded runtime-cache file bytes: only ctime changes on otherwise identical
shared runtime files are allowed. Registry, application, HOME, mode, size, mtime,
inode, content, link-count or protected-state changes still refuse.

A local test initially lacked the staged readback import; it was corrected without
any Windows launch. A package build correctly refused uncommitted evidence inputs
before compilation or transfer. These are development refusals, not admitted runs.
