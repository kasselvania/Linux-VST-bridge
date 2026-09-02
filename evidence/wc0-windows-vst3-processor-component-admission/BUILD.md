# WC0 Windows build and custody

Private workflow
`.github/workflows/wf0-windows-msvc-build.yml` at Git blob `8e456d24ff4af1130cb3ed4e5cabca0e6715b724` ran as exact
run `33666394555`, attempt
`1`, on `windows-2022` for this source.

The supported toolchain was Visual Studio `17.14.37614.0`
with MSVC `19.44.35228`, v143, x64, Windows SDK
`10.0.19041.0`, and CMake `3.31.6`.
The official VST3 SDK was `3cdf9ca5d1f5b1b21e0a86832aa4abe55607bd96` at tree `38343890fd1a0cedd48b7ec80ef17da15231b6c8`
with all seven locked recursive submodules clean and no source patch.

Two distinct clean build roots compared `43` complete
transfer paths at level `byte_identical`. PE/export/import/dependency
verification passed. Scanner SHA-256: `51b899b7936921b24265ead9ff12180f14249d49b532ead9a18414559f5f83f7`. Exact AGain module
SHA-256: `60aa9ff6b9918d4330449e7b3ab34b588dd93cba09f37413a3cd91f6e7d2e18f`.

The exact Actions artifact is ID `9861033341`, name `wc0-windows-build-9c0096930df86fc5b171cdebec40b306a198316a-run-33666394555-attempt-1`,
upload-action bare digest `db23a2a1781e9eb88dc43fbe9599cb74ff14f7c4f67a2884ca83113a274ff642`, REST
digest `sha256:db23a2a1781e9eb88dc43fbe9599cb74ff14f7c4f67a2884ca83113a274ff642`, and raw wrapper SHA-256
`db23a2a1781e9eb88dc43fbe9599cb74ff14f7c4f67a2884ca83113a274ff642`. The three values name the same bytes under
their typed representations. This is byte custody, not signing, provenance,
SLSA, or release suitability.
