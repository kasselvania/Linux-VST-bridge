# Current capability: managed bridge upgrade and recovery

## Authority and outcome

The operator approved the managed-refresh policy on 2026-10-04: one normal
**Update Bridge** action prepares and verifies replacement bridge components before
switching. The customer closes the DAW when needed, without per-plug-in refreshes,
compilers, vendor reinstalls or terminal work. Preparation failure leaves the prior
selection intact. **Restore previous setup** remains a normal, verified operation.

Base commit `22e6569cd0afa9f0112518819d5b95e47932bbc8`, tree
`436e5b30903ff01e37502318671e6f8c569d239f`. Branch
`codex/audio-completion-contract`, draft PR #207, stacked on #206. Basis:
AGENTS.md “Work in complete capability increments”; GOVERNANCE.md “Evidence and
completion”; architecture 18.6 “One configuration, distinct responsibilities” and
18.7 “Shared configuration, separate facts”; accepted decision D-029.

Primary claim: a populated installation can update the bridge through normal
controls, refresh every affected selected native publication into the new admission
contract, reopen its saved state, and recover a failed update or restore a retained
setup without manual publication repair. Preserve vendor module/class, environment
and licensed machine identity, saved objects and supported explicit preferences.

## Existing proof and exact remaining gap

[Modern admission source evidence](evidence/audio-recovery/2026-10-04-loaded-engine-admission-source.json)
records the real audio9 loader mismatch, repaired factory refusal, LVB5 identity
binding, peer-generation checks and source regressions. Its applicable CI checks
passed at `22e6569c`; the dispatch-only unfamiliar fixture job was skipped. This
is source proof, not installed migration or physical acceptance.

The operator decision resolves the previously recorded policy gap. Pre-contract
LVB1–4 publications must be refreshed before executing under the repaired manager.
Retain original files/history; do not rewrite their provenance or silently treat
old handshakes as proof of cached metadata. Old proxy binaries are no longer an
eligible execution route under the repaired manager. Normal restoration must use
verified compatible bridge components for the exact retained vendor/configuration.
A retained full package predecessor requires its coherent manager/publication pair.

The coordinated source implementation now prepares against the explicit target,
uses the existing package journal and exact publication intents for update/restore,
and connects normal Setup controls to service recovery. Legacy LVB1–4 admission
is refused under the modern manager. Current source validation is recorded in
[managed refresh source evidence](evidence/audio-recovery/2026-10-04-managed-refresh-source.json).
The retained [installed baseline](evidence/audio-recovery/2026-10-04-managed-refresh-installed-baseline.json)
contains six selected classes and original instrument/effect state and output. A
subsequent parallel CI run exposed a temporary-listener inheritance race in
transport initialization. The [bound-only socket repair](evidence/audio-recovery/2026-10-04-denial-socket-fork-repair.json)
retains the refusal checks, reproduces the old failure with a forked child, and
passes the corrected Linux module checks in default and candidate builds.
The first package attempt remains retained and uninstalled: its private cache
layout and stale Windows source roster were refused. The committed refresh1 build passed its native tests, paired rebuild and applicable
CI using the matching Windows input. Its first installed check is retained in
[status failure evidence](evidence/audio-recovery/2026-10-04-managed-refresh-status-failure.json):
the normal installer staged application files, but Setup timed out before offering
Update. A standalone status call took 34.65 seconds. A live process snapshot
showed 3.18 GB of aggregate reads while a Wine runtime DLL was open; source
inspection identifies repeated full-runner verification in this status path.
No update was submitted; the six-class audio9 baseline remained
exact and healthy. The [source correction](evidence/audio-recovery/2026-10-04-package-status-boundary-source.json)
now separates bounded status records from executable launch/mutation verification,
with consistent refresh action/provenance and guarded frontend handoff. Its Linux
checks and independent source review pass. The committed refresh2 successor's
[installed attempt](evidence/audio-recovery/2026-10-04-managed-refresh-inventory-failure.json)
now reaches normal Update; explicit status readback took 0.309 seconds. The
deliberately missing-capability package was refused with automatic service
restoration and exact predecessor preservation. The valid package then failed
`preparation_inventory_superseded`: its retained published Completion fixtures
no longer have matching current discovery entries. Fresh target inspection
completed and retired; no selection changed, and the prior service recovered.
The held old caller was never released, so stale-caller acceptance remains
unperformed. The [retained-publication source repair](evidence/audio-recovery/2026-10-04-retained-publication-refresh-source.json)
now connects preparation, package publication, ordinary restoration and truthful
selected readback. Its unchanged-base regression reproduces the failure; Linux
library and affected manager suites, strict lint and independent source review
pass. New-discovery freshness remains strict, with exact predecessor, environment,
module, inspection and runtime verification retained. Build the committed successor
and resume the original installed acceptance; source success does not replace the
failed installed result.

