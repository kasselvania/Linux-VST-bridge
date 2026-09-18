# NAO1 — Native Access owned session

Active basis:

- merged NAD2: `a86f03e8a5d5302d9872f995a0a5ba376a0ab6d5`
- merged tree: `11a9fefbbe8b184c52058f4b4610da3a5e3b06e4`
- qualified NAD2 executable source: `81bbf198e8bbe7df337312f2529c84b318c1ed64`
- installed Native Access identity: exact 3.26.0 application retained by NAUI2
- installed dependency: exact NTKDaemon 1.32.0 payload and `NTKDaemonService`

NAD1 and NAD2 established exact dependency identity, fresh same-prefix process and
listener readiness, one-stop ownership, truthful stop-response characterization,
bounded process cleanup, renderer/browser-return ownership, and a prior rendered
signed-in Native Access session.

The remaining product blocker is the historical preparation gate. Native Access launch
currently requires a prior dependency operation that both proved readiness and achieved
graceful SCM retirement. The real daemon proved ready but remained `RUNNING` after the
single submitted stop request, so no preparation receipt was created even though exact
owned cleanup restored the bridge and keepers.

Selected next slice: **NAO1 — Native Access owned session**.

Authoritative slice document: `docs/NAO1.md`.

NAO1 makes the Native Access application operation own its exact NTKDaemon dependency
for the same session. Launch must use the retained exact installed-artifact authority
and fresh per-session readiness; it must not depend on a historical graceful-retirement
receipt.

At session close, the manager still attempts one graceful SCM stop. If graceful
retirement is not confirmed, exact-owned process cleanup may complete the Native Access
session only when all owned processes and listeners are absent and all identity,
privacy, preservation, and bridge-recovery checks pass. The result must continue to
report `service_retirement_confirmed=false` and `forced_cleanup_used=true`; it must not
create a dependency-preparation receipt or claim graceful shutdown.

This is an exact Native Access 3.26.0 / NTKDaemon 1.32.0 compatibility policy, not a
general Windows-service framework.

During the implementation PR:

- do not install the candidate;
- do not launch Native Access or transition the real daemon;
- do not change the selected `--disable-gpu` renderer policy;
- do not weaken exact process, listener, prefix, application, or software identity;
- do not change Arturia, VST, audio, editor, product-publication, or capacity behavior;
- do not rewrite prior evidence.

Return one draft PR, uninstalled and unmerged, for independent tech-lead review. After
that source is accepted, the next action is one installation and one real Native Access
session—not another daemon-diagnostics slice.
