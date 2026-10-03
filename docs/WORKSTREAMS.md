# Product work allocation

Updated by the operator's 2026-10-03 compatibility-platform direction. This file
coordinates separately authorized work. It is neither a compatibility certificate
nor authority to start every listed lane. `../CURRENT_SLICE.md` names this branch's
current outcome; `SUPPORT_MATRIX.md` and `FAILURE_CLASSES.md` retain support and
observed failure scope. Previous allocations remain in Git history.

## Shared platform, distinct execution lanes

| Lane | Product relationship | Engineering boundary |
| --- | --- | --- |
| Native Linux DAW bridge | Primary compatibility and beta-delivery goal | Shared machine/runtime assessment, unfamiliar plug-in preparation, audio/editor/state contracts, manager operations and recovery. Fixtures and existing implementations do not limit the design. |
| Managed Windows DAW | Separately authorized workspace capability | A Windows DAW loads Windows plug-ins directly within its workspace. Reuse platform management where its contract applies; do not insert a Linux proxy or import its latency/count claims. See [workspace architecture](WINDOWS_DAW_WORKSPACES.md). |
| ARM/manufacturer appliance | Separately authorized appliance work | Share validated abstractions where appropriate. Architecture, device and audio evidence remains specific; native Linux or Windows-workspace results do not qualify an appliance. |

The platform design accounts for shared contracts across lanes. Implementation
ownership and mutable application environments stay explicit. One lane's scope
must not silently redefine another's completion, delay it behind unrelated work,
or create a second manager, compatibility database or lifecycle authority.

## Planning and integration

Select work through the operator's current goal and the capability method in
[AGENTS.md](../AGENTS.md). Keep patches reviewable while completing the necessary
connections across owners. No standing allocation orders another Serum-only run,
Gaming Mode transition, frontend redesign, FL/Ableton bring-up or catalogue campaign.
Existing separately authorized tasks continue within their own scope and custody.

A shared-platform change must consider its affected consumers, installed runtime
policies, host/proxy pairs, state and recovery behavior. Reuse sound mechanisms;
revise wrong abstractions rather than treating current code as architecture.
Unmerged dependencies must be explicit. Review actual changes and relevant evidence
before integration, without treating one lane's success as universal support.

## Machine and repository custody

- Work in the canonical repository with one branch/worktree per coherent outcome.
  Do not change another owner's checkout or experiment.
- Each lane owns its mutable environments, sessions and evidence. Reuse immutable
  runners through verified references and coordinate shared-package changes.
- Serialize shared GUI/audio work, installation, service replacement and runtime
  transitions with the current machine custodian. Preserve user projects and
  unrelated installed publications.
- Respect declared CPU, memory, process and spending limits. Build concurrency
  does not override the user's reserved capacity or a physical test's conditions.
- Use the agreed handoff or current task record to transfer custody; do not invent
  a scheduling service or another mandatory permission/receipt framework.

Historical [WD0](WD0.md), [WD1](WD1.md) and other fixture reports retain their original
results and limitations. Consult current machine readback for installed state;
historical commit names and old allocation text are not current deployment facts.
