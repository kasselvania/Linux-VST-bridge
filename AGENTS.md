# Working on Linux VST Bridge

## Goal

Build a usable Windows plug-in bridge for native Linux music applications. The user should not have to administer Wine prefixes, proxies, runtime versions or recovery machinery to make music. Steam Deck, Bitwig and AGain are current fixtures, not the limits of the product.

## Work to an outcome

Read this file and `CURRENT_SLICE.md`, then the architecture/code relevant to the requested change. Do not reread the entire historical corpus or repeat slice selection when the user has selected the goal.

An approved implementation task includes ordinary code changes, necessary builds, focused tests, debugging and verification within that task. The engineer chooses private types, file layout and algorithms. A routine repair does not need a new design, exact-source approval document, maintenance campaign or separate PR. Record what actually ran; do not confuse source identification with permission to iterate.

The operator's current instruction takes precedence over repository process documents. These rules replace the older mandatory selection/receipt/diagnostic/acceptance sequence. `docs/campaigns/`, `docs/maintenance/`, `docs/process/` and previous slice instructions retain historical context, not standing requirements for new work. Platform/tool approvals and security restrictions are separate and remain in force; never route around a denial.

## Keep the engineering safeguards

- Use the actual Windows plug-in, not substitute DSP or fabricated state. Respect SDK interfaces, object lifetime, thread affinity and explicit protocol boundaries. Keep Rust primary and C++ limited to the SDK edges.
- Keep blocking I/O, allocation, logging and process work out of real-time audio callbacks. Preserve truthful latency and explicit failure behavior.
- Protect existing projects, installations and credentials. Use disposable test projects/environments where practical. Restore settings changed for a test. Clean up only processes and files the task owns; investigate uncertain ownership before starting another instance.
- Retain useful failure details before cleanup. Private local diagnostics may contain the paths/process identifiers needed for debugging; do not publish them or credentials, license data, proprietary binaries or presets.
- Never fabricate a pass, silently change the tested fixture, erase failed attempts or relabel old evidence. A green build, a fake-peer test and an actual DAW test establish different things.

## Verification and review

Test the behavior changed and the failures that matter. Reuse working binaries, fixtures, supervision and observations. Do not replay an entire historical matrix because a report, timeout or document changed. Preserve valid sub-results and rerun only affected parts unless a genuine end-to-end uncertainty requires more.

A meaningful successful development test on the delivered implementation may support review without being repeated under a different label. Keep its original provenance and limitations. Acceptance is the reviewer's decision about the code and evidence, not a requirement to reserve a special class of run in advance. Recorded results from the older system remain exactly as recorded.

Use normal build/test commands and the existing supervised helpers. `tools/proof-run.py` is an optional legacy transaction interface, not a mandatory gateway to every local test, Deck interaction, GUI action or merge. Do not weaken its historical checks, reset its ledgers or disguise a new launch as recovery. Existing explicit resource limits still apply until the operator changes them; there is no universal two-tries rule for new tasks.

Ask before changing the goal, making destructive changes to unrelated/user-owned data, installing paid software, exceeding an explicit spending limit or changing a security boundary. Ordinary compile errors, GUI pacing and in-scope harness repairs belong to the engineer. If a test repeatedly teaches nothing new, diagnose it rather than blindly rerunning it.

Publish one PR with what works, how it was checked, relevant versions, remaining limitations and cleanup. Review before merge; include the current-status update in the same PR. No separate audit/closure artifact is required by default. Write additional design only for a consequential unresolved decision, not to memorialize every implementation choice.

## Concurrent work

Do not change another agent's branch, checkout or running experiment. The active AP4 agent continues the operator's direct save/reopen goal. This cleanup does not require it to stop, rebuild, restart its tests or adopt new orchestration. Reconcile process-document conflicts when its PR is integrated, preserving its product code and original observations.

Reuse established Moonlight/Sunshine and SSH under their existing permissions. No streamed terminal or alternate tool route to bypass Computer Use restrictions. Remote access is development tooling, not a plug-in runtime dependency.