FC-MGMT-008 remains open until the committed package completes the installed
update, current-caller recall, stale-caller refusal, failure/interruption recovery
and both ordinary restoration routes. Source tests and the prior baseline do not
establish that new installed behavior.

## Implementation boundary

Trace and extend the existing package adoption/transition journal, preparation
kit and candidate/history owners, publication/registry and normal manager controls.
Prepare against an explicit target package without temporarily selecting it or
inventing a parallel configuration database. Recheck exact affected inputs before
commit; retain a recoverable old/new state on partial failure or interruption.
Make progress and a specific failure visible. Resume/recovery must not require the
customer to know an internal revision or manually restart services.

Scope includes `package_authority`, `setup_install`, reusable preparation/build,
publication/history, admission and readback, manager bootstrap/operator controls,
and corresponding contract/installed fixtures. Dependencies permitted to change:
existing package/record contracts needed for this migration with explicit versioning
and old-record handling. No new engine, runtime experiment, vendor campaign, audio
wait budget, Windows DSP change or graphics expansion. No saved-state rewriting,
licensed-prefix recreation or unrelated host privilege changes.

## Acceptance

Use the retained populated Ubuntu installation and first-party instrument/effect
with meaningful saved state, automation and deterministic output. Its currently
selected audio9 manager and audio4 native predecessor are historical baselines,
not dependable-audio claims. Preserve exact identities before mutation.

- One normal update prepares all affected published classes, verifies exact new
  engine/descriptor/host bindings and selects only after successful preparation.
  Stable external class IDs, module/environment and explicit settings survive.
- A cached old caller and a mismatched modern caller receive specific refusal
  before runtime/DSP/transport admission; matching refreshed callers succeed.
- Existing saved state reaches the same selected vendor implementation; returned
  values/audio and newly captured provenance remain correct; retirement completes.
- Inject preparation and switch failures and interruptions. Preserve the original
  failure and prior selection, or retain an explicit recoverable transition until
  exact old/new ownership is established. Healthy siblings and saved bytes survive.
- Restore the exact retained vendor/configuration through ordinary controls with
  compatible bridge components. Test package-predecessor consistency as well as
  publication recovery; no reboot, manual record deletion or CLI workaround counts.
- Test unsupported/missing retained metadata, foreign edits, active DAW/owners,
  insufficient capacity and unresolved cleanup with truthful, actionable refusal.
- Retain sanitized source, artifact, whole-run and recovery evidence; independently
  review source and installed results. Commit/push coherent completed work.

## Machine custody and cost

The constrained builder is in use for source validation and candidate construction;
the retained Ubuntu VM is stopped. Audiobookshelf is running.
Use one builder or VM at a time, two CPUs on host 2–3, 256 processes. Builder 4 GiB;
Ubuntu 6 GiB guest / 8 GiB container; combined outer memory/swap equals memory. Reserve
CPU 0–1 and capacity for Audiobookshelf. Root owns machine mutations and artifacts.
No Deck mutation during source development. Preserve all original failed runs.
Implementation agents Sol 5.6 xhigh; reviewers Astra 6 xhigh; computer use Sol 6.1 High.

## Following physical gate

No more VM timing candidates. After installed migration/recovery is reviewed, use
one frozen Deck candidate in explicit Buffered and SameCallback D=0 modes. Measure
thousands of callbacks at actual 256/128/64-frame blocks: median/p99/max of the
request/reply interval and whole callback separately, actual output/missing spans,
exact identities and effective native/Windows render-thread scheduling. Keep Fs,
maximum/actual block, bridge delay and vendor latency distinct. An unavailable
realtime policy is a reported limit, not authority for blanket privilege changes.

Audio9's Buffered512 off/on lifetimes retained exact output/state/retirement but
overall timing failed. Its 107 calls / 49,668 four-lane frames represent 1.03475 seconds,
not endurance or SameCallback timing. See
[installed audio9](evidence/audio-recovery/2026-10-04-audio9-start-overlap-comparison.json)
and the [allowance audit](docs/RESEARCH_BASIS.md#2026-10-04-callback-allowance-and-audio8-scope).
Physical DAW/serial-chain/reconfiguration and musician interaction/soak remain open.

Keep #207 draft. The roadmap selects splitting #200, retaining the original stack
and recording its dependency map before extraction. No merge or beta promotion.
