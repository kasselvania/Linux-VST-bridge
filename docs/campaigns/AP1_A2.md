# AP1-A2 — Fresh verification of output silence on Proton 11.0-2c

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
authorized_source_commit: a2e12c52df0db34c6476c1b337065e1f53e92638
authorized_source_tree: 33c582b8e40572732dc145b1626fc8d23c804e44
authorized_plan_id: ap1-linux-windows-audio-acceptance-v1
authorized_product_contract_identity: ap1-linux-windows-audio-v1
authorized_product_contract_sha256: 4ce972f4f68bc38bf1795896dc2f1c9478a6bce4d7330c28bdb85bf6991b856a
authorized_plan_content_sha256: 95fdac448b5934502bde990cb875b561e1750c30a9bb638a5d8126c73b602166
acceptance_candidate_identity: d3953b583ad60230deff0c306d7d6d015f5969ca8b48c03d2bb0937eebfc5eb6
acceptance_batch_budget: 1
accepted_product_frontier: AP0
current_product_target: AP1
```

The operator and AGENTS.md AP1 runtime-update continuation explicitly authorize this second and final acceptance candidate for the existing silence repair on the completed official Proton 11.0-2c update. CURRENT_SLICE.md permits candidate two after this reviewed repair even though the first measured candidate succeeded. The first acceptance receipt, result and all historical PC0/AP0 runtime bindings remain unchanged.

The exact tested source and retained host/native artifacts passed diagnostic batch 2: all 1502 Linux-read samples exact across the original eight and two added silence cases, output masks 3 for zero gain/non-silent input and 0 for one-channel silence, complete containment/retirement and unchanged protected state. Diagnostic reservation 55671028529aabb295fc7faff5e8f6d09415108bacf8e76634d45777e4a65260 remains acceptance-ineligible. This candidate obtains a fresh observation; no diagnostic promotion or replay is authorized.

One batch is authorized for this candidate. Before it, cumulative consumption is 2 of 6 Windows producers, 2 of 10 diagnostic reservations in the same AP1 campaign, and 1 of 2 acceptance candidates. Retain all consumed counts. Verify the selected Proton version/build/depot and 33 runtime inputs at preflight, launch and post-run. Keep one host, AGain instance, mapping and loopback connection through the ten cases. Preserve ownership, no-replay, timeouts, cleanup and protected state. AP0 remains accepted pending final AP1 review and merge.

```json
{
  "execution_class": "ACCEPTANCE_CANDIDATE",
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
      "declared_runtime_inputs_sha256": "e701be47751c64e1cef522a4021cc5e283f110ab1124fd535286a53b657c1925",
      "remote_pc0_plan_sha256": "501829c4bf88988afb13ad984d5220839b73315d1ba89c8ca2e77600e58dc248",
      "runtime_proton_identity_sha256": "20064247dc297d0228bbf63d5a49050238c518d14fbb77b61a8c73351db5ebf7",
      "stopped_pc0_archive_ref": "refs/heads/codex/archive/pc0-v3-stopped-309b8918",
      "stopped_pc0_source_commit": "309b8918c128c0b9e6701d0453dc841a111d5ac5",
      "stopped_pc0_source_tree": "a6a123b554eb220cbdfb9bf9afab3d08d4a8ac92"
    },
    "schema": "linux-vst-bridge-closed-proof-plan/v1"
  },
  "plan_content_sha256": "95fdac448b5934502bde990cb875b561e1750c30a9bb638a5d8126c73b602166",
  "plan_id": "ap1-linux-windows-audio-acceptance-v1",
  "product_contract_identity": "ap1-linux-windows-audio-v1",
  "product_contract_sha256": "4ce972f4f68bc38bf1795896dc2f1c9478a6bce4d7330c28bdb85bf6991b856a",
  "schema": "linux-vst-bridge-proof-plan-descriptor/v1"
}
```
