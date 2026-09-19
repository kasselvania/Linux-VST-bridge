# Kontakt through Native Access on the Deck

Observed continuation result, 2026-09-19: source `9d47606` is installed as an
experimental candidate on the Deck. Actual Kontakt 8 Player 8.13.1 loaded Factory
Selection Pad - Noir in Bitwig 6.1, played the owned MIDI clip, and produced
finite nonzero stereo audio. Full Bitwig close/restart restored the instrument
and produced audio again. Two 45-second captures and all seven actual load
attempts are retained in `evidence/kontakt-native-access-deck/shared-bus-playback.md`
and its sanitized JSON. Observed underruns, taskbar foreground workaround,
124 MiB maximum extra output storage, and unproved graceful Windows retirement
remain explicit limits. This is the requested first sound/recall test, not
ordinary profile qualification, broad multi-output routing or timing acceptance.

Operator continuation: implement shared bus handling and get Kontakt audio into
Bitwig for a first real test. Start from `544c02e2781e77d08c8e7ce7aa9e7cec49c69e45`.
Preserve every declared output identity and negotiate the complete layout. The first-stereo-only milestone was tried and failed: Bitwig activates
output 31 before running Kontakt. The continuation therefore carries every
active stereo output through a versioned shared mapping and bounded, owner-
allocated native storage. No output is silently discarded. This mechanism has
no Kontakt-specific executable branch. Keep the installed NI environment and existing published
profiles intact; build a separate preparation kit and experimental candidate.
Verify bus metadata/activation/buffer safety, actual Kontakt audio and a
disposable Bitwig save/reopen. Retain the actual result and any remaining failure.

Transport continuation: protocol 1.13 selects mapping layout 2 (two input planes,
64 output planes, 68,176 bytes); 1.12/layout 1 remains readable by the Windows
host. SDK bus indices map to consecutive stereo planes. The existing 2,048
completion descriptors retain their capacity. Extra planes use one pre-touched
slot pool, allocated during inactive setup: 4 MiB per additional stereo output,
124 MiB at 32 outputs, and no extra pool for a single output. Slots remain owned
until callback delivery/discard; no callback allocation or deallocation. This is
a bounded first implementation, not a many-instance memory-efficiency claim.


Operator continuation, 2026-09-18: install Kontakt using Native Access on the
Deck, then discover/publish the real VST3 for Bitwig and test it. Each observed
stage is reported separately; installation alone does not establish audio.

Base commit: `0f7d122bb08e5b491461847a8c818a19fe3d2c86`.
Base tree: `8142b9ca0aec5883a1d8631ea6d6f4c72fddda88`.
Primary outcome: the existing NI environment uses the corrected pinned runner
and Native Access performs the real Kontakt installation.

Scope: the MSI correction, runner/environment transition and existing Native
Access session admission; subsequent exact Kontakt scan/profile/proxy work
needed for the requested Bitwig test. Rust owns product changes. Existing
supervision is reused; there is no replacement vendor installer. Preserve
historical installation evidence and the existing prefix/machine identity.

Fixture: Steam Deck Galileo, existing Native Access 3.26.0 environment and lawful
Kontakt 8 Player 8.13.1 package, pinned Proton 11.0-2c candidate documented below.
Acceptance proceeds through actual Native Access install and reopen recognition,
SDK class discovery, native proxy publication, then a disposable Bitwig project
with editor/audio/state checks. Report any concrete failure at its actual stage.
Before live mutation retain a private prefix/metadata rollback copy, close owned
sessions, and verify the exact runner modules. Do not alter other environments,
Steam's installed runner, account state, or vendor product registration manually.

Observed result: Native Access installation and recognition after reopening
completed; scan and SDK inspection completed. Proxy preparation refused Kontakt's
32 stereo outputs at the current single-output implementation. No proxy was
published and no Bitwig audio/editor/state test was run. Preserve these completed
sub-results. The remaining work is actual bus support across the proxy/Windows
host, not another installer implementation or a removed bounds check. See
`evidence/kontakt-native-access-deck/README.md`.

The earlier investigation follows as retained evidence and reproduction detail.

# Kontakt MSI runner correction investigation

Selected by the operator's 2026-09-18 reset: identify the actual Kontakt
installation failure, correct the compatibility boundary, and retain the lesson.

- Base: `745974b2325e9ace4f7aef21c82f62f29e7e2ff3`
- Base tree: `b6213087918811f1e90f27d827285dc30785bb3f`
- Basis: operator-provided AGENTS.md, Mission / Repository roles /
  Compatibility-profile rules; checkout AGENTS.md, Work to an outcome /
  Keep the engineering safeguards; GOVERNANCE.md, What evidence means;
  docs/ARCHITECTURE.md, 2.1 Rust owns the product / 5.2 Runner builder and
  inventory / 5.3 Environment manager / 5.4 Installer supervisor.
- Fixture: retained lawful Kontakt 8 Player 8.13.1 package; Steam Deck Galileo;
  pinned Proton 11.0-2c, Wine `dc26e61847081a1b5cb0733dc30feba6ee575482`.

Primary claim: determine whether ambiguous MSI string-reference widths remain
misparsed after the two earlier upstream backports, and whether correcting that
runner defect lets the vendor installer finish in a fresh disposable prefix.

Scope: this file, `docs/KONTAKT_MSI_FINDINGS.md`,
`evidence/kontakt-msi-root-cause/**`, and
`kontakt-msi-ambiguous-strrefs.patch`. Private Wine build/source and disposable
prefixes are permitted outside the repository. No new product deployment engine.
The K8I1 candidate remains preserved on its existing draft branch.

Acceptance: explain the exact observed row-width error from the retained package
and Wine source, build the narrow MSI correction, and run one bounded ordinary
vendor installation in a fresh disposable environment. The first correction run
deployed the final payload and exited in 232.22 seconds with Linux status 100.
One additional fresh run with InstallAware's documented native logging switch
finished in 232.65 seconds with identical final binary hashes. Its native MSI
INSTALLEND record establishes a successful MSI execute sequence. Outer Linux
status 100 and vendor variable COPYERROR=TRUE remain unexplained; full wrapper
success is not claimed. The runner correction remained identical.
Record actual deployment or the next actual failure. A successful table read alone
is not installation.

Negative acceptance: preserve the original failing cases; do not claim success
from wrapper exit, staged files, tests, or partial registration. Stop at a new
failure rather than silently broadening this correction.

Evidence: sanitized package identities, table sizes/counts/reference validity,
source/build identities, exact installer outcome, final payload observations,
remaining gaps, and a reproducible route. No proprietary payload or MSI tables.

Cleanup: retire only disposable processes from this operation, retain their
private logs and prefix for inspection, and preserve the working managed NI
environment and earlier candidates. No managed-prefix migration, licensing,
Native Access interaction, DAW/audio acceptance, merge, or product release.

Earlier NAO1 authority follows as retained context, not the active task.

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
  readiness frames, or missing/aliased/replaced hold authority;
- a stopped fixture service established and queried through a separate exact Proton
  reference entry remains visible to a cold product runtime without reinstalling it;
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
