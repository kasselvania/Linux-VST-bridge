# AP2-A1 — Fresh SDK-hosted native VST3 offline verification

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
authorized_source_commit: 6c3ccd5888c17f2d236f433d45fe0ee0b8dc70fb
authorized_source_tree: 35a73d7d15b47baa77448b266d16be61af3b3b0c
authorized_plan_id: ap2-native-vst3-offline-acceptance-v1
authorized_product_contract_identity: ap2-native-vst3-offline-v1
authorized_product_contract_sha256: a0bbc29a2d21f602c20c3780b3442b2f0835ce2877c8cc7ad0d10b9dfdf1133d
authorized_plan_content_sha256: 4981acb11291913466cc3dbe40c09334f99f1660e5884ac963454e48490198b8
acceptance_candidate_identity: aed0b88682de07cfaefe164fd9713e73b9043a4452155bece29f5aaf3c4e6bdf
acceptance_batch_budget: 1
accepted_product_frontier: AP1
current_product_target: AP2
```

The AP2 work order in CURRENT_SLICE.md and AGENTS.md authorizes this first acceptance candidate with the tested exact executable source and retained native/Windows artifacts. One fresh batch contains the persistent activation and one clean unload/reopen. It preserves the existing AGain fixture and selected AP1 Proton 11.0-2c/Runtime 4 inputs. The SDK host chooses fresh inputs after activation and compares every actual returned sample through standard VST3 interfaces.

Diagnostic reservation 9e20709de8389f8200dfc281bf0cb863c7f95738758aefb62a8e62cb50aed30a retained 1730 exact host-returned samples, maximum error 0.0, correct independent output silence flags, 14 real Windows process calls, clean unload/reopen, zero owned descendants, absent disposable stage and unchanged protected state. That observation remains acceptance-ineligible; this candidate requires its own fresh run.

Before this candidate, cumulative AP2 consumption is two of six Windows producers, one of ten batches in the one AP2 diagnostic campaign, and zero of two acceptance candidates. No old result, reservation or count changes. Unknown containment or changed protected state stops live work. The unused second candidate requires a specific tested implementation/review repair. AP1 remains accepted until AP2 review and merge.

```json
{
  "schema": "linux-vst-bridge-proof-plan-descriptor/v1",
  "plan_id": "ap2-native-vst3-offline-acceptance-v1",
  "execution_class": "ACCEPTANCE_CANDIDATE",
  "product_contract_identity": "ap2-native-vst3-offline-v1",
  "product_contract_sha256": "a0bbc29a2d21f602c20c3780b3442b2f0835ce2877c8cc7ad0d10b9dfdf1133d",
  "plan_content": {
    "artifact_requirement": {
      "artifact_id": 9973099999,
      "host_manifest_sha256": "a3013d846a3a43ccf60fbc99b6654d39d529a24e661f7b419be32ffdc0b1faf6",
      "native_client": {
        "input_sha256": "5a8cadddfe8cd12e03cbb2e604c15dba960e49432e4519a1e89c910a8edc9b93",
        "manifest_sha256": "0bf14a50ed00e2216260e44d3e639a5044e269817e6c37dff81df511f4067e7a",
        "source_commit": "47673cc268e2c4ac15729a43083e7dce8220b1aa"
      },
      "producer_run_attempt": 1,
      "producer_run_id": 33978589618,
      "producer_source": {
        "commit": "f1402568d20d8f3e3331db7cae65aab6953edb9a",
        "identity_sha256": "249cfc7323f9a99c08eea078ccb69c89b1bb6ad00969101afa19cd31a141bcc6",
        "manifest_sha256": "46beb1420c3feb130be55b0938508bd7cfd78ba1f4d8c1d40cdc19c129803f5a",
        "parent": "47673cc268e2c4ac15729a43083e7dce8220b1aa",
        "ref": "refs/heads/codex/ap2-native-vst3-offline-bridge",
        "tree": "3b2556151ba76180df5cc245ae5f429071dd3aa8"
      },
      "windows_build_input_identity": "af9df27c46cae37a83430fc131573e4376b25157a951e04b64c981c8b65f46a3"
    },
    "fixture_requirement": {
      "accepted_fixture_identity_sha256": "6c87be964d26a7ad06e7a4c69c5c5261d1046e9cfb0b17a225fd24c3e40d0ba6",
      "again_bundle_manifest_sha256": "bfaa1dce4d2e189f89cee41493838824efe647e361e81436676b7b3a86ff5164",
      "again_module_sha256": "60aa9ff6b9918d4330449e7b3ab34b588dd93cba09f37413a3cd91f6e7d2e18f"
    },
    "operation": "ap2-sdk-host-process-and-reopen",
    "runtime_requirement": {
      "declared_runtime_inputs_sha256": "e701be47751c64e1cef522a4021cc5e283f110ab1124fd535286a53b657c1925",
      "remote_pc0_plan_sha256": "501829c4bf88988afb13ad984d5220839b73315d1ba89c8ca2e77600e58dc248",
      "runtime_proton_identity_sha256": "20064247dc297d0228bbf63d5a49050238c518d14fbb77b61a8c73351db5ebf7",
      "stopped_pc0_archive_ref": "refs/heads/codex/archive/pc0-v3-stopped-309b8918",
      "stopped_pc0_source_commit": "309b8918c128c0b9e6701d0453dc841a111d5ac5",
      "stopped_pc0_source_tree": "a6a123b554eb220cbdfb9bf9afab3d08d4a8ac92"
    },
    "schema": "linux-vst-bridge-closed-proof-plan/v1"
  },
  "plan_content_sha256": "4981acb11291913466cc3dbe40c09334f99f1660e5884ac963454e48490198b8"
}
```
