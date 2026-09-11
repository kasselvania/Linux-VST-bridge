# AP17 — Real-project capacity, topology, and recovery envelope

## Authority and basis

AP17 is the active implementation slice.

- Integrated AP16 main: `bee44977a9db6b512f5a67076ceedabec5c656b4`
- Active issue: [#91 — AP17: Real-project capacity, topology, and recovery envelope](https://github.com/kasselvania/Linux-VST-bridge/issues/91)
- Branch: `codex/ap17-capacity-recovery-envelope`
- Product setting: 512 added frames per proxy, selected/supported/recommended
- 256-frame setting: available but unqualified and outside AP17

Continue in one implementation PR against `main`. Leave the PR open and unmerged for independent technical review. Ordinary implementation, focused tests, exact builds, reversible installation, protected-project work, one controlled failure campaign, and one normal reboot or user-session recovery check are authorized under [AGENTS.md](AGENTS.md).

AP16 is accepted and integrated. Its private tmpfs transport storage, ordinary revision-7 profiles, direct detached editors, stable processor/controller identities, exact revision-3 rollback targets, state/save/recall, automation, truthful gap/fault accounting, and positive ownership retirement are the known-good baseline.

The repository remains publicly readable but proprietary. Preserve [COPYRIGHT.md](COPYRIGHT.md), [CONTRIBUTING.md](CONTRIBUTING.md), and the all-rights-reserved README notice.

## Product outcome

Determine and enforce the exact real-project operating envelope of the bridge on the Steam Deck:

```text
multiple ordinary bridged instances are added incrementally
-> native and Windows process/module topology is observed
-> global, per-class, native-hard, and qualified limits are distinguished
-> a maximum useful exact-fixture project is selected from evidence
-> admission above that limit refuses before partial ownership
-> project save/reopen preserves every independent instance
-> one instance may be removed or fail without poisoning healthy siblings
-> service restart and one normal reboot recreate volatile sessions from durable state
```

The result may support fewer than eight simultaneous instances. A lower measured, enforced, and recoverable envelope is better than an unqualified larger claim.

AP17 is not a throughput contest. Do not raise limits or tune the machine merely to produce a larger number.

## Source-grounded ambiguity to close

Current source contains limits with different scopes:

1. `bridge-manager/src/main.rs` retains no more than eight live service request threads through `threads.len() >= 8`. That check occurs before the greeting is classified, so DSP, inspection, qualification, and vendor-access connections share a mechanical worker bound. A connection beyond it is currently dropped rather than receiving a stable product capacity result.

2. `native-vst3-proxy/backend/src/instances.rs` has a generation-checked `CAPACITY` of four. The Rust backend is statically linked into each generated native VST3 module. The effective hard-limit scope therefore depends on the actual module/process topology Bitwig chooses.

3. Every admitted DSP instance owns a separate supervised Windows process, tmpfs transport, controller, state owner, and editor. The environment keeper is separate and loads no plug-in.

4. The manager README currently calls eight simultaneous admissions supported while explicitly noting that eight has not been measured as a project-performance result.

AP17 must replace this ambiguity with explicitly named scopes. Do not force unlike limits to the same number.

## Capacity scopes

Establish and report at least:

- **service worker capacity** — mechanical concurrent connection/owner-thread bound;
- **global DSP admission capacity** — live bridged processing instances across the service;
- **per-class DSP admission capacity** — live instances of one exact registered class;
- **native image/process hard capacity** — the four-slot backend registry as actually loaded;
- **qualified exact-fixture capacity** — largest configuration that passes the real project checks;
- **parallel topology envelope** — instances on independent tracks;
- **serial topology envelope** — bridged devices in one chain and cumulative reported latency;
- **editor concurrency envelope** — direct vendor editors exercised simultaneously;
- **maintenance capacity** — keeper, inspection, qualification, and vendor-access accounting.

The native hard ceiling may be higher than the supported product limit. The supported product limit may be lower because of CPU, memory, scheduling, state, editor, or project behavior. Readback and refusal output must keep these meanings separate.

## Preserved fixture

Use the accepted exact matrix unless a demonstrated capacity defect requires a narrowly reviewed change:

- Steam Deck / SteamOS 3.8.16;
- Bitwig Studio 6.1 Flatpak, launched normally through Applications;
- current Bitwig plug-in hosting/sandbox mode, recorded and held constant;
- exact pinned Proton-SLR environment and runner;
- Pure LoFi 1.0.0.6121;
- Efx FRAGMENTS 1.0.0.2925;
- ordinary revision-7 `verified_exact_fixture` profiles and publications;
- AP15 direct detached vendor-editor lifecycle;
- AP16 private tmpfs transport;
- 48 kHz, float32;
- actual host maximum 512;
- 512 added frames per proxy;
- bridge CPUWeight unset/effective 100;
- original vendor files, authorization, projects, profile history, publication history, and revision-3 rollback.

Residual delivery classes remain tracked in [#90](https://github.com/kasselvania/Linux-VST-bridge/issues/90). Capacity-induced misses may be classified here, but AP17 must not become another unbounded delivery campaign.

## Topology discovery

Before changing any capacity constant or admission law:

- record Bitwig’s current plug-in hosting mode;
- bind exact PID plus start-time identities for Bitwig application/audio/plugin-host processes;
- bind each native proxy module/process;
- bind each supervisor and Windows host cohort;
- bind the environment keeper;
- record loaded module paths where safely available;
- determine whether same-class instances share one native image/process and one four-slot registry;
- determine whether LoFi and FRAGMENTS carry independent backend images/capacities;
- determine which current service threads are DSP, inspection, vendor-access, or maintenance owners.

Do not infer topology from process names alone.

## Incremental project matrix

Start with the accepted one-LoFi/one-FRAGMENTS baseline. Add only one controlled variable at a time.

Where the previous step remains safe and clean, exercise:

1. Pure LoFi same-class counts 1, 2, 3, 4, then one over-limit attempt.
2. FRAGMENTS same-class counts 1, 2, 3, 4, then one over-limit attempt.
3. Mixed LoFi/FRAGMENTS counts up to the candidate global service limit.
4. Parallel-track configurations, so process/CPU scaling is not confused with serial latency.
5. Serial chains of increasing bridged depth, with exact per-device and cumulative reported latency.
6. At least two simultaneous direct vendor editors at the candidate supported maximum; open more only when useful and safe.

Do not assume that four instances of each class imply a supported eight-instance project. Do not stop solely because the current service worker constant is eight if the observed scope proves different.

Give each instance a unique visible parameter/state signature. Same-class project recall must prove instance A did not receive instance B’s state.

## Safe escalation

Stop increasing the project before machine-wide instability when any of these becomes the first limiting condition:

- swap or memory pressure threatens Bitwig or the desktop;
- OOM activity or persistent pressure-stall evidence;
- thermal or power throttling invalidates the comparison;
- repeated active-musical misses attributable to the added load;
- a terminal bridge, vendor, or DAW fault;
- save/reopen becomes untrustworthy;
- cleanup, lease, or admission ownership becomes unresolved.

Retain the first limiting condition and the largest prior clean configuration. Do not use global CPU affinity, extreme CPUWeight, `nice -20`, `SCHED_FIFO`, machine-wide tuning, or process pooling merely to reach a larger number.

## Admission and refusal law

The final product must reserve and release capacity at the correct scopes.

Required laws:

- DSP capacity is counted separately from generic service worker threads.
- Keeper, inspection, qualification, and vendor access have explicit accounting and cannot accidentally consume or bypass DSP support.
- A reservation occurs before the native binding, tmpfs session, lease, Windows host, or editor becomes externally relevant.
- Startup failure before exposure returns the reservation.
- Exposed ownership is released only after positive native and Windows retirement.
- Unconfirmed retirement remains blocked and survives service restart.
- An over-limit request refuses deterministically before partial ownership.
- Existing siblings remain active.
- After one supported instance retires, exactly one unit of capacity becomes reusable.
- Repeated refusal/retry cannot leak permits or over-admit.
- The native side receives a bounded stable capacity failure rather than an unexplained disconnect or startup timeout.

If a handshake or native API change is required for a typed refusal, version it explicitly, add malformed/stale tests, and preserve historical artifacts and profiles.

## Resource and musical envelope

For each retained configuration, collect bounded, low-overhead facts:

- admission/startup duration;
- exact process and thread counts;
- per-instance and aggregate RSS/private memory where available;
- service, supervisor, native-host, and Windows-host CPU;
- tmpfs bytes and files;
- file descriptors;
- request/result high-water marks;
- admitted, delivered, missing, and expired frames;
- terminal faults;
- editor-open cost;
- explicit save duration and state result;
- retirement duration.

Every timing and CPU value must name its clock/counter domain. Do not turn one instantaneous sample into a support guarantee.

At the selected supported maximum, run at least two independent trace-off active-musical intervals under the same saved note/automation workload. The supported envelope must not introduce a reproducible sustained-playback failure attributable to capacity pressure. Existing startup/lifecycle gaps remain counted and separately classified.

Serial-chain evidence must state reported plug-in/bridge latency only. It is not physical or acoustic latency evidence.

## Maximum-project save and recall

At the candidate supported maximum:

- save a protected project copy;
- close and reopen it normally;
- verify exact instance count, class, order/routing, and unique visible state;
- verify opaque state where available;
- verify automation and direct editor access;
- prove same-class instances remain independent;
- quit normally and require positive retirement for every instance.

Do not overwrite the accepted AP15 or AP16 projects.

## Failure and recovery cases

Run these separately.

### Individual removal and replacement

Remove one exact instance from the maximum supported project, preserve all siblings, then admit one replacement. Capacity must return exactly once and the replacement must receive a fresh session/transport/editor identity.

### One Windows-host failure

Use exact PID/start identity or an existing bounded fault seam to fail one instance. The affected device may fail explicitly. Healthy siblings must preserve audio, state ownership, editor ownership, and cleanup. Do not silently substitute guessed DSP or guessed state.

### Service restart with devices closed

With the maximum project saved and Bitwig closed, restart the installed service, recreate the keeper, and reopen the project without changing profile/publication identity.

### Unexpected service loss

Use a bounded smaller project. Live instances must fail explicitly; no guessed recovery may occur. Ownership remains inspectable, and reopening the saved project creates fresh instances.

### Normal reboot or graphical-session restart

With Bitwig closed and the project saved, perform one normal Deck reboot or user-session restart. The volatile tmpfs sessions must disappear; durable software, profiles, publications, vendor state, and project state must remain. Service and keeper restart, and the saved project reopens at the selected supported limit.

## Deterministic verification

Use production helpers. Add tests for the selected model, including:

- global and per-class reservation/release;
- maintenance versus DSP accounting;
- actual same-image native registry exhaustion;
- stale-handle refusal after slot reuse;
- over-limit refusal before partial ownership;
- startup failure returning an unexposed reservation;
- exposed-but-unconfirmed ownership retaining its reservation and block;
- one-time release after positive retirement;
- service restart and persisted-lease reconciliation;
- sibling survival after one failure;
- capacity reuse after removal;
- malformed/stale capacity response refusal if a protocol changes;
- no capacity bookkeeping, lock, allocation, I/O, or wait in an audio callback;
- no editor or state crossover between same-class instances.

Preserve AP13 state-delivery, AP15 editor, AP16 transport-storage, publication/rollback, returned-result, and process-containment tests.

## Artifact and profile law

Characterization and manager-only capacity accounting do not automatically require a profile revision.

If only manager/supervisor bytes change, install a new immutable software revision and retain exact software rollback. Ordinary revision-7 profiles may remain unchanged.

If a native or Windows profile-bound artifact changes:

- preserve revision 7 byte-for-byte;
- create a new immutable `review_candidate` revision;
- bind an exact AP17 qualification marker and revision-7 parent;
- keep ordinary activation verified-only;
- restore revision 7 after qualification pending independent review.

Do not raise native `CAPACITY` merely to match the service’s current eight-thread mechanic. Change it only when the observed topology and selected support envelope require it.

## Explicit non-goals

AP17 does not include:

- opening, executing, or installing Arturia Software Center;
- downloading or installing Pigments;
- Serum or another vendor;
- a general vendor-acquisition UI;
- another residual delivery repair from #90 unless a newly demonstrated capacity defect directly blocks the envelope;
- 256-frame promotion;
- process pooling or multi-instance Windows hosting without evidence that process-per-instance is the limiting owner;
- FRAGMENTS rendering polish;
- another DAW, Wayland, CLAP, sidechains, arbitrary multibus, float64, broad MPE, or offline rendering;
- clean installer/public-release work;
- licensing, contribution, or repository-visibility changes.

The ASC installer may be downloaded and retained privately for AP18 preparation. It must remain unopened and unexecuted, outside the repository and outside AP17 evidence.

## Completion threshold

The implementation PR must retain:

1. observed Bitwig/native/Windows module and process topology;
2. explicit service-worker, native-hard, global, per-class, maintenance, and qualified capacities;
3. the same-class, mixed, parallel, and serial configurations actually exercised;
4. clean over-limit refusal and later capacity reuse;
5. resource/delivery measurements at the selected supported maximum;
6. two independent trace-off active-musical intervals at that maximum;
7. protected maximum-project save/reopen with unique instance state;
8. individual removal/replacement and one-instance failure isolation;
9. service restart plus one reboot/user-session recovery result;
10. exact installed software/profile/publication state and rollback;
11. positive cleanup, zero unresolved DSP leases, no pending publication transaction, tracing off, and normal CPUWeight;
12. exact limits and nonclaims;
13. confirmation that 512 remains selected/supported/recommended and 256 remains unqualified unless independent review selects otherwise.

## Handoff

Update one AP17 implementation PR with exact final head, parent and tree; topology; capacity model; changed owners; tests; project matrix; resource measurements; refusal behavior; failure/recovery evidence; exact installed state; residual limits; and the private unopened ASC-installer handoff status if the operator supplies it separately.

Leave the PR open and unmerged for independent technical review.

## Current implementation result — awaiting independent review

AP17 selected and exercised six DSP instances: at most three Pure LoFi and four Efx FRAGMENTS, with native hard capacity four per class image, sixteen service workers, one maintenance operation, three parallel tracks, serial depth three and two simultaneous editors. Eight produced repeated active-playback misses and is excluded. Seven is unqualified. These are exact-fixture engineering results, not a general capacity or hard real-time guarantee.

Review 5173438303 accepted the original refusal and sequential removal/replacement evidence. Its R1 is repaired: native insertion distinguishes full, temporary owner contention and terminal generation exhaustion without callback retry. Revision-9 ReviewCandidates contain the rebuilt native artifacts; revision 8 remains immutable history. The R1 regression failed before and passed after. See `evidence/ap17/r1/source-validation.json`.

The remaining campaign is now retained: fresh three-plus-three maximum-project save/reopen with distinct visible signatures and six separately associated state inputs; the two-plus-four serial corner with two independent 90-second closed-editor and one 90-second two-editor interval, all with zero additional gaps; exact one-host containment with five surviving owners; normal service restart; bounded unexpected service loss with explicit native faults and conservative ownership; and one normal reboot followed by six fresh sessions, saved signatures and local nonzero audio. Full opaque byte-equivalence is not claimed where LoFi state digests differ. Normal restart needed one automatic retry after a diagnostic/status lock overlap. Unattributed CPUWeight overrides are excluded from performance comparisons and restored.

Both native candidates have been restored physically to exact ordinary revision 7, with revision 3 retained underneath. The immutable R1 manager software remains installed. Final readback has service and keeper active, zero DSP leases, no pending transaction, tracing off, CPUWeight unset/effective 100, preserved project hashes, 512 selected/recommended and 256 unqualified. See `docs/AP17.md` and `evidence/ap17/r1/installed-final.json`. PR #92 remains open and unmerged for independent review; no candidate has been relabeled as verified.
