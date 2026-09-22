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

## ARM standalone binding

`rpi1/standalone` is the isolated Pigments appliance executable. It reuses the
accepted RPI0 real-time audio, MIDI, shared-memory, editor and state boundary,
but owns a separate configuration and supervisor for the demonstrated SLR4 ->
Proton -> Box64 launch. RPI0 source and its direct-Wine fixture remain unchanged.

On the accepted Pi, generate a private configuration from the already verified
environment with `rpi1/prepare_standalone.py`, then build natively:

```sh
cargo build --release --manifest-path rpi1/standalone/Cargo.toml --features jack-runtime
```

The generated configuration pins the exact entry points, emulator adapter,
manifests, host, Pigments module and X11 authority. It also preserves the
demonstrated `PYTHONHOME=/usr` requirement and Pigments compatibility policies.
It contains fixture-local paths and must not be committed.

The retained Pi transfer/preflight result is in
`evidence/rpi1/runtime-transfer-preflight.json`. Its core process checks are
functional, with its SLR setup faults retained.

The subsequent normal/override comparison is retained in
`evidence/rpi1/account-free-uia-preflight.json`. It passed and completes Phase
A with the setup faults preserved; ASC and Pigments remain unrun.

The exact accepted ASC 2.12.0.3157 installer admission and private Pi transfer
are retained in `evidence/rpi1/asc-installer-admission.json`. Admission did not
execute the installer.

Those are historical checkpoints. The subsequent Pi Pigments census passed on
2026-09-22 UTC using the existing bounded supervisor. The earlier exit 82 came
from malformed shell-expanded Windows readiness/gate paths; both regular-file
and pipe stdout reach readiness with correct arguments. See
`docs/experiments/RPI1_CENSUS_RESULT.md` and
`evidence/rpi1/pigments-census-diagnosis.json` for the exact comparison, successful
census, cleanup, preserved faults and next standalone-host boundary.

`box64-emulator-adapter.c` is the narrow AArch64 executable required by the
Steam Runtime emulator-manifest interface. It preserves ordinary ELF targets,
translates the runtime's explicit `ld-linux-x86-64.so.2 --library-path ...`
vector into Box64's library-path environment, and recognizes only the exact
Proton Python shebang. Unknown loader options and malformed vectors refuse.
The adapter does not replace Proton, Wine, pressure-vessel or the bridge.

## Bounded headroom comparison

The standalone command `master` requests the existing controller refresh and
reports only the exact Pigments `Master Volume` / `dB` parameter (ID 0).
`master NORMALIZED` accepts a finite value from 0 to 1, queues it through the
existing audio parameter path, updates the controller, and requests readback.
The normalized value is not a dB value. A queued record alone is not proof of
applied gain; use controller readback and measured audio. Save the initial
private state and restore it after a comparison. No editor needs to open.

The existing JACK fixture accepts `polyphony`: five six-second windows with
1, 2, 4, 6 and 8 simultaneous notes at velocity 96, held for three seconds,
followed by note-offs and CC123. It emits 47 messages on channel 1. The input
controller should remain disconnected from the test's JACK MIDI port.

All fixture modes optionally accept `--capture NEW_PRIVATE_F32LE_FILE`.
The recording buffer is allocated and its writable pages touched before
activation. The callback only copies into that fixed buffer; file writing
starts after JACK retirement. The destination must be new and is created with
mode 0600. Output is stereo interleaved little-endian float32 at 48 kHz, retaining
samples above full scale. A complete polyphony capture is 30 seconds / 11,520,000
bytes. Keep recordings private; publish aggregate evidence. Neither captured
nonzero samples nor a successful fixture exit establishes artifact-free audio.

## Current physical playing level

The operator selected Master Volume ID 0 at normalized `0.45` after the exact
1/2/4/6/8-note comparison. Pigments returns the float32-rounded
`0.44999998807907104`. Apply and verify that value before routing MIDI/audio in
the next supervised session. The selected state and playing-preference record
remain private on the Pi. This is a retained physical setup choice, not automatic
boot configuration or qualification of every preset. See
[`RPI1_POLYPHONY_LEVELS.md`](../docs/experiments/RPI1_POLYPHONY_LEVELS.md).
