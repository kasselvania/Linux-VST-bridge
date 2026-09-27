# Native manager frontend

The Rust/egui frontend projects the paired manager's operator schema-9 snapshot. Manager and frontend must be installed as a pair; retained installer, workspace, and operation records keep their separate durable schemas. Home keeps bridge readiness, capacity, published plug-ins, current work and attention visible. Plug-ins has the searchable exact product library; Activity shows live and recent bridge sessions plus sanitized incidents; Setup groups installer setup by exact artifact, with vendor applications, environments and transaction reconciliation under deeper surfaces; Diagnostics retains system and capture detail. Workspaces shows an existing canonical managed DAW workspace, its selected and installed releases, current state, history and only manager-offered actions. It stays empty when no workspace exists.

The manager remains the sole authority. Buttons use only its closed offered actions, exact state token and refusal reasons. Foreground import/action feedback stays outside page scrolling until dismissed, while background readback stays quiet; uncertain acknowledgments never resubmit an operation. The manager's read-only session identity distinguishes Activity rows, while the current session record still identifies only a plug-in class, not a historical installed build. Closing the frontend does not stop audio or vendor applications.

Published plug-ins are manager publications for Bitwig; Bitwig may still need its own browser rescan. Product history, candidate evidence, exact hashes, installer outcomes, receipts and raw technical data remain under labeled Details. Disabled reasons are printed for touch use as well as hover.

## Local checks and previews

```sh
cargo test --manifest-path manager-ui/Cargo.toml --locked
cargo clippy --manifest-path manager-ui/Cargo.toml --locked --all-targets -- -D warnings
cargo run --manifest-path manager-ui/Cargo.toml --locked --example operator_preview -- /tmp/home.png 960 home busy light
cargo run --manifest-path manager-ui/Cargo.toml --locked --example operator_preview -- /tmp/home-narrow.png 560 home busy light
```

The operator preview renders the production views with synthetic records and never starts the manager client. Its arguments are `OUTPUT.png [WIDTH] [PAGE] [busy|idle|unavailable|cleanup|shared] [light|dark]`; supported pages are `home`, `plugins`, `workspaces`, `activity`, `setup` and `diagnostics`. The Setup preview contains a clearly synthetic Lunacy discovery card and exact product links; Workspaces contains a synthetic uninstalled FL workspace. The preview saves its own frame and closes without input. The older `library_preview` remains for isolated library checks. [UI0 source-owned captures](../evidence/ui0/README.md) and [UI1 captures](../evidence/ui1/README.md) show the hierarchy before and after this slice.

User instructions remain in the [Crash capture guide (PDF)](../output/pdf/Plug-in-Crash-Capture-Guide.pdf) and [editable source](../docs/user/plug-in-crash-capture.md). This source change does not install a new frontend or qualify a plug-in.
