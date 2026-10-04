# Current task: physical Deck audio timing

## Authority and one claim

On 2026-10-04 the operator explicitly prioritized physical timing immediately after
one successful installed update plus instrument/effect recall and output checks.
That checkpoint has passed. Remaining recovery acceptance stays open and must not
become a prerequisite for this measurement again.

Primary claim: measure current installed Buffered and SameCallback delivery on the
physical Deck over thousands of fixed-size callbacks, with actual output, truthful
callback/request timing distributions and effective scheduling readback. This is a
physical SDK-host measurement, not a real-DAW, device-deadline or endurance claim.

Basis: AGENTS.md “Work in complete capability increments” and “Verification that
matches the claim”; GOVERNANCE.md “Evidence and completion”; architecture 18.6
“General preparation and compatibility experimentation” and 18.7 “Platform
execution convergence”; accepted D-029; the operator's revised order above.

Preserved comparison product source: commit `e8a79926084df8d621f6dff36694218935d0c955`, tree
`0d47b93e49627467bd945a18a29dc6bbfb739a0c`. Independent test-host extension:
`fbb318988d7f9cec27613f036e0fe6669fdd2987`, tree
`771d45aa78bbc4a12967b6a2d9a23b7ffb494a07`. The extension changes only
`tools/beta/completion_host.cpp` and its tests; the private independent analyzer is
separately frozen. Source review and both Linux measurement-host role tests passed.

### Physical deployment blocker

[Refresh3deck deployment evidence](evidence/audio-recovery/2026-10-04-refresh3deck-deployment-refusal.json)
records target-compatible packaging, 13 native tests and independent paired
backend/engine verification. The normal Deck installer staged the package, but
its single Update action refused `candidate_preparation_required` before selection.
Pigments is the first selected legacy ordinary profile without a managed Candidate;
the other seven selected publications have matching candidates. The updater
incorrectly assumes all retained publications already have that newer record.

The selected software, all eight publications, exact artifacts and explicit
preferences remain unchanged; the predecessor service is healthy with no DSP or
maintenance owners and no unconfirmed cleanup. Normal old-manager reinspection
and preparation completed without changing publication. The resulting review
candidate does not establish a matching Candidate for the still-selected ordinary
profile. It was retained, not experimentally selected; no update retry occurred.

Physical audio timing has **not run**. The operator has now approved the narrow
legacy-publication migration repair and continuation to Deck timing. Amend the
frozen source only for that prerequisite; freeze its tested successor before the
next installed attempt. Do not silently replace Pigments, fabricate preparation
provenance, bypass admission, or reopen the broader recovery matrix.

Approved repair base: commit `b0d5a1b0c25872a4aa5d6e426287b26adb8c38f8`, tree
`4a1973495a6a27c8673b3a9e0b6b66afa8525287`.

The repair must recognize a valid retained ordinary profile-backed revision that
predates managed Candidate records. Validate the retained revision/census/report,
class, module, environment, compatibility and registration; fresh-inspect and
prepare the target pair through existing owners. Preserve the distinction between
a predecessor revision and a predecessor Candidate. Keep current-candidate checks
strict and preserve exact class IDs, configuration, buffering, selected-state
checks and failure cleanup. Add a failing legacy-revision regression and focused
malformed/stale-input coverage. Scope is preparation and package-refresh consumers,
their tests and installed evidence; no audio, Windows host, runtime or graphics
behavior changes. Share the corrected preparation contract with existing ordinary
restoration, without running or claiming the deferred recovery matrix.

The [source repair validation](evidence/audio-recovery/2026-10-04-legacy-publication-refresh-source.json)
now passes the original-failure regression, 259 Linux library tests, 47 preparation
tests, package-update tests in both feature configurations and all-target/all-feature
static checks. Independent exact-source review found no remaining blocker. Normal
Deck update and physical timing remain unperformed on this successor.

## Completed installed checkpoint

[Refresh3 installed evidence](evidence/audio-recovery/2026-10-04-managed-refresh-installed-checkpoint.json)
records one normal successful update on the retained Ubuntu VM. All six selected
first-party publications use the new engine. Vendor module/class/environment,
explicit preferences and original saved objects stayed intact; the target service
is active and ownership/transition cleanup is confirmed.

