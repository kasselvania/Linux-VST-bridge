# FRG1 verified scanner relocation

Base: canonical main merge `55d692f982eb0d03d7656264ecf4f06f5d8293cb`,
tree `bc5c370164fa344fb6476aa59c2f2620a1881f80`.

## Primary claim

An exact healthy inventory remains current when immutable Ubuntu setup moves
the scanner host to a new pathname without changing its verified bytes.
Missing or changed host bytes still refuse publication.

## Basis and fixture

- `docs/ARCHITECTURE.md` inventory identity and revisioned publication.
- `docs/FRG1.md` sealed revision-12 Ubuntu qualification.
- Ubuntu-lab PR #5 generation
  `ec2fb7d3ebfba771aa596bcbe20dd6609de3114b57b27834c6633a9b37cfde8e`:
  service active, exact healthy inventory retained, revision 12 staged, but
  publication refused `frg1_current_inventory_required` before mutation.
- The scanner host SHA-256 and source-manifest identity are unchanged; only
  the software-generation pathname differs from the retained scan.

Only `bridge-manager/src/frg1.rs` inventory-current authority and regression,
plus slice/result documentation, are in scope. No scanner, Wine/Proton runner,
Windows host, module, native proxy, audio, editor, protocol, profile, or Ubuntu
sandbox topology change is in scope.

## Acceptance and nonclaims

- Both retained and current host artifacts verify, their SHA-256 identities
  match, and existing source/environment/module checks remain required.
- Byte-identical relocation succeeds; altered bytes refuse.
- Manager library/binary tests, strict Clippy, and diff checks pass.
- Source success is not a physical publication or plug-in result.

## Physical continuation

After normal merge, update the Ubuntu product lock and wrapper stamp, build and
install one immutable generation, verify retained inventory/adoption and idle
ownership, then stage/publish the already built revision 12. Do not rescan.
If publication passes, continue the bounded Bitwig load/editor/audio/state/
retirement ladder and restore. Stop at the first new physical failure.
