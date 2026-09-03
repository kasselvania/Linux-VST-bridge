# Current Slice: PC0 — Windows VST3 Pre-Setup Processing Contract Census

## Status

```text
status: active_implementation_slice
authority_phase: implementation
implementation_authorized: true
slice: PC0
target: main
selection_basis_commit: 858c240b104e090aaed8bd23ace04fd9a0dfd20e
selection_basis_tree: 8a560faf07b793de8faab91952ee6f34f15a1e73
selection_revision: pc0-selection-v2
selection_receipt: docs/slices/PC0/SLICE_SELECTION.md
superseded_selection_commit: 5ec7ef4f2c5f19faffcf1f2b01656d698ac7cd7a
design_gate: required
design_revision: pc0-design-v2
design_status: approved_for_implementation
design_commit: 996ee557d33ea55d6acf7ef2242f703c2c63f262
design_tree: bb1fe157bef42854812ebeb0b73b1a5332c93cf6
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
design_review_blob: 7444c490e620c16903e5a59b052376d8dcaef3e9
technical_lead_review_id: 5105712590
technical_lead_review_result: PC0_DESIGN_V2_CLEAR
design_approval: docs/slices/PC0/DESIGN_APPROVAL.md
design_approval_blob: 7a7bccc08cca218249e8c9d43ed582027143343f
design_branch: codex/pc0-windows-vst3-processing-contract-design
implementation_branch: codex/pc0-windows-vst3-pre-setup-processing-contract
implementation_basis: exact approved design-authority merge commit and tree, established by merged-PR and main readback before implementation
successor_selection_authorized: false
```

## Primary claim

> On the exact accepted AGain lifecycle, while the processor remains in the Initialized state, the supervised Windows host performs one bounded read-only census of all audio/event buses, all existing `BusInfo` records, current audio speaker arrangements, and `kSample32`/`kSample64` support, retains one exact normalized pre-setup processing contract, and completes the accepted interface/component/factory/module shutdown without mutating processing state.

## Exact implementation authority

The immutable [`pc0-design-v2`](docs/slices/PC0/IMPLEMENTATION_DESIGN.md), technical-lead review `5105712590 / PC0_DESIGN_V2_CLEAR`, retained [adversarial review history](docs/slices/PC0/ADVERSARIAL_DESIGN_REVIEW.md), and [operator approval receipt](docs/slices/PC0/DESIGN_APPROVAL.md) authorize this bounded implementation. The design card remains byte-identical to reviewed blob `ec0683fc66028239d7481640ba72e2dd9a060a2c`, SHA-256 `20e653a6b1fad720ea5fc888a8d531bda44c338840f5eb96e488612d197de992`.

The approval binds both implementation clarifications from the technical-lead review:

- `COST_AND_INVALIDATION.json` uses schema `linux-vst-bridge-pc0-cost-and-invalidation/v1`; incompatible PC0 keys are not published under the DX0 v1 label.
- Failed durable `call_started` publication means no VST3 call occurred. With no earlier PC0 blocker it is `PC0_EVIDENCE_BLOCKED` and only accepted physical containment may be claimed; an earlier blocker remains primary.

## Accepted boundaries consumed

PC0 begins from these accepted facts and must not redesign them:

- WA0 owns one initialized AGain `IComponent`, one exact `IAudioProcessor` lease, balanced interface/component/host references, and clean factory/module retirement.
- DX0 owns split Windows-build, accepted-fixture, Deck-execution, renderer, and complete-source identities; accepted-fixture storage; one-command build/custody/handoff/Deck/evidence orchestration; retained-result admission; recovery; and renderer-only zero-external-work reuse.
- Accepted DX0 source is `be046dd2d44a7915ca408c01a06212639ccea51e`, tree `1322e4eb7b5bd54244bd2fa7fc83327ea6a0c183`; accepted evidence is `85840920844693f7611306492298817e16dd2a6c`, tree `35ddde4e56a810ab1ce44970313bf2f42e585908`; implementation merge is `1f71487717eabdb3cd5285a4559df2ce2915c8d8`.
- The accepted AGain module SHA-256 is `60aa9ff6b9918d4330449e7b3ab34b588dd93cba09f37413a3cd91f6e7d2e18f`.
- The accepted AGain bundle-manifest SHA-256 is `bfaa1dce4d2e189f89cee41493838824efe647e361e81436676b7b3a86ff5164`.
- The accepted Runtime 4 / Proton 11 identity is `2d64df1d36786ca2d0e955c553005423dc2b5bdd714bd0a17872622e33912547`.
- The protected Bitwig 6.1 installation remains unlaunched and is not part of PC0 execution.

