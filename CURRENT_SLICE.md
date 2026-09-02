# Current Slice: WA0 — Windows VST3 Audio-Processor Interface Admission

## Status

```text
status: active_design_slice
authority_phase: reconnaissance_and_design
implementation_authorized: false
slice: WA0
target: main
selection_basis_commit: 68f52e5b9678d87b13547d3bb37078a0e6a1255c
selection_basis_tree: 776a01340eca9ec456b9c063cf9201d31bd9f3e3
selection_receipt: docs/slices/WA0/SLICE_SELECTION.md
design_gate: required
design_revision: wa0-design-v1
design_status: proposed_for_adversarial_review
design_card: docs/slices/WA0/IMPLEMENTATION_DESIGN.md
design_blob: d927c6dae430ffe7a811193e3d9119e573cfe316
design_sha256: eb5bd8f3aa439944f5933ecc9f0c4bcb90d46b3657499b365d11a82c0e2858ac
design_approval: docs/slices/WA0/DESIGN_APPROVAL.md
design_branch: codex/wa0-windows-vst3-audio-processor-interface-design
implementation_branch: not_authorized
successor_selection_authorized: false
```

## Primary claim

> On the exact accepted Steam Deck fixture, the existing initialized AGain processor component exposes the mandatory `Steinberg::Vst::IAudioProcessor` interface through the exact interface query, the repository-owned Windows host acquires and retires exactly one such interface reference without calling any audio-processor method, and the accepted WC0 component, factory, module, process, environment, and protected-state shutdown remains exact.

## Exact new boundary

```text
accepted WC0 initialized AGain IComponent
    -> query exact IAudioProcessor IID
    -> verify result and output-pointer consistency
    -> own exactly one acquired IAudioProcessor reference
    -> release that reference exactly once to the component-owner baseline
    -> continue accepted WC0 terminate, component release, factory release,
       module exit, unload, process drainage, and environment retirement
```

WA0 owns only the processor-interface lease. It does not call an `IAudioProcessor` method and does not create the edit controller.

## Why this is the next product edge

WC0 proved that the exact AGain processor class can be created as one initialized `IComponent` under the accepted Runtime 4 / Proton 11 path. The immediate unresolved processor-side question is whether that same live object exposes the mandatory audio-processing interface that a later host must use.

This slice stops at interface admission and balanced ownership. It does not combine interface discovery with bus negotiation, processing setup, audio buffers, controller creation, connection points, IPC, or Bitwig integration.

## Design authority

This approval authorizes read-only reconnaissance and one implementation-design revision only.

The design must define, at minimum:

- the exact owner of the acquired `IAudioProcessor` reference;
- the exact query point within the accepted WC0 lifecycle;
- the result/output-pointer consistency law;
- the exact logical IID and raw Windows TUID representation;
- the reference-count and release law;
- the extension to WC0 object quiescence requiring the audio-processor lease to be absent before component termination and retirement continue;
- deterministic attribution for an unmatched query or release call;
- the minimum focused fault family needed to distinguish query failure, inconsistent result/output, and incomplete interface retirement;
- retained evidence and explicit claim ceiling;
- the exact source/configuration and evidence path envelopes.

The design must not use pointer-address equality as interface identity. Multiple-interface C++ objects may return adjusted interface pointers. Identity must be established through the exact IID, ordinary result, non-null output contract, balanced reference ownership, and final object retirement.

## Required lifecycle constraint

The intended positive ordering is bounded to:

```text
WC0 component initialized
    -> query IAudioProcessor
    -> retain one interface lease
    -> release IAudioProcessor back to component-owner baseline
    -> WC0 terminate
    -> WC0 component release to zero
    -> inherited factory/module shutdown
```

A materially different ownership or ordering requirement discovered during reconnaissance must be returned for review rather than silently expanded.

## Accepted infrastructure reused without redesign

WA0 consumes the accepted WC0 and WF0 mechanisms as prerequisites:

- supported Windows Server 2022 / Visual Studio 2022 / MSVC build plane;
- exact source and Actions-artifact custody on the Mac;
- ordinary SSH handoff to the rootless Steam Deck;
- clean detached Deck execution worktree;
- content-addressed artifact admission;
- Runtime 4 / Proton 11 launch and process supervision;
- disposable environment ownership and retirement;
- exact AGain module and factory census;
- WC0 processor creation, host context, initialization, termination, release, object-quiescence gate, and inherited shutdown.

These mechanisms receive exact identity preflight and only the bounded regression needed by WA0. They are not successor-slice design subjects.

