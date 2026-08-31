# LabHostProbe

`LabHostProbe` is an original deterministic HP0 test instrument: a no-editor Linux x86_64 VST3 stereo gain effect. It is not the bridge proxy.

Displayed name: `LAB Host Probe`

Vendor: `Kasselvania Research`
Version: `0.1.0`

Fixed identities:

| Identity | Value |
|---|---|
| Processor class | `6F4E7A5392E54B54A98AD6F714E0C201` |
| Edit-controller class | `B9C42F0736C34E218E5A71D40C8F1B62` |
| Normalized gain parameter | `0x4C485001` |
| Bypass parameter | `0x4C485002` |

The component state is a fixed little-endian record: `LHP0` magic, schema version `1`, IEEE-754 gain, and a checked integer bypass value. Controller state uses distinct `LHC0` magic with the same version and values.

The processor owns no dynamic buffer. Its callback handles in-place and separate buffers for 32-bit and 64-bit samples, processes parameter changes without allocation, treats missing/inactive buses as bounded silence/no-op conditions, and performs no logging, filesystem, network, process, thread, or synchronization work.

SDK helpers and public examples were consulted as API references. No SDK sample implementation is copied into this fixture.
