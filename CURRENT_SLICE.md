# Current task: completed physical Deck audio timing

## Authority and one claim

The operator prioritized physical timing immediately after the installed update and
instrument/effect recall/output checkpoint. The approved legacy-publication migration
prerequisite and normal physical Deck update now pass. Full recovery remains deferred.

Primary claim: measure installed Buffered and SameCallback delivery on the physical
Deck over thousands of fixed-size callbacks, with captured output, truthful callback
and request distributions, effective scheduling readback and exact owner retirement.
The declared 24-cell first-attempt matrix is complete. **Eight complete audio/state
lifetimes pass; sixteen fail. The installed candidate fails full audio acceptance.**
Native real-time scheduling was not achieved.

Basis: AGENTS.md “Work in complete capability increments” and “Verification that
matches the claim”; GOVERNANCE.md “Evidence and completion”; architecture 18.6
“General preparation and compatibility experimentation” and 18.7 “Platform execution
convergence”; accepted D-029; the operator's revised order and machine/model custody.

## Frozen artifacts and installed result

Product source commit `b137ca61043f1086cde1a71727f446985e0710cf`, tree
`67166c967f311b8b2a04253bae7ecc6695960aa3`; version `0.12.0refresh4deck`.
The approved source repair is limited to preparation/package-refresh authority and
its tests. [Source validation](evidence/audio-recovery/2026-10-04-legacy-publication-refresh-source.json)
retains the original regression, Linux suite and independent exact-source review.
Audio engine/Windows host source is unchanged from `e8a79926`; their frozen artifact
hashes are separate facts. The target engine/paired backend was rebuilt and verified.

Independent measurement-host source `fbb318988d7f9cec27613f036e0fe6669fdd2987`, tree
`771d45aa78bbc4a12967b6a2d9a23b7ffb494a07`; both role tests passed on the builder
and physical Deck. Its consumer, callback audit, runner and output oracle remained
frozen. Supplemental offline audits do not replace that declared output oracle.

The [physical report](evidence/audio-recovery/2026-10-04-refresh4deck-physical-timing.md)
and [exact evidence](evidence/audio-recovery/2026-10-04-refresh4deck-physical-timing.json)
record one normal installer/Setup Update action. All eight commercial class/module/
environment/configuration identities, explicit preferences and current/predecessor
artifacts are preserved. The former [legacy-publication refusal](evidence/audio-recovery/2026-10-04-refresh3deck-deployment-refusal.json)
remains retained; no experimental Pigments substitution occurred.
Completion 1.0.0 instrument/effect were scanned, inspected, prepared and experimentally
published through actual offered installed actions in their separate environment.
They do not acquire commercial qualification from these test results.

## Measurement outcome and gaps

At 48 kHz, M=N=256,128,64, both D0 SameCallback and D256 Buffered ran diagnostics OFF
and separately ON, once each. Each cell planned 4,000 unpaced main calls, four initial
N0 calls, D+13 tail frames and final N0. All raw rows/output and failed lifetimes remain.
Twenty-three main windows completed with exact captured audio. Instrument D0/N64 ON
stopped during main call 34, including its failed row and 256 unexpected zero values.
A matching retained phase records an unsatisfied completion predicate and 1.378-ms
wait against 1.333-ms allowance. Its request histogram covers 33 observed requests.

Ten SameCallback lifetimes refuse the N13 tail; one Buffered instrument N256 ON
lifetime loses tail audio/events despite SDK success. All four Buffered N64 lifetimes
return complete correct audio then refuse final N0 before state capture. Eight
passing lifetimes capture meaningful state and exact recall. Processing-failed
lifetimes did not reach state capture; state corruption is not established.

Native workers remained SCHED_OTHER policy 0/priority 0. RTKit commands exited 1,
with acceptance/refusal unknown; their exact DBus reason is an observability gap.
Windows RR/reset-on-fork priority 5 was read back in 23 lifetimes; the short failed
instrument D0/N64 ON lifetime lacks effective Windows readback. No scheduling
continuity or native real-time qualification follows. Local N/Fs is not a DAW graph
deadline. ON histograms describe observer-delivered requests, with bucket-upper-bound
quantiles and retained coverage counters; they are separate from raw main callbacks.

The first instrument D0/N256 OFF invocation began without an environment keeper;
remaining invocations used the managed warm keeper. OFF/ON are single first-attempt
comparisons with that startup condition disclosed, not causal diagnostic attribution.

The next bounded architectural target is the shared legal short/zero-frame completion
and containment contract: request/result/predicate attribution and a justified deadline
policy, retaining the observed main N64 failure. Distinguish unfinished rendering from
completed work blocked by presentation/control ordering. Review one callback-entry
time origin without resetting/stacking allowances; the observed N/Fs and 1 ms constants
are not authoritative general policy. The policy needs DAW/device justification.
Native scheduling failure-reason
observability remains a gap. No further source/policy change is authorized by this
measurement; no rerun, wait-budget tuning or expanded recovery campaign occurred.

## Machine custody and remaining scope

All 24 authenticated test DSP owners retired with confirmed transport/cleanup.
The selected service is active/running, DSP=0 and maintenance=0, with no cleanup
uncertainty or package transition. One healthy product-owned environment keeper
remains; it is distinct from retired test DSP sessions. Both first-party fixtures
remain SameCallback D0 with remembered Buffered 256. Commercial settings are unchanged.
Manager and Moonlight stream closed normally; Bitwig was not run. The builders and
capacity VM are stopped; Audiobookshelf remains running. Raw evidence, frozen package,
build logs and GUI evidence are preserved in verified durable allowlisted archives.

Execution/implementation custody: Sol6.1 xhigh. GUI: Sol6.1 high. Reviewers: Astra6
xhigh. Root orchestrates; the executor owns mutations. This supersedes the prior
Sol5.6 preference. Keep #207 draft; no merge or beta promotion is authorized.

FC-AUDIO-001 and FC-MGMT-008 remain open. Full package/history restoration,
switch interruption/recovery and modern stale-caller installed restoration remain
deferred. Real Bitwig processing, serial chains/reconfiguration, editors/automation,
interaction/soak, save/reopen/reboot, catalogue/platform coverage and distribution
obligations remain separate. Preserve the existing stack when separating review/
landing branches; the broader roadmap still selects splitting #200 with dependencies.
