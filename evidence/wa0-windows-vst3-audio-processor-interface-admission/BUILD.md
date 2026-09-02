# WA0 Windows build and custody

Private workflow
`.github/workflows/wf0-windows-msvc-build.yml` at Git blob `940487013fb7eedbc43ba9050a462e64d07b0e59` ran as exact
run `33689659595`, attempt
`2`, on `windows-2022` for this source.

The supported toolchain was Visual Studio `17.14.37614.0`
with MSVC `19.44.35228`, v143, x64, Windows SDK
`10.0.19041.0`, and CMake `3.31.6`.
The official VST3 SDK was `3cdf9ca5d1f5b1b21e0a86832aa4abe55607bd96` at tree `38343890fd1a0cedd48b7ec80ef17da15231b6c8`
with all seven locked recursive submodules clean and no source patch.

Two distinct clean build roots compared `35` complete
transfer paths at level `byte_identical`. PE/export/import/dependency
verification passed. Scanner SHA-256: `6ba5dab82d03cc2f736adc5c5d65b585b5673bdf0871ce445af8b59ba0a06122`. Exact AGain module
SHA-256: `60aa9ff6b9918d4330449e7b3ab34b588dd93cba09f37413a3cd91f6e7d2e18f`.

The exact Actions artifact is ID `9869994854`, name `wa0-windows-build-24b7e6da7e29a5bd358097a6b89c5c59b747c413-run-33689659595-attempt-2`,
upload-action bare digest `81f3d431a0cb00c4cc121799818ce5ac1b8487dfde583d03eb48b98adda7bf3b`, REST
digest `sha256:81f3d431a0cb00c4cc121799818ce5ac1b8487dfde583d03eb48b98adda7bf3b`, and raw wrapper SHA-256
`81f3d431a0cb00c4cc121799818ce5ac1b8487dfde583d03eb48b98adda7bf3b`. The three values name the same bytes under
their typed representations. This is byte custody, not signing, provenance,
SLSA, or release suitability.
