# Deck six-product readback — 2026-09-23

This is a read-only manager/registry/inventory/physical-link reconciliation, not
a six-instance pass. Canonical source was `b0ba8164da0e568309387ee152f3c4ba37910767`
(tree `8d510364df7a7ac93ef55dc818cd6ba8fa562219`); installed generation was
`6abc850bb4e3991c5842591a72d1f03fd0105ee27f2a9a401d97b3a3ce8fedf3`.
The manager was active with one retained keeper, zero DSP/maintenance/transactions/
stale transports, and `cleanup_unconfirmed=false`. Blackhole and Kontakt's recent
operator-confirmed opens were accepted without replay.

Every row below had `Published` registry state and a physical `~/.vst3/LVB_<class>.vst3`
link to the listed exact revision. No link, inventory, prefix, runner, module,
proxy, or publication was changed by this readback.

| Product / VST3 class | Role and support posture | Environment / runner | Inventory scan / selected revision | Native proxy SHA-256 | Host SHA-256 / source manifest SHA-256 | Module SHA-256 |
| --- | --- | --- | --- | --- | --- | --- |
| Pigments `41727475415649534B61743150726F63` | Instrument; `ordinary_verified`, manager `ready` | `1de8d28b-d98b-4a41-8462-c07cca8e1609` / `proton-11.0-2c-25118279-slr4-4.0.20260805.254769` | `820f8b5163841beaef59173797b8c886` / `23cbf3bdbe4cf6d4ee3d2056103557ea` | `56987399359d05b4b113f1818c48e6c57142cc8bf073dae255606d1e663ecc38` | `186ba6d18f231a438041f3b8322259b73622a5f982835d7a48202efb2585e2f8` / `aab9ed5088e7f2a6a52665fb67150ce3fbe1a52893f7eedb68a63c55c06da498` | `bdc91ebef8e5b486c8f998f1eef6a99626dd5a0b46d986263eeb1980b96a3c07` |
| Pure LoFi `417274754156495350724C4650726F63` | **Instrument**, `needs_attention` (`installed_host_mismatch`) | `1de8d28b-d98b-4a41-8462-c07cca8e1609` / same Arturia runner | `820f8b5163841beaef59173797b8c886` / `51f241c4da508aa1fbbc9d6d172eb123` | `ce7531e4c76e53e85abe1153354fac890a6ba641b3b6333f5891a25f49843d59` | `3ee36fd34384b2c282ec9cb6ae6c48d17f83940eaaf6d8f9a1fa130b65274939` / `6ca742eaaa2547eeea866bcd2e8121c9ee400c57d0394c752e3ac2a9794432b5` | `b3e8ca7477487d0dc7fbd3f8815e2f9aec7c8b701f4cb65cf96433c14fa11636` |
| Efx FRAGMENTS `41727475415649536772616E50726F63` | Effect; `needs_attention` (`installed_host_mismatch`) | same Arturia environment / runner | `820f8b5163841beaef59173797b8c886` / `e340c6f8ad5ca494d2fb477214c9937e` | `11615d88cb253e22ca6f6beecbe946bdb5efd46882d091b65f9c461cd6757428` | same exact Pure LoFi host / source | `d7ed0361558d6275e6ea3ce217bd0fe69cb32e9881d967899ab659b0d367fa26` |
| Serum 2 `56534558667350736572756D20320000` | Instrument; `engineering_selected`, candidate B `f6af02eba109d3632b2ecc786f2c9006ec6bc1d433772001d24d2969fb944b44` | `4db060b14388e41103834fc4dfdd023a` / Arturia-version Proton runner | `993ea0d2864f7d8bbbadddd4ec7f5a34` (historical scanner selection) / `6105dee8ac8aa27855fc611abb794e7c` | `afd9bdac62b0f63a0e2353f97aa11048945823c35cc2abbea936ee6d255bfd77` | `348a4bbc6ea34f57fc5899c279d9e43999bf9ecae4563a6cfee88967d36f66be` / `0ce0ecae2264bab24290712d0e3d2ed9e8c092ce6cb7a521fd0fac71ca010ac3` | `501e7bb3dd9cafe416b3412df3d4e084c01b7468201e5690b9009d7ecd4e5283` |
| Blackhole Immersive `5653544248496D626C61636B686F6C65` | Effect; `engineering_selected`, candidate `80e06590b7dc3f4e15d2903237541bdb466c012c8dfb647f6d0843daefb68b7d` | `8b064373d533b72230cb4009952eaab4` / `proton-11.0-2c-dcomp-c27f058-reference` | `56b87f7aaf8f8ea523de9eee3bc8eb7b` / `103c94edceb97c1b376d1eb757523aea` | `aa1df1690c5e7e5808aa27d3358216c25831214284e90bd24b9065664f0ff361` | `5fa0907b045df5ccb993a135fccffe932b365182bec7fa16c1a48a126b1869c2` / `268208f352ca21d7b6f4017802db3bfbb7d7081a59358eb72fd84b261b5a123b` | `b8a33c57b04eded38d0ca717bb5e3721330439f4931c401ea70e5246b516e90b` |
| Kontakt 8 `5653544E694B386B6F6E74616B742038` | Instrument; `engineering_selected`, candidate `2185e031fee4552232839ef305cceec7760c44271b0b1e2886a992150692949e` | `627d2cba97edbecf113c22504eb4c81b` / `proton-11.0-2c-25118279-slr4-4.0.20260805.254769-ni-msi-ed4a47d3b167` | `f36cd5c114b9017d6d23a8ec3e2b2113` / `81fcd5f28dea4b47514655e6bbd6da9a` | `2727eabc92c462f23fb0ed03e259358a84298de7ea41a56a93470d6bf595501e` | same exact Blackhole host / source | `b4a3d79b194adfc307e5741059c05bdbd7d8e3cbfed4ca7a8e8e8cf755d06563` |

The Arturia environment projects `Refresh installed products`; Serum's current
candidate actions say its inventory/scanner selection is historical. No scan was
run: the immediate refusal for Pure LoFi and FRAGMENTS was not missing module
bytes or a corrupt link. Their retained host and source-manifest bytes still
match the exact profile requirements and remain private, nonsymlink, read-only
files under an immutable software generation. The **current software catalogue**
omits that host pair after the default host changed. Manager projection reports
`host_valid=false` for only these two; the other four have valid host/module/
native readback. A new scan or proxy build would not fill that catalogue gap.

Manager-projected primary actions were rollback/capture for the ordinary Arturia
products, observe/review/disable for selected engineering candidates, and
environment refresh where shown above. None was invoked. The operator confirmed
that Pure LoFi is an instrument and approved a four-track six-instance layout:
Serum 2→Blackhole, Pigments→Efx FRAGMENTS, Kontakt alone, Pure LoFi alone.
The individual smokes and mixed-six project remain **not run** until the host
catalogue refusal is repaired and installed through normal immutable setup.