Both original instrument/effect states restore meaningful values and pass independent
whole-output comparison: 672,424 and 666,280 sample values, zero mismatches,
zero unexpected zeros, complete retirement. The cached legacy SDK caller receives
actual protocol refusal 8 before a new owner/result is created. A deliberately
missing-capability package is refused with the exact predecessor preserved and its
service restored automatically. This is SDK/VM functional evidence, not audio timing.
Original refresh1/refresh2 failures remain retained. The update's lengthy preparation
on this constrained VM is not a reason to start another manager optimization now.

## Immediate work and boundary

1. Repair and test the approved legacy-publication prerequisite, review the exact
   diff, then freeze and build target-compatible SteamOS packaging. The
   Ubuntu artifact uses Python 3.14 and must not be installed on the Deck's 3.13.
   Use existing assembly/signing, pinned tools and matching Windows host inputs.
2. Compile and run the focused measurement host tests on the bounded Linux builder.
   Preserve its separate source/binary identity from the product package.
3. Preserve/read back the Deck's current selected setup and use supported installed
   selection/preparation controls. No licensed-prefix recreation, vendor binary/state
   rewriting, manual registry repair, ambient runtime upgrade or rootfs change.
4. Measure at Fs=48 kHz with actual M=N=256,128,64 in SameCallback D=0 and Buffered
   D=256. Buffered D=128/64 are not supported selections. Use the same declared
   fixture/workload/settings for the comparisons; record exact versions and identities.
5. Retain 4,000 fixed-N main calls per measured run plus all zero/tail calls and
   complete captured output. Report median/p99/max complete SDK callback duration;
   keep failed recorded calls in statistics and retain full-lifetime failure results.
   Use existing bounded request/reply histograms in a separately declared diagnostics
   invocation, reporting histogram precision and observer effects honestly.
6. Record the actual native and Windows render-thread scheduling policy/priority,
   permissions/limits and relevant hardware conditions. Do not infer real-time
   scheduling from a launcher nice value or make blanket privilege changes.

The test host's sustained scenario is explicitly unpaced. Its local N/Fs comparison
is not the available deadline of a whole DAW graph. Distinguish startup, fixed-N
main callbacks and zero/tail calls; do not remove failures to produce a clean window.
Bounded 4,096-row storage covers the largest 4,010-row run. Preserve actual returned
audio and independently analyze it, including any failed lifetime.

Scope: target packaging, focused measurement host/analysis, supported configuration
selection, existing diagnostics and physical execution. No engine rewrite, further
VM timing candidates, audio wait-budget tuning, graphics/runtime expansion or
manager/recovery implementation beyond the approved prerequisite above. A newly observed blocker requires explicit
attribution and the smallest safe route to measurement, not automatic reopening of
the full manager matrix. No physical success has yet been observed for this candidate.

## Remaining product and beta work

FC-MGMT-008 remains open. Full package predecessor restoration, normal individual
history restoration, switch interruption/recovery and modern stale-caller restoration
remain unperformed on refresh3. The separate experimental `rollback_exact` route
may refuse a pre-contract predecessor and has no installed result. Keep these in
the managed-update workstream; this checkpoint does not close them.

Real Bitwig processing, serial chains, legal reconfiguration, editor/automation,
interaction and soak, save/reopen/reboot, cross-platform/catalogue qualification,
release signing and distribution obligations remain separate open gates.

## Machine custody and cost

The retained Ubuntu VM shut down normally after the checkpoint. The SteamOS-target
builder was restarted for the approved migration validation; no VM or second builder
is running, and Audiobookshelf remains running. Builder limits: CPUs2–3,
two CPUs, 4 GiB memory with equal combined memory/swap limit, 256 processes. Reserve
CPU0–1 and capacity for Audiobookshelf. The Deck is reachable (Galileo, Python3.13.5,
glibc2.41); readback found no Bitwig process, and its installed selection is unchanged
after the refused update and retained preparation. Setup was closed normally.
The first-party Completion instrument/effect installer completed in a separate
managed environment with confirmed cleanup and manifest-matching modules; neither
has been prepared or published yet. The commercial selections are unchanged.
Root owns machine mutations and artifacts. Implementation agents Sol5.6 xhigh;
reviewers Astra6 xhigh; computer use Sol6.1 high.

## Review and landing

Keep #207 draft. Audio/START/admission ends at `22e6569c`; managed-refresh policy
and implementation start at `80835886`. Preserve the existing stack and evidence
when separating review/landing branches. PR separation must not delay physical
measurement. The broader roadmap still selects splitting #200 with its dependency
map retained. No merge or beta promotion is authorized by these results.