Every authority, source, evidence, merge, tree, and blob identity named by the implementation must resolve as the exact Git object and relationship required by the approved design. Lexical hash shape is not identity proof.

## Exact implementation envelope

```text
new architectural owners:          1
stable owner states:                9
unique PC0 VST3 operation types:    4
expected positive PC0 calls:       11
focused proof rows:                16
blocked outcomes:                  10
source/configuration changed paths: 14
evidence changed paths:             5
ordinary Mac driver commands:       1
manually copied identifiers:        0
Windows acceptance producers:       1 maximum
accepted AGain rebuilds:             0
accepted fixture reseeds:            0
positive Deck batches:               1 maximum
live negative exercises:             0
Bitwig launches:                      0
Serum launches:                       0
```

The ordinary closed plan is `pc0-pre-setup-processing-contract-v1`. The implementation must use the accepted DX0 transaction machinery rather than restoring manual GitHub workflow lookup, artifact custody, identifier copying, source bundles, SSH transfers, Deck refs/worktrees, Runtime/Proton invocation, result retrieval, or evidence placement.

The selected positive call surface is exactly four `getBusCount`, one `getBusInfo` per reported bus, one `getBusArrangement` per reported audio bus, and `canProcessSampleSize` for `kSample32` and `kSample64`. Expected AGain total is eleven calls. Source expectations are not acceptance facts until the authorized live positive transaction completes.

## Evidence and provenance law

The private `linux-vst-bridge-pc0-transaction-result/v1` object owns producer P, original Deck execution E, and the original live observation. It does not contain or require a later consumer C.

The tracked `linux-vst-bridge-pc0-evidence-packet/v1` object in `TRANSACTION.json` separately binds P, E, and the current consumer C; strict private-result admission; observation disposition; whether C executed or freshly inspected Deck state; the nested immutable pre-setup processing contract; exact call facts; quiescence; shutdown; cleanup; protected-state comparison; all sixteen proof dispositions; renderer identity; and acyclic integrity.

The five tracked evidence paths are exactly:

```text
evidence/pc0-windows-vst3-pre-setup-processing-contract/BASIS.md
evidence/pc0-windows-vst3-pre-setup-processing-contract/COST_AND_INVALIDATION.json
evidence/pc0-windows-vst3-pre-setup-processing-contract/FINDINGS.md
evidence/pc0-windows-vst3-pre-setup-processing-contract/TRANSACTION.json
evidence/pc0-windows-vst3-pre-setup-processing-contract/hashes.sha256
```

A renderer-only correction over an admitted result performs zero Windows builds, downloads, custody operations, transfers, or Deck executions and truthfully reports reuse of E's historical observation.

## Absolute claim ceiling

PC0 may implement only read-only observation in the Initialized state. It must not call or prepare:

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
- packaging, signing, release suitability, product-runner selection, another plug-in/DAW, or general compatibility.

Latency and tail remain deliberately deferred to a separately selected Setup Done boundary that explicitly owns `setupProcessing` and its state mutation.

## Material-discovery stop law

Stop and return to the design gate if truthful implementation requires a processing-state mutation, another architectural owner, another live negative family, a changed accepted fixture, redesigned DX0 transaction architecture, broader claim, more than the exact fourteen source or five evidence paths, a second Windows producer, a second positive Deck batch, or another independent uncertainty domain.

## Next action

Merge the exact authority-finalization commit, read back the resulting `main` merge commit/tree, create the implementation branch from that exact merge, and implement only the approved `pc0-design-v2` envelope. No successor is authorized.
