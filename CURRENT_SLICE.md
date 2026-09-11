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

## Retained qualification checkpoint — subsequently accepted by review 5174642190

AP17 selected and exercised six DSP instances: at most three Pure LoFi and four Efx FRAGMENTS, with native hard capacity four per class image, sixteen service workers, one maintenance operation, three parallel tracks, serial depth three and two simultaneous editors. Eight produced repeated active-playback misses and is excluded. Seven is unqualified. These are exact-fixture engineering results, not a general capacity or hard real-time guarantee.

Review 5173438303 accepted the original refusal and sequential removal/replacement evidence. Its R1 is repaired: native insertion distinguishes full, temporary owner contention and terminal generation exhaustion without callback retry. Revision-9 ReviewCandidates contain the rebuilt native artifacts; revision 8 remains immutable history. The R1 regression failed before and passed after. See `evidence/ap17/r1/source-validation.json`.

The remaining campaign is now retained: fresh three-plus-three maximum-project save/reopen with distinct visible signatures and six separately associated state inputs; the two-plus-four serial corner with two independent 90-second closed-editor and one 90-second two-editor interval, all with zero additional gaps; exact one-host containment with five surviving owners; normal service restart; bounded unexpected service loss with explicit native faults and conservative ownership; and one normal reboot followed by six fresh sessions, saved signatures and local nonzero audio. Full opaque byte-equivalence is not claimed where LoFi state digests differ. Normal restart needed one automatic retry after a diagnostic/status lock overlap. Unattributed CPUWeight overrides are excluded from performance comparisons and restored.

Both native candidates have been restored physically to exact ordinary revision 7, with revision 3 retained underneath. The immutable R1 manager software remains installed. Final readback has service and keeper active, zero DSP leases, no pending transaction, tracing off, CPUWeight unset/effective 100, preserved project hashes, 512 selected/recommended and 256 unqualified. See `docs/AP17.md` and `evidence/ap17/r1/installed-final.json`. PR #92 remains open and unmerged for independent review; no candidate has been relabeled as verified.

## AP17 ordinary acceptance transition — review 5174642190

Independent review 5174642190 selected head `20f2c3a7382aa7dd0abb973c7ab09d708919ea29` and tree `7e4fa0866b51a7ec415cb4a4039444835178ced1`. Revision 9 remains the immutable successful `review_candidate`; revision 8 retains the pre-R1 candidate. New root revision 10 is `verified_exact_fixture`, with exactly revision 9's technical constraints and only `capacity_under_qualification` removed. Revision 7 is retained under `compatibility/ap15/revision-7` and remains the exact immediate rollback parent; revision 3 remains beneath it.

The no-argument `linux-vst-bridge accept-capacity` command is the sealed software transition for this exact installed fixture. It checks the compiled review, evidence digests, prior installed software record, exact candidate publications and completed transactions, active verified parents, artifacts and current local bindings before creating software. It reuses immutable staged native/host bytes through the existing setup/catalogue owner. It accepts no profiles, hashes, paths, review IDs or capacity values. After setup, `linux-vst-bridge managed publish` performs ordinary publication. ReviewCandidate remains ineligible for ordinary activation; qualification history is not ordinary authority.

Canonical capacity readback derives verified authority from physically active ordinary revision-10 publications, not merely the manager version. The exact envelope remains six global DSPs, LoFi three, FRAGMENTS four, native-image hard ceiling four, sixteen service workers, one maintenance operation, three parallel tracks, serial depth three and two simultaneous editors. Seven is unqualified; eight is excluded for the tested workload. This is the exact SteamOS 3.8.16 / Bitwig 6.1 / pinned Arturia fixture, not a universal Arturia, DAW or hardware limit. A new class requires an explicit versioned policy extension.

The sealed transition and ordinary publication ran on the Deck. Both revision-10 publications had no qualification marker, exact revision-7 parents, accepted AP17 native hashes, unchanged external IDs and verified exact-fixture capacity readback. The protected two-LoFi/four-FRAGMENTS project admitted six owners. A seventh LoFi request received `admission_global_capacity` without a new lease, transport or Windows cohort. One FRAGMENTS retirement returned exactly one unit; Undo Delete admitted one fresh session while five sibling identities stayed unchanged. Two real direct editors opened on the intended owners. A 30-second active interval had nonzero local output, but retained 10 additional LoFi gap groups / 2,560 missed and expired frames. This is not a performance-improvement claim.

**The final ordinary smoke did not pass.** Normal Bitwig Quit stopped the project and all six bridge owners retired positively, but the main Bitwig application remained open. Further normal close requests did not finish. Its log recorded a frontend connection EOF and native plug-in-host broken pipe; several Windows owners reported control disconnection during containment. Positive cleanup is not evidence of a clean SDK/application exit. The exact remaining quit owner is unclassified. The unchanged native and Windows bytes are the accepted revision-9 artifacts; no new causal attribution to those components or manager policy is asserted.

