# Current Slice: PC0 — Windows VST3 Pre-Setup Processing Contract Census

## Status

```text
status: active_design_slice
authority_phase: reconnaissance_and_design
implementation_authorized: false
slice: PC0
target: main
selection_basis_commit: 858c240b104e090aaed8bd23ace04fd9a0dfd20e
selection_basis_tree: 8a560faf07b793de8faab91952ee6f34f15a1e73
selection_revision: pc0-selection-v2
selection_receipt: docs/slices/PC0/SLICE_SELECTION.md
superseded_selection_commit: 5ec7ef4f2c5f19faffcf1f2b01656d698ac7cd7a
design_gate: required
design_revision: pc0-design-v2
design_status: proposed_for_adversarial_review
design_card: docs/slices/PC0/IMPLEMENTATION_DESIGN.md
design_card_git_blob: ec0683fc66028239d7481640ba72e2dd9a060a2c
design_card_sha256: 20e653a6b1fad720ea5fc888a8d531bda44c338840f5eb96e488612d197de992
prior_design_commit: f6938e7dd501a4c86a382243670d82d060d1b75a
prior_design_tree: bacf308af5a7c993e1da45059497fb392a3b8845
prior_design_blob: 0f2d1c26aea3d009ce93d1d5086b52ca03fc8ce1
prior_design_sha256: 29db8b0f884407ba9ea75c80cab6ddd290908f2449d73e4502dd61347c490e86
prior_design_review: 5105496167 / PC0_DESIGN_V1_REPAIR_REQUIRED
reconnaissance_record: docs/slices/PC0/RECONNAISSANCE.md
design_review_record: docs/slices/PC0/ADVERSARIAL_DESIGN_REVIEW.md
design_branch: codex/pc0-windows-vst3-processing-contract-design
successor_selection_authorized: false
```

## Primary claim

> On the exact accepted AGain lifecycle, while the processor remains in the Initialized state, the supervised Windows host performs one bounded read-only census of all audio/event buses, all existing `BusInfo` records, current audio speaker arrangements, and `kSample32`/`kSample64` support, retains one exact normalized pre-setup processing contract, and completes the accepted interface/component/factory/module shutdown without mutating processing state.

## Exact revised selection authority

The original PC0 selection included `getLatencySamples` and `getTailSamples`. Bounded pinned-source reconnaissance established that both methods are legal only in the VST3 **Setup Done** state, while PC0 begins in **Initialized** and explicitly prohibits `setupProcessing`. The inherited AGain `setupProcessing` implementation mutates processing configuration. The design agent therefore correctly returned `PC0_MATERIAL_DESIGN_DISCOVERY` without changing the repository or contacting the fixture.

The operator explicitly revised PC0 to the pre-setup claim above. The exact basis, accepted fixtures, DX0 reuse requirements, protected state, design gate, and `implementation_authorized: false` posture remain unchanged. This authority replaces the previous PC0 selection claim; it does not authorize implementation.

The exact revised selection receipt is [`docs/slices/PC0/SLICE_SELECTION.md`](docs/slices/PC0/SLICE_SELECTION.md). Implementation remains blocked until one exact design revision has received independent adversarial review, technical-lead clearance, explicit operator approval, an approval receipt, and matching implementation authority here.

## Accepted boundaries consumed

PC0 begins from these accepted facts and must not redesign them:

- WA0 owns one initialized AGain `IComponent`, one exact `IAudioProcessor` lease, balanced interface/component/host references, and clean factory/module retirement.
- DX0 owns split Windows-build, accepted-fixture, Deck-execution, renderer, and complete-source identities for its closed proof transaction; successful retained observations survive Mac-only corrections.
- The accepted AGain module SHA-256 is `60aa9ff6b9918d4330449e7b3ab34b588dd93cba09f37413a3cd91f6e7d2e18f`.
- The accepted AGain bundle-manifest SHA-256 is `bfaa1dce4d2e189f89cee41493838824efe647e361e81436676b7b3a86ff5164`.
- The accepted Runtime 4 / Proton 11 identity is `2d64df1d36786ca2d0e955c553005423dc2b5bdd714bd0a17872622e33912547`.
- The protected Bitwig 6.1 installation remains unlaunched and is not part of PC0 execution.

DX0 is currently accepted for the closed `wa0-positive-regression-v1` plan. PC0 design must define one new reviewed closed plan and exact identity rosters while reusing the accepted transaction mechanisms. It must not copy the old manual workflow, custody, SSH, Deck-admission, or evidence choreography back into implementation instructions.

## Bounded reconnaissance-and-design authority

Design work is repository- and pinned-source-only. It may inspect the exact accepted implementation, evidence, design authority, pinned VST3 SDK contracts, and pinned AGain source needed to freeze:

- one pre-setup processing-contract census owner;
- exact operation order and call attribution;
- output zero-initialization and result-consistency laws;
- bus-count, bus-record, name, channel, enum, flag, and arrangement bounds;
- exact `kSample32`/`kSample64` interpretation;
- immutable normalized output and failure precedence;
- accepted shutdown and physical-containment behavior;
- one new closed DX0 proof plan and its build, Deck, and renderer identity rosters;
- the smallest exact implementation and evidence path envelopes;
- developer-cost acceptance proving no return to manual cross-plane operation.

The positive call surface to verify in design is:

```text
getBusCount:           4
getBusInfo:            once per reported bus; expected AGain count 3
getBusArrangement:     once per reported audio bus; expected AGain count 2
canProcessSampleSize:  kSample32 and kSample64
expected total:        11 calls
```

Source expectations are not implementation facts and must be checked before the design is frozen.

The design must preserve these cost constraints unless a material discovery stops the slice:

```text
ordinary Mac driver commands:      1
manually copied identifiers:       0
Windows acceptance producers:      1 maximum
accepted AGain rebuilds:            0
accepted fixture reseeds:           0
positive Deck batches:              1 maximum
negative live exercises:            0
Bitwig launches:                     0
Serum launches:                      0
source/configuration paths:          target 12–14; hard stop above 16
evidence paths:                      target 5; hard stop above 6
```

A renderer-only correction after a valid retained PC0 observation must require zero Windows builds, downloads, custody operations, transfers, and Deck executions.

## Absolute claim ceiling

PC0 may design only read-only observation in the Initialized state. It must not authorize or perform:

- `setIoMode`;
- `activateBus`;
- `setActive`;
- `setBusArrangements`;
- `setupProcessing`;
- `setProcessing`;
- `process`;
- `getLatencySamples`;
- `getTailSamples`;
- audio or event buffer allocation or transport;
- parameter, automation, state, preset, controller, or connection-point work;
- native proxy publication;
- C ABI, IPC, shared memory, broker, or real-time behavior;
- Bitwig or Serum execution;
- packaging, signing, release suitability, product-runner selection, or general compatibility.

Latency and tail are deliberately deferred to a separately selected Setup Done boundary that explicitly owns `setupProcessing` and its state mutation.

## Material-discovery stop law

Stop and return to the design gate if truthful PC0 design requires a processing-state mutation, an additional architectural owner, a live negative family, a changed accepted fixture, redesigned DX0 transaction architecture, broader claim, path envelope above the selected hard ceiling, or another independent uncertainty domain.

## Next action

Submit `pc0-design-v2` for fresh independent adversarial review against this revised selection and the retained V1 review. Do not implement PC0 and do not create an implementation prompt or approval receipt.
