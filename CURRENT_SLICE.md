# Current task: buffering and delivery controls with the frontend open

## Goal

Make ordinary buffering and delivery changes complete while the frontend remains
open. A read-only refresh must not become a false "operation already running"
refusal. Preserve exact target, preference and affected-owner mutation checks.

## Best explanation and next change

Pulse and current readback take the registry guard without action serialization.
Buffering/delivery admission releases its registry guard; the preference owner
then takes that guard fail-fast. A harmless overlapping read can therefore refuse
the action after admission. Wait within the existing non-audio operation bound
at that final acquisition, then recheck identities and owners before one write.
Keep standalone controls and real-time admission fail-fast; replay no action.

## Done

- A synchronized concurrency regression reaches both ordinary action owners,
  reproduces the old refusal and proves completion after readback releases.
- Sustained contention, stale target/preference and active affected owners refuse
  without changing the requested preference.
- Independent review, green checks and a normal installed update; the frontend
  stays open through 512→1024→512, with completed receipts and exact readback.

## Retained capability and following work

Test28 companion Stop/retry, sibling rescan/publication, changed-state/automation
recall and settings Try/Apply/Keep/Restore pass; protected-reference PCM is exact.
Fresh-read refusals remain a separate open defect. Two audio deadline misses,
slow cold start/update and older-manager usability remain open.
Vendor-state rollback is not promised.
Test23 BEAM handoff is operator-reported smooth at 512/48k, SameCallback D=0/L=0.
The shared workflow still comes first; beta/platform/audio acceptance and later
work remain in docs/AUDIO_RECOVERY_ROADMAP.md and docs/INTEGRATED_BETA_DELIVERY.md.
