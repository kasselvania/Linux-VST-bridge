# Current Work: AP1 — Repair output-silence handling

**Review repair implemented; the operator has authorized AP1 binding to installed Proton 11.0-2c and completing fresh verification in PR #51.** Branch: `codex/ap1-linux-windows-audio-roundtrip`. Technical-lead review `5120241348` covers implementation head `6bf0d541323fc864945344ce0a82b0d3e754224d`. AP0 remains accepted. The active AP1 ruling in AGENTS.md continues to authorize implementation, necessary builds, focused tests and exact candidate binding without an intermediate planning handoff.

The repair and rebuilt endpoints passed focused tests and were delivered. Read-only preflight found Proton 11.0-2c/build 25118279 instead of pinned 11.0-2/build 24867889; launcher bytes also differ. No new live reservation was consumed. Current processes/stages are absent and protected state matches the first acceptance. See [review-repair result](docs/slices/AP1/RESULT_A2.md). The operator explicitly approved binding AP1 to installed Proton 11.0-2c after the automatic update was identified. Record its exact runtime identity for AP1 only, preserve historical PC0/AP0 bindings and all prior results, reuse unchanged endpoint artifacts, and finish the remaining verification under the existing allowance. No runtime installation or update-policy change is authorized or required.

## Preserve the successful result

The first AP1 acceptance reported 1,344/1,344 Linux-read samples with maximum error 0.0 across eight changing blocks, one instance/mapping/connection, complete cleanup and unchanged protected state. See [RESULT.md](docs/slices/AP1/RESULT.md). That observation remains successful for its actual vectors; do not relabel it as failed, reset its budget or alter its immutable result. Review found a valid-request case those vectors do not cover.

AP1's goal remains native Linux-owned samples -> shared mapping -> Windows AGain -> mapped output independently checked on Linux. Preserve the selected offline single-instance, single-slot contract in [CONTRACT.md](docs/slices/AP1/CONTRACT.md), subject only to the correction below. No DAW, real-time, editor, commercial plug-in, runtime installation or general transport work is added. The sole runtime-selection change is the explicitly approved installed Proton 11.0-2c binding for AP1.

## The repair

`MappedSession::done` wrongly requires output silence flags to equal input silence flags. The parser admits normalized gain [0,1] and input masks 0..3, but the pinned AGain legitimately returns mask 3 for gain zero on ordinary input, and mask 0 while processing a partly silent stereo input. Both currently become transport failures despite successful processing.

Treat output silence as a result, not a copy of input. Check legal output bits and zero-valued samples for channels actually marked silent; do not require all zero-valued channels to be flagged. Preserve the actual flags in the appropriate response or retained observation. Keep exact sample comparison, buffer guards, request correlation, no-replay, deadlines and cleanup intact. Do not merely disable result checking or narrow away these ordinary supported inputs.

Use focused tests exercising the actual native result handler for zero gain, one flagged-silent channel, existing normal/all-silent cases and invalid silence claims. Preserve the original eight numerical cases and include the two exposed valid-request cases in the repaired real-fixture verification. The peer's flags need not equal Linux's flags; the independently expected samples must still match. Necessary small client/checker, protocol documentation and shared-helper edits are included. Choose the smallest implementation; no new framework.

The source basis is the pinned AGain `process` implementation in public.sdk commit `586dc5e6c8012c3e4b01c79389375cbe96bdb1da`, `samples/vst/again/source/again.cpp`, especially its ordinary-output and near-zero-gain silence handling. Historical AP0/PC0 checks and evidence remain unchanged.

## Finish under the existing allowance

Current consumption after the review repair: **2 of 6 Windows producers, 1 of 10 AP1 diagnostics, 1 of 2 acceptance candidates**. No count is reset or ceiling increased. The lead explicitly permits the remaining second acceptance candidate after this review-driven repair even though the first measured candidate succeeded; this supersedes the earlier condition limiting candidate two to an execution failure/inconclusive result. This is correction of an admitted-input bug, not a new product selection.

Test locally, build the changed endpoint through the existing producer, bind the actual source/artifacts/plan, and obtain fresh verification through existing classified commands. Reuse unchanged SDK/AGain and working supervision/retention; AP1 alone uses the exact operator-approved installed Proton 11.0-2c binding. Diagnostics are available only as useful within the remaining allowance. Lost acknowledgement is reconciled, never replayed. Preserve all earlier private records and Git history; the current PR evidence may be superseded by a clearly identified new result, never by relabeling the first result.

Continue PR #51; leave it unmerged for focused rereview. Report actual numerical and silence-handling results, cleanup, cumulative costs and exact head. Do not stop at tests or a successful build when authorized verification can proceed. Unknown containment, changed protected state, exhausted allowance or a genuinely different ownership/transport decision stops live work; ordinary local repairs do not require another approval.

## Authority — default command guard

This unchanged mapping blocks unspecified live commands, not AP1's scoped task. Exact AP1 receipts authorize its tested source and remaining candidate.

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
