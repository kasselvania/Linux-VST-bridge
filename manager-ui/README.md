# Native manager frontend

The plug-in library projects the manager's schema-7 snapshot. It groups records
by vendor, searches name/vendor/type/version, and filters by role and publication
status. Unknown statuses remain visible under Needs attention. Publication is
not a claim that Bitwig has already rescanned or that a plug-in is qualified.

Vendor launch/focus and rescan controls are copied from the manager's offered
actions using the exact environment identity. Names never select an environment.
An older snapshot without vendor environment metadata simply omits the shortcut;
its original vendor controls remain available below the library. Requests retain
the current state token, inactive requirements, disabled reasons and existing
submission/recovery behavior.

## Local checks and preview

```sh
cargo test --manifest-path manager-ui/Cargo.toml --locked
cargo clippy --manifest-path manager-ui/Cargo.toml --locked --all-targets -- -D warnings
cargo run --manifest-path manager-ui/Cargo.toml --locked --example library_preview
```

The example renders the production library widget with synthetic records.
It has no manager client and its buttons never execute operations. To save a
frame and exit, append `-- OUTPUT.png [WIDTH] [SEARCH]`. It needs a graphical
session and captures only its own rendered frame; it sends no keyboard or pointer
input. The production frontend accepts no new command-line arguments.

Recorded local verification and images: [manager library](../evidence/manager-library/README.md).