## Read-only reconnaissance permitted

The design agent may inspect:

- repository authority, accepted WC0 source, and WC0 retained evidence;
- the exact pinned official VST3 SDK and recursive submodule sources already identified by the repository;
- official interface definitions and AGain inheritance/query-interface implementation relevant to `IAudioProcessor`;
- existing source and evidence schemas necessary to specify a bounded extension.

No build, workflow run, artifact download, source handoff, Steam Deck command, Runtime/Proton launch, Bitwig launch, Serum inspection, environment creation, or external fixture mutation is authorized during design.

## Exact fixture

```text
selection basis commit:
68f52e5b9678d87b13547d3bb37078a0e6a1255c

selection basis tree:
776a01340eca9ec456b9c063cf9201d31bd9f3e3

accepted WC0 implementation merge:
cb831c38e1be88f4bb6a0ab6f2fca2d94164891b

accepted WC0 source commit:
9c0096930df86fc5b171cdebec40b306a198316a

accepted WC0 evidence head:
77bb40dbf35e19fd93b9f79b5286a43d56b9cc21

accepted WC0 source-manifest SHA-256:
38699a1d2026cb1078a569dc1997122b0111c78e608f294777dcf4afc49c8b25

accepted WC0 scanner SHA-256:
51b899b7936921b24265ead9ff12180f14249d49b532ead9a18414559f5f83f7

accepted WC0 artifact-manifest/cache SHA-256:
25bd47471f01ef57b06b3c8232bb6cc7e40c767f281c30186fa5426130f8ae82

Runtime 4 / Proton 11 digest:
2d64df1d36786ca2d0e955c553005423dc2b5bdd714bd0a17872622e33912547

AGain module SHA-256:
60aa9ff6b9918d4330449e7b3ab34b588dd93cba09f37413a3cd91f6e7d2e18f

processor class ID:
84E8DE5F92554F5396FAE4133C935A18

processor raw Windows TUID:
5FDEE8845592534F96FAE4133C935A18

accepted component interface:
Steinberg::Vst::IComponent
logical IID: E831FF31F2D54301928EBBEE25697802
raw Windows TUID: 31FF31E8D5F20143928EBBEE25697802

candidate audio-processor interface:
Steinberg::Vst::IAudioProcessor
logical IID: 42043F99B7DA453CA569E79D9AAEC33D
raw Windows TUID: 993F0442DAB73C45A569E79D9AAEC33D

Bitwig:
6.1, protected and unlaunched
```

## Anti-distraction law

WA0 may not redesign or re-prove:

- GitHub authentication or Steam Deck credential persistence;
- Windows build or artifact-custody architecture;
- Mac-to-Deck transfer;
- SteamOS persistence;
- Runtime or Proton selection;
- process-supervisor architecture;
- the factory census;
- the WC0 component and host-context lifecycle;
- Bitwig installation or behavior.

WA0 may not absorb:

- any `IAudioProcessor` method call, including `setBusArrangements`, `getBusArrangement`, `canProcessSampleSize`, `getLatencySamples`, `setupProcessing`, `setProcessing`, `process`, or `getTailSamples`;
- edit-controller creation or initialization;
- `IConnectionPoint` pairing;
- component bus enumeration or activation;
- parameters or state;
- audio, events, timing, automation, or presets;
- GUI/editor behavior;
- native Linux proxy, C ABI, IPC, or shared memory;
- Bitwig execution;
- Serum execution or authorization;
- packaging, signing, release, or product-runner selection.

A need for any of those owners is a material discovery and returns WA0 to the design gate.

## Allowed design paths

The cumulative reconnaissance-and-design branch is limited to:

```text
CURRENT_SLICE.md
docs/slices/WA0/SLICE_SELECTION.md
docs/slices/WA0/IMPLEMENTATION_DESIGN.md
```

A later independent adversarial review may add its separately authorized review record. No product source, workflow, fixture, tool, evidence, or governance path is authorized by this selection.

## Next lawful action

From the exact activation head, perform read-only reconnaissance and write one bounded `wa0-design-v1` implementation design at:

```text
docs/slices/WA0/IMPLEMENTATION_DESIGN.md
```

Update `CURRENT_SLICE.md` only to bind the resulting design commit, tree, blob, SHA-256, revision, and review-required status. Open one draft design PR targeting `main`.

Implementation remains unauthorized. A fresh independent adversarial design review and a separate exact operator approval are required before any implementation branch, build, workflow, Deck operation, or runtime workload.
