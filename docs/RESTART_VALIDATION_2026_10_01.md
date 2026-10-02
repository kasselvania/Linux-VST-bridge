# Restart validation

The tested product baseline was frozen at `0a31b79d5d119d311d0f5652afe8a5d4eb320207`
(tree `82c56ca2fc67dc4e04d3fca58b8602b8d1e0a8e6`). Internal49's
committed failure remains a failed qualification result. Internal50 is the
already-written recovery correction. It was tested unchanged before selecting
the connected ownership corrections below. No passing result is assembled from
these failed development runs.

## Refresh and resume ownership

1. `operator_cli::EnvironmentRescan` retains the exact operator/environment
   lock and records the selected manager and prior service state in
   `operator/resume.json` before stopping the service.
2. Discovery runs under its existing owner and persists inventory. Its result
   does not confirm service restoration.
3. `resume_owned` holds `operator-resume.lock`, verifies the same owner and
   selected manager, starts the service if previously active, and sends LVE1.
   Connecting has a five-second bound; reading the readiness acknowledgment
   has a 70-second bound. The resume record is removed only after restoration.
4. The service's LVE1 worker snapshots one current registration per environment,
   checks exact paired components and shares process-owned byte verification
   with startup preparation. Its overall preparation/readiness deadline is
   60 seconds. Initial preparation holds no registry reservation while hashing;
   the subsequent stage/readiness verification does hold that reservation.
5. Internal49 reserves registry access with a two-second lock bound, stages
   one keeper, waits until that keeper is ready, verifies its binding, then
   advances to the next environment. All environments share the same deadline.
6. The current correction stages all exact bindings before waiting, then
   requires actual readiness and matching registration/components for each.
   Its reservations use the remaining overall budget; this changes the lock
   wait bound and must be measured alongside initialization, not assumed safe
   from the helper tests. The overall deadline was not increased.

## What the failed run establishes

The request began at 19:37:49 UTC. The first retained keeper generation was
recorded at 19:40:22 and became ready at 19:40:32. The operator refused resume
at 19:40:37 with an incomplete acknowledgment. Another generation was recorded
at 19:40:43 and became ready at 19:40:50; later product-owned recovery retired
the pending resume record. Inventory had persisted, but restoration had not
completed when the operation was refused.

The service journal's `admission_service_busy` is an observation. The old
serial staging and shared deadline are confirmed in source. The retained run
does not correlate each generation with a particular restoration request or
separate preparation time from registry contention. It therefore does not
prove which consumed the deadline, or that serial staging was the sole cause.

The installed reproduction will record request/reply times, new generation
and readiness events, registry-lock contention, and the exact manager/host
pair. It will exercise the production restoration crossing on the populated
account; it is a development regression, not a musician qualification pass.
Run the current correction before deciding whether another repair is needed.

## Native CI defect

Run `36918491062` registers ten tests and passes nine. CTest cannot find
`ap18-whole-block-tests`: the target is declared and registered in CMake but
omitted from the workflow's explicit build target list. This is a build-list
defect, not an observed failure of the whole-block executable. Repair that
list and require all ten tests to actually run.

