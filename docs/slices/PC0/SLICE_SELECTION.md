# PC0 Slice Selection Receipt

```yaml
schema: linux-vst-bridge-slice-selection/v1
repository: kasselvania/Linux-VST-bridge
basis_commit: 858c240b104e090aaed8bd23ace04fd9a0dfd20e
basis_tree: 8a560faf07b793de8faab91952ee6f34f15a1e73
selected_slice: PC0
selected_title: Windows VST3 Read-Only Processing Contract Census
primary_claim: >-
  On the exact accepted AGain lifecycle, the supervised Windows host performs
  one bounded read-only census of all audio/event buses, all existing BusInfo
  records, current audio speaker arrangements, kSample32/kSample64 support,
  initial latency and initial tail, retains one exact normalized processing
  contract, and completes the accepted interface/component/factory/module
  shutdown without mutating processing state.
exact_fixture:
  - accepted AGain VST3 module SHA-256 60aa9ff6b9918d4330449e7b3ab34b588dd93cba09f37413a3cd91f6e7d2e18f
  - accepted AGain bundle-manifest SHA-256 bfaa1dce4d2e189f89cee41493838824efe647e361e81436676b7b3a86ff5164
  - accepted Runtime 4 / Proton 11 identity 2d64df1d36786ca2d0e955c553005423dc2b5bdd714bd0a17872622e33912547
  - accepted WA0 initialized IComponent and IAudioProcessor lifecycle
  - accepted DX0 split-identity, fixture-reuse, retained-result, and one-command transaction mechanisms
  - protected Steam Deck Galileo / SteamOS 3.8.16 / x86_64 / read-only posture
  - protected Bitwig 6.1 installation remains unlaunched
authority_phase: reconnaissance_and_design
implementation_authorized: false
design_gate: required
design_gate_reason: >-
  PC0 introduces a new third-party VST3 call owner, bounded output and UTF-16
  normalization, operation-level timeout/crash attribution, one new closed DX0
  proof plan with exact invalidation rosters, and a retained processing-contract
  compatibility claim. These ownership, third-party-runtime, identity, failure,
  and claim boundaries require an approved implementation design.
design_card_path: docs/slices/PC0/IMPLEMENTATION_DESIGN.md
design_approval_path: null
allowed_changed_paths:
  - CURRENT_SLICE.md
  - docs/slices/PC0/SLICE_SELECTION.md
  - docs/slices/PC0/RECONNAISSANCE.md
  - docs/slices/PC0/IMPLEMENTATION_DESIGN.md
  - docs/slices/PC0/ADVERSARIAL_DESIGN_REVIEW.md
permitted_external_mutation:
  - create and push codex/pc0-windows-vst3-processing-contract-design
  - open one draft design pull request targeting main after a complete design card exists
protected_state:
  - current main commit 858c240b104e090aaed8bd23ace04fd9a0dfd20e and tree 8a560faf07b793de8faab91952ee6f34f15a1e73
  - all accepted SR0, HP0, HP1, WR0, WR0A, WF0, WC0, WA0, and DX0 authority and evidence
  - accepted AGain fixture stores and manifests
  - accepted Windows host-artifact stores and custody receipts
  - accepted DX0 retained transaction results and evidence-renderer boundary
  - WR0 environment and Steam compatdata
  - Bitwig, Flatpak overrides, and Bitwig 6.1 protected state
  - Serum artifacts, commercial content, credentials, and authorization state
  - SteamOS read-only posture
explicit_nonclaims:
  - no setIoMode
  - no activateBus
  - no setActive
  - no setBusArrangements
  - no setupProcessing
  - no setProcessing
  - no process
  - no audio or event buffer allocation or transport
  - no parameters, automation, state, presets, controller, or connection point
  - no native proxy, C ABI, IPC, shared memory, broker, or real-time behavior
  - no Bitwig or Serum execution
  - no packaging, signing, release suitability, product-runner selection, or general compatibility
material_discovery_requires_stop: true
successor_selection_authorized: false
operator_approval_text: >-
  I explicitly approve selecting PC0 — Windows VST3 Read-Only Processing
  Contract Census and replacing the no-active-slice card with its bounded
  reconnaissance-and-design authority. This approval does not authorize
  implementation. Implementation requires a separate approved design revision.
approved_at: 2026-09-03T04:42:35-07:00
```

## Selection rationale

PC0 is the smallest coherent product increment immediately beyond WA0. It observes the complete initial processing-facing contract required to choose a truthful setup or native-proxy boundary, but stops before any processing-state mutation.

A sample-format-only slice would be artificial fragmentation. A native proxy or processing-setup slice would be premature because the exact bus, arrangement, sample-format, latency, and tail contract is not yet retained.

## Development-cost constraint

DX0 is an accepted prerequisite, not a suggestion. PC0 design must make the eventual implementation use one reviewed closed proof plan through the existing Mac driver. It must not reintroduce manual workflow lookup, artifact custody, identifier copying, SSH choreography, Deck worktree creation, or evidence transport.

The future implementation target is one Mac command, at most one Windows acceptance producer when the new Windows host binary is not reusable, zero accepted-fixture rebuilds or reseeds, at most one positive Deck batch, and zero negative live exercises. Deterministic failure and normalization cases belong in deterministic production-owner tests.

No implementation prompt may be produced until an exact design revision is independently reviewed and explicitly approved.