The protected project remained byte-identical. After retaining the failure, a PID/start-time-bound SIGTERM closed only the stuck test application. Both discovery links were rolled back through normal managed rollback to their exact original revision-7 targets, with revision 3 retained. Revision 10 remains inactive immutable publication history. The new immutable manager/software remains installed and validates the retained revision-7 targets. Final state: service and keeper active, zero DSP leases, no pending transaction or stale transport session, no Bitwig/debugger process, tracing off, bridge CPUWeight unset/effective 100. Capacity readback conservatively returns `engineering_candidate` after restoration because revision 10 is not physically active.

See `evidence/ap17/acceptance/transition.json`, `ordinary-smoke.json`, `installed-final.json` and `validation.json`. The first Bitwig scan also required refreshing the existing VST3 search location; monitoring was restored off, and the same locations remain (their display order changed). No vendor file, authorization or project was replaced. 512 remains selected/supported/recommended; 256 remains unqualified. Short delivery gaps (#72), FRAGMENTS rendering (#80), and the newly retained quit failure remain explicit. ASC/Pigments were not executed. PR #92 stays open and unmerged; `AP17_ACCEPTED_AND_ORDINARY_CAPACITY_PROFILE_READY` is **not** claimed.

## Focused shutdown investigation — review 5175233255

The investigation on source head `de29758d67f7b46c722b18c1a28ba8100e96be67` did not reproduce the earlier surviving Bitwig frontend/audio-engine hang. No production source, profile, native or Windows artifact changed. This is **not a causal repair** and does not assign the prior hang to Bitwig.

Normal Applications-launched comparisons completed with frontend, engine and plug-in-host processes absent in two bounded post-quit snapshots: revision-7 saved 2+4; revision-10 saved 1+1 and 2+4; native-image refusal; global refusal; one FRAGMENTS removal/Undo; and two intended editors. The full prior sequence also exited twice: global seventh refusal, undo failed copy, FRAGMENTS removal/Undo, two same-instance editors, active saved scene, File → Quit and No to temporary changes. Every exposed owner retired positively, including each replaced instance. No manual process containment was used in these comparisons.

**The fixed-baseline final gate remains incomplete.** Both full-sequence exits encountered an unexpected `CPUWeight=10000` runtime override during shutdown. The measured 30-second playback intervals had effective weight 100, but a bounded external observer on the second quit caught the transition to 10000 before the frontend exited and before five of six final cleanup receipts. The writer is now identified: `foreground_booster` PID 3409 (`plasma-foreground-booster.service`) logs switching from Bitwig to `steam_app_0` and explicitly setting `linux-vst-bridge.service` to 100 times normal weight at both quit boundaries. The generated runtime drop-in and bounded weight samples corroborate those journal records. This explains the scheduling override, not the original quit hang; the system-wide booster was not disabled or reconfigured. These runs establish complete exits but are excluded from the required constant-scheduling confirmation. No further full-sequence retry or performance claim follows from them.

Status 90 does not identify an interface-query hang in the observed commercial path. `inspect_module.cpp` assigns 90 in its exception catch; retained records show `control disconnected/IO` after processing-stop completion and worker join, followed by editor/handler/component teardown. Several complete application exits also lack native final summaries and include plug-in-host broken-pipe exit 1/encoded256. These are abnormal teardown results, not a claim of clean SDK shutdown. Existing sequence-only Windows records do not establish an exact common-clock ordering for every native, SDK and frontend release. The prior hang lacks the surviving-thread/FD capture required for conclusive ownership attribution.

The fixed conditions otherwise remained SteamOS 3.8.16, Bitwig 6.1 Flatpak/Together, exact pinned Arturia artifacts, 48 kHz/float32, actual host maximum 512/internal chunks 256 and 512 added frames per proxy. Search-location refresh and the subsequent normal relaunch needed for initial revision-7 LoFi discovery are retained separately from comparison A. No vendor files, authorization or saved project were changed. Local nonzero output and all additional gap counters are retained; the earlier ordinary-smoke 10 LoFi gap groups / 2,560 missing and expired frames remain under delivery issue #90, without a demonstrated shutdown link.

Both test revision-10 publications were rolled back through normal managed rollback to exact revision 7: LoFi `90e23e3e71322c27e3f9e4ce1021b9d8`, FRAGMENTS `d7c50af638553b3387f89a1064546aca`. Revision 3 remains beneath them; revisions 8, 9 and 10 remain immutable inactive history. The acceptance manager software is unchanged. Final physical readback confirms service/keeper active, no DSP lease, pending transaction, tmpfs session, Bitwig or debugger, tracing off, CPUWeight unset/effective 100 and protected project hashes unchanged. 512 remains selected/supported/recommended; 256 remains unqualified. ASC/Pigments were not executed.

See [comparisons](evidence/ap17/shutdown/comparisons.json), [retirement](evidence/ap17/shutdown/retirement.json), [host exits](evidence/ap17/shutdown/host-exits.json), [scheduling confound](evidence/ap17/shutdown/scheduling-confound.json), [final readback](evidence/ap17/shutdown/installed-final.json) and [validation](evidence/ap17/shutdown/validation.json). PR #92 remains draft/open/unmerged. Disposition: `SHUTDOWN_HANG_NOT_REPRODUCED`, `FIXED_BASELINE_COMBINED_CONFIRMATION_BLOCKED`, `REVISION_7_RESTORED`. Acceptance readiness is not claimed.
