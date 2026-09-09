# AP14 exact fixture evidence

These are allowlisted, sanitized observations from 2026-09-09 for PR #85. They are evidence, not runtime policy. `$MANAGED` denotes the existing bridge-owned data root, `$HOME` the operator's home, `$RUNNER` the pinned Proton location and `$RUNNER_CONTAINER` its pinned SLR location. No vendor modules, installer, preset/state payload, account/license data, audio capture or prefix export is retained here. File/revision hashes were read from the original local bytes; these path-normalized display copies are not those byte-identical hashed files.

- `installed-identities.json`: initial installed registration/software/environment and final canonical status plus physical link/target/revision/native/module readback. Full exact profile/census/transaction identities are retained; runner critical file digests are public software identity, not an environment export.
- `transactions.json`: repeated managed publication, incompatible-runner refusal, fixed candidate-ready interruption, reconcile, exact rollback and restored current publication. The late rollback admission records identify the genuinely retained older LoFi host separately from the current sibling/keeper host.
- `editor-failure.json`: original and stopped-transport failures, independently retained fault stages, exact bounded X11 symbol stack and old-keeper comparison. The first unsuccessful debugger attachment is explicitly not causal evidence.
- `musical-smoke.json`: observed native/vendor control recall, existing automation replay, speaker-monitor metrics, instance identity brackets, full gap/expired/priming counts, positive close/cleanup and independent removal. State records contain only operation/size metadata, never opaque bytes. The late unknown CPUWeight override and restoration are kept distinct.
- `manager-tests.txt`, `runtime-tests.json`, `validation.json`: actual focused automated results, CI runs, durable failure boundaries and earlier failed attempts.

## Commands and authority

The ordinary operator used the installed command, after product setup had exited:

```sh
linux-vst-bridge managed environments
linux-vst-bridge managed preview
linux-vst-bridge managed publish
linux-vst-bridge managed status
linux-vst-bridge managed rollback 1 18f3da27dda66a44ed19677d4921af8c
linux-vst-bridge managed publish
```

Diagnostic checks used `managed check-candidate $TASK/incompatible-profile.json` (retained profile with only the runner ID deliberately mismatched; cannot activate arbitrary input), `managed fault-check 1`, and ordinary `managed reconcile`. No caller-authored class/hash/registration/compatibility/native-path or Wine command was needed for managed publication. The JSON outputs in this directory are readback, not hand-authored registration inputs.

Software maintenance cross-built the manager using Rust 1.90.0 with the existing Zig wrapper targeting `x86_64-linux-gnu.2.35`, downloaded the repository's exact Windows CI artifact, assembled the normal bridge software package and ran its existing `setup` command. Setup adopted only the already owned and profile-pinned AP13 native images into the immutable product catalogue. It did not rebuild/replace vendor files or expose a build-tree dependency in the ordinary managed operation. An execute-bit omission in temporary package transfer was corrected before successful final setup.

```sh
cargo test --manifest-path bridge-manager/Cargo.toml --locked
cargo clippy --manifest-path bridge-manager/Cargo.toml --locked -- -D warnings
python3 -m unittest discover -s bridge-manager/runtime -v
# Equivalent Linux process tests on the Deck's temporary copy of those sources:
python3 -m unittest discover -s "$TASK" -p test_session.py -v
```

CI URLs are `https://github.com/kasselvania/Linux-VST-bridge/actions/runs/RUN_ID`, with IDs and exact tested implementation head in `validation.json`. The deployed Windows artifact is from run 34383224230 at PR merge source `83c5cd34b04267451c259839c567a0006a25ddac` (branch source `f867159b6f29a7cc481103ef4cf7a8790f151297`), not falsely attributed to the later manager/evidence head. Its source manifest and pinned SDK lock hash are retained. The later Windows test lane on implementation head `1236e377ab3f8035f5e918ae193465d6e7ff82a1` also passed.

The one debugger probe attached only to the failing LoFi host, loaded X11 symbols, set `break XUnmapWindow if $rsi == 0x1400001` and `break _XDefaultError`, and used `set print frame-arguments none`. Wine's SIGUSR1/SIGUSR2 and handled SIGSEGV were passed through. It detached after two bounded symbol backtraces (6.189 seconds); no memory dump or vendor-state inspection. Temporary debugger scripts/units and raw local speaker captures were removed; numerical metrics and bounded fault visibility remain.

## Claim limits

The complete candidate was tested with the current exact operator artifacts, Bitwig 6.1 and pinned runtime. Normal Applications launches, explicit save/reopen and sibling removal are direct GUI observations; process counters and audio monitor metrics substantiate different parts of the result. The historical AP13 recorded automation evidence is reused, not relabeled as a new gesture qualification. The remaining short preparation/publication gaps are retained; the main repaired-session totals are not a uniform benchmark. The late CPUWeight change has unknown provenance and excludes that interval from performance comparisons. No AP14 CPU/dropout improvement, 256 qualification, general plug-in support, licensing guarantee or full-reboot result is claimed. 512 remains supported/recommended and installed.
