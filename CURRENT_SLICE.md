# Current slice — recover failed work through the product (#468)

Goal: a failed admission or inspection must not leave every setup unusable.
Reconcile remains reachable, and proven retirement restores ordinary actions
without an external terminal or service restart. Architecture and beta scope
remain in `docs/ARCHITECTURE.md`.

Reference comparison:
- Test210 exposed a failed update with an unresolved verification lease.
- Test214–216 exposed a process-wide sticky cleanup flag after failed inspection
  or the manager's own vendor-operation pause. Restart followed by Reconcile
  recovered those cases; the product must provide that recovery itself.
- #465 recovery and #469 named beta migration are complete; #470 and #471 landed.
  Preserve their working installations, changed state and mixed-use projects.

Next changes, in small reviewed increments:
1. Separate operation failure from uncertain process custody. Retain uncertain
   ownership; clear the block when exact retirement is proved. Keep Reconcile
   reachable and keep maintenance scoped to affected setups.
2. Give full runtime verification a useful lifetime tied to exact manifest and
   stable file identity, explicit invalidation and cold-verification progress.
   Do not broadly skip launch integrity checks or extend the callback exception.

Working completion:
- Reproduce a failed admission/inspection, then recover from the ordinary UI.
- No restart or manual ownership deletion is required after proven retirement.
- Unresolved live custody still refuses conflicting execution.
- Unrelated setups keep working; affected setup retries successfully.
- Update/recovery and mixed-use state recall pass on the resulting package.

Serum transparency #330, live overview refusal #382, no-view editor #472 and
older Bitwig project baseline #473 remain separate. Fresh r7 NI installation,
fresh SteamOS packaging and supervisor source delivery remain release work.
No manual managed-state cleanup. Preserve licensed setups, activation, original
projects and predecessor runtimes used by setups or rollback. No synthetic Deck
key events; mouse and captures are allowed. On homelab, confirm Storage2 is
mounted before writing build/test files below /mnt/Storage2/LinuxPluginBridge.
