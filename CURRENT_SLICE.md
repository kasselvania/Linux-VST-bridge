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

## Controlled capacity observation

Diagnostic package `0.12.0config5observe`, source `c9117d6a`, retains the config4
audio engine, Windows host and runtime. The [memory comparison](evidence/preparation/2026-10-03-controlled-capacity-observation.json)
completed one 102.4-second reference-instrument lifetime at each of 3 GiB guest /
4 GiB container and 6 GiB guest / 8 GiB container, with an independent effect's
ordinary settings apply during audio. Both compared 9,830,400 samples with no
mismatches, missing spans or overruns, then retired and restored exactly. The
larger guest had no measured swapping and lower memory-pressure time. Fixed order,
cache carryover, tracing and one lifetime per size prevent causal or qualification
claims. The earlier failed workflow remains failed; preliminary tool failures
are retained separately.

The source also preserves pending UI action ownership (96 frontend tests and
strict Linux lint passed). This does not qualify the incomplete GUI recovery.
Bounded control timing and exact-cohort diagnostics are audit-feature-only;
applicable CI at `c9117d6a` passed. Neither is an audio repair.

## Next bounded work

Close the observed render-host ownership/mapping gap through existing per-launch
custody. The authenticated bootstrap and descendant tracker do not provide an
explicit final Windows-host witness. Both observed cohorts had zero exact mapped
render targets, before RealtimeKit. Establish the final host's generation, namespace
and session/status binding before selecting and reading back its render thread.
Do not infer ownership from a name or import all keeper/sibling processes. Missing
cohort membership versus mapping projection remains unassigned.

Separate ordinary display/offer readback from deep worker/launch admission using
the existing control records and transaction owners. Observed product reads took
13.24–14.36 seconds, dispatch 10.62–10.66 seconds and workers 14.32–21.14 seconds.
Retain exact execution validation; presentation must not claim fresh byte
verification from metadata. These costs are measured, but are not the demonstrated
cause of the earlier missing block. Correlating worker service at a future gap still
requires exact thread identity; clean runs supplied no gap-based native worker join.
Preserve the failed lifetime; clean repetitions cannot replace it.
Also resolve the ordinary recovery interaction: its incomplete GUI path cannot be
qualified from a successful CLI fallback.

Source review confirms that ordinary projection/admission repeats candidate,
history and complete runtime-tree checks. Native scheduling remained ordinary
because the existing DAW process RT budget was unavailable; Windows exact-owned
render-thread selection failed before RealtimeKit invocation. Those are observed
capability/cost gaps, not a demonstrated cause of this missing block. Do not raise
buffers, change the runtime or priorities, or widen privileges as a guessed repair.

## Selected implementation endpoint

### Active architectural hardening

The operator selected these shared repairs on 2026-10-03. Implementation starts
from commit `9dd5367e69ec42fbf232aef4b22371fb7ba78fb3`, tree
`6130ae769eef0ce3ea8ce9dfa472c4266fd4ea7a`. Basis: Architecture 18.7,
"Shared configuration, separate facts" and "Capability and ownership scope";
the controlled observation and next bounded work above. The primary claim is
that ordinary configuration controls and exact execution custody use their proper
existing owners without weakening execution admission. This is a prerequisite
repair within the unchanged endpoint below, not completed installed acceptance.

Scope is the manager's record/projection/action consumers and the supervisor's
per-launch process custody and scheduling adapter, with their focused regressions.
Ordinary readback may validate bounded control records, bindings, transaction state
and the selected physical pointer. It must identify that scope truthfully and must
not claim a fresh check of executable payloads. The queued worker executable is
freshly verified; workers and launch retain deep payload verification and final
ownership/publication rechecks. No persisted cache becomes execution authority.

The supervisor must establish the final Windows host's authenticated process
generation and exact session/status binding before render-thread selection. A name,
self-reported PID, or membership in the shared keeper cannot grant instance custody.
Retain uncertainty if the implemented runtime cannot establish that binding. Do not
change scheduler policy or privileges to conceal a missing owner.

Source acceptance requires: ordinary overview/product/offer reads avoid bulk payload
hashing, runtime-tree walks and preparation-kit execution; altered payloads still
refuse at worker/launch admission; altered bindings, stale actions and affected
owners refuse at their authoritative boundary. Custody tests cover unrelated
senders, partial/coalesced messages, PID reuse, startup failure and exact retirement.
Existing siblings, saved state, unrelated preferences and historical identities
remain preserved. Independent review precedes installed claims.

Installed comparison uses the same disposable Ubuntu reference fixture, pinned
runtime and 1024-frame settings described in the retained observation. Verify
ordinary controls, current custody/scheduling readback, actual captured audio and
positive retirement together. Retain the failed original and all new attempts;
the comparison cannot establish a root cause or erase the failed workflow. Rollback
uses retained paired package/publications and ordinary exact restoration. No engine,
runtime, dependency, buffer or privilege experiment is included; the Deck is unchanged.

### Source checkpoint

The [foundation source result](evidence/preparation/2026-10-03-foundation-source.json)
records the reviewed exact file identities and Linux validation: 392 runtime tests,
560 manager tests (two existing opt-in tests excluded), 97 frontend tests, audit
and report tests, packaging-helper tests and strict manager/frontend lint passed.
The final Windows writer uses kernel-authenticated, generation-pinned custody
and exact status mapping through the existing launch channel. Ordinary controls
use bounded records; workers and launch retain deep execution validation. Explicit
history completion and lineage-before-candidate publication replace incidental
migration during readback. Earlier failed test attempts remain retained.
This is source acceptance only. Paired installation, exact pinned-runtime custody,
integrated audio/settings and ordinary GUI restoration remain to be performed.

### Complete configuration workflow

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
The Ubuntu comparison VMs were shut down normally after exact restoration.
Retained VM disks and private evidence are preserved; Audiobookshelf remains running.
The next observation uses one VM or builder at a time, at most two CPUs on host
CPU 2–3 and 256 processes. Reserve host CPU 0–1 and capacity for Audiobookshelf.
On 2026-10-03 the operator authorized an 8 GiB memory allowance for a controlled
capacity comparison. Retain the original 3 GiB guest / 4 GiB container as the
baseline; compare a 6 GiB guest / 8 GiB container, leaving room for VM overhead.
Combined memory/swap equals each container's memory limit. Change memory separately
from runtime, buffering, scheduling and software; record actual pressure and quota
deltas. The builder retains its 4 GiB limit. No memory setting establishes audio
qualification. The operator also requested a clean-room comparison with current
yabridge and relevant Proton/Wine mechanisms. Other independently authorized owners
retain their work and machine-custody boundaries.

The operator's delegation preference is implementation on `gpt-6.1-sol` at `max`,
review on `gpt-6-astra` at `xhigh`, and computer use on `gpt-6.1-sol` at `high`.
