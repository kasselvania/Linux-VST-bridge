#!/bin/sh
# Development build; not an operator command or product dependency on a checkout.
set -eu
MF1_ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
MF1_TARGET=x86_64-unknown-linux-gnu
rustup target add --toolchain 1.95.0 "$MF1_TARGET"
if [ "$(uname -s)" = Darwin ]; then
    MF1_LINKER=$(mktemp)
    trap 'rm -f "$MF1_LINKER"' EXIT HUP INT TERM
    printf '%s\n' '#!/bin/sh' 'exec zig cc -target x86_64-linux-gnu "$@"' > "$MF1_LINKER"
    chmod 700 "$MF1_LINKER"
    export CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER="$MF1_LINKER"
fi
export RUSTFLAGS="--remap-path-prefix=$MF1_ROOT=."
cargo +1.95.0 build --manifest-path "$MF1_ROOT/bridge-manager/Cargo.toml" --locked --release --target "$MF1_TARGET" --bin linux-vst-bridge
cargo +1.95.0 build --manifest-path "$MF1_ROOT/manager-ui/Cargo.toml" --locked --release --target "$MF1_TARGET"
