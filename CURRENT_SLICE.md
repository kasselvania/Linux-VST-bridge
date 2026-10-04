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

Frozen product source: commit `e8a79926084df8d621f6dff36694218935d0c955`, tree
`0d47b93e49627467bd945a18a29dc6bbfb739a0c`. Independent test-host extension:
`fbb318988d7f9cec27613f036e0fe6669fdd2987`, tree
`771d45aa78bbc4a12967b6a2d9a23b7ffb494a07`. The extension changes only
`tools/beta/completion_host.cpp` and its tests; the private independent analyzer is
separately frozen. Source review passed; Linux execution remains to be completed.

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

1. Build target-compatible SteamOS packaging from the frozen product source. The
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
VM timing candidates, audio wait-budget tuning, graphics/runtime expansion or new
manager/recovery implementation. A newly observed blocker requires explicit
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
builder is running; no second builder or VM may run concurrently. Builder: CPUs2–3,
two CPUs, 4 GiB memory with equal combined memory/swap limit, 256 processes. Reserve
CPU0–1 and capacity for Audiobookshelf. The Deck is reachable (Galileo, Python3.13.5,
glibc2.41); readback found no Bitwig process, and its installed selection is unchanged.
Root owns machine mutations and artifacts. Implementation agents Sol5.6 xhigh;
reviewers Astra6 xhigh; computer use Sol6.1 high.

## Review and landing

Keep #207 draft. Audio/START/admission ends at `22e6569c`; managed-refresh policy
and implementation start at `80835886`. Preserve the existing stack and evidence
when separating review/landing branches. PR separation must not delay physical
measurement. The broader roadmap still selects splitting #200 with its dependency
map retained. No merge or beta promotion is authorized by these results.
