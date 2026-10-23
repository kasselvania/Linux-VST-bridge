# Bridge native window

Bridge owns the application. `ui/` assembles the native window, draws Bridge
nodes and delivers input through a persistent worker; `presentation/` composes
Bridge screens; `live.rs` adapts shared owner readbacks and exact requests.
`preview.rs` supplies development samples only. The service, `lvb-model`,
`lvb-view-model` and `lvb-client` retain product truth and request custody.
Generic controls, platform integration and input stay in private Makepad.

This is the #504 structure migration of original Bridge code from
`apps/bridge-pilot` at Makepad commit
`8025a1f9e3702aada20f83beee00e79ab17c09e4`. The supported native application
source is now this directory. No toolkit source is published here. The retained
`manager-ui` egui frontend stays available until schema 30, all six journeys and
installed replacement qualification are complete.

Both Makepad dependencies and its three registry patches pin that exact commit.
The separate workspace/lockfile lets ordinary service, client and model builds
run without private-repository access. The production executable keeps its
installed name and selected-generation checks.

```sh
cargo build --manifest-path crates/window/Cargo.toml --locked --release \
  --no-default-features --features live -j2
cargo test --manifest-path crates/window/Cargo.toml --locked --release \
  --all-features --lib -j2
cargo build --manifest-path crates/window/Cargo.toml --locked --release \
  --no-default-features --features samples --bin bridge-window-preview -j2
python3 crates/window/tests/remote_ui.py
python3 crates/window/tests/console_layout.py
python3 crates/window/tests/live_unavailable.py
```

The native tests launch only owned hidden windows with
`MAKEPAD_HIDE_WINDOWS=1 --remote` and close them normally with `/gq`. Logs and
captures go outside Git. Live startup outside an installed generation refuses
service requests and offers retry; it never falls back to sample records.
Production builds use only `live`. `--package-info` exits before native startup
and refuses unless the build helper supplied an embedded declaration.

Use [the frontend build helper](../../tools/beta/README.md) for installed
qualification. It always builds this application against the recorded toolkit.
It does not build the old private pilot. Package selection, public trust roots,
bootstrap and update handoff remain the shared client's responsibilities.

## Private CI access

The `Bridge native window` workflow requires one repository Actions secret:
`MAKEPAD_READONLY_DEPLOY_KEY`. The operator must register the corresponding
public SSH key as a **read-only deploy key on `kasselvania/makepad`**, with write
access unchecked, and put its private half in that secret on
`kasselvania/Linux-VST-bridge`. Do not use a personal token or commit either key.
No key is created by this change, and private CI cannot pass until it is wired.

The workflow reads the pinned revision from this manifest and checks out it
alongside Bridge using the deploy key. Checkout removes its authentication
configuration; Cargo fetches from that exact local commit. No private sources,
Cargo caches or binaries are uploaded as public workflow artifacts. GitHub's
SSH host key is handled by `actions/checkout`. Fork PRs do not receive the key;
ordinary public service-only CI never fetches Makepad. Build machines need the
same read-only repository access for the initial private checkout; the package
helper then uses disposable committed sources without credentials.
