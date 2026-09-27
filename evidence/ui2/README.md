# UI2 source-owned Plug-ins captures

These frames are synthetic previews rendered from the public UI2 integration
source. They use the production egui Plug-ins view without starting the manager
client or changing the Steam Deck. The example names and identities are fixture
data; no BEAM-family support is inferred from these images.

| Capture | Logical window | State shown |
|---|---:|---|
| `guided-960.png` | 960 × 720 | Experimental test configuration and result route |
| `guided-new-560.png` | 560 × 720 | Unchecked product and one offered check |
| `guided-pending-560.png` | 560 × 720 | Problem retained while cleanup is unconfirmed; Finish disabled with visible reason |

Regenerate with:

```sh
cargo run --manifest-path manager-ui/Cargo.toml --locked --example operator_preview -- evidence/ui2/guided-960.png 960 plugins guided light
cargo run --manifest-path manager-ui/Cargo.toml --locked --example operator_preview -- evidence/ui2/guided-new-560.png 560 plugins guided_new light
cargo run --manifest-path manager-ui/Cargo.toml --locked --example operator_preview -- evidence/ui2/guided-pending-560.png 560 plugins guided_pending light
```

The PNGs are 2× Retina captures of the stated logical window sizes. They prove
layout rendering only; they are not installation, input-device, audio or
commercial plug-in acceptance evidence.
