# AP16 — Causal 512-frame delivery reliability

## Authority and basis

AP16 is the active implementation slice.

- Integrated basis: `510d88bd89adbf8b101140907f27dc9885af9f25`
- Integrated AP15 product basis: `d4076e9cc02028bf268089ca0bb86aeac36ef947`
- Active issue: [#72 — AP16: Causal 512-frame delivery reliability](https://github.com/kasselvania/Linux-VST-bridge/issues/72)
- Branch: `codex/ap16-causal-delivery-reliability`
- Product setting: 512 added frames per proxy, selected/supported/recommended
- 256-frame setting: available but unqualified and outside the initial decision

Continue in one implementation PR against `main`. Leave the PR open and unmerged for independent technical review. Normal implementation, focused diagnostics, deterministic tests, necessary builds, reversible installation and proportionate Steam Deck evidence are authorized under [AGENTS.md](AGENTS.md).

AP15 is complete. Its revision-7 direct-editor profiles, exact artifacts, stable processor/controller identities, saved projects, environment, service ownership and revision-3 rollback targets are the known-good baseline. Do not reopen AP15 editor architecture or reinterpret its retained failures.

The repository is publicly readable but proprietary. Preserve [COPYRIGHT.md](COPYRIGHT.md) and [CONTRIBUTING.md](CONTRIBUTING.md); AP16 grants no licensing, contribution, publication or redistribution authority.

## Current implementation result

The selected active-phase miss is causally assigned to an ext4/JBD2 journal wait while the native worker writes its shared audio mapping. The installed manager/supervisor now give native and Windows owners the same private tmpfs-backed hot files, preserving the pinned host's durable C: readiness/gate handshake. No native/Windows artifact, protocol, profile, buffer, delay or scheduling policy changed.

Matched 90-second trace-enabled playback went from FRAGMENTS losing 1,536 frames behind a 28.502 ms preparation stall to zero missed frames on either device. Two separate 90-second trace-off confirmation intervals also had zero new gaps; the matched trace-off baseline lost 2,304 FRAGMENTS frames. Full-session startup, editor/removal and closing gaps remain counted and unresolved. Direct editors, local output, explicit protected-project save/reopen and independent FRAGMENTS removal were checked. Both devices retired positively; service and keeper remain active, with zero DSP leases, no pending transaction, tracing off and bridge CPUWeight unset/effective 100.

The corrected immutable software is installed for independent review. Ordinary revision-7 publications and exact revision-3 rollback targets remain unchanged. The scoped Flatpak runtime-directory grant was explicitly approved. The first installation's exit-82 handshake failure and the repair are retained. See [docs/AP16.md](docs/AP16.md), [matched results](evidence/ap16/matched-comparison.json), [preservation](evidence/ap16/preservation.json), and [final physical readback](evidence/ap16/installed-final.json). PR #89 remains open and unmerged; review determines acceptance. 512 remains selected/supported/recommended; 256 remains unqualified.

## Product outcome

Under ordinary revision-7 Pure LoFi → Efx FRAGMENTS operation at 48 kHz and 512 added frames per proxy:

```text
an actual request misses its presentation boundary
-> the exact request and callback deadline are retained
-> elapsed time is assigned to a real owner
-> one demonstrated project-owned mechanism is repaired
   or one demonstrated external mechanism receives a truthful bounded mitigation
-> matched before/after runs preserve musical timing and product behavior
```

AP16 does not promise to eliminate every historical gap. It must close one reproducible residual stall class causally rather than collecting another undifferentiated timing matrix.

## Preserved baseline

The following remain fixed unless the demonstrated cause requires a narrowly reviewed change:

- ordinary revision-7 `verified_exact_fixture` profiles;
- exact Pure LoFi 1.0.0.6121 and Efx FRAGMENTS 1.0.0.2925 modules;
- pinned Proton-SLR environment and runner identity;
- Bitwig 6.1 Flatpak launched normally through Applications;
- SteamOS 3.8.16 reference fixture;
- float32, 48 kHz, actual host maximum 512;
- 512 added frames per proxy;
- AP15 direct detached vendor-editor lifecycle;
- opaque state, automation and stable external identities;
- exact revision-3 rollback targets;
- service/keeper ownership and normal CPUWeight baseline;
- truthful gap, fault and cleanup counters.

AP13's concurrent read-only state capture and supervisor-cost repair remain accepted. Reuse their evidence; do not replay the full AP13 campaign or restore the old state-delivery barrier as a comparison mechanism.

## Exact engineering question

For one request that actually misses its presentation boundary, distinguish:

1. callback admission and remaining budget;
2. callback-to-transport-worker queue wait;
3. native worker CPU execution versus runnable/off-CPU delay;
4. native preparation and send;
5. mailbox/socket wait and Windows dispatch;
6. vendor `process()` elapsed and CPU time where available;
7. Windows result publication and wakeup;
8. Linux reply, validation and publication;
9. publication-to-callback presentation;
10. overlapping editor, controller, state or host callback activity.

Wall time is not CPU time. A mean is not a tail guarantee. Correlated non-plug-in elapsed time is not automatically scheduler delay. A gap counter recorded while transport is stopped is not automatically an audible dropout. Preserve those distinctions in code, evidence and claims.

## Source ownership

Start from the current production owners:

- `native-vst3-proxy/backend/src/queued.rs`: `ap3-transport`, request admission, worker lifecycle, mailbox delivery, validation and output publication;
- `native-vst3-proxy/backend/src/observer.rs`: bounded optional observations and gap/request correlation outside the callback;
- `native-vst3-proxy/backend/src/performance.rs`: fixed histograms, slow-request summaries and inactive-only performance settings;
- `native-audio-client/`: mapped mailbox, callback-facing presentation and transport primitives;
- `windows-factory-probe/source/offline_processing.*`: vendor process timing;
- `windows-factory-probe/source/mapped_processing.*`: mapped completion diagnostics and Windows-side ownership;
- `bridge-manager/runtime/session.py`: supervised process cohort, trace opt-in and retained cleanup reports;
- retained evidence under `evidence/AP9`, `evidence/AP10`, `evidence/AP13`, and `evidence/ap15`.

The existing trace already carries `queued`, `started`, `prepared`, `sent`, `replied`, `validated`, `published`, Windows `process_ns`, request sequence/position and callback ancestry. It does not by itself distinguish executing, runnable-but-unscheduled, blocked, page-faulting or waiting on another owner. Add only the smallest bounded evidence needed to decide among the live alternatives.

## Diagnostic law

Diagnostics are optional consumers, not transport authority.

- No file reads, logging, allocation, locks, sleeps, process inspection, priority changes or diagnostic waits enter the audio callback.
- Prefer fixed scalar capture on the existing worker path and non-real-time observation outside it.
- Every timestamp or CPU/scheduler counter names its clock domain and collection point.
- Retain a bounded window around the first relevant miss, plus explicit dropped/unretained counts.
- Preserve trace-disabled behavior and measure trace overhead.
- Do not add a permanent polling loop or background profiler to ordinary playback.
- Never retain raw audio, proprietary binaries/state, presets, account data, authorization material or complete prefixes.

A narrowly bounded thread CPU clock, scheduler-stat bracket, `perf sched` capture, Windows thread-cycle/CPU counter or fault-injection seam is allowed only when it answers the selected causal question. Do not add them ceremonially.

## Repair law

Repair only a demonstrated owner and mechanism.

A valid repair may address a proved queue/wakeup race, avoidable worker sleep/backoff, project-owned priority inversion, page-fault/preallocation gap, unnecessary serialization, result-publication ordering defect or equivalent source-grounded cause.

If the selected class is controlled by vendor execution, the kernel, the compositor or another external owner, do not fabricate a core speedup. Use a truthful bounded mitigation or classification while retaining 512.

Prohibited shortcuts:

- suppressing or renaming gaps;
- dropping notes, events, automation or results;
- changing sample positions or host timing semantics;
- moving work into the callback;
- adding hidden latency/buffering without profile and host compensation;
- global CPU pinning, extreme CPUWeight, `nice -20`, `SCHED_FIFO` or machine-wide tuning as the product fix;
- unbounded busy-spinning;
- weakening fault, timeout, state, editor or cleanup laws;
- claiming causality from a single clean retry.

## Completion threshold

The implementation PR must contain:

1. one reproducible residual stall class under ordinary revision-7/512 operation;
2. one exact end-to-end missed-request timeline tied to its presentation deadline;
3. sufficient evidence to distinguish CPU execution, runnable delay, blocking/queueing and vendor processing for that class;
4. one causal repair or truthful bounded mitigation;
5. matched before/after runs under the same project, setting, runner, products, editor phase and CPUWeight;
6. a trace-off result supporting the same conclusion;
7. deterministic regressions using production helpers;
8. unchanged sample/event/automation ordering and no stale/future output publication;
9. preserved direct editors, state/save/recall and sibling behavior where touched;
10. positive cleanup with no unresolved DSP lease or publication transaction;
11. exact remaining gaps and nonclaims;
12. final installed readback preserving revision 7 and revision-3 rollback.

A successful result may narrow the supported reliability statement for the tested workload. It does not automatically qualify 256, establish a hard real-time guarantee or prove arbitrary customer hardware.

## Explicit non-goals

AP16 does not include:

- promotion of 256 or a broad latency matrix;
- FRAGMENTS Advanced expansion/redraw issue #80;
- Serum authorization/qualification issue #77;
- multi-instance capacity qualification;
- Wine/Proton migration;
- embedded editors, Wayland hosting, another DAW or CLAP;
- arbitrary sidechains, buses, float64 or broad MIDI/MPE expansion;
- consumer installation/release work;
- repository-wide cleanup or a transport rewrite;
- changes to repository ownership, licensing or contribution policy.

## Handoff

Update the single AP16 PR with the exact final head, parent and tree; selected stall class; trace/instrumentation contract and overhead; causal timeline; changed ownership; deterministic tests; local and hosted validation; matched Steam Deck before/after; final physical/install state; remaining limits; and confirmation that 512 remains supported/recommended unless independent review explicitly accepts a stronger profile.
