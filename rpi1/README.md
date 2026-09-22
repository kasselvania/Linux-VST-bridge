# RPI1 transfer baseline

RPI1 starts from the accepted RPI0 head and the already-integrated Steam Deck
Arturia/Pigments lineage. It does not recreate a generic Wine configuration.

Validate the public transfer contract before acquiring or executing any private
runtime, installer or plug-in bytes:

```sh
python3 rpi1/validate_transfer.py
python3 -m unittest rpi1.test_validate_transfer rpi1.test_box64_emulator_adapter
```

The validator checks the exact revision-18 Pigments profile, Deck Proton runner
identity, accepted ASC installer/application identities, process-local UI
Automation policy, and RPI0 Box64/executable custody. It intentionally does not
read or validate private account, authorization, preset, content or binary
payloads.

Physical execution is governed by `docs/experiments/RPI1.md`.

The retained Pi transfer/preflight result is in
`evidence/rpi1/runtime-transfer-preflight.json`. Its core process checks are
functional, with its SLR setup faults retained.

The subsequent normal/override comparison is retained in
`evidence/rpi1/account-free-uia-preflight.json`. It passed and completes Phase
A with the setup faults preserved; ASC and Pigments remain unrun.

`box64-emulator-adapter.c` is the narrow AArch64 executable required by the
Steam Runtime emulator-manifest interface. It preserves ordinary ELF targets,
translates the runtime's explicit `ld-linux-x86-64.so.2 --library-path ...`
vector into Box64's library-path environment, and recognizes only the exact
Proton Python shebang. Unknown loader options and malformed vectors refuse.
The adapter does not replace Proton, Wine, pressure-vessel or the bridge.
