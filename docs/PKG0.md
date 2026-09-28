# PKG0 — canonical package authority

## Claim and basis

PKG0 starts from public main `1120efa349f84250fc9602503c28ea2fbd6fb8b9`,
tree `1956a0071f626ec7fa0808369b618520549d1825`. It applies the installed
software, exact identity, rollback, licensing and evidence laws in `AGENTS.md`,
`GOVERNANCE.md`, and `docs/ARCHITECTURE.md`. The private DIST0 source supplied
a reviewed fixed-path package builder and a first immutable binding proof. It
explicitly refused an update with a populated catalogue. This public slice adds
that transition and interruption recovery; those parts require their own review.

## Package and user state

The package builder accepts one declared x86-64 artifact roster and paired
manager/frontend build head, tree and operator schema. It refuses a mismatched
build identity, undeclared file, changed digest, source-tree escape, secret-like
bytes, or bundled Proton/SLR runtime. It produces an exact adoption manifest and
release roster. The package verifier compares every archive entry, size, mode
and SHA-256 with that roster. Signing uses a supplied isolated GPG key. An
independent verifier requires a **separately supplied trusted key and expected
fingerprint**; a public key included in the bundle is never self-authorizing.
An internal test key is not a beta release key.

The user-owned adoption entry point reads only fixed `/usr` package locations.
It verifies the complete adoption roster, source identity, operator schema,
root-owned regular-file posture and exact bytes. It copies the manager,
frontend, supervisor helpers and Windows host/source pair into one immutable
`software/<generation>/` directory. The package does not write product state in
the home directory by itself. Package adoption selects that generation through
the existing user command links, desktop entry, user service unit and
`software.json`. The switch is retained before any route changes. After an
interruption, `package-recover` either confirms the fully applied switch or
restores the exact predecessor; a foreign edit refuses and leaves the journal.

The current native catalogue is copied byte-for-byte into the successor
generation. Its native proxies, environment runners and supplemental host pairs
remain exact references to retained immutable artifacts. With a populated
catalogue or registry, a changed default Windows host/source pair refuses; that
requires a separate reviewed migration. Product registry, publications,
installers, environments, candidates, observations, workspaces, projects,
preferences and authorization are never recreated by PKG0. One predecessor is
retained in the generation record for exact rollback. Older generations remain
available rather than being garbage-collected in this slice.

The package records the exact required external Proton/SLR runtime identity and
manifest digest. It ships no Proton/Wine/SLR files and makes no runtime support
claim merely because the package is assembled. The manager still verifies each
selected environment runner through its existing immutable runner records.
First-run readiness and acquisition of the prerequisite belong to PB0/PB1.

## Operations and boundaries

`package-adopt`, `package-rollback`, and `package-recover` are fixed closed
commands. They require the user service stopped and reject active or unresolved
DSP ownership, pending transactions, stale transports and active FL workspace
operations. They take no caller path, executable, PID, runner or target
generation argument. An idempotent reinstall with the same manifest retains the
selected generation. Package-manager removal removes `/usr` files only;
user-owned software and data remain. Reinstalling the same verified package can
adopt the retained generation. A routine package update never auto-selects a
new generation, and rollback never silently chooses an arbitrary older one.

## Validation and nonclaims

Source fixtures cover fresh adoption, exact pairing, nonempty catalogue update,
owner/transaction/cleanup refusal, interruption, rollback, reinstall, changed
source and foreign generation refusal, and preservation of unrelated user
records. Package tests cover roster, build pairing, external-runtime exclusion,
tampering, and signature verification where GPG is available. No Deck package
is installed by PKG0. Before a physical replacement, PB0-R must reconcile and
prove **every** installed manager-owned record remains readable under the
canonical source, and the selected package generation needs independent review.
Clean-machine installation, graphical first-run, audio, plug-in support,
release-signing key selection and runtime redistribution remain outside PKG0.
