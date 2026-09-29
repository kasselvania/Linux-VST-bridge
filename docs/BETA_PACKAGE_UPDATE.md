# Ordinary installed-package update handoff

## Selected boundary

Canonical base: `13ed1d85e830d581ec297760e04e9f9433bfd671`,
tree `f281a2e518298f466b2486f3ca7c01a80e84aa28`.
The fixture is a disposable CachyOS 260809 graphical system with one
selected internal-test package generation and a newer exact Arch package
installed under the package manager's root-owned `/usr` intake. The
[retained failure](https://github.com/kasselvania/Linux-VST-bridge/pull/197)
showed that the package updated successfully but the Applications launcher
opened the older selected frontend without offering adoption.

The claim here is narrow: the fixed system frontend can offer and complete an
explicit switch from a verified installed package to a new immutable user
generation while preserving the selected generation as rollback predecessor.
Package installation alone still does not select user software, stop a
service, start a service, or change a publication.

The package installs a distinct **Linux VST Bridge Setup and Updates** desktop
entry. Its desktop ID differs from the user-owned **Linux Audio Compatibility
Manager** entry, so the latter cannot shadow the updater in the Applications
menu. The Library entry continues to open the selected version, including
after rollback; setup and updates remain explicit.

## Authority

`package-bootstrap-status` verifies the fixed installed manifest and its
complete declared artifact roster. It compares that exact manifest digest
with the selected generation record, then runs the existing no-write
predecessor plan when the package differs. A changed artifact, foreign route,
incompatible host/source pair, unreadable selected generation or pending
package transition refuses. A changed package is called an installed package
change; the package manager owns whether that change is an upgrade or
downgrade.

For an active selected service, the ordinary frontend offers a deliberate
stop. The existing package owner checks idle DSP, installer, vendor and
workspace state; pending transactions; cleanup certainty; and exact service
identity before stopping. A keeper whose retirement is not confirmed leaves
the update waiting. Once stopped cleanly, `package-adopt` rechecks the
root-owned package and creates an immutable successor using the exact
selected predecessor. The existing journal owns interruption recovery and
route switch. `package-activate` then reads back the exact service route and
starts the selected user service. The system frontend opens the selected
frontend only after the selected generation is active.

The source tests cover active and stopped package changes, changed package
artifacts, clean-stop refusal, keeper-retirement delay, exact predecessor
retention, selected-route readback and independent software rollback. The
graphical fixture must additionally prove the ordinary Applications journey;
a CLI-only adoption does not close the original failure.

## Limits

This owner does not package Proton/SLR or the VST3 SDK, install on the
protected-base Steam Deck, create a native proxy publication, qualify a
commercial plug-in, or make a customer release. Package rollback and native
publication rollback remain distinct actions. A verified package change
does not by itself qualify audio, graphics or project recall.
