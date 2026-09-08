# Current position: AP10 complete; ready to select the next outcome

AP8 (#67), AP9 (#69) and AP10 (#71) are accepted for integration into `main`. AP10's reviewed implementation is `fb0faed7cfc097ecb78653421d6e704bb9f25a00`; its payload-lifetime R1 passed technical rereview 5136977791. This integration changes documentation, not the reviewed production code or retained test evidence.

There is no active successor implementation task. Do not resume the old AP10 handoff or rerun its completed tests merely because a historical report says unmerged/incomplete. Selecting and preparing the next outcome is permitted under the normal [AGENTS.md](AGENTS.md) workflow; no additional permission-document chain is required.

## Baseline to preserve

Real Serum instrument and FRAGMENTS stereo-effect processing, controls and project recall; shared-memory delivery; supported bidirectional VST3 results with callback-lifetime payload ownership; counted aligned audio gaps and genuine-failure recovery. The [AP10 report](docs/AP10.md) and its linked evidence state the actual coverage. Earlier tests retain their own sources and limitations.

512 added frames at 48 kHz stays the desktop recommendation. A 241-microsecond observed mean non-plug-in service is not fixed audio latency or a worst-case guarantee. No vendor-editor, simultaneous-commercial, broad routing, reboot or hardware round-trip claim is added.

## Next selection inputs

The next user-facing candidate is a practical vendor editor for the same processing instance: open/close, real edits, host synchronization and recall, without putting UI work in the audio delivery path. Product-derived names belong there. This is a candidate, not an implementation order or a requirement to build the complete manager.

Keep latency and reliability concrete: [#72](https://github.com/kasselvania/Linux-VST-bridge/issues/72) tracks residual stalls/lower-latency suitability; [#73](https://github.com/kasselvania/Linux-VST-bridge/issues/73) tracks the historical engine-close crash; [#74](https://github.com/kasselvania/Linux-VST-bridge/issues/74) tracks prompt notification of known endpoint failure. Do not make every backlog item a prerequisite to independent useful work, or equate later success with an explanation of old failures.

New implementation starts from current `main` on a fresh task branch. Preserve dirty local work and retained vendor/project environments; no hard reset, installation change or live device test is part of this integration cleanup. The [completed AP10 work order](https://github.com/kasselvania/Linux-VST-bridge/blob/5be898e18760cebefe309abdd7fe33049392391f/CURRENT_SLICE.md) remains available at its original source.
