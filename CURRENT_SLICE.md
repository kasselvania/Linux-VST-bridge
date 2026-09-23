# FRG1 removed-predecessor setup transition

Base: canonical main merge `684518ae580d497cd4539cdc6d9b64d1a23f96db`, tree
`626c5ce04ba035cac21715eecdaefa5224696b02`.

## Primary claim

An immutable product setup over the exact normally removed FRG1 revision-11
registry retains its catalogue-free managed authority instead of attempting
ordinary installed-product catalogue adoption. The subsequent revision-12
publication still requires the original exact predecessor and remains a
nonactivating Ubuntu review candidate.

## Basis and scope

- `docs/ARCHITECTURE.md` managed catalogue and revisioned publication boundaries.
- `docs/FRG1.md` sealed Ubuntu FRG1 adoption, inventory, and candidate route.
- Ubuntu-lab PR #5 first revision-12 install attempt, which stopped in setup
  with `adoption_requires_existing_managed_artifact` before publication.
- `bridge-manager/src/main.rs` setup package path,
  `bridge-manager/src/catalogue.rs` ordinary adoption, and the existing
  `frg1::catalogue_free_registry` exact authority check.

No scanner, Wine/Proton runner, Windows host, module, native proxy, audio,
editor, protocol, profile, or Ubuntu sandbox topology changes in this slice.

## Acceptance and nonclaims

- Empty and exact sealed FRG1 registry states yield no ordinary catalogue.
  A live, altered, or foreign FRG1 predecessor refuses; ordinary managed
  catalogue adoption retains its existing behavior.
- The existing revision-12 transition test covers the removed exact
  predecessor and changed-adoption refusal through the production setup
  authority helper. Manager library/binary tests and strict Clippy pass.
- This is source qualification only. The failed Ubuntu install is retained;
  no second setup, revision-12 publication, Bitwig load, editor, audio, state,
  restart, or restoration result is claimed here.

## Physical continuation

First recover the failed Ubuntu generation to the exact previously installed
CPI2 generation without rewriting the removed FRG1 history, healthy inventory,
adoption, vendor prefix, or module. Then build/install this merged product as
one new immutable generation and stage/publish the already pinned revision-12
package. Only after verified publication ask the operator to load FRAGMENTS
once in Bitwig. Preserve and stop at the first new physical failure.
