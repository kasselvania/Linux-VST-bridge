# Current Work: AP1 — Native Linux / Windows Audio Round Trip

**Active implementation work order.** Issue #50; branch `codex/ap1-linux-windows-audio-roundtrip`; accepted basis `f285560c05f2eedf59ddce91c4f0e1d917cf775f`. The scoped AGENTS.md ruling authorizes the complete task before merge. AP0 and PC0 remain accepted; their evidence and spent allowances are unchanged.

## Observed completion pending review

Fresh AP1 acceptance verified 1344 Linux-read samples across eight changing blocks with maximum absolute error 0.0, one mapping/connection/instance, complete shutdown and unchanged protected state. See `docs/slices/AP1/RESULT.md` and the retained acceptance packet. One of six Windows producers, one of ten diagnostics and the first of two permitted acceptance candidates were consumed. AP0 remains accepted until this implementation is reviewed and merged.

## Product outcome

A native Linux caller supplies changing stereo sample buffers and gain requests to one supervised Windows AGain instance under the existing Steam Deck / Runtime 4 / Proton 11 fixture, receives the actual processed samples through shared memory, verifies them independently, and closes cleanly.

AP0 proved processing with buffers generated inside the Windows host. AP1 moves input ownership and result consumption to Linux. This is the first audio-bearing process crossing toward the native Linux DAW proxy, not another self-contained Windows recipe. It does not yet prove DAW integration or real-time suitability.

## Selected transport and ownership

**Audio:** one fixed-capacity, private, same-file shared mapping. Linux uses a shared read/write mapping; Windows opens that exact local backing file with CreateFileMapping / MapViewOfFile read/write, not copy-on-write. Put it in the existing owned disposable session tree, visible through its existing Wine drive mapping. No global shared-memory namespace or broad filesystem permission change. Do not replace live shared access with per-block file reads/writes, remapping, or samples serialized through logs/control messages.

**Control:** one ephemeral IPv4 loopback TCP connection carrying small versioned frames only. The native Linux client listens on `127.0.0.1` with an OS-selected port; the supervised Windows host connects to that exact port. Authenticate with a fresh private capability supplied through the owned launch/session channel, not a predictable reservation ID. Never listen on all interfaces or open an external network route. Preserve stdout/stderr as diagnostics, not the audio return channel. This control mechanism is selected for the offline proof; it is not the future real-time wakeup mechanism.

The existing supervisor owns launching, timeouts and cleanup of BOTH processes. Add a small native Rust caller/transport module, not a second broker framework; C++ remains the VST3/Windows edge. Linux owns mapping creation, size and retirement; Windows borrows the mapping for the session. Setup validates that both sides see the same backing bytes before AGain processing. The exact Runtime/container path and loopback reachability are to be tested, not assumed. Necessary narrow staging/launch changes belong to this task.

Use one instance, one shared slot and at most one outstanding block. Negotiate protocol/layout version and a fixed capacity before activation: float32 planar stereo, 48000 Hz, 1–256 frames per request, at most 64 blocks in this bounded session. Gain is finite normalized input in [0,1], applied at offset zero through the normal SDK queue; bypass is off. No events or general automation/controller protocol.

Control has explicit Hello/Ready, Process/Done, Close/Closed and Error behavior. Frames are length-bounded (at most 4 KiB), explicitly encoded fixed-width little-endian fields, and identify session, instance, request sequence, kind and payload length. Parent call ID is zero for this single non-reentrant test protocol. No pointers, native struct dumps, paths selected by the peer, STL objects or Rust/C++ atomic objects cross the interface. Document the actual wire/layout sizes with the code and use cross-language test vectors; private types/module organization remain engineering choices.

The slot handoff is **Linux writable -> Windows owns processing -> Linux readable/writable**, correlated to a strictly increasing request sequence. Linux writes inputs and poisons output, then publishes Process; it neither reads output nor overwrites/reuses the slot before matching Done. Windows validates lengths/offsets/capacity, snapshots the small request metadata, processes once, finishes all output writes, then publishes Done. Linux reads the mapping, not a host-reported PASS flag. Done identifies the exact request and output extent. Keep the mapping and instance alive across requests. No header field supplied by the peer can widen a validated region.

Implement explicit memory-ordering/visibility at handoff for this x86-64 Linux/Wine fixture; socket message order or volatile alone is not a documented memory model. Narrow unsafe mapping access must not leave Rust references live while the other process owns/writes those bytes. No shared language-atomic ABI or lock-free claim. Windows may use preallocated private buffers around the SDK call; AP1 does not require zero-copy processing. Control I/O and diagnostic serialization stay outside process calls.

## Failure and lifetime contract

Reject incompatible versions, wrong session/sequence, duplicate requests, malformed/truncated frames and out-of-capacity descriptors before the corresponding plug-in call. A response with the wrong sequence or extent is not usable even if its numbers look right. The first protocol or processing error terminates this session; do not silently substitute silence or last-good output as success.

A disconnect, deadline or lost response makes an outstanding request failed/unknown. Never resend it into the same plug-in state or reconnect transparently. Retain earlier completed observations, but do not call a partial batch successful. Reconcile existing evidence/cleanup; a genuinely new attempt needs a new session and consumes its actual task allowance. Document finite setup/request/close deadlines before device testing and retain the existing outer watchdog.

On graceful close, finish the outstanding block, stop/join the processing thread, deactivate/release the plug-in and retire endpoint mappings/handles; Closed and supervisor readback together prove completion. On uncertain stop or a hung endpoint, do not release/unload objects while processing may still use them. The supervisor contains the owned processes, and only after both endpoints are gone retires the backing file/stage. Unknown containment blocks new live work. This changes no protected fixture/runtime files.

## Discriminating proof

