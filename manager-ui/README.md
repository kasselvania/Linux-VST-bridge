# Native manager frontend

PB0 shows the manager's separate supported-system readiness result on Home and
Setup. The outcomes are ready, action required, unsupported and unknown. The
first screen shows one next step; Setup holds fact provenance and the explicit
local support-export control. The frontend does not decide platform or product
support. A changed snapshot token or capacity readback hides an old assessment
until fresh manager readback arrives.

The Rust/egui frontend receives a schema-1 overview containing the manager's schema-8 snapshot and matching supported-system assessment in one readback. Home keeps bridge readiness, capacity, published plug-ins, current work and attention visible. Plug-ins has the searchable exact product library; Activity shows live and recent bridge sessions plus sanitized incidents; Setup contains installer attempts, vendor applications, environments and transaction reconciliation; Diagnostics retains system and capture detail. Workspaces shows an existing canonical managed DAW workspace, its selected and installed releases, current state, history and only manager-offered actions. It stays empty when no workspace exists.

The manager remains the sole authority. Buttons use only its closed offered actions, exact state token and refusal reasons. The request banner stays outside page scrolling; uncertain acknowledgments never resubmit an operation. The manager's read-only session identity distinguishes Activity rows, while the current session record still identifies only a plug-in class, not a historical installed build. Closing the frontend does not stop audio or vendor applications.

Published plug-ins are manager publications for Bitwig; Bitwig may still need its own browser rescan. Product history, candidate evidence, exact hashes, installer outcomes, receipts and raw technical data remain under labeled Details. Disabled reasons are printed for touch use as well as hover.

## Local checks and previews

```sh
cargo test --manifest-path manager-ui/Cargo.toml --locked
cargo clippy --manifest-path manager-ui/Cargo.toml --locked --all-targets -- -D warnings
cargo run --manifest-path manager-ui/Cargo.toml --locked --example operator_preview -- /tmp/home.png 960 home busy light
cargo run --manifest-path manager-ui/Cargo.toml --locked --example operator_preview -- /tmp/home-narrow.png 560 home busy light
```

The operator preview renders the production views without starting the manager client. Its arguments are `OUTPUT.png [WIDTH] [PAGE] [busy|idle|unavailable|cleanup|shared|readiness_ready|readiness_action|readiness_unsupported|readiness_unknown] [light|dark]`; supported pages are `home`, `plugins`, `workspaces`, `activity`, `setup` and `diagnostics`. The Workspaces preview contains a clearly synthetic uninstalled FL workspace and prior history. A `real` preview state may be supplied with exact snapshot and readiness JSON filenames as two additional arguments; it never executes actions and labels the capture as a readback preview. It saves its own frame and closes without pointer or keyboard input. The older `library_preview` remains for isolated library checks. [UI0 source-owned captures](../evidence/ui0/README.md) show ordinary and narrow layouts; [PB0 captures](../evidence/pb0/README.md) show the four readiness outcomes.

User instructions remain in the [Crash capture guide (PDF)](../output/pdf/Plug-in-Crash-Capture-Guide.pdf) and [editable source](../docs/user/plug-in-crash-capture.md). This source change does not install a new frontend or qualify a plug-in.
