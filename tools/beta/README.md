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

The SDK consumer's sibling mode defaults to 2400 callbacks. Its optional ninth
argument selects a strict decimal count from 2400 through 4800; other ordinary
modes reject that argument, while migration retains its capture-prefix meaning.
Timing storage is allocated before audio for the maximum count. Rebuild the
consumer after changing this source and retain its source and executable digests.

For unfamiliar-module coverage, freeze and retain a package first, then run
`build_unfamiliar.py` on Windows with that package and engine digest. It builds
two first-party module revisions with newly generated classes through the same
pinned SDK contract tests. Pass its output with `--unfamiliar-fixtures`, the
exact `--revision` and `--frozen-candidate` to the installed driver. The driver
checks package, software, descriptor and engine identities and derives the
native class IDs independently. `--recall-from` uses the exact saved states of
a previously passing one-pair run. An explicit `--selected-candidate` can name
a later manager package; it is independently verified and must preserve the
original frozen engine. It does not relabel the original frozen-package run.
The installed 1.0.1-to-1.0.2 comparison currently refuses the earlier component
state before vendor restore; retain that failure. The consumer also compares
complete state envelopes for an unchanged version. A future update-compatible
state test must separately account for bridge provenance changing while checking
actual vendor state, parameter readback and output; do not rewrite the saved file
or waive the restore result to manufacture a pass.

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
The independent consumer retains a fixed 480-row timing array outside the
plug-in callback: scheduled block time, actual start, callback duration and
comparison mismatches. It emits those rows after joining audio and records the
concurrent state-capture interval. This distinguishes short callbacks from late
host wakes without changing pacing, queues, delay or deadline. Timing probes
against a frozen candidate remain development measurements; they cannot turn
an earlier failed qualification into a pass.
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

`run_installed_configuration.py` exercises the managed settings journey using
only first-party stateful reference modules and the same independent SDK consumer
and callback audit library. Run it as the populated disposable fixture user with
a new private output directory. Obtain the input shape with `--example-config`;
replace every placeholder with exact selected package, release-manifest,
consumer/audit, environment, module, publication and native class identities.
The processor/controller IDs come from that publication's exact revision, not
the Windows class IDs or friendly names. Both classes must be selected in the
same managed environment, with runtime-default graphics, accessibility enabled
and the explicit 1024-frame buffering preference. Run with each role as the
target when both instrument and effect coverage is claimed.

```sh
python3 tools/beta/run_installed_configuration.py --example-config
python3 tools/beta/run_installed_configuration.py \
  --config /PRIVATE/configuration-input.json --output /PRIVATE/configuration-run
```

The driver validates release and selected software identities, exact published
native/descriptor bytes, class mapping and idle capacity before starting. It
records meaningful component/controller state and recalls it through the real
installed proxy. Two separate trials isolate the implemented controls: first
WineD3D11 replaces runtime-default graphics; then Windows accessibility is
disabled for the target host while WineD3D11 remains selected. Each trial retains
its exact predecessor. `--single-trial` exercises graphics only and reports a
subset rather than the complete settings journey.

All changes consume current operator offers and their state tokens:

- `candidate_settings_prepare` creates the retained settings successor without
  changing either publication. Preparation and application run while the other
  class processes audio in the shared environment; their operation intervals
  must fall inside that sibling's measured audio interval.
- `compatibility_publish_test` or `experimental_replace` applies the exact
  candidate and predecessor. Module, environment, Windows host and native bytes
  remain exact; a new publication may own a different bundle path.
- With the target processing, `experimental_disable` and `buffering_set` must
  refuse with the actual target-busy offer. The refused actions must preserve
  publication and buffering. The driver subsequently recalls the original state
  under each selected trial and validates independent sample output, controls,
  callback audit, effective process DLL overrides and positive owned retirement.
- `compatibility_result` records those bounded checks as a worked, partial
  experimental result. It must not publish ordinary support or alter selection.
- After selecting 512 added frames through `buffering_set`,
  `experimental_disable` restores both exact predecessors in reverse order.
  Each restore preserves the current buffering preference and sibling publication.
  The driver restores the initial 1024-frame preference, recalls the original
  unchanged saved files, and requires zero remaining DSP owners.

This configuration driver uses 4800 sibling callbacks during preparation and
apply at 1024 frames and 48 kHz, a bounded 102.4-second observer window. The
active-target refusal checks retain their 2400-callback, 51.2-second windows.
Every audio and timing row must match its declared count; the callback, state,
sample and retirement oracles remain unchanged. The consumer deadline derives from the audio duration plus
60 seconds for startup and 20 seconds for retirement. Exact command intervals
are retained so projection and request time can be separated from operation time.

Baseline selection is verified through the exact registry reference, hashed
retained revision, full registration, physical publication and current selected
publication readback with effective default launch settings. A newest unselected
trial may still appear as `another_configuration`; its advisory settings do not
describe the selected baseline. Fresh preparation must bind the exact baseline
in `expected_current`. A changed SDK consumer requires fresh record/recall runs
and separate tool-source provenance; it cannot inherit the earlier consumer's
passed sample counts through resume.

`--resume-from /PRIVATE/failed-run` can reuse passed baseline consumers and a
completed, retained first settings preparation. The original selected entries,
preparation request/result, predecessor and fresh publish offer must still match.
Resume independently verifies both previous and current release-manifest hashes,
source heads/trees and package versions. Only `source_head`, `package` and
`release_manifest` may differ in the input; consumer/audit, manager path, root,
fixtures, publications and every other input value remain identical. Native
engine, Windows host and Windows host source-manifest bytes must remain the same.
The new output retains the exact previous input and both artifact identities.
Inherited run/sample counts remain attributed to their original source/package;
fresh recall through the current manager is mandatory before continuation. A
changed module, runtime, publication or execution pair requires a fresh run.

This is private maintainer instrumentation. The driver writes raw commands,
operator inputs/readbacks, environment/ownership records, logs and opaque
first-party state with private directory/file permissions. Keep every raw output
outside the checkout and public evidence; copying a whole output tree is not an
export. Export only an explicit allow-list of sanitized fields such as frozen
source/package and artifact digests, first-party class/role, typed choices,
comparison counters, retirement/restore outcomes and original-source attribution
for inherited counts. Exclude paths, usernames, host identifiers, environment or
owner records, saved state and arbitrary diagnostic text. The driver does not
produce a public diagnostic export or authorize proprietary fixtures.

A passing result establishes the declared installed SDK configuration regression:
meaningful original-state recall, separate launch choices, sibling preservation,
target-busy refusal, exact restoration and owned cleanup at 1024 host frames.
It is not a frontend interaction, real DAW project, editor/rendering acceleration,
low-latency workload, reboot/update, commercial compatibility or beta completion
claim. Applied DLL overrides do not establish the editor's actual renderer.

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
