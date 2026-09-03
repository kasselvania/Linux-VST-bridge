# Current Slice: DX0 — Split Build Identity and Single-Command Proof Transaction

## Status

```text
status: active_implementation_slice
authority_phase: implementation
implementation_authorized: true
slice: DX0
target: main
selection_basis_commit: 859422be75f65da4dd7dc51394b84470ad594358
selection_basis_tree: 7b9a721f3e691974dd1720c20d9ef030932978a9
selection_receipt: docs/slices/DX0/SLICE_SELECTION.md
design_gate: required
design_revision: dx0-design-v2
design_status: approved_for_implementation
design_card: docs/slices/DX0/IMPLEMENTATION_DESIGN.md
design_commit: f27695862f9287b225af739264169c2ca3f407ad
design_tree: e08455a23a0891d09bc89fbf0ee50164bbecb7a4
design_blob: 5dd758d681f9712de02ab4c45d58d3d815aef53a
design_sha256: 85de95acfe171678c0efedbbdc6aebdc0ff134a716dfd8d29f52feed9c53df6c
review_history: docs/slices/DX0/ADVERSARIAL_DESIGN_REVIEW.md
technical_lead_review_id: 5096747625
technical_lead_review_result: DESIGN_CLEAR
design_approval: docs/slices/DX0/DESIGN_APPROVAL.md
design_branch: codex/dx0-split-build-identity-proof-transaction-design
implementation_branch: codex/dx0-split-build-identity-proof-transaction
implementation_basis: exact approved design-authority merge commit and tree, established by merged-PR and main readback before implementation
successor_selection_authorized: false
```

## Primary claim

> From one exact Mac-side command and one closed proof plan, the existing proof system derives a Windows-build-input identity separate from the complete implementation-source identity, reuses an accepted content-addressed AGain fixture without rebuilding it, produces or reuses the exact compatible Windows host artifact, performs source custody, Mac-to-Deck handoff, detached-worktree admission, one supervised Deck proof batch, cleanup, and evidence return, and proves that a non-build-only source change causes zero Windows builds.

## Exact implementation authority

The immutable [dx0-design-v2 card](docs/slices/DX0/IMPLEMENTATION_DESIGN.md), technical-lead review `5096747625 / DESIGN_CLEAR`, and [operator approval receipt](docs/slices/DX0/DESIGN_APPROVAL.md) authorize this bounded implementation. The design card remains byte-identical to the reviewed blob; its original proposed/unauthorized metadata is historical, not a replacement for this current authority and approval receipt.

D-026 governs the execution-cost objective. No additional VST3 capability is selected. DX0 must reduce invalidation and manual cross-plane work, not merely wrap repeated work in a command.

The exact approved implementation consists of the ten source/configuration paths and five evidence paths listed in the card. The 17-record Windows build-input roster is a dependency identity, not an additional changed-path allowance.

```text
architectural owners: 3
identity domains: 6
transaction states: 10
material operations: 12
focused proof rows: 14
blocked results: 10
source/configuration changed paths: 10
evidence changed paths: 5
new Windows C++ product paths: 0
new VST3 calls: 0
new fault-fixture modules: 0
Windows acceptance producer transactions: 1 maximum
live positive Deck exercises: 1 maximum
```

The one acceptance producer contains two clean host-build roots. That does not authorize repeated producer attempts. The separately approved one-time `seed-fixture` operation reuses exact accepted WA0 custody bytes and the existing Mac-to-Deck lane; ordinary transactions cannot rebuild AGain or silently reseed it.

## Non-negotiable reuse and execution laws

- Keep the true host producer P, original Deck execution E, and current evidence consumer C distinct. Never relabel a historical build or execution as having run a newer consumer.
- Accept a cached result only through the complete typed identity, custody, positive-result, quiescence, shutdown, cleanup, and original protected-state predicate in the card.
- With a valid retained result, an evidence-renderer-only correction performs zero Windows builds, artifact custody operations, transfers, or Deck executions.
- Persist exact-input intent and stable phase nonces before remote effects. Recover lost acknowledgements and concurrent starts without duplicate expensive work; unresolved outcomes remain blocked.
- Keep GitHub authentication on the Mac. The Deck performs no GitHub operation and receives no forwarded agent or credentials.
- Reuse the accepted build, custody, transport, Runtime/Proton, supervision, containment, and protected-state mechanisms with only the adaptations approved in DX0.
- Preserve accepted historical evidence and protected fixtures. No Bitwig or Serum execution is authorized.

## Implementation topology and completion

Create the implementation branch from the exact merged design-authority commit/tree after readback, not from this unmerged design branch or the earlier selection basis.

The completed implementation has one source commit directly above that basis and one evidence-only child commit: ten source/configuration changes, then five evidence files. Producer, execution, and renderer provenance remain independently truthful under the approved reuse model.

Run local deterministic validation before the expensive lane, freeze source, and use the implemented driver for the approved end-to-end proof. A normal defect inside the approved owners may be repaired within the approved scope and remaining execution budget; it does not grant another build or live exercise. A material design discovery returns to the design gate.

The implementation agent obtains the required fresh pre-PR audit, opens one ordinary non-draft implementation PR, and leaves it unmerged. Technical-lead acceptance and separate status closure follow. No successor or design amendment is authorized.
