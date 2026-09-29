# PKG1 — immutable native preparation-kit selection

## Source claim

PKG1 starts from post-MIDI0 canonical main
`0385faaef3251e5c1036741b4e440ec5b66e5133`, tree
`87f3e9c5ccc446c85db77d17a1d774c748431fa0`. It closes one package
ownership gap: PKG0 carried a native proxy file but retained the previous
`Software.preparation_kit`, so a new manager could still build old proxy code.

A schema-2 package adopts one fixed `preparation-kit.zip` together with its
paired manager/frontend and declared host/source pair. The kit is included in
the signed package roster and the exact root-owned adoption manifest. The
builder and package verifier check the bounded recipe, required native compile
inputs, exact SDK pins, contained file digests and package host/source
identities. The verifier binds every selected adoption artifact to the signed
release roster. Release assembly and release-key signing additionally compare
the kit's complete source roster and bytes with a clean exact Git head/tree;
synthetic package fixtures do not prove a real kit compiles. The ordinary-user
intake reads
the fixed `/usr` kit through the same checked descriptor rule as the other
root-owned package inputs. It copies the exact bytes into the immutable user
software generation and selects that artifact in `software.json`.

The generation record binds the kit SHA-256. A same-generation reinstall
verifies the retained kit without creating a self-predecessor. A new generation
retains the complete previous `Software` record, including its earlier kit, for
exact rollback. Package adoption, recovery and rollback keep their existing
service-inactive, ownership, transaction, cleanup and route gates. The native
catalogue, publication registry, installer and UI2 histories, workspaces,
projects, preferences and vendor authorization are not rewritten.

Historical schema-1 six-file packages and generation records remain readable
with their prior kit-retention law. The schema-2 seven-file roster is required
to select a kit; a schema-1 package cannot smuggle one into adoption. The
release archive format itself remains schema 1; its adoption manifest is
schema 2 when the kit is present.

## Boundaries

The preparation kit contains project-owned native build inputs and the exact
registered backend archive and host/source pair. It is not a proprietary
plug-in, installer, preset, account state or license payload. The external
Proton/SLR prerequisite remains external. This slice neither chooses a
customer signing key nor establishes clean-machine package support.

Selecting the kit does not replace a published native proxy. UI2 must still
build an exact new candidate from the selected kit, expose it experimentally,
and retain the physical musical and retirement result before any publication
or Push note-release claim changes. The existing Pure LoFi publication remains
selected on the Deck during this source-only work.

## Verification

Source-owned package tests exercise schema-2 archive assembly and signing
boundary verification, source/host/kit mismatch refusal, missing compile
inputs, SDK-pin drift, source-tree drift, full adoption-roster mismatches,
schema-1 history,
selected-kit identity, same-generation no-op, changed input and retained-byte
refusal, interrupted switch recovery, populated legacy kit/catalogue rollback,
and preservation of unrelated records. A real source-produced kit must also
pass a pinned-SDK native build smoke before customer release signing. The Linux
root-owned intake fixture
adds an actual root-owned kit, changed-kit and writable-kit cases. Exact test
counts and hosted check links belong to the PR at its reviewed head.
