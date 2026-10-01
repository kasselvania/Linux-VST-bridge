# Installed development regressions

These tools load the delivered product. They are maintainer instrumentation;
customers do not need compilers, the SDK, or these scripts. Source and frontend
qualification remain distinct from these installed development checks.

`trace_installed_refresh.py` consumes an actually enabled environment refresh
offer, or uses `--restoration-only` to exercise LVE1 after an idle user-service
restart. `--registry-contention-ms 3000` deliberately owns the real registry
lock for three seconds without writing data. It refuses existing DSP work or
pending recovery; it never deletes an inventory or forces a stale offer.

Build `lifecycle_host.cpp` with `build_lifecycle_host.sh`, using official SDK
commit `3cdf9ca5d1f5b1b21e0a86832aa4abe55607bd96` and its existing SDK libraries.
The build emits the independent SDK consumer and callback audit library. Run
`run_installed_lifecycle.py` as the populated fixture user, passing `--host`,
`--audit` and a new private `--output` directory. An optional `--role effect
--pairs 1` is an initial probe; only the default two-role, four-pair run can
report the complete development suite passing.

Before a run, install and publish both reference fixtures through Setup and
product controls. Replace older publications through those controls. The
driver refuses a predecessor host, changed module/native bytes, unpublished
fixtures, existing work, uncertain retirement, or another delay configuration.
The predeclared workload is 48 kHz, 1024 added frames, three record/recall pairs
at 1024 host frames and one at 1008 for each role. Each process uses 480 paced
callbacks and the exact real native/Windows class mapping. It asserts independent
sample output, sparse automation through offset 1007, note onset/release,
meaningful state capture before and during processing, byte-identical
component/controller recall, callback audit, phase attribution and positive
owned DSP retirement. It does not manufacture empty state or copy vendor data.
Retirement includes the manager's exact lease release within the original
20-second bound. Readback is sampled while that lease remains; a counted DSP
owner is valid, and an unresolved owner refuses the run. Both supervisor cleanup
and transport retirement must be positive, and the native final phase must be
`retired`. The consumer does not grant free capacity from a cleanup report.

The driver retains allow-listed native phase counters and retirement fields.
Opaque first-party state remains in the private test directory. Arbitrary
vendor diagnostic text is omitted; only its byte count and digest are retained.
A failed probe or test-owned timeout remains failed. Test-process destruction
alone never confirms product retirement.

`trace_worker_exhaustion.py` is an installed, idle-service fault check. It
temporarily lowers only the owned service's task cap to one and requires three
bounded worker refusals without a manager restart or admission. It restores
the exact prior service limit and restarts that idle service in cleanup; the
enclosing CPU, RAM, swap and task budget remains unchanged. It is not a GUI
recovery pass or proof about a playing customer project.

Verify CPU, RAM, combined RAM/swap and task caps before execution. Run one VM
or builder at a time and shut down inactive guests normally. These checks do
not replace actual DAW project save/reopen, reboot, frontend recovery, populated
update/rollback or licensed hardware acceptance.
