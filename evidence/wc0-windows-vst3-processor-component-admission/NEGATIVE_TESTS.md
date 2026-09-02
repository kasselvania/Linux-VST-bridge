# WC0 focused negative proofs

All
`20` deterministic owner checks passed.
Only the two direct AGain create failures and the exact sixteen WC0 fault
fixtures ran live. The inherited archive-negative suite and standalone adapter
runtime exercise were intentionally not replayed.

- `again-unknown-processor` -> `WC0_COMPONENT_CREATE_BLOCKED`; classification `scanner_blocked`; in flight `none`; clean in-process shutdown `true`
- `again-unsupported-interface` -> `WC0_COMPONENT_CREATE_BLOCKED`; classification `scanner_blocked`; in flight `none`; clean in-process shutdown `true`
- `wc0-create-failure-null` -> `WC0_COMPONENT_CREATE_BLOCKED`; classification `scanner_blocked`; in flight `none`; clean in-process shutdown `true`
- `wc0-create-success-null` -> `WC0_COMPONENT_CREATE_BLOCKED`; classification `scanner_blocked`; in flight `none`; clean in-process shutdown `true`
- `wc0-create-failure-nonnull` -> `WC0_COMPONENT_CREATE_BLOCKED`; classification `scanner_blocked`; in flight `none`; clean in-process shutdown `true`
- `wc0-controller-id-failure` -> `WC0_CONTROLLER_ID_BLOCKED`; classification `scanner_blocked`; in flight `none`; clean in-process shutdown `true`
- `wc0-controller-id-mismatch` -> `WC0_CONTROLLER_ID_MISMATCH`; classification `scanner_blocked`; in flight `none`; clean in-process shutdown `true`
- `wc0-initialize-failure` -> `WC0_COMPONENT_INITIALIZE_BLOCKED`; classification `scanner_blocked`; in flight `none`; clean in-process shutdown `true`
- `wc0-initialize-hang` -> `WC0_COMPONENT_INITIALIZE_BLOCKED`; classification `call_timeout`; in flight `initialize_component`; clean in-process shutdown `false`
- `wc0-initialize-crash` -> `WC0_COMPONENT_INITIALIZE_BLOCKED`; classification `abnormal_termination_in_flight`; in flight `initialize_component`; clean in-process shutdown `false`
- `wc0-terminate-failure` -> `WC0_COMPONENT_TERMINATE_BLOCKED`; classification `scanner_blocked`; in flight `none`; clean in-process shutdown `true`
- `wc0-terminate-hang` -> `WC0_COMPONENT_TERMINATE_BLOCKED`; classification `call_timeout`; in flight `terminate_component`; clean in-process shutdown `false`
- `wc0-terminate-crash` -> `WC0_COMPONENT_TERMINATE_BLOCKED`; classification `abnormal_termination_in_flight`; in flight `terminate_component`; clean in-process shutdown `false`
- `wc0-release-nonzero` -> `WC0_COMPONENT_RELEASE_BLOCKED`; classification `scanner_blocked`; in flight `none`; clean in-process shutdown `false`
- `wc0-release-hang` -> `WC0_COMPONENT_RELEASE_BLOCKED`; classification `call_timeout`; in flight `release_component`; clean in-process shutdown `false`
- `wc0-release-crash` -> `WC0_COMPONENT_RELEASE_BLOCKED`; classification `abnormal_termination_in_flight`; in flight `release_component`; clean in-process shutdown `false`
- `wc0-host-object-request` -> `WC0_HOST_CONTEXT_BLOCKED`; classification `scanner_blocked`; in flight `none`; clean in-process shutdown `true`
- `wc0-host-reference-leak` -> `WC0_HOST_CONTEXT_BLOCKED`; classification `scanner_blocked`; in flight `none`; clean in-process shutdown `false`

The nonzero-release fixture entered `component_retirement_incomplete` after one
release. Nonzero release, unresolved host reference, and release/initialize/
terminate timeout or crash paths could not reach inherited factory release,
`ExitDll`, or `FreeLibrary`. Unexpected host-object creation failed closed.
