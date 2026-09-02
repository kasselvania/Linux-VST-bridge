# WF0 negative and adapter proofs

All
`13` deterministic checks passed with zero
failures. The loader adapter executed through the accepted Runtime/Proton lane,
enumerated all 15 closed operations, and proved paired
`free_library` false-result attribution. The source scanner contains no
class-instantiation call expression.

The complete approved 22-module live family produced:

- `wf0-missing-factory` -> `WF0_FACTORY_GET_BLOCKED`; classification `scanner_blocked`; final in-flight `none`
- `wf0-null-factory` -> `WF0_FACTORY_GET_BLOCKED`; classification `scanner_blocked`; final in-flight `none`
- `wf0-no-entry` -> `bounded_success_path`; classification `scanner_completed`; final in-flight `none`
- `wf0-factory1-only` -> `bounded_success_path`; classification `scanner_completed`; final in-flight `none`
- `wf0-factory2-only` -> `bounded_success_path`; classification `scanner_completed`; final in-flight `none`
- `wf0-factory3-fallback` -> `bounded_success_path`; classification `scanner_completed`; final in-flight `none`
- `wf0-init-false` -> `WF0_MODULE_ENTRY_BLOCKED`; classification `scanner_blocked`; final in-flight `none`
- `wf0-factory-info-false` -> `WF0_FACTORY_INFO_BLOCKED`; classification `scanner_blocked`; final in-flight `none`
- `wf0-count-negative` -> `WF0_CLASS_ENUMERATION_BLOCKED`; classification `scanner_blocked`; final in-flight `none`
- `wf0-count-excessive` -> `WF0_CLASS_ENUMERATION_BLOCKED`; classification `scanner_blocked`; final in-flight `none`
- `wf0-class-info-false` -> `WF0_CLASS_ENUMERATION_BLOCKED`; classification `scanner_blocked`; final in-flight `none`
- `wf0-duplicate-class-id` -> `WF0_CLASS_ENUMERATION_BLOCKED`; classification `scanner_blocked`; final in-flight `none`
- `wf0-hang-entry` -> `WF0_MODULE_ENTRY_BLOCKED`; classification `call_timeout`; final in-flight `init_dll`
- `wf0-hang-factory` -> `WF0_FACTORY_GET_BLOCKED`; classification `call_timeout`; final in-flight `get_plugin_factory`
- `wf0-hang-class` -> `WF0_CLASS_ENUMERATION_BLOCKED`; classification `call_timeout`; final in-flight `get_class_info_unicode`
- `wf0-crash-entry` -> `WF0_MODULE_ENTRY_BLOCKED`; classification `abnormal_termination_in_flight`; final in-flight `init_dll`
- `wf0-crash-factory` -> `WF0_FACTORY_GET_BLOCKED`; classification `abnormal_termination_in_flight`; final in-flight `get_plugin_factory`
- `wf0-crash-class` -> `WF0_CLASS_ENUMERATION_BLOCKED`; classification `abnormal_termination_in_flight`; final in-flight `get_class_info_unicode`
- `wf0-hang-release` -> `WF0_FACTORY_RELEASE_BLOCKED`; classification `call_timeout`; final in-flight `release_factory_3`
- `wf0-crash-release` -> `WF0_FACTORY_RELEASE_BLOCKED`; classification `abnormal_termination_in_flight`; final in-flight `release_factory_3`
- `wf0-exit-false` -> `WF0_MODULE_EXIT_BLOCKED`; classification `scanner_blocked`; final in-flight `none`
- `wf0-create-instance-tripwire` -> `bounded_success_path`; classification `scanner_completed`; final in-flight `none`

Required-export absence never resolved or called `InitDll`; optional
entry/exit absence remained an observation; crash and timeout attribution used
the final unmatched call attempt; successful fixture paths released acquired
factory interfaces in reverse order; exit-false still attempted unload; and the
create-instance tripwire remained absent.