After the built Windows endpoint is ready and the mapping is established, the Linux caller chooses reproducible input data not compiled into that endpoint. Retain the recipe/seed and actual requested words; do not send the recipe/expected answers to Windows. Windows must consume the supplied buffers, not regenerate AP0's pattern.

In one persistent positive session, reuse the same slot over multiple blocks with distinct left/right signed binary-fraction patterns, gains 0.5, 0.25 and 0.75, lengths including 1, 16, 63 and 256, and silence. Change both data and gain across requests and include repeated lengths with different data. The independent checker derives expected output from the Linux-owned request, not a returned gain or production output. Compare every returned sample with zero numerical tolerance, reject nonfinite/unwritten/stale/channel-swapped results, and check unchanged input, boundary guards and unused output capacity. Use bounded exactly representable values; signed zero may compare numerically. Record maximum error, block/sample counts and per-request correlation. A Windows-only result or correct socket transcript without mapped output is insufficient.

Focused local tests exercise the actual Rust/C++ codec and state transitions: fragmented/coalesced control I/O, bad lengths/version/session/sequence, duplicate/stale responses, peer disconnect/timeout, and partial-write/reporting failure. Use test peers/fault injection for these negative paths; no new hostile plug-in campaign is required. Preserve useful error checkpoints and no-duplicate-launch behavior. Keep AP0 regressions and old evidence unchanged.

The successful real-fixture verification includes exact caller/host binaries, mapping visibility/reuse, real AGain output, thread/lifecycle ordering, graceful close, no surviving owned processes/mappings/stage and unchanged protected state. Report only the fixture behavior actually observed. This is not general memory-safety, crash-recovery, reentrant callback, zero-copy, real-time deadline or sandbox-security certification.

## Execute and deliver

Read the relevant architecture §§2, 5.8–5.9 and 6–7, AP0 contract/result, and the actual processing/runner source. Implement the smallest coherent extension. Necessary native Rust crate/lockfile, Linux build, Windows host rebuild, artifact delivery, loopback/mapping setup and narrow plan/adapter/renderer integration are authorized. Do not send the new mode to AP0's fixed-recipe artifact, or force this protocol through PC0's call-count expectations. Reuse the unchanged AGain binary, SDK pin, deployed runtime and accepted process supervision. Generalize only the seams this crossing needs; no new transaction framework, broad refactor or full transport platform.

Ordinary implementation, protocol serialization details, local tests and mechanical exact-source/artifact/plan/candidate binding belong to the same engineer. No intermediate design approval or process-only PR. If a live setup prerequisite fails, retain its actual error and repair within scope; do not turn it into another slice. Only a genuine transport/ownership change, protected-state change, unknown containment or exhausted allowance requires lead input.

AP1 has fresh task ceilings: **six Windows producer attempts, ten diagnostic batches in one AP1 campaign, two acceptance candidates maximum with one batch each**. Mapping/control-only live bring-up probes count toward diagnostics, not a separate free campaign. One batch may contain the bounded multi-block session; it is not one reservation per audio block. Local builds/tests are normal development; reuse unchanged artifacts and do not dispatch redundant CI builds. Candidate two is allowed only after a failed/inconclusive first candidate has a tested repair and established cleanup. Preserve all prior attempts/counts. Source/artifact/plan revisions inside this contract use the existing explicit atomic adjustment path; they do not reset the campaign. No new budgeting service.

Use existing classified commands with AP1-specific scoped receipts after the actual adapters pass local tests. Final verification is a fresh observation, never promoted diagnostic output. Stop live spending on repeated unchanged failures; fix ordinary harness problems locally and continue. Open one non-draft PR closing #50, with implementation, focused tests, concise protocol/usage notes and actual sample/cleanup evidence. Leave it unmerged for review; AP0 remains the accepted frontier until then.

Out of scope: Bitwig/DAW launch or proxy publication, live speakers/audio devices, commercial plug-ins, editors, state/presets, 64-bit processing, general callbacks/automation, runtime replacement, simultaneous instances and real-time optimization. Mac is orchestration only; the native caller and Windows host both execute on the Deck.

## Grounding and evidence ceiling

The architecture already specifies a Linux caller, Windows host, bounded control frames and separately shared audio. AP0's accepted implementation is the processing basis. Standard Win32 [file mapping](https://learn.microsoft.com/en-us/windows/win32/api/memoryapi/nf-memoryapi-mapviewoffile) uses offset-addressed same-file views; Valve's [Proton 11 Wine mapping source](https://github.com/ValveSoftware/wine/blob/proton_11.0/dlls/ntdll/unix/virtual.c) selects MAP_SHARED for writable file views. These are feasibility evidence, not proof of our exact installed binary/container combination. AP1 must establish that combination empirically. No device execution or transport success is claimed by this preparation.

## Authority — default command guard

This compatibility mapping blocks unspecified live commands, not the active work order above. Leave it closed until AP1 scoped receipts and their actual registered adapter authorize an exact run. Existing readiness here describes the accepted PC0/AP0 adapters, not an already-implemented AP1 adapter.

```yaml
status: no_active_slice
authority_phase: no_active_slice
change_class: PROOF_HARNESS_MAINTENANCE
maintenance_id: AP0
maintenance_status: complete
repository: kasselvania/Linux-VST-bridge
maintenance_implementation_authorized: false
product_implementation_authorized: false
live_execution_authorized: false
permitted_execution_class: none
classified_backend_core_ready: true
classified_backend_ready: true
production_adapter_registry: pc0_and_ap0_diagnostic_and_acceptance
accepted_product_frontier: AP0
current_product_target: AP1
pc0_status: accepted
ap0_status: accepted
diagnostic_campaign_authorized: false
acceptance_candidate_authorized: false
successor_selection_authorized: false
```
