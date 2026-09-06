# Current work: AP6 — Recover one failed plug-in instance

## Accepted baseline

**AP5 is accepted and merged.** PR #61 was reviewed at `24cae167c4b3e61fc59f07fa51ec6cce9e638996` (passing rereview 5125675629) and merged with operator agreement as `6694f6e410cb1ee7395879b570ae817b7ceed38f`; #60 is complete. Two real Windows AGain instances saved/recalled independently in one native Bitwig host, with 61,495,808 compared samples at zero error. Removing one left the other running. R1's reporting/cleanup repair passed focused checks without replaying the desktop run. [AP5 results](docs/AP5_RESULT.md) retain their original provenance. **The earlier isolated transport loss remains unexplained**, not fixed by the later success or R1. AP4/D9/A2 records remain unchanged.

## Outcome and authorization

Recover a failed Windows instance from its last confirmed complete state without restarting Bitwig, re-entering settings or interrupting its healthy sibling. Issue #62; branch `codex/ap6-instance-recovery`. The operator approved this direction. Necessary builds, routine repairs, targeted fault injection into our disposable Windows instance and focused desktop checks belong to this implementation task. Prior tasks' consumed transactions remain history; no per-source receipt, duplicate campaign or GUI countdown is required. Normal tool approvals and spending safeguards in AGENTS.md remain applicable.

## Behavior that matters

Reuse the per-instance backend, state path, owner and Windows launcher. Provide one explicit per-instance recovery action, through a suitable host/SDK lifecycle path or a minimal bridge control. Do not crash/restart the shared native host to obtain a DAW reload button. The engineer chooses the small control surface and private implementation, not a new broker framework.

Keep a confirmed complete component snapshot outside the failing transport. Capture it from successful real state get/set operations, with instance/plug-in identity and capture provenance. Do not assume every Bitwig Save produces a new getState call. Recovery restores that identified snapshot, not necessarily the latest edit. Expose that limitation; a missing or invalid snapshot refuses recovery rather than using defaults. Do not overwrite the project or present an older snapshot as a fresh successful save. Preserve the last good snapshot when a later capture fails.

Recovery storage and coordination treat state as opaque bytes; no gain-only reconstruction or new dependence on AGain's 12-byte layout. Keep reference decoding and exact-byte oracles separate from recovery. Retain existing state-format compatibility and controller synchronization; restored controls, host parameter values and actual Windows state must agree before normal operation resumes. The current host still has reference-specific identity/validation: AP6 is not general commercial compatibility, nor a requirement to implement every vendor interface now.

Retain the first useful fault before teardown, including the already-bounded native I/O explanation. Quiesce and contain only the failed endpoint before replacement; use a fresh connection/generation and discard old queued work. No automatic retry loop or replay of prior audio. Failed/uncertain restore stays failed with explicit status. Blocking startup/state work stays outside audio callbacks; the failed instance emits the documented failure output while healthy siblings continue. Keep 1024-sample latency and restart alignment truthful. This restores state, not lost audio buffers or tails.

## Enough evidence and delivery

Investigate the retained loss with targeted first-fault evidence and fix demonstrated causes. An unrepeatable historical loss stays unresolved; do not turn the task into indefinite soak testing. Controlled recovery does not explain that earlier event.

Use focused local checks for snapshot isolation, missing/mismatched state, failed restore and sibling survival. Reuse a non-gain state case to expose a gain-only shortcut. In the existing two-track project, launch Bitwig from Applications, confirm distinct captured states, deliberately end only one owned Windows endpoint, recover it and verify actual restored output without re-entering values while the other instance keeps running. Check that saving/reopening after recovery remains sound. No arbitrary sample count or compulsory second run.

Reuse SSH/Moonlight and prepared runtime/artifacts. Preserve user projects and restore temporary setup. Keep float32 stereo, 48 kHz and up-to-256-frame callbacks. No commercial/editor/instrument, installer, reboot-persistence or latency-tuning work. Publish one PR referencing #62, with useful results, remaining limitations and cleanup; leave it unmerged for review.

Targeted sources: `native-vst3-proxy/source/processor.cpp`, `source/factory.cpp`, `backend/src/queued.rs`, `backend/src/state.rs`, `backend/src/instances.rs`; `tools/ap4_preview.py`; [architecture](docs/ARCHITECTURE.md) §§5.8–5.9 and 6.5; Steinberg [persistence](https://steinbergmedia.github.io/vst3_dev_portal/pages/FAQ/Persistence.html) and [hosting](https://steinbergmedia.github.io/vst3_dev_portal/pages/FAQ/Hosting.html). A host may decline `kReloadComponent`; observe the chosen control path rather than assume support.
