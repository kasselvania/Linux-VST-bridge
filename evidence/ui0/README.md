# UI0 source-owned manager captures

These are local Rust/egui renders from `manager-ui/examples/operator_preview.rs` using the repository's synthetic `library-preview.json` fixture. The example disables manager requests. The captures are design and layout evidence, not installed, live, Deck, audio, or physical touch acceptance.

## Hierarchy

Before: one scroll surface held the product library, vendor applications, environments, diagnostics, installer workflow, attempts, incidents, receipts, and raw JSON. See the earlier [ordinary](../manager-library/library-wide.png) and [narrow](../manager-library/library-narrow.png) source captures.

After: a persistent health and request band sits above Home, Plug-ins, Workspaces, Activity, Setup, and Diagnostics. Home leads with published plug-ins, direct attention, running sessions, and active setup work. Plug-ins holds per-product state and manager-offered actions. Activity owns live and recent sessions, attention, and incidents. Setup owns onboarding, vendor applications, environments, and reconciliation. Workspaces is an explicit empty state until the canonical manager projects one. Exact history, receipts, identities, hashes, refusal reasons, and raw data remain behind labeled details.

## Captures

| Capture | What it shows |
| --- | --- |
| [Home 960×720](home-960.png) | Two-column music-first summary and live bridge activity |
| [Home 560×720](home-560.png) | Narrow health, direct attention, and published products in the first screen |
| [Home dark 560×720](home-dark-560.png) | System dark visuals |
| [Plug-ins 960×720](plugins-idle-960.png) | Published product and one emphasized manager-offered action |
| [Plug-ins busy 960×720](plugins-960.png) | Busy state and product details |
| [Activity 560×720](activity-560.png) | First-class live sessions |
| [Setup 560×720](setup-560.png) | Installer and vendor work remain reachable |
| [Workspaces 560×720](workspaces-560.png) | Honest empty managed-DAW area |

Recreate a capture from the repository root with:

```sh
cargo run --manifest-path manager-ui/Cargo.toml --locked --example operator_preview -- evidence/ui0/home-560.png 560 home busy light
```

The arguments are output path, width, page (`home`, `plugins`, `workspaces`, `activity`, `setup`, `diagnostics`), synthetic state (`busy`, `idle`, `unavailable`, `cleanup`), and theme (`light`, `dark`). Height is 720. The preview does not contact or replace the installed manager.

## Remaining acceptance

Keyboard, trackpad, and physical touch behavior need an installed frontend pass after Deck custody is handed to UI0. The preview cannot prove Bitwig browser publication, audio, DAW workspace behavior, or Game Mode state.
