# HP1 deterministic negative tests

| Case | Result |
|---|---|
| wrong_hp0_module_hash | `passed` |
| invalid_hp0_publication_receipt | `passed` |
| wrong_bitwig_app_commit | `passed` |
| wrong_bitwig_runtime_commit | `passed` |
| changed_user_override | `passed` |
| changed_system_override | `passed` |
| bitwig_already_running | `passed` |
| unrelated_mapping_rejected | `passed` |
| valid_descendant_mapping_accepted | `passed` |
| stale_pid_rejected | `passed` |
| second_session_identity_reuse_rejected | `passed` |
| forbidden_process_blocks | `passed` |
| wrong_or_reused_nonce_rejected | `passed` |
| state_candidate_cap_propagates_unknown | `passed` |
| symlinked_session_or_evidence_ancestor_rejected | `passed` |
| override_before_after_mismatch_invalidates | `passed` |
| serum_hash_size_mtime_drift_invalidates | `passed` |

Aggregate: `passed` (17/17). Fixtures were synthetic and stayed beneath the canonical cache test root.
