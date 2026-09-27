# PB0 source preview and bounded Deck readback

**Claim level:** source-owned fixtures and local UI captures; the real Deck
assessment and support export are **not established**.

Canonical source base: `65f113b463ed1dbba7d142f1b833720575932a0d`
(`bf364f138849f8744ca18319380858545c0a1cf4`). The intermediate staged
x86-64 PB0 manager binary had SHA-256
`eac13a5229da6226072ec3cde3f974ae607765ac078dd45a7510ccdda6c3f285`.
It was copied only to `/tmp`, run for one read-only `operator snapshot` request,
and removed. No installed executable or service was replaced.

Source implementation continued after this refused readback; the staged
digest is not the eventual PR head's build identity. The Deck's installed
manager SHA-256 before and after was
`414a953b8e7b1186ae1bf3ce969f09be8c5a011b12dcbcfb83f1c1637449ad71`.
Its current operator snapshot was schema 11. The staged PB0 source refused the
managed catalogue while reading it:

```text
unknown field `onboarding_runtime`, expected one of `schema`, `natives`,
`environments`, `hosts`
```

That field belongs to a newer installed authority than the exact canonical
source base. The refusal is a version boundary, not evidence that the Arturia
product is unready. No catalogue field was stripped, ignored or rewritten to
force a PB0 result. Consequently there is no real PB0 outcome, no physical UI
card, and no real PB0 support export in this receipt. Reattempt only after
normal integration of the installed authority into canonical source.

## Preserved physical state

The installed manager's readback before and after the staged refusal reported:

- Steam Deck Desktop Mode, x86-64, SteamOS 3.8.16, Bitwig 6.1 Flatpak;
- service active, DSP 0/6, maintenance 0, pending transactions 0, stale
  transports 0, cleanup uncertainty false;
- ordinary selected Pigments, Pure LoFi and Efx FRAGMENTS among the six
  established native publications;
- FL workspace still in its prior `ready` state.

The six established publication records were compared before and after using
exact class ID, module digest, environment, runner and publication identity.
The sorted allowlisted comparison was byte-identical; both SHA-256 values were
`8ea17b75499440b540011c09a421a4368433d90fd9695ef3468cd1f8b7c98881`.
The staged `/tmp` binary was absent at final readback. This comparison does not
qualify the other products or exercise their audio paths.

## Source-owned captures

The eight `readiness_*` Home PNGs render ready, action-required, unsupported
and unknown fixtures at 960 and 560 logical pixels. The narrow Setup PNG shows
the local export button and technical Details surface in the dark theme. All
captures are synthetic and execute no manager action. They are layout proof,
not physical Deck readiness proof.
