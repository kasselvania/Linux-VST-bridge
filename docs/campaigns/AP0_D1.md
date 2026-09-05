# AP0-D1 — Offline sample diagnostic

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
authorized_source_commit: b5c788b622f3e4982b6fb1c01a4dd0f79a8e2cc7
authorized_source_tree: 0fa9ad46132a11d01fd4a4d2f378da2f5ab4e152
authorized_plan_id: ap0-offline-again-diagnostic-v1
authorized_product_contract_identity: ap0-offline-again-v1
authorized_product_contract_sha256: ebd654929ae68fbf4481a75f7684f557471f7882cba4f10a1faaccc2ec248879
authorized_plan_content_sha256: 1d025d5592bde57decff6966b52301d06406cb350a961c4a30994763d9f97e90
diagnostic_campaign_identity: 1b104be869d0e4b6ecf8bfb13df9328c76edcafa7c767c8ab6c0ac3111411f6f
diagnostic_batch_budget: 8
accepted_product_frontier: PC0
current_product_target: AP0
```

The active AP0 ruling delegates this bounded diagnostic for the implemented contract in docs/slices/AP0/CONTRACT.md. This observes the new product increment; it does not authorize another product change or promote diagnostic output. The campaign has eight cumulative reservations. Before each new launch use tested, committed source, exact host inputs and current clean containment. Source-only harness repair may rebind the source fields while preserving the campaign and all consumed records. A lost acknowledgement is reconciled without relaunch. Raw/checkpoint material remains private and acceptance-ineligible.

The exact retained fixture, new host artifact and producer are bound in AP0_ARTIFACT.json and the closed descriptor below. The adapter enforces current Windows build-input equivalence with the retained producer. The shared supervisor and worker retain bounded errors before retirement. Scoped processing calls are AP0 only; PC0 histories remain untouched.

```json
{
  "execution_class": "DIAGNOSTIC_NON_AUTHORITATIVE",
  "plan_content": {
    "artifact_requirement": {
      "artifact_id": 9961756527,
      "host_manifest_sha256": "7e6093770aaf4767736b9aa7ef9db3ec37c861e3ead702ec49b5f122d7769c5f",
      "producer_run_attempt": 1,
      "producer_run_id": 33940737728,
      "producer_source": {
        "commit": "3a7553065733c0516567692929e84499fc8ed483",
        "identity_sha256": "511c095c748595c125560bbe3951adc9cbc1e8b8eb7a3525a23491d76dc36db3",
        "manifest_sha256": "edb0a6158d02437b45be518b087f700ef273e4e268e893b074ef8c17e2b7d61b",
        "parent": "56b7910d86e89a467c340ed29fb6d9b78ed0c919",
        "ref": "refs/heads/codex/ap0-offline-again-processing",
        "tree": "1bf22e8a61aa1a158b577089741e179505564e91"
      },
      "windows_build_input_identity": "32e68da8b2386526d07995cb97d45deac8b04131be7c28778587e7e7db320f1f"
    },
    "fixture_requirement": {
      "accepted_fixture_identity_sha256": "6c87be964d26a7ad06e7a4c69c5c5261d1046e9cfb0b17a225fd24c3e40d0ba6",
      "again_bundle_manifest_sha256": "bfaa1dce4d2e189f89cee41493838824efe647e361e81436676b7b3a86ff5164",
      "again_module_sha256": "60aa9ff6b9918d4330449e7b3ab34b588dd93cba09f37413a3cd91f6e7d2e18f"
    },
    "operation": "ap0-offline-stereo-three-blocks",
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
  "plan_content_sha256": "1d025d5592bde57decff6966b52301d06406cb350a961c4a30994763d9f97e90",
  "plan_id": "ap0-offline-again-diagnostic-v1",
  "product_contract_identity": "ap0-offline-again-v1",
  "product_contract_sha256": "ebd654929ae68fbf4481a75f7684f557471f7882cba4f10a1faaccc2ec248879",
  "schema": "linux-vst-bridge-proof-plan-descriptor/v1"
}
```
