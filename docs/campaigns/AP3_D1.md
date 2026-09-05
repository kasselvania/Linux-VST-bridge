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
authorized_source_commit: 19e468f96008c8e4df9f86066f9735645b610ab9
authorized_source_tree: 3c59a0dc41fb80924812c78e2ad9a2a76dea8a74
authorized_plan_id: ap3-sustained-audio-diagnostic-v1
authorized_product_contract_identity: ap3-sustained-audio-v1
authorized_product_contract_sha256: cba129f770a8d16b1def770c99bd56fe632aa5d6352ebf08a4445b75ea2834aa
authorized_plan_content_sha256: 65e5dff0225daa148511dbfaa0d6cdf8bb83ac33ebb5d130cde409b328b42d70
diagnostic_campaign_identity: 7110e1871dab383c2e002ecb8733ecfea0d663acb46197e4ef2469dff98c36bb
diagnostic_batch_budget: 10
diagnostic_plan_revision_authorized: true
diagnostic_previous_plan_content_sha256: 65e5dff0225daa148511dbfaa0d6cdf8bb83ac33ebb5d130cde409b328b42d70
diagnostic_previous_product_contract_sha256: cba129f770a8d16b1def770c99bd56fe632aa5d6352ebf08a4445b75ea2834aa
accepted_product_frontier: AP2
current_product_target: AP3
```

The active AP3 ruling authorizes this exact repaired source, native artifact and closed continuation in the SAME campaign. Four reservations remain consumed: D1 retained a first-stream comparison but failed during the second stream; its missing first-fault detail remains unknown. D2 verified 5,764,096 samples at maximum error 0.0 and clean unchanged protected state. D3 reached the GUI setup timeout; its retained owned stage was separately verified and retired with protected state unchanged. D4 verified all numerical streams and both Bitwig processing sessions; admission rejected the second application exit after the 30-second GUI wait. Its plug-in shutdown was clean and its stage retired with protected state unchanged. The GUI exit wait is now 60 seconds; injected-clock tests verify orderly completion after 35 seconds. Nested checkpoint tests preserve the full bounded contract fields. No diagnostic can be promoted. One Windows producer is consumed; its inputs remain unchanged and its retained artifact is reused.

The scoped old-to-current plan adjustment preserves all four reservations and the ten-batch ceiling. The current four-instance batch contains the primary 30-second 128/256-frame streams without active Moonlight, a separately confirmed short stream-active SDK run, and two fresh disposable Bitwig sessions with the exact native publication. Failed native, Windows, GUI or reporting admission stops further segments. All useful facts are checkpointed before validation/retirement. Existing ownership, timeouts, mappings, cleanup and artifact custody remain responsible. No new campaign, count reset or automatic replay is permitted.

Bitwig setup was completed before the comparison baseline: only test search locations and 48 kHz/256-frame audio settings changed; original preference bytes are privately backed up. Existing projects and plug-in installations are preserved. Actual agent-through-Moonlight control is separately established. AP2 remains accepted; final AP3 acceptance requires a fresh complete candidate and review.

```json
{
  "schema": "linux-vst-bridge-proof-plan-descriptor/v1",
  "plan_id": "ap3-sustained-audio-diagnostic-v1",
  "execution_class": "DIAGNOSTIC_NON_AUTHORITATIVE",
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
