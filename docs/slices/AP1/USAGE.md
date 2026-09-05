# AP1 build and verification notes

The normal `tools/proof-run.py` registry contains the AP1 diagnostic and acceptance plans when both committed artifact bindings exist. Use a clean checkout of the receipt's exact executable source and the separately updated authority checkout. The default CURRENT_SLICE command guard stays closed for unspecified work. Do not rerun a successful session to regenerate evidence: the retained result is the review artifact.

```sh
python3 -B tools/proof-run.py diagnose \
  --authority /path/to/authority/docs/campaigns/AP1_D1.md \
  --source 28ea95d04db5c1ef93f14d3d3fa74dfd9eb4d219 \
  --plan ap1-linux-windows-audio-diagnostic-v1 \
  --campaign 78c32c41080c2afb3f8bf85967474dfe49016eb84c76f62ac0bb272118fbd755 \
  --preflight-only
```

Only an authorized new batch omits `--preflight-only`. Acceptance uses `accept`, AP1_A1.md, plan `ap1-linux-windows-audio-acceptance-v1` and its `--candidate` identity. Existing closed reservations reconcile without relaunch. A new source or artifact requires the explicit receipt revision and preserved cumulative budget described by the active work order.

The existing host-only Windows workflow builds the AP1 branch's 22 exact Windows inputs in two MSVC roots; it reuses the retained AGain binary. The native crate has no third-party Rust dependencies. `tools/ap1_build_client.py --source <full-commit> --output <private-client-store>` uses the installed rustup stable compiler, its matching musl target and bundled rust-lld to produce a static x86-64 ELF. It records compiler/linker, source-input and binary identities, and writes AP1_CLIENT.json. Do not rebuild unchanged inputs merely because documentation changes.

Deliver the closed host/native trees through the existing exact artifact transfer. That transfer installs files read-only: the new native caller alone needs owner execute permission (0500), after verifying its manifest, binary digest, non-symlink status and ownership. Never run a binary just to check deployment. The normal AP1 preflight verifies its identity and execute permission, existing host/fixture/runtime identity, absence of prior caller processes and current safe state. The supervisor launches both endpoints inside one marker-owned disposable session and owns their cleanup.

Local focused checks:

```sh
python3 -B -m unittest tools/test_ap1_client.py tools/test_ap1_execution.py
RUSTC="$(rustup which --toolchain stable rustc)" rustup run stable cargo test \
  --manifest-path native-audio-client/Cargo.toml --locked --offline
c++ -std=c++20 -Wall -Wextra -Werror -I windows-factory-probe/source \
  tools/ap1-tests/codec.cpp -o /tmp/ap1-codec-tests
/tmp/ap1-codec-tests "$(cat tools/ap1-tests/golden.txt)"
```

The native integration tests use local loopback sockets and mapped files; they launch no plug-in. Relevant existing AP0/PC0/runtime/policy/backend suites remain in the same CI job. Full actual words are retained before comparison can reject a block, and the worker saves bounded independent checkpoints outside the stage before normalization or retirement. Missing/partial output never becomes acceptance evidence.
