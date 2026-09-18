# NAD2 review-repair qualification

Executable source: `81bbf198e8bbe7df337312f2529c84b318c1ed64`  
Executable tree: `4713bfe14f2bfc57909f671254ee19e68e8e3a6b`  
Repair parent: `c46a1cd1831ed88b1ce6ce4cdf143b46da5753d7`

This supplemental receipt qualifies the two diagnostic-truthfulness corrections
requested during review. It does not replace or relabel the immutable qualification
for source `0ea735e471b2d34a68d9ce817506fe89fdd1ca23`.

`NAD2_STOP_SUBMITTED_PROGRESSING` now requires observed progress toward retirement:
`STOP_PENDING` in the successful control-return or final status, exact-process exit
progress, or monotonic listener disappearance. Initial or final process-wait loss,
initial or final listener-census loss, and unrelated SCM states remain
`NAD2_STOP_OBSERVATION_UNAVAILABLE`. Controls-mask-only changes do not establish stop
progress.

The frontend now distinguishes no call from a refused call using `control_count`.
An initial stopped, stopping, or unavailable observation reports that no
`ControlService` request was issued; only an attempted and refused request reports a
retained nonzero control error.

Production-law and production-parser regressions cover initial wait failure followed
by timeout, controls-only change under unchanged running state, a final paused state,
and initial listener-census loss followed by an ordinary census. Frontend regressions
cover no-call stopped, no-call stopping, no-call unavailable, and a refused call with
error 5. Existing exact-generation admission and retirement success requirements are
unchanged.

Local qualification passed 24 NAD1/NAD2 tests, 248 runtime tests with 44 platform
skips, 144 manager-library tests, 84 manager-binary tests, five manager-report tests,
and 31 frontend tests. Strict manager and frontend Clippy, Python compilation, and
Windows x86-64 adapter/fixture cross-compilation passed.

Exact-source hosted validation:

- [AP8](https://github.com/kasselvania/Linux-VST-bridge/actions/runs/35358431083) — passed
- [AP12](https://github.com/kasselvania/Linux-VST-bridge/actions/runs/35358431069) — passed
- [PX2](https://github.com/kasselvania/Linux-VST-bridge/actions/runs/35358431096) — passed

All 689 evidence files present at the repair parent are unchanged; this receipt and
its three companion records are additions. No installation, real daemon transition,
Native Access session, DAW, plug-in, installer, updater, or production operation
occurred. Reliable real-daemon shutdown remains unqualified.
