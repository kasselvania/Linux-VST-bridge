# PKG0 — canonical package authority

## Claim and basis

PKG0 starts from public main `1120efa349f84250fc9602503c28ea2fbd6fb8b9`,
tree `1956a0071f626ec7fa0808369b618520549d1825`. It applies the installed
software, exact identity, rollback, licensing and evidence laws in `AGENTS.md`,
`GOVERNANCE.md`, and `docs/ARCHITECTURE.md`. The private DIST0 source supplied
a reviewed fixed-path package builder and a first immutable binding proof. It
explicitly refused an update with a populated catalogue. This public slice adds
that transition and interruption recovery; those parts require their own review.
The focused package-authority repair is source commit
`153a58fd66073c66e84b8c04f9c05f173ae6ea02`, tree
`f96355d50a6dec193d2b78199956efbbe8c93b8a`. The PR body records the
final documentation head and tree.

## Package and user state

The package builder accepts one declared x86-64 artifact roster and paired
manager/frontend build head, tree and operator schema. It refuses a mismatched
build identity, undeclared file, changed digest, source-tree escape, secret-like
bytes, or bundled Proton/SLR runtime. It produces an exact adoption manifest and
release roster. The adoption/generation identity includes the package name,
ordinary version and package release alongside source head/tree. The builder emits no `.INSTALL`
hook. Before signing, the package verifier requires one exact `.PKGINFO` with
the declared name, version/release, x86-64 architecture and dependency set; it
refuses `.INSTALL`, duplicate or unexpected metadata, and changed package
files. `.BUILDINFO` and `.MTREE` have bounded regular-file/count rules.
Signing uses a supplied isolated GPG key. An
independent verifier requires a **separately supplied trusted key and expected
fingerprint**; a public key included in the bundle is never self-authorizing.
An internal test key is not a beta release key.

The user-owned adoption entry point reads only fixed `/usr` package locations.
It opens the root-owned manifest and artifacts with `O_NOFOLLOW`, verifies owner,
regular-file, non-group/other-writable and extent posture, reads and hashes each
through the same descriptor, and checks stable identity/extent. It parses the
manifest from those checked bytes. It verifies the complete adoption roster,
source identity, operator schema and exact bytes. It copies the manager,
frontend, supervisor helpers and Windows host/source pair into one immutable
`software/<generation>/` directory. The package does not write product state in
the home directory by itself. Package adoption selects that generation through
the existing user command links, desktop entry, user service unit and
`software.json`. The switch is retained before any route changes. After an
interruption, `package-recover` either confirms the fully applied switch or
restores the exact predecessor; a foreign edit refuses and leaves the journal.
Re-adopting the selected exact manifest checks and repairs missing or stale
package-owned command, desktop, callback and unit routes without creating a
self-predecessor or rewriting an unchanged software selection. Foreign routes
still refuse. The copied native catalogue remains byte-exact and non-writable.

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
commands. They accept an absent first-adoption user unit or an inactive/failed
loaded unit, and refuse active, transitional, unavailable or ambiguous service
readback. They reject active or unresolved
DSP ownership, pending transactions, stale transports and active FL workspace
operations. They take no caller path, executable, PID, runner or target
generation argument. An idempotent reinstall with the same manifest retains the
selected generation. Package-manager removal removes `/usr` files only;
user-owned software and data remain. Reinstalling the same verified package can
adopt the retained generation. A routine package update never auto-selects a
new generation, and rollback never silently chooses an arbitrary older one.
After adoption, rollback or recovery reaches an exact disk route, the manager
runs a bounded `systemctl --user daemon-reload` and checks the effective unit
fragment and `ExecStart` against the selected immutable manager. The transition
journal is removed only after that readback succeeds. A failed reload/readback
leaves the exact transition recoverable. PKG0 does not enable or start the unit.

## Validation and nonclaims

Source fixtures cover fresh adoption, exact pairing, nonempty catalogue update,
owner/transaction/cleanup refusal, interruption, rollback, reinstall, changed
source and foreign generation refusal, same-generation route repair, service
state/reload recovery, non-writable retained catalogue, and preservation of
unrelated user records. A hosted Linux fixture creates an actually root-owned
package tree and executes intake as the ordinary runner user, including owner,
symlink, write-mode, byte and extent refusals. All six cases passed on the
hosted Linux manager job at repair predecessor `607daea`; final-head status is
linked from PR #184. Package tests cover an Arch
archive with exact `.PKGINFO`, optional metadata, no install hook, roster, build
pairing, external-runtime exclusion, tampering, and signature verification where
GPG is available. The exact hosted result is recorded in PR #184. No Deck package
is installed by PKG0. Before a physical replacement, PB0-R must reconcile and
prove **every** installed manager-owned record remains readable under the
canonical source, and the selected package generation needs independent review.
Clean-machine installation, graphical first-run, audio, plug-in support,
release-signing key selection and runtime redistribution remain outside PKG0.
