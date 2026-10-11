# Native manager frontend

The Rust/egui frontend shows the paired manager's current plug-ins, setup jobs and offered actions through versioned readback. Home separates bridge-service health from plug-in availability, keeps **Add a plug-in** at the top, and shows the next useful installation, preparation or recovery step. Each plug-in opens its exact setup and settings. A missing computer observation does not erase a recorded DAW publication, and an available publication does not claim tested sound or project recall.

Setup starts with **Choose Windows installer**. Selecting a file adds a managed copy; starting installation is a separate action. Vendor installation and sign-in happen in the vendor's own window. Discovery links to the exact found classes on Plug-ins, where the user checks/prepares them and makes them available to the DAW. Plug-ins has search and vendor grouping; opening one removes list-only filters and keeps the current primary action, recovery and vendor controls visible. Alternative candidate/profile choices appear under **Other setup options**, and exact retained restore offers appear under **Previous setups**; both preserve offered refusals. Available plug-ins appear as native VST3s; a native Linux DAW may need its own VST3 rescan. Project save and reopening belong to the DAW.

Activity shows current and recent sessions and sanitized incidents. Workspaces shows a canonical managed Windows DAW workspace when one exists. Diagnostics retains system, capture and history detail. Shared-setup controls state affected plug-ins and vendor-state consequences beside the action. Compatibility controls appear only when the manager supplies their current offers.

The fixed system package desktop launcher first asks the package owner for a typed, read-only first-run status. It does this even if the user command route is missing, since an existing verified pre-PKG0 generation may still be selected. A fresh or verified legacy installation offers explicit **Set up application** or **Apply installed package**; a retained transition offers **Finish interrupted setup**. A missing or stale package-owned route with the exact selected service still running offers **Stop bridge service for setup** only after the manager confirms clean idle ownership. The user must press it; the manager then rechecks the selected service, stops it, and offers package reapplication. Foreign routes and ambiguous service identity remain refusals. Successful adoption is followed by an explicit **Start bridge service** action. The frontend opens the paired selected copy only after the manager verifies the active generation, routes, effective service command and health. No page load adopts, stops or activates a generation.

The manager remains the sole authority. Buttons use only its closed offered actions, exact state token and refusal reasons. Foreground import/action feedback stays outside page scrolling until dismissed, while background readback stays quiet; uncertain acknowledgments never resubmit an operation. The manager's read-only session identity distinguishes Activity rows, while the current session record still identifies only a plug-in class, not a historical installed build. Closing the frontend does not stop audio or vendor applications.

Product history, compatibility results, exact hashes, installer outcomes and technical observations remain under labeled details. Disabled reasons are printed for touch use as well as hover. Current status refresh preserves navigation and pending controls; it never assumes that an operation completed.

## Local checks and previews

```sh
cargo test --manifest-path manager-ui/Cargo.toml --locked
cargo clippy --manifest-path manager-ui/Cargo.toml --locked --all-targets -- -D warnings
cargo run --manifest-path manager-ui/Cargo.toml --locked --example operator_preview -- /tmp/home.png 960 home available dark
cargo run --manifest-path manager-ui/Cargo.toml --locked --example operator_preview -- /tmp/plugin-narrow.png 560 plugins plugin_action dark
cargo run --manifest-path manager-ui/Cargo.toml --locked --example bootstrap_preview -- /tmp/first-run.png 560 first
```

The operator preview exercises the shipped current-overview Home/Setup views and exact product-detail path with clearly marked sample records. It never contacts the manager or executes an operation. Arguments are `OUTPUT.png [WIDTH] [PAGE] [STATE] [light|dark] [HEIGHT] [SNAPSHOT.json] [OPEN_LABEL]`; the default size is 960 by 720. The optional final label clicks a rendered disclosure before capture, for example `Other setup options`. Pages are `home`, `plugins`, `workspaces`, `activity`, `setup` and `diagnostics`. Useful states are:

- `first_use`: empty product and runtime-acquisition offer.
- `populated` or `available`: mixed current products or all available products.
- `setup_many`: several prior installers, with Add still at the top.
- `install_running` or `recoverable`: a current job alongside an older dormant import.
- `pending` or `stale`: pending request or unavailable current status.
- `plugin_ready`, `plugin_action` or `plugin_details`: focused availability, preparation action or expanded technical detail.
- `retained_installation`: available plug-ins alongside a retained installation record needing refresh, without a current offered continuation.
- `plugin_history` or `plugin_history_open`: focused current setup with twelve retained restore choices, collapsed or expanded.
- `pb0_unsupported`: specific required architecture failure without an offered action.
- `recorded_current` or `recorded_expanded`: load an explicitly supplied, allow-listed snapshot to reproduce current Home/Setup or exact plug-in detail, with disclosures collapsed or expanded. Keep captured snapshots outside Git and exclude account, license, proprietary content and session data.

Historical busy/guided states remain available. The first-run `bootstrap_preview` accepts `OUTPUT.png [WIDTH] [first|legacy|stop|activate|attention]` and never invokes package commands. Screenshot files belong outside Git. The older isolated `library_preview` is useful for library-only checks; it does not replace current application renders.

User instructions remain in the [Crash capture guide (PDF)](../output/pdf/Plug-in-Crash-Capture-Guide.pdf) and [editable source](../docs/user/plug-in-crash-capture.md).