That repair is committed at `602f187723e70f7c696d824d3e1c91a6a71c0b80`.
[Run 36922549242](https://github.com/kasselvania/Linux-VST-bridge/actions/runs/36922549242)
built and executed `ap18-whole-block-tests`; all ten registered tests passed.
This is a CI-only change. The delivered product remains frozen at the source
identity above.

## Installed development reproduction

The original reference-environment inventory is already refreshed, so the
manager correctly no longer offers that exact historical request. No stale
request was forced and no inventory record was deleted to manufacture it.
The still-offered FRAGMENTS environment refresh exercised the same global
restoration crossing on internal49. It completed in 118.21 seconds, including
snapshot/request admission and discovery. Two actual keepers became ready;
four capacity readbacks exceeded their two-second registry acquisition bound.
This run did not reproduce the original refusal.

An idle service restart followed by LVE1 completed in 50.14 seconds from the
test start. A separate, explicitly controlled three-second registry-lock hold
made internal49 close the readiness crossing after 2.00 seconds from the LVE1
request, before the lock was released. That establishes a repeatable failure
of its short acquisition bound. It does not establish that this was the sole
cause of the historical internal49 run. Retain both positive and negative
traces, and run the same arrangement on the unchanged internal50 correction.

Internal50 passed package intake and selection through the actual frontend on
the same populated Ubuntu account. The selected manager digest is
`ebb8c123783af01ecb30c9bcc8f79f41d2000bc470567ef019fab182c3931aa1`;
the selected Windows host is
`a49c6be190f3e018ce5f253b93750701c10f93b877fccf7066cd55ebdfd6b17b`.
The original Ardour project and all three publications remained unchanged.
This is selection evidence, not an audio or recall pass.
The retained frontend image caught status refresh still in progress; it does
not independently demonstrate the final ready presentation.

Its unchanged correction got past the controlled three-second hold but closed
LVE1 after 22.89 seconds. The service journal retained
`artifact_changed_during_verification`; one new keeper later reported ready.
There was no complete readiness acknowledgment. The exact failed artifact was
not included in the original diagnostic, so its identity remains an attribution
gap. Internal50 does not pass this installed regression.

Read-only inspection found that the runtime's `libelf.so.1` and six retained
runtime-var copies share one inode. Its change timestamp moved during keeper
startup, while its expected bytes and mode still matched. This corroborates
the upstream [pressure-vessel copy operation](https://gitlab.steamos.cloud/steamrt/steam-runtime-tools/-/blob/main/pressure-vessel/runtime.c),
which hard-links support files into the mutable sysroot. The implemented
recovery loop launches a keeper and immediately repeats full runtime
verification, under registry ownership, while that preparation is still running.
This is a connected ownership conflict to resolve; it is not evidence that the
specific `libelf.so.1` check was the one that failed.

A diagnostic syscall trace reached its byte cap and changed the failure to
`service_recovery_deadline`. Its extra overhead makes it unsuitable for startup
acceptance or attribution of the uninstrumented refusal. No timeout was changed
and no passing result is inferred from it. Routine reproduction must use lighter
phase/identity observations rather than repeat this broad trace.

The repeatable driver uses an actually enabled production refresh offer or
the production LVE1 crossing, observes owned lock identities and keeper
reports, and writes no registry, lease, runtime, module, or vendor data.
Its optional contention is a bounded test arrangement, not a musician action.

## Fixed regression workload before frontend qualification

The SDK consumer loads the real installed reference instrument/effect proxy.
It uses independent sample calculations, sparse points including offset 1007,
known note onset/release, state capture before processing and during live
processing, separate component/controller state and a fresh process for recall.
It does not link a bridge helper or mock its manager/Windows host.

The declared workload is 48 kHz, 1024 added bridge frames, 480 paced callbacks.
Run three record/recall pairs per role at 1024 frames and one pair per role at
1008 frames, with no sample mismatch, non-finite output or rejected callback.
Retain all frames, including the advertised initial latency. Observe actual
owned DSP retirement separately from successful native object destruction.
Callback audit must pass its positive control and report no callback effects.
These are installed development regressions; actual DAW project recall,
frontend recovery and commercial hardware qualification remain separate.

The first SDK probe refused before bridge admission: it confused the Windows
fixture's class ID with the generated native class ID. A read-only factory
census established the two native identities; the test consumer now receives
and verifies those separately. The failed probe is test-harness evidence and
does not measure bridge audio, state or retirement. The corrected consumer also
requires byte-identical component and controller state after fresh-process recall.

## Connected startup and retirement correction

Scope: manager restoration/native admission and durable session ownership;
`bridge-manager/src/{main,lib,capacity}.rs` and the three existing spawn callers
in `operator_cli.rs`, `preparation_cli.rs` and `vendor_product_cli.rs`. The
broker/supervisor ownership boundary in Architecture 5.9, state boundary in 6.5,
and selected integrated delivery assignment remain the basis. No runtime,
Windows host, native proxy, IPC, audio queue, buffer or deadline is replaced.

Restoration now waits for all exact keepers to report ready before full byte
verification, performs that work outside registry ownership, then takes fresh
registry authority and checks the same ready keeper generations again before acknowledgment. Native
admission similarly verifies after readiness outside the reservation and
requires the same keeper generation and publication on reentry. Fresh byte
observations remain owned by this running manager; a disk cache cannot grant
execution. Source tests explicitly permit independent registry acquisition
during verification and refuse a readiness change before acknowledgment.

The independent installed effect consumer compared 983,040 stereo samples,
captured recognizable live state, exercised sparse automation and retired its
native objects with no rejected callbacks or unexplained processing frames.
Those observations do not make the entire run pass. Its required capacity
readback refused after supervisor cleanup. The bounded follow-up trace retained
the exact detail `active_lease_unresolved`: the same DSP lease remained present,
its temporary `owner.json` had gone, and its actual report confirmed both process
cleanup and transport retirement. No registry owner was observed. This is an
ownership-lifetime defect distinct from registry contention and from audio.

Before publishing a new lease, the manager now retains the exact session spec in
its own private, schema-1 `runtime/lease-owners` record. Supervisor transport
cleanup cannot remove that record. Admission, readback and inactive checks use
the same record until exact manager release; legacy leases preserve their prior
lookup and still refuse missing authority. A report alone never frees capacity.
Service-owned release acquires registry authority with its existing two-second
control bound. Negative tests retain capacity after temporary-directory removal,
including manager reconstruction, and refuse changed schema, report or location.

The installed driver samples readback during retirement and requires process
cleanup, transport retirement, absence of the exact manager lease, and fresh
zero-DSP readback within its already-declared 20-second retirement bound. A
correctly counted retiring lease is allowed; unavailable ownership is not.
The actual native phase must end in `retired`. Earlier failed traces remain
unchanged. These source corrections still need fixed-candidate installed
validation; they are not qualified by the source tests alone.

The complete manager suite ran with one test thread: 200 library tests and 281
binary tests passed; two existing optional tests remained ignored. A subsequent
focused run passed all eight recovery tests, including an actual owned-process
replacement after confirmed retirement. Clippy passed with warnings denied.
Source tests and installed regressions remain distinct.

## Resource envelope

Only one test VM or local builder may run at a time. Each has a one-core quota,
low CPU shares, a 256-task limit and zero additional swap. VM containers have
a 4-GiB RAM limit; builder containers have a 2.5-GiB RAM limit. Both Docker
configuration and effective cgroup limits were read back before resuming work.
Both guests were shut down normally and exited without OOM. Inactive paused
guests must not be retained as a substitute for releasing their RAM.

Both bounded disposable harnesses now advertise 3 GiB and one guest CPU inside
the fixed 4-GiB, one-core container envelope. They preserve their original
accounts, disks and firmware. Original and bounded harnesses sharing a disk
must never run together. Effective swap is zero. The viewers are separately
capped at 0.1 core, 128 MiB RAM, zero additional swap and 32 tasks. Ubuntu was
shut down normally before building the SDK consumer; its viewer and local
forward were stopped. No resource cap was expanded.

## Qualification remains one installed outcome

Internal51 is frozen at `e9107f4685182cc6de4390beddb4b2044c342622`
(tree `0db6eaa643200f10f809903a9cb748f88f15f37e`). Both platform installers
are retained with their component identities in
`internal51-component-manifest.json`. All checks at that head are green,
including all ten actual native tests. This does not establish installed
acceptance.

Before selecting internal51, resource preparation found an enabled guest swap
file and 345 tasks in the existing fixture user's session. The guest user slice
was bounded to one core, 2.5 GiB RAM, zero swap and 256 tasks; the outer VM
retained one core, 4 GiB RAM, zero additional swap and 256 QEMU-container tasks.
These are distinct process boundaries. Guest swap was disabled across reboot.
The 256-task guest limit refused further forks. A lighter desktop package was
prepared from Ubuntu's official repository, but its subsequent session did not
establish a working graphical baseline within that limit. No product test ran.

The retained preparation receipt records that constraint and normal shutdown.
The operator subsequently approved raising only the guest user task limit to
512. CPU, RAM and RAM-plus-swap limits remain unchanged; the outer QEMU task
limit remains 256. Internal51 was selected through normal package and Setup
controls on the existing Ubuntu account, with the original project unchanged.

The first restoration reproduction on GNOME reached 512 tasks and recorded 15
new task-limit denials. The service's main thread panicked on OS thread creation
error 11, exited with status 101, and closed the readiness crossing. Both owned
keepers retained positive cleanup. This is a measured resource-exhaustion
failure, preserved in [the GNOME result](../evidence/self-service-delivery/internal51-gnome-contention-failure.json),
not attribution of the historical internal49 refusal.

The same account then used a declared Openbox/X11 development desktop. GDM and
its original configuration remain recoverable. LightDM and Xorg are fixture
tools installed from Ubuntu's repository, not product prerequisites. This
baseline brought idle task use below 200 without changing any cap. The same
fixed internal51 source passed the three-second contention reproduction: both
keeper generations reported Ready, then LVE1 acknowledged readiness after
43.307 seconds. Peak task use was 354, with no new guest task-limit or OOM
events. [The exact trace and limits](../evidence/self-service-delivery/internal51-openbox-contention.json)
retain its development-only claim. Outer memory-limit reclamation was observed
afterward; no before/after counter was retained for that timed run, so no
zero-pressure claim is made.

Normal production operator authority replaced the reference instrument's
predecessor with its already-prepared host/proxy pair. This was a development
operator run, not frontend qualification. The full SDK suite then failed at
the effect's first state capture, before DSP admission. The supervisor's exact
refusal was `host Xauthority exact alias unavailable`; the generic native
reply was `admission_binding_invalid`. The native desktop supplied its private
home-directory authentication file, while the implementation searched only the
runtime directory for a copy. The [failed installed result](../evidence/self-service-delivery/internal51-sdk-lifecycle-failure.json)
retains that distinction and the absence of audio or state acceptance.

The source successor verifies the peer-selected host-visible private file as
the same actual file before using its path. A private mount namespace still
requires the existing exact authenticated alias; no ambient authentication or
session bus is substituted. Source filesystem cases cover the native file,
different-inode copy, symlink and previous namespace/denial paths. X.Org's
[authentication contract](https://www.x.org/guide/communication/) permits the
selected file or the home-directory default; a runtime-directory-only search
was an implementation restriction.

The same successor uses fallible thread creation for runtime preparation and
request workers. A failed request-worker spawn retains an inspectable service
incident and returns the existing bounded, unclassified refusal without
exposing a session. Rust's [thread creation interface](https://doc.rust-lang.org/std/thread/struct.Builder.html)
returns the OS error instead of panicking. The installed fault driver lowers
only the owned service's temporary task cap to one, requires repeated bounded
refusals with the same live manager, and restores its exact previous limit.
Installed successor validation is required; source checks do not establish it.

The bounded source run passed all 14 graphical filesystem cases and 282 manager
binary tests, with two existing optional tests ignored. All-target Clippy passed
with warnings denied. The one-core, 2.5-GiB, no-swap, 256-task builder recorded
no task-limit, memory-limit or OOM events. The [source-check receipt](../evidence/self-service-delivery/internal52-source-correction-checks.json)
retains the exact changed file digests; the successor's installed run remains
pending.

Routine regressions use the stateful first-party instrument/effect and the
production manager, supervisor, native proxy and Windows host. Include state
capture before processing and while processing: the official IComponent
contract permits both. Preserve component/controller separation, whole-block
automation offsets, processing readiness and positive retirement as distinct
facts. A reference SDK host is development instrumentation, not real DAW
project-recall acceptance.

## Internal52 installed results and remaining delay

Internal52 is the fixed product source `2446836aa8d75033d85bf9c4636c780aaf818130`,
tree `e5d47b9d628574c776d696d751e7a162c9a9e393`. Normal installer intake,
Setup selection and service start passed on the populated Openbox/X11 account.
The original Ardour project stayed unchanged. This development desktop fits
the approved 512-task guest limit; it does not qualify GNOME or make Openbox a
customer requirement. A missing Xorg input driver was installed from Ubuntu's
official repository before the run, under a separate one-core/512-MiB/zero-swap/
64-task maintenance scope. No product or resource cap changed for that repair.

Three installed development regressions passed:

- Deliberate owned-service task denial returned three bounded refusals without
  losing the manager or granting a lease. Restoring the exact original service
  limit, without restarting it, restored fresh capacity readback. The four
  task-denial events are intentional; the 512-task user budget was not exhausted.
- Both actual keepers acknowledged readiness under the same three-second
  registry contention. Request to LVE1 acknowledgment was 43.963 seconds;
  peak task use was 371 with no new task or OOM events.
- Both SDK reference roles completed all four record/recall pairs: 16 fresh
  consumers, 15,667,200 stereo samples compared, meaningful component/controller
  state and sparse automation, zero processing missing frames, positive native
  and supervisor retirement, and zero maintainer repairs. Startup priming of
  16,320 frames and processing priming of 64 frames remain explicit. The suite
  took 369.747 seconds; peak task use was 372 and peak user RAM 2,008,346,624 bytes.
  Outer VM memory reclamation increased by 445 events without OOM.

These tests use the retained coherent native51/Windows-host47 publications with
manager52. Kit52 is selected but its new proxy bytes are not yet published.
They establish neither real DAW project recall nor frontend publication,
commercial, hardware, CachyOS or Deck qualification. Exact identities and
limits are retained in `internal52-component-manifest.json`,
`internal52-thread-denial.json`, `internal52-openbox-contention.json` and
`internal52-installed-sdk-lifecycle.json`. All eight checks on the frozen head
passed, including all ten native tests in
[run 36942534945](https://github.com/kasselvania/Linux-VST-bridge/actions/runs/36942534945).

State access remains limited: the first launch took 24.806 seconds and the
remaining fifteen warm launches took 11.367–12.674 seconds, exceeding the
prospective five-second warm target. Retained manager
phases place 8.070–8.599 seconds before binding verification, with an already
ready keeper taking about three milliseconds. A separate two-launch, read-only
sampling probe passed functional processing and retirement, observed 6,799
runtime regular-file identities change (653,805,161 bytes), and measured
1,457,664,541 manager read characters. This is not a claim that those bytes
changed content or that all reads were runtime hashing.

The acquired runner has a complete pinned tree but lacks the standalone
`native-command-session.json` component required by the existing shared
command owner. Both actual instance receipts omit `native_command_child`;
`NativeProtonSession.selected` returns None and `run_owned` therefore starts
another outer runtime command. Repeated runtime-copy construction invalidates
the byte observations needed by the next launch. This selects a delivery repair,
not looser integrity checks or a larger deadline.

The source successor derives the closed client/service pair from that exact
acquired runner's pinned tree, then reuses the existing live keeper, private
endpoint, inherited-fd handshake and process retirement owner. It changes no
installed runner file, prefix or licensing identity. Unknown runners keep their
prior route; missing or changed manifests/tools refuse. The exact packaged
command tools' help interfaces confirm the socket, variable and descriptor
forwarding contract. The new selection regression fails against frozen52 and
passes against the correction; all 32 command-session tests passed, including
actual Linux process-group custody. These source results did not qualify the
packaged child boundary.

## Internal53 installed owner failure and connected correction

Fixed source `201b914b45a9c17a63b275e5b5dba28139675489`, tree
`608ee099ee5776572122069522b38ee70d60e73d`, passed normal installer intake and
Setup Stop/Select/Start on the populated account. All six selected components
matched the immutable package. All eight GitHub checks passed, including ten
actual native tests. Its first installed SDK effect state capture then failed
before any Windows DSP or audio admission. This candidate failed qualification;
the valid internal52 results remain separate.

The exact keeper report proves a missing application cache parent. The owner
assigned its endpoint before creating the exclusive directory, so finalization
attempted to remove a nonexistent path and persisted unconfirmed cleanup. Fresh
capacity incorrectly remained available because its in-memory DSP cleanup guard
did not include that retained failed keeper report. The failed lease, report,
original project and source are preserved. No directory was manually created
and no lease/report was edited to advance the run.

A separate isolated probe of the actual selected supervisor inside the actual
pinned runtime returned `RuntimeError: Bad magic number in .pyc file`: runtime
Python 3.13.5 has bytecode magic `f30d0d0a`, while the host-built supervisor has
`2b0e0d0a`. This is a second measured boundary defect, not a guess about the
initial cache failure. The first instrumentation attempt had an inaccessible
working directory and was corrected only in the private probe.

The connected correction claims a directory only after exclusive creation,
creates/verifies its private application roots, and retires only the exact owned
directory. It replaces the cross-interpreter supervisor invocation with one
fixed isolated text bootstrap preserving the nonce, kernel identity and exec
handshake. A retained keeper report explicitly disputing cleanup now refuses
capacity/admission instead of projecting available capacity.

Both fresh-cache finalizer reproductions fail against frozen53. All 37 corrected
command-owner tests pass in the capped Linux guest, including compiled-parent
argv and actual kernel custody. The actual pinned command service/client also
executed the text bootstrap in runtime Python 3.13.5 with the same kernel
identity across exec and positive process cleanup, in 3.940 seconds. No new guest
task, memory or OOM events occurred. These are development source/interface
results. After restoring the repository-relative fixtures omitted from the
first private test copy, all 344 runtime tests passed without skips in 32.343
seconds; guest task/memory events remained unchanged. Product source did not
change for that test-copy correction. Installed Windows processing and DAW qualification of the successor
remain required. The [failed53 receipt](../evidence/self-service-delivery/internal53-runtime-owner-failure.json)
retains the exact failure and the independent runtime observations.

Final acceptance uses one fixed delivered candidate through installation,
publication, automation, meaningful saving, close/reopen, reboot/reopen,
populated update and rollback. Any maintainer repair fails that run. Execute
Ubuntu and CachyOS sequentially under the resource envelope; protect the
selected licensed Deck installation until its controlled update is justified.

SDK references: [component state and thread/lifecycle contract](https://steinbergmedia.github.io/vst3_doc/vstinterfaces/classSteinberg_1_1Vst_1_1IComponent.html),
[processing lifecycle](https://steinbergmedia.github.io/vst3_doc/vstinterfaces/classSteinberg_1_1Vst_1_1IAudioProcessor.html),
and [automation queues](https://steinbergmedia.github.io/vst3_doc/vstinterfaces/classSteinberg_1_1Vst_1_1IParamValueQueue.html).
