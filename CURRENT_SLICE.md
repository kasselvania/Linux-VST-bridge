# FRG1 retired-predecessor startup reconciliation

Base: canonical main merge `243ad5b557f476e852f23af35511bb81fe5014dc`,
tree `47bcb35709ccbdd97d8bd02e2c019e569526b804`.

## Primary claim

Service startup accepts the exact normally removed revision-11 FRG1 record as
retained history while refusing a live or altered predecessor. It does not
require that historical record to have the parent reserved for the uncreated
revision-12 candidate.

## Basis and scope

- `docs/ARCHITECTURE.md` revisioned publication and service reconciliation.
- `docs/FRG1.md` sealed revision-11 to revision-12 Ubuntu transition.
- Ubuntu-lab PR #5 physical install of merged PR #146: setup succeeded, but
  the new service exited at startup with `frg1_predecessor_absent` before
  staging, publication, or Bitwig. The exact service was stopped after its
  first observed restart loop; its failure is retained.
- `bridge-manager/src/frg1.rs` exact retired-predecessor verification and
  qualification restoration invoked by `Manager::reconcile()`.

No scanner, Wine/Proton runner, Windows host, module, native proxy, audio,
editor, protocol, profile, or Ubuntu sandbox topology changes are in scope.

## Acceptance and nonclaims

- The production reconciliation path succeeds with the exact removed
  revision-11 predecessor and leaves its registry identity and publication
  state unchanged.
- The same path refuses changed adoption authority. Revision-12 staging and
  normal restoration retain their existing exact-parent behavior.
- Manager library/binary tests, strict Clippy, and diff checks pass.
- Source success is not a physical plug-in result. The installed Ubuntu
  generation remains stopped pending a new immutable build/install.

## Physical continuation

Preserve the failed generation and retained vendor, inventory, adoption, and
revision bytes. After normal merge, update only the Ubuntu product lock, build
and install one immutable corrected generation, then stage/publish revision 12
and ask the operator for one Bitwig FRAGMENTS load. Stop at the first new
physical failure or finish the already-authorized qualification ladder.
