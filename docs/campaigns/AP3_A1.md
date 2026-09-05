# AP3-A1 — Fresh sustained audio and controlled Bitwig verification

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
authorized_source_commit: 5823ec68a719194e281b9f6f3b6d0853e2baf3b0
authorized_source_tree: a94f8bd386bee20fca4a857c4770687878c0f3fc
authorized_plan_id: ap3-sustained-audio-acceptance-v1
authorized_product_contract_identity: ap3-sustained-audio-v1
authorized_product_contract_sha256: cba129f770a8d16b1def770c99bd56fe632aa5d6352ebf08a4445b75ea2834aa
authorized_plan_content_sha256: 65e5dff0225daa148511dbfaa0d6cdf8bb83ac33ebb5d130cde409b328b42d70
acceptance_candidate_identity: 074da5c26cefdfa8118a7e22ccf38a7a75e490ebaabe8b9b13149291235fd7ec
acceptance_batch_budget: 1
accepted_product_frontier: AP2
current_product_target: AP3
```

The active AP3 work order authorizes this first acceptance candidate, its exact tested source and retained native/Windows artifacts. The fresh four-instance batch contains 30-second 128/256-frame SDK streams without active Moonlight, a separately confirmed 602,048-sample stream-active check, and two controlled disposable Bitwig sessions. The same-build GUI path uses standard VST3, the small gain control, normal stop/removal and clean app exit. Existing supervision, process ownership, timeouts, protected-state checks and result admission apply. The GUI exit window is separately bounded at 60 seconds; Windows workload timeouts remain unchanged.

Before this candidate: one of six Windows producers, five of ten diagnostic reservations in the same campaign, zero of two acceptance candidates consumed. D1's missing first-fault details stay unknown. D3 and D4 failures remain retained; D3's separately verified stage retirement is operational cleanup, not a rewritten receipt. D5 completed all numerical/GUI checks and shutdown, then encountered a local combined-result size limit. The tested envelope-only repair closed that SAME reservation with no launch and preserved its original authority, source and count. Its result 139301bbcd51a1383cae71eb03485e648e59fbec0b671bd0fee847d894b549a2 remains permanently acceptance-ineligible.

The component observation/admission limits remain 256 KiB; only their combined result envelope is 512 KiB. Local tests cover the original failure and same-reservation recovery without duplicate execution, and retain rejection of oversized components. No diagnostic is relabelled. This candidate must obtain its own fresh results. The second candidate requires a concrete tested repair; no automatic replay or new identity can reset totals. AP2 remains accepted pending AP3 review/merge.

Temporary Bitwig publication, test-only search locations and 48 kHz/256-frame settings were established before measurement. Original preference bytes are privately backed up for restoration after the test. Existing plug-ins and music projects are preserved. Actual agent-through-Moonlight control is separately demonstrated; compressed streaming is never the sample/timing oracle.

```json
{
  "schema": "linux-vst-bridge-proof-plan-descriptor/v1",
  "plan_id": "ap3-sustained-audio-acceptance-v1",
  "execution_class": "ACCEPTANCE_CANDIDATE",
  "product_contract_identity": "ap3-sustained-audio-v1",
  "product_contract_sha256": "cba129f770a8d16b1def770c99bd56fe632aa5d6352ebf08a4445b75ea2834aa",
  "plan_content": {
    "artifact_requirement": {
      "artifact_id": 9974517171,
      "host_manifest_sha256": "155d7bcea5d201da647c170f10ad7df0cf163e00710c8a4a4611099ae30a2ec1",
      "native_client": {
        "input_sha256": "1eae06fc129e3d79ce617e3f8a81b944661419b4e62d7f1bcbe18b81b5c746c6",
        "manifest_sha256": "609e5e6ce3ec9334fd8506ac19594f4546de828167cc092f973958035a02319d",
        "source_commit": "52e12cfbcee1d2c2c033a274be45ff89e84c9cc1"
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
    "operation": "ap3-paced-streams-and-bitwig-reopen",
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
  "plan_content_sha256": "65e5dff0225daa148511dbfaa0d6cdf8bb83ac33ebb5d130cde409b328b42d70"
}
```
