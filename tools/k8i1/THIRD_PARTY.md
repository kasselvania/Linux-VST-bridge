# K8I1 third-party basis

The forwarding-shim architecture was independently implemented after review of
`selimbucher/native-instruments` at
`139b8bf3dd4a4f0fad3dd22d5fe3af9e7d423e12` (MIT), including its earlier shim
implementation at `b353ed9a738a32aba42f24607e4f150f6bfa4db8`. The upstream
MIT license is retained in `LICENSE.ni-wine-MIT`.

The pinned export names and ordinals are derived from Wine's
`dlls/msi/msi.spec` at Wine revision
`dc26e61847081a1b5cb0733dc30feba6ee575482`, whose exact source-file SHA-256 is
`5a6085ce66f541d3c52552164ade070129ea05a0f8a8b1ed2c660132366aceb0`.
Wine is LGPL-2.1-or-later. A distributed `msi_lvb_real.dll` must retain the
corresponding source, build instructions, notices, and the ordered upstream
patches:

- `24bbf46c6f055077df428341595873fe23475a60`
- `e0130972d5ffe578a4a25d75f4c1d0229c880a8c`

No Native Instruments package, payload, account, activation, or licensing data
is included in this repository.
