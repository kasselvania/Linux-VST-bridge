# RPI1 transfer baseline

RPI1 starts from the accepted RPI0 head and the already-integrated Steam Deck
Arturia/Pigments lineage. It does not recreate a generic Wine configuration.

Validate the public transfer contract before acquiring or executing any private
runtime, installer or plug-in bytes:

```sh
python3 rpi1/validate_transfer.py
python3 -m unittest rpi1/test_validate_transfer.py
```

The validator checks the exact revision-18 Pigments profile, Deck Proton runner
identity, accepted ASC installer/application identities, process-local UI
Automation policy, and RPI0 Box64/executable custody. It intentionally does not
read or validate private account, authorization, preset, content or binary
payloads.

Physical execution is governed by `docs/experiments/RPI1.md`.
