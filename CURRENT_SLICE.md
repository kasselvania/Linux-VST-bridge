# Current work: one managed plug-in configuration

Selected by the operator on 2026-10-03 after the completed platform assessment.
Status: installed acceptance failed on `codex/platform-configuration`, based
on `e1f8033bcad08a2b008384a1671d3c510b55c362`. Paired package `0.12.0config4` is
frozen at source `19c5888bd3332c827c317387365b177587d9d805`, tree
`c35c9ada601f39f8dac401f69da842fdeb97884d`, with green CI. The endpoint below
is unchanged by individual plug-in examples. No vendor-specific investigation
is selected, and the integrated capability is not yet complete.

## Completed outcome

The [platform assessment](docs/PLATFORM_ARCHITECTURE_REVIEW.md) traces production
configuration, host capabilities, runtime/dependencies, discovery, SDK/audio,
editors, state, supervision and ordinary delivery/recovery. It distinguishes
implemented behavior from intended contracts and retained observations, records
retain/repair/replace/retire decisions, and supplies owner changes plus representative
acceptance. [Architecture section 18.7](docs/ARCHITECTURE.md#187-platform-execution-convergence),
D-028 and the [roadmap](docs/AUDIO_RECOVERY_ROADMAP.md) now agree on that direction.
The assessment is the design basis; this implementation must connect the product
owners and verify their user-visible workflow.

## Implementation checkpoint

The existing candidate/history owners now connect ordinary preparation, typed
graphics/accessibility preferences, shared registration/readback, launch and
trial/keep/restore. Retained profiles own the defaults resolved at preparation;
later advice cannot reinterpret them. Refresh resolves current advice, carries
explicit preferences through generation binding and reuses exact native bytes.
Historical record IDs, logical class identity and saved objects remain preserved.

Publication and buffering changes quiesce every affected-class instance while
allowing independent sibling DSP. Canonical ownership, maintenance and unresolved
cleanup still constrain the final mutation. Unfamiliar managed classes use the
existing native slots within the shared service ceiling. Readiness separates
capability eligibility, observed consumer liveness and historical qualification;
native/sandbox consuming-context observations remain exact, bounded evidence.
Scanner/runtime/environment maintenance remains globally exclusive.

Interrupted managed publications remain inspectable as needing attention, with
canonical owners still counted and the normal reconcile action reachable after
required retirement. Readback does not grant publication or execution admission;
pending/invalid bindings and ownership gaps continue to refuse those operations.
The frozen source has green CI, including the focused refresh, capacity and normal
recovery regressions. Interrupted-publication recovery is covered by Linux tests;
the SDK workflow does not establish an installed fault-injection result.
Installed checks used [the retained driver](tools/beta/run_installed_configuration.py)
at tool commit `9c3d9b0c`, independently of frozen product source `19c5888b`.
The [installed result](evidence/preparation/2026-10-03-managed-configuration-installed.json)
retains the exact package, modules, runtime, observer and earlier failed attempts.

- Existing unfamiliar instrument: both settings trials, independent sibling
  processing, affected-class refusals, original-state recall, partial keep and
  exact restoration passed. Thirteen complete SDK lifetimes compared 63,897,600
  samples with no errors or callback overruns. This is not a continuous soak.
- New reference effect: the full workflow failed during its second trial's apply
  operation. Its independent instrument sibling returned one silent 1024-frame
  span (2044 sample mismatches); the affected callback took 21.375 ms. Eight earlier
  complete lifetimes passed, but they do not make the effect workflow a pass.
- The failure occurred during request dispatch; the actual worker/mutation start
  was not observed. Both consumers retired cleanly. No causal attribution or
  qualification retry follows from this result.
- GUI recovery changed buffering to 512 and restored the exact first predecessor,
  retaining that preference. The original-version action produced no durable
  request after two mouse attempts and one Return; input/application attribution
  remains open. Supported CLI rollback then restored the exact original, retained
  512, and explicitly returned to 1024. One separate original-state recall passed
  983,040 samples with no errors/overruns and full retirement. Separate final
  readback confirms all four exact original entries, each preference at 1024,
  unchanged saved objects/package artifacts and no active or uncertain owners.

## Next bounded work

Measure the shared control-work/audio scheduling boundary before choosing a timing
repair: bracket durable request creation, projection validation, queued receipt,
worker start and mutation begin/end; correlate actual output with bounded native/
Windows render-thread scheduling observations and interval resource deltas.
Distinguish runnable starvation, transport wait and SDK service.
Preserve the failed lifetime; another clean repetition cannot replace it.
Also resolve the ordinary recovery interaction: its incomplete GUI path cannot be
qualified from a successful CLI fallback.

Source review confirms that ordinary projection/admission repeats candidate,
history and complete runtime-tree checks. Native scheduling remained ordinary
because the existing DAW process RT budget was unavailable; Windows exact-owned
render-thread selection failed before RealtimeKit invocation. Those are observed
capability/cost gaps, not a demonstrated cause of this missing block. Do not raise
buffers, change the runtime or priorities, or widen privileges as a guessed repair.

## Selected implementation endpoint

Prepare, explain, run and restore an unfamiliar plug-in through one managed
configuration. Extend existing candidate/history and registration owners; connect
profile advice, explicit preferences, capability assessment, normal controls,
admission, launch and affected-owner recovery. Separate eligibility, liveness and
qualification. Preserve exact execution bindings, class identity, saved objects,
licensed environment identity and independent siblings.

Complete this as an integrated capability, not a new configuration type followed
by another handoff. Both a new class and an existing publication must reach ordinary
preparation, actual execution, meaningful recall, a settings trial and keep/restore;
missing required capabilities and genuine shared-resource conflicts must produce
specific recovery actions. Use native/sandbox host contexts and representative
multiple-instance/shared-environment cases. The assessment names the source owners
and fuller acceptance; there is no new policy database or approval sequence.

Runtime/dependency operations and DAW execution extend the same contract next.
Graphics and portability inform it from the start. Do not automatically choose the
latest Pure LoFi symptom as the task, postpone platform design behind one vendor,
or start a second host/lifecycle architecture. Routine engineering choices belong
to the engineer; physical execution continues to respect current machine custody.

## Retained physical state

Package `0.12.0general5deck` and paired Pure LoFi/FRAGMENTS publications remain on
the Deck. Installation and short changed-control recall passed; the interaction run
ended in terminal instance failure and never reached its planned soak. Disappearing
session status was initially misreported as missing audio. Terminal records showed
zero queued underruns before failure; the trigger remains unattributed.

The exact raw result, saved projects and predecessors are preserved privately.
[PR #204](https://github.com/kasselvania/Linux-VST-bridge/pull/204) remains draft and
unmerged; it retains the implementation and
[corrected physical result](evidence/audio-recovery/2026-10-03-general5-deck-installation.json).
The instruction rewrite is separately reviewable from main in
[PR #205](https://github.com/kasselvania/Linux-VST-bridge/pull/205). Neither this
assessment nor policy adoption approves the beta or changes the installed machine.

Installed config4 acceptance uses the disposable Ubuntu fixture under the main
engineer's machine custody. No Deck run is selected.
The Ubuntu VM was shut down normally after recovery; the builder is stopped and
Audiobookshelf remains running. Retained VM disks and private evidence are preserved.
Retain one VM or builder at a time, at most two CPUs, 4 GiB memory and 4 GiB combined
memory/swap, and 256 processes on the declared build fixture. Reserve host CPU 0–1
and capacity for Audiobookshelf. Other independently authorized owners retain their
work and machine-custody boundaries.

The operator's delegation preference is implementation on `gpt-6.1-sol` at `max`,
review on `gpt-6-astra` at `xhigh`, and computer use on `gpt-6.1-sol` at `high`.
