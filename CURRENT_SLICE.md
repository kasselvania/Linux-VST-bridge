# FRG1 registered native proxy correction

Base: canonical main merge commit `407a37679cbe9ccc3bed86d1b33c9ed49728c7da`, tree `ce3aa3679cb0512a49a52e17b35e9445578cbebd`.

## Primary claim

The first Ubuntu Bitwig load failed because the revision-11 native proxy was built without the Rust backend's `registered` feature. An exact revision-12 successor pins a registered x86-64 Linux proxy and can be published only after the unchanged revision-11 publication is normally restored. The prior adoption, inventory and immutable revision remain intact.

## Basis and scope

- `docs/ARCHITECTURE.md` revisioned publication and managed runtime boundaries.
- `docs/FRG1.md` exact module/host/runner/descriptor authority.
- Ubuntu-lab PR #5 first Bitwig activation failure and retained failed-publication evidence.
- Canonical FRG1 profile/package identity and manager transition; source-owned transition tests.
- Pinned native source commit `3f3ae6235a85a8d5e311b6275b2d7555a088eb92`, descriptor SHA `a1fd81c9f91c0371b633e9b4c7597e020c60b97da0494e9ce11becb5c68eb9f5`, and VST3 SDK commit `3cdf9ca5d1f5b1b21e0a86832aa4abe55607bd96`.

The Windows host, scanner, runner, module, native DSP protocol, Ubuntu namespace topology, and vendor prefix do not change.

## Acceptance

- The new ELF is x86-64 Linux, stripped, and SHA-256 `f29e4cf0d3157308a78097b25f10a05264277291203c77a62db6cc1a2cfa4c1a`; its build enables `registered`, contains the managed runtime path and excludes the legacy AP9 performance path.
- Revision 11 stays byte-identical history. Revision 12 changes only revision and native proxy digest, and binds one exact package manifest.
- A live or foreign predecessor refuses. A normally removed exact revision 11 permits revision-12 staging and publication with an immutable parent, retained adoption and inventory, and normal successor restore.
- Manager library/binary and focused backend tests pass; `git diff --check` passes. Static checks prove source/artifact contract only. No corrected-proxy Bitwig, editor, audio, state, or restart pass is claimed.

## Physical continuation

Keep Ubuntu PR #5 and the current manager/publication intact until this product PR is reviewed and merged. After merge, restore revision 11 through the currently installed product, verify clean retirement, update Ubuntu's exact product/artifact locks, install one new immutable generation, and stage/publish revision 12. Continue the Bitwig ladder once. Preserve the first new failure. Do not contact the Steam Deck, run ASC, rescan, replace the healthy inventory, modify vendor custody, or claim physical success from deterministic proof.
