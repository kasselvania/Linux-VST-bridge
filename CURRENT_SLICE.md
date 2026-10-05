# Current task: legal short/zero-frame completion and containment

## Authority and one claim

The operator authorized continuation after the failed physical timing endpoint.
Primary product claim: legal short and zero-frame calls preserve one ordered
request/result contract and return their complete due audio/events/parameters
within a declared completion/containment policy, or explicitly fail while keeping
ownership safe. The observed D0/N64 main failure remains in scope alongside short
tails and Buffered final-N0 predecessor debt. This claim is not yet established.

Base commit `35bbcac4f3399d18d4de57f10ff2848fe4414d1d`, tree
`8782bca8c4b56f8d3197f1d1d88e8adbbb633c10`, branch
`codex/audio-completion-contract`; keep #207 draft, no merge.
Basis: AGENTS.md “Design the shared system before selecting patches”, “Work in
complete capability increments” and “Engineering protections”; GOVERNANCE.md
“Responsibility and authority” and “Evidence and completion”; architecture
18.1–18.4; accepted D-022 and D-028; the operator's continuation and machine/model
custody. Preserve existing ownership, epoch, identity, transport and state semantics.

## Current observation and pending decision

The [retained physical report](evidence/audio-recovery/2026-10-04-refresh4deck-physical-timing.md)
records eight passing and sixteen failing full audio/state lifetimes out of 24.
The installed refresh4 candidate remains failed for full audio acceptance. Native
workers remained ordinary scheduled; the native RTKit failure reason is unobserved.
These facts are unchanged by the current source investigation.

Real Session/IPC15 worker tests now reproduce exact N0 refusal behind preceding
Buffered operations in two controlled states: a reply still withheld, and a reply
already validated with owned native output but not yet published. Both preserve
repeated sample position and exact ticket. A finite positive-control harness
checks real FIFO mixed short/repeated-N0 requests and returned N0 parameter points;
its one-second harness bound is not a product policy. The D0/N64 refusal control
retains uncertainty if mapped request admission was not observed. No lost-wakeup
or correlation defect has been established. Tests discriminate possible states;
they do not assign causes to every retained physical failure.

**Explicit authority conflict:** architecture 18.4 currently prescribes N/Fs and
N0 1 ms, and forbids using the worker's five-second bound synchronously. Newer
research and physical evidence establish that N/Fs, M/Fs and 1 ms are local policy,
not observed device-period/serial-graph deadlines. The [reviewed proposal](evidence/audio-recovery/2026-10-04-short-completion-proposal.md)
uses the existing AUDIO five-second outer containment ceiling from one callback-entry
origin; each admitted AUDIO retains its originating bound, earlier predecessor
bounds are never renewed, and death/cancellation may fail earlier. It includes
both D0 exact ticket and Buffered due-frontier/control dependencies through final
SDK sink delivery. It can impose a seconds-long DAW-thread wait. The requested
five-second expiry is not a guaranteed wall-clock return during descheduling or
an unpreempted host SDK sink. This consequential amendment is **pending operator
decision**. Production policy and physical software remain unchanged meanwhile.

## Scope and acceptance

Current source changes are test-only: the real worker discrimination harness and
a cfg(test) publication hold after Session validation. No production branch or
storage is introduced. The [test evidence](evidence/audio-recovery/2026-10-04-short-completion-discrimination.json)
retains exact source/validation scope. The proposed policy amendment, exact source
review and operator decision precede any production repair or hardware mutation.

After acceptance of the contract, make the coherent minimal change across Rust
completion/worker policy and the native C ABI/SDK final check. Test FIFO predecessor
debt, short/full/repeated-N0 calls, due results, serialized control, cancellation,
dead peers, stale epochs, late publication and SDK delivery crossing the same
absolute bound. No per-stage reset, stacked allowances, fixture/product exceptions,
concurrent state restoration/DSP or invented external deadline is permitted.

Only a reviewed frozen candidate warrants a new installed physical comparison:
retain every first attempt, captured complete audio/state, diagnostics OFF and
separately ON, effective scheduling scope and positively confirmed retirement.
Keep raw timing/performance separate from complete-output acceptance. No real-time,
DAW, soak, recovery-matrix, beta or general platform claim follows from this slice.

## Machine custody and non-goals

The selected physical package remains source `b137ca61043f1086cde1a71727f446985e0710cf`,
tree `67166c967f311b8b2a04253bae7ecc6695960aa3`, version `0.12.0refresh4deck`.
Commercial preferences and licensed environments remain untouched. All 24 prior
test DSP owners retired; the healthy product-owned environment keeper remains.
Manager/Moonlight are closed, no Bitwig session is running, builders/capacity VM
are stopped and Audiobookshelf remains running. These are retained last-checkpoint
facts; no new physical invocation or machine mutation occurred in this step.
Use one builder/VM at a time and reserve Audiobookshelf resources. Do not change
native RT privileges or runtime, graphics, package/recovery policy, authorization
or licensed state. No hardware mutation is authorized until a reviewed plan/frozen
candidate is ready.

Execution/implementation: Sol6.1 xhigh. GUI: Sol6.1 high. Reviewers: Astra6 xhigh.
Root orchestrates; the executor owns mutations. FC-AUDIO-001 and FC-MGMT-008 remain
open. Full recovery, Bitwig processing/recall, catalogue/platform and distribution
qualification remain separate. No merge or beta promotion is authorized.
