# AP1-D1 — Linux / Windows mapped audio

## Authority

```yaml
status: active_diagnostic_campaign
authority_phase: proof_harness_maintenance
change_class: PROOF_HARNESS_MAINTENANCE
repository: kasselvania/Linux-VST-bridge
product_implementation_authorized: false
maintenance_implementation_authorized: true
live_execution_authorized: true
permitted_execution_class: DIAGNOSTIC_NON_AUTHORITATIVE
classified_backend_core_ready: true
classified_backend_ready: true
acceptance_eligible: false
authorized_source_commit: fd3d70a36b65e1bba9fefc4914ebb1685b10faae
authorized_source_tree: 63328b0b7936f5ea7ed2812cba19493dfec1ac2f
authorized_plan_id: ap1-linux-windows-audio-diagnostic-v1
authorized_product_contract_identity: ap1-linux-windows-audio-v1
authorized_product_contract_sha256: ad5ad1a5ebe0efec9df93f69f91091ce3c9c405c00437a7f3dee700fd4b2868b
authorized_plan_content_sha256: 03ac8e98cd9212b34ff518e6ec75ddeb17ad38997ff104bf329affd3a3707ed0
diagnostic_campaign_identity: 78c32c41080c2afb3f8bf85967474dfe49016eb84c76f62ac0bb272118fbd755
diagnostic_batch_budget: 10
diagnostic_plan_revision_authorized: true
diagnostic_previous_plan_content_sha256: 358cb5f4cc63eef6a5dcfc65b32b0e0bad5b6f61fefd06a8df8a37662c62498b
diagnostic_previous_product_contract_sha256: 9de6ea4c76d676891cee8f080892e9302828310606f5b6a450cd87b091026bfb
accepted_product_frontier: AP0
current_product_target: AP1
```

This review-repair binding preserves diagnostic reservation 1 and its historical authority. CURRENT_SLICE.md at c62a5950d286ee2aac328a332998fb2190e85ba2 explicitly authorizes the repair, necessary rebuild and fresh verification; cumulative producer consumption is now 2 of 6, with 1 of 10 diagnostics and 1 of 2 acceptance candidates consumed before this binding. The locked atomic plan adjustment changes only the current diagnostic plan/contract identities; it does not reset consumption or rewrite old transactions.

This mechanical binding implements AGENTS.md Active AP1 ruling and CURRENT_SLICE.md Execute and deliver. Local adapter, worker, ownership, protocol, sample and failure-retention tests passed. One AP1 campaign has ten cumulative diagnostic reservations; the separate task ceiling is six Windows producer attempts and two fresh acceptance candidates with one batch each. No historical campaign or accepted evidence changes.

Reuse the bound native client, Windows host, AGain and runtime. Linux chooses inputs after Ready and independently checks every mapped sample, retaining request words and actual output. Keep one mapping, host, instance and authenticated loopback connection through ten bounded blocks, preserving the original eight and adding zero gain on non-silent input and a correctly formed left-channel-silent request. Both endpoints must be contained before stage retirement; protected state stays unchanged. Diagnostic results are permanently acceptance-ineligible. No replay after acknowledgement loss.

Exact source/artifact/plan revisions for tested in-scope repairs use the existing explicit locked atomic adjustment path; preserve all consumed reservations and original authority. AP0 remains accepted pending AP1 review and merge.

```json
{
  "execution_class": "DIAGNOSTIC_NON_AUTHORITATIVE",
  "plan_content": {
    "artifact_requirement": {
      "artifact_id": 9971070716,
      "host_manifest_sha256": "f38dee93baf0e4054f3d729e561f9aa8756605935174ccb4cd889cd9d74cf1dc",
      "native_client": {
        "binary_sha256": "9191a64e92faf92559cf0121208f73b52c2c2d5d4534d75f6c44ede27ebe2225",
        "input_sha256": "d5208c43e0de8fe6a3217f0885ac838233ad450f26b825032077576955881ba3",
        "manifest_sha256": "cabc63622f73a9c541941a8483f605bc929c2a11b04467928f079a1760cb7ce8",
        "source_commit": "34699b682439ed75ea9fd0992b98222253b6f1da"
      },
      "producer_run_attempt": 1,
      "producer_run_id": 33971421519,
      "producer_source": {
        "commit": "34699b682439ed75ea9fd0992b98222253b6f1da",
        "identity_sha256": "382f6f771a1673adfb679bcebb50f95f36240898bebd541e79db903b2bb8ed51",
        "manifest_sha256": "34fe27bbc5ceb249d8e92de5a212795cd5c5f51dd2bff44ab3a7df3ae4c054c7",
        "parent": "c62a5950d286ee2aac328a332998fb2190e85ba2",
        "ref": "refs/heads/codex/ap1-linux-windows-audio-roundtrip",
        "tree": "1c1c713141e4c937a1fc68549cdfcaf03e964e32"
      },
      "windows_build_input_identity": "0233840739306c03a3fc237ad1af1fd61eccf32f303637b7cdfb0b56a773afe7"
    },
    "fixture_requirement": {
      "accepted_fixture_identity_sha256": "6c87be964d26a7ad06e7a4c69c5c5261d1046e9cfb0b17a225fd24c3e40d0ba6",
      "again_bundle_manifest_sha256": "bfaa1dce4d2e189f89cee41493838824efe647e361e81436676b7b3a86ff5164",
      "again_module_sha256": "60aa9ff6b9918d4330449e7b3ab34b588dd93cba09f37413a3cd91f6e7d2e18f"
    },
    "operation": "ap1-mapped-stereo-ten-blocks",
    "runtime_requirement": {
      "declared_runtime_inputs_sha256": "44cabf121629011dea5472330af03451774fc8c6bfc2c21ee2f9eb69009aca86",
      "remote_pc0_plan_sha256": "501829c4bf88988afb13ad984d5220839b73315d1ba89c8ca2e77600e58dc248",
      "runtime_proton_identity_sha256": "2d64df1d36786ca2d0e955c553005423dc2b5bdd714bd0a17872622e33912547",
      "stopped_pc0_archive_ref": "refs/heads/codex/archive/pc0-v3-stopped-309b8918",
      "stopped_pc0_source_commit": "309b8918c128c0b9e6701d0453dc841a111d5ac5",
      "stopped_pc0_source_tree": "a6a123b554eb220cbdfb9bf9afab3d08d4a8ac92"
    },
    "schema": "linux-vst-bridge-closed-proof-plan/v1"
  },
  "plan_content_sha256": "03ac8e98cd9212b34ff518e6ec75ddeb17ad38997ff104bf329affd3a3707ed0",
  "plan_id": "ap1-linux-windows-audio-diagnostic-v1",
  "product_contract_identity": "ap1-linux-windows-audio-v1",
  "product_contract_sha256": "ad5ad1a5ebe0efec9df93f69f91091ce3c9c405c00437a7f3dee700fd4b2868b",
  "schema": "linux-vst-bridge-proof-plan-descriptor/v1"
}
```
