# NAO1 — Bind the owned session to Proton's initialized command service

Focused repair basis:

- merged qualified-recovery repair: `c38b5c9deb8bd32f5a509258c30f31b3e795fe59`
- merged tree: `6d661a80a707157f30d3f4815e04a72b1cb38586`
- installed immutable generation: `91ff69f049863e074f907a051d5c4c5913cb04c9633d71db3b2838246ed03b2e`
- retained failed physical operation: `ca95619b7609089e3a9b3a439780785d`
- retained error: `dependency_qualified_registration_absent`

The second physical NAO1 attempt correctly preserved the complete qualified-recovery
authority but again stopped before readiness or application launch after two exact SCM
queries returned error 1060. Read-only comparison then established that this was not a
missing installation: the intended physical prefix was unchanged, its selected control
set contained the exact fixed `NTKDaemonService` registration, and the daemon image was
unchanged. Direct queries through the exact pinned Proton runtime saw that service as
`STOPPED`.

The defect is the operation-private runtime topology. It starts a bare
`srt-launcher-service` outside Proton and then submits a nested `proton runinprefix` as a
child of that service. That constructed runtime therefore queried a different Wine/SCM
universe from the initialized prefix and truthfully returned 1060 for the wrong
universe.

The selected repair keeps one exact source-owned Windows anchor alive as the single
Proton `runinprefix` root wrapped by the verified Pressure Vessel launcher interface.
It binds the private command-service name emitted by that exact root and submits every
dependency helper and the Native Access launch adapter through the verified launch
client and exact Proton `runinprefix` in that same session. The retained anchor keeps
the Wine/SCM universe alive; Proton reconstructs its own closed Wine environment for
each inserted command. Proton `run` is excluded: the pinned runner routes it through
built-in `steam.exe`, and the first disposable candidate produced a source-owned
qualification assertion rather than an admissible product runtime. Bare Wine is also
excluded because it loses Proton's prefix/session environment. A bare service without
the interface remains excluded. The Windows root validates and retains one exact
operation/nonce-bound hold-file generation; forwarded stdin is not lifetime authority.
Only removal of that exact hold after dependency retirement requests root closure. The
anchor, helpers, daemon, and application stay inside the existing renderer cgroup and
cleanup boundary. No bus name, command, path, service, or environment value becomes
caller authority.

Generated qualification must prove that:

- the exact entry point, Proton script, launcher interface, launch client, Wine image,
  Windows adapter, operation, and nonce are verified and bound;
- one verified launcher-interface + Proton `runinprefix` root is retained for the
  operation and all closed commands reuse its one private command service;
- the old bare-service plus `runinprefix` topology is absent from the dependency and
  application path;
- startup refuses missing, duplicate, malformed, changed, or mismatched identities and
  readiness frames, or missing/replaced hold authority;
- continuous draining, callback privacy, exact SCM/process/listener ownership, one-stop
  authority, and exact-owned cleanup remain unchanged;
- a generated artifact-absent qualified-recovery session reaches readiness, launches
  its source-owned application fixture, closes truthfully, and can reopen once.

This repair does not reinstall or repair NTKDaemon registration. The physical
installation is already registered in the intended initialized Proton session. During
implementation and qualification, do not install the candidate, launch Native Access,
start or stop the real daemon, replay the installer, or mutate the real prefix. Preserve
both failed physical operations and the private recovery checkpoint. Return one draft,
uninstalled PR for independent source review; direct user interaction resumes only
after an exact reviewed generation is installed.

The earlier NAO1 authorities follow and remain in force where they do not conflict with
this later, physically established repair.

# NAO1 — Qualified-recovery readiness handoff repair

Focused repair basis:

- merged NAO1 source: `a544645792aa1428d0388191e4fee45e7801b921`
- merged tree: `d438c67d68f2f64ff94cd7110aaff390c05e9e04`
- installed immutable generation: `c4380746a60f85d711c9bd39fc9e5e48bafc501a18eddff7a49d4d68b0cdc3c0`
- retained failed physical operation: `d3762548dc0597464d52ef2b63a77b5a`
- retained disposition: `REAL_NATIVE_ACCESS_SESSION_ADMISSION_FAILED_DEPENDENCY_PREPARE_REQUIRED`

The first physical NAO1 attempt admitted the exact
`qualified_recovered_installation` origin and reverified the installer and daemon
images. Its only SCM query then reported exact error 1060 (`service does not exist`).
No start/readiness loop or application launch followed. The retained record cannot
distinguish a transient private-runtime SCM initialization observation from persistent
missing registration.

This repair carries the complete validated session authority into `Nad1Owner` instead
of reducing it to the daemon image. An initial exact 1060 observation for either closed
session origin receives exactly one delayed query in the same operation-owned runtime.
Exact registration is not process-generation ownership or retirement authority. After
that delayed query reports exact registration, the owner revalidates the physical
prefix and runs one fresh bounded process census. `RUNNING` or `START_PENDING` may
continue only with one exact same-prefix generation already owned by this renderer
cohort. `STOPPED` may continue only with no candidate, through this operation's one
owned start. Unavailable, ambiguous, foreign, deleted-prefix, or same-prefix unowned
candidates refuse before application launch and leave stop authority absent. Another
SCM absence or an unavailable observation also refuses. The repair performs no
registration mutation, installer replay, artifact/preparation synthesis, or second
physical operation. A persistently absent real registration remains a separately
reviewed product/design decision.

The original NAO1 authority follows and remains in force.

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
for the same session. Launch must use one of two mutually exclusive exact installation
origins: the retained installation-artifact record, or the fixed qualified-recovery
record while that artifact pointer remains absent. Both origins require the current
application, software, installer, daemon, environment, and prefix identities plus fresh
per-session readiness; launch must not depend on a historical graceful-retirement
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
