# WA0 focused negative proofs

All
`23` deterministic owner checks passed. The
exact eight WA0 fault fixtures ran live; the WF0 and WC0 negative families and
standalone adapter runtime exercise were intentionally not replayed.

- `wa0-query-failure-null` -> `WA0_INTERFACE_QUERY_BLOCKED`; classification `scanner_blocked`; in flight `none`; clean in-process shutdown `true`
- `wa0-query-success-null` -> `WA0_INTERFACE_QUERY_INCONSISTENT`; classification `scanner_blocked`; in flight `none`; clean in-process shutdown `true`
- `wa0-query-failure-nonnull` -> `WA0_INTERFACE_QUERY_INCONSISTENT`; classification `scanner_blocked`; in flight `none`; clean in-process shutdown `true`
- `wa0-query-hang` -> `WA0_INTERFACE_QUERY_BLOCKED`; classification `call_timeout`; in flight `query_audio_processor`; clean in-process shutdown `false`
- `wa0-query-crash` -> `WA0_INTERFACE_QUERY_BLOCKED`; classification `abnormal_termination_in_flight`; in flight `query_audio_processor`; clean in-process shutdown `false`
- `wa0-release-unexpected-count` -> `WA0_INTERFACE_RELEASE_BLOCKED`; classification `scanner_blocked`; in flight `none`; clean in-process shutdown `false`
- `wa0-release-hang` -> `WA0_INTERFACE_RELEASE_BLOCKED`; classification `call_timeout`; in flight `release_audio_processor`; clean in-process shutdown `false`
- `wa0-release-crash` -> `WA0_INTERFACE_RELEASE_BLOCKED`; classification `abnormal_termination_in_flight`; in flight `release_audio_processor`; clean in-process shutdown `false`

The failure/null and success/null query cases proved no lease and permitted
ordinary WC0 teardown. Failure/non-null retained and released the anomalous
reference once while preserving the query inconsistency as primary. Unexpected
release count 2 and every unmatched query/release operation prevented later
WC0 terminate, component/factory release, `ExitDll`, and `FreeLibrary`.
