# AP3-D1 — Sustained audio diagnostic

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
authorized_source_commit: 14e9b274dbac8b96ab1802c374ba99f10e6f9ba6
authorized_source_tree: 0a39c398fa997a765662e11241bc3ae20f35b709
authorized_plan_id: ap3-sustained-audio-diagnostic-v1
authorized_product_contract_identity: ap3-sustained-audio-v1
authorized_product_contract_sha256: d60bc93960b282957018c47b5e017efc7d3d11078905fc0f638ef9eb529723c0
authorized_plan_content_sha256: d86da4df6439ea762ee0776f093053e1258e7e80fabb74d49b90cc10339922d6
diagnostic_campaign_identity: 7110e1871dab383c2e002ecb8733ecfea0d663acb46197e4ef2469dff98c36bb
diagnostic_batch_budget: 10
accepted_product_frontier: AP2
current_product_target: AP3
```

The AP3 product-contract and execution ruling in AGENTS.md/CURRENT_SLICE.md authorizes this exact diagnostic binding. One campaign has ten cumulative diagnostic batches. Six Windows producers and two fresh acceptance candidates are separately bounded; one producer has been consumed. No old reservation is reset or replayed. This initial batch exercises one Windows instance through both 30-second SDK-host streams with actual AGain, a reused mapping, stop/restart and preserved gain. Bitwig and stream-active observations remain separate required work before final AP3 acceptance; this diagnostic cannot support an acceptance claim.

Native SDK-loaded positive and failure tests passed before this binding, including full-length substituted-peer timeline/digest verification and callback instrumentation. Sunshine/Moonlight setup and actual agent control are recorded in docs/DECK_REMOTE_DESKTOP.md. Primary timing uses no active stream. Planned desktop changes preceded this baseline; preserve historical installs, source, fixtures and evidence. Existing custody, bounded supervision, checkpoints, cleanup and protected-state checks own the run. Retain any original error before retirement and fix ordinary scoped bugs within the remaining allowance. AP2 remains accepted.

```json
{
  "schema": "linux-vst-bridge-proof-plan-descriptor/v1",
  "plan_id": "ap3-sustained-audio-diagnostic-v1",
  "execution_class": "DIAGNOSTIC_NON_AUTHORITATIVE",
  "product_contract_identity": "ap3-sustained-audio-v1",
  "product_contract_sha256": "d60bc93960b282957018c47b5e017efc7d3d11078905fc0f638ef9eb529723c0",
  "plan_content": {
    "artifact_requirement": {
      "artifact_id": 9974517171,
      "host_manifest_sha256": "155d7bcea5d201da647c170f10ad7df0cf163e00710c8a4a4611099ae30a2ec1",
      "native_client": {
        "input_sha256": "42750af1837e25b74b3e21a521193e9b7ccf9bd3f115c4552eeacca9aff00c9b",
        "manifest_sha256": "86fc51ebb49943205f40e1abdcb27743017e0c8acec37328a90a46f1b1441ec9",
        "source_commit": "9cefdb317f00896a7f9cf1b531e222d31bc03c3e"
      },
      "producer_run_attempt": 1,
      "producer_run_id": 33983608691,
      "producer_source": {
        "commit": "eb08e8eba607ae121e02a236787f5245053f8b16",
        "identity_sha256": "e990307e4828a07fd0ba62698c1bde2e0cc3a10cfccadb109082e1e920e75e1b",
        "manifest_sha256": "cd1e9779be7247a6bf1f4f57a1e2d67d8c1aefde62621c82bbcea3e7209ee1fc",
        "parent": "4add25748b82eb091ef4c0ba90ee23a266a4d3c9",
        "ref": "refs/heads/codex/ap3-sustained-audio-and-remote-desktop",
        "tree": "a0ba39044f6b885435b0d3d2b67fcc88976a057f"
      },
      "windows_build_input_identity": "d55132cff7113b78d61d757bb2a46f892c6304e04c022369b3b3a5d2ebbe8c78"
    },
    "fixture_requirement": {
      "accepted_fixture_identity_sha256": "6c87be964d26a7ad06e7a4c69c5c5261d1046e9cfb0b17a225fd24c3e40d0ba6",
      "again_bundle_manifest_sha256": "bfaa1dce4d2e189f89cee41493838824efe647e361e81436676b7b3a86ff5164",
      "again_module_sha256": "60aa9ff6b9918d4330449e7b3ab34b588dd93cba09f37413a3cd91f6e7d2e18f"
    },
    "operation": "ap3-sdk-host-paced-streams",
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
  "plan_content_sha256": "d86da4df6439ea762ee0776f093053e1258e7e80fabb74d49b90cc10339922d6"
}
```

Continuation: the first reservation remains consumed. Its 128-frame stream
reported 2,882,048 samples and zero maximum error; the second interval failed.
Cleanup completed and protected state was unchanged. The older checkpoint
omitted AP3 fault/timing fields, which remain unknown for that transaction.
This tested retention repair preserves those fields and sanitized original
errors; it changes neither native nor Windows artifact inputs. Nine diagnostic
batches remained before this new source binding. No diagnostic is acceptance.
