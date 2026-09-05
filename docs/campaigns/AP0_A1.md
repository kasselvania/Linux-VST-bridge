# AP0-A1 — Fresh offline sample acceptance

## Authority

```yaml
status: active_acceptance_candidate
authority_phase: implementation
change_class: PRODUCT_CONTRACT_CHANGE
repository: kasselvania/Linux-VST-bridge
product_implementation_authorized: true
live_execution_authorized: true
permitted_execution_class: ACCEPTANCE_CANDIDATE
classified_backend_core_ready: true
classified_backend_ready: true
authorized_source_commit: d9f4a0dec920b71b50fa07460b288d9e7407313b
authorized_source_tree: 89541481201e862767743dbee47a82aff2747beb
authorized_plan_id: ap0-offline-again-acceptance-v1
authorized_product_contract_identity: ap0-offline-again-v1
authorized_product_contract_sha256: 803d8374a6c46a86b3789c23af4375edf8c881054c92b612774489197de5adba
authorized_plan_content_sha256: d83f8313e6dff10c5ea60759d97fc117f2ca6f5075c3df6b201c7330fb3e3b83
acceptance_candidate_identity: 7968963e0756e148da9e6c3156c8b444a92ff48cc1639db431abdbf2459a5405
acceptance_batch_budget: 1
accepted_product_frontier: PC0
current_product_target: AP0
```

The active AP0 ruling delegates this first fresh acceptance candidate for the tested offline processing path and exact retained host artifact below. Execute only after current containment/preflight succeeds. This candidate has one workload reservation; a lost acknowledgement is reconciled without relaunch. Prior diagnostic observations remain permanently acceptance-ineligible. The task permits at most two acceptance candidates, the second only after a specific tested correction and verified current cleanup. No task counter is reset. PC0 remains the accepted frontier pending AP0 review and merge.

Validate all 96 returned samples against the independent, fixed, zero-tolerance contract in docs/slices/AP0/CONTRACT.md. Retain the actual notification returns, lifecycle/thread/shutdown and protected-state results. Only a successful fresh candidate may render the sanitized acceptance evidence. Preserve any failure privately using the shared retention path.

```json
{
  "execution_class": "ACCEPTANCE_CANDIDATE",
  "plan_content": {
    "artifact_requirement": {
      "artifact_id": 9962278098,
      "host_manifest_sha256": "ad84f6339c30fbcfbff854be85c7c2f6d811c3536edb8ab882eed4ebb8aa01f0",
      "producer_run_attempt": 1,
      "producer_run_id": 33942379173,
      "producer_source": {
        "commit": "0add6e51bb9c0f2063906630679777503967f9fc",
        "identity_sha256": "31f41720f53c92d0335b084c727c2c8e482968545227a9f26dcd9331db0416e6",
        "manifest_sha256": "a6244f2052ca4c24bdfe822a96cbf5ee4eb61ebcf6e544e5b26252cce6d8dc20",
        "parent": "e96ca7f59135d50baceff7fd0ca5dffe09fdbac1",
        "ref": "refs/heads/codex/ap0-offline-again-processing",
        "tree": "415dd6ec4387f855920570eaff11909c55dd4ca5"
      },
      "windows_build_input_identity": "2502c86980162bcb51c474b6cb952c670b8d45a019c26d1a8ed050a88ac4616b"
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
  "plan_content_sha256": "d83f8313e6dff10c5ea60759d97fc117f2ca6f5075c3df6b201c7330fb3e3b83",
  "plan_id": "ap0-offline-again-acceptance-v1",
  "product_contract_identity": "ap0-offline-again-v1",
  "product_contract_sha256": "803d8374a6c46a86b3789c23af4375edf8c881054c92b612774489197de5adba",
  "schema": "linux-vst-bridge-proof-plan-descriptor/v1"
}
```
