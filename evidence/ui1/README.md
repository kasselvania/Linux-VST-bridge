# UI1 source-owned Setup captures

These are synthetic preview frames rendered from the public UI1 integration
source. They exercise the production egui Setup view without starting a manager
client or changing the Steam Deck.

| Capture | Window | Content |
|---|---:|---|
| `setup-960.png` | 960 × 720 | Ordinary Setup layout, idle bridge |
| `setup-560.png` | 560 × 720 | Narrow Setup layout and 3 × 2 navigation |

Regenerate with:

```sh
cargo run --manifest-path manager-ui/Cargo.toml --locked --example operator_preview -- evidence/ui1/setup-960.png 960 setup idle light
cargo run --manifest-path manager-ui/Cargo.toml --locked --example operator_preview -- evidence/ui1/setup-560.png 560 setup idle light
```

The Lunacy item, installer identity, and product links in these frames are
fixtures. They are not Deck readback or commercial plug-in acceptance evidence.
The earlier private UI1 physical check remains attached to its original
private source generation and is not transferred to this public integration.
