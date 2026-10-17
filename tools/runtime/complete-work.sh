#!/usr/bin/env bash
# Internal entry; complete.sh owns storage, lock and cgroup limits.
set -euo pipefail
recipe_dir=$(cd -- "$(dirname -- "$0")" && pwd)
output=$1
python3 "$recipe_dir/prepare.py" --verify-prepared-source "$output/proton/wine"
python3 "$recipe_dir/redist-inputs.py" --build-root "$output"
git -C "$output/proton/gst-plugins-rs" diff --exit-code HEAD -- \
    Cargo.toml Cargo.lock video/dav1d/Cargo.toml version-helper/Cargo.toml
cp -- "$recipe_dir/build-inner.sh" "$output/build-inner.sh"

# Cargo fetch resolves this upstream workspace, including cache-only packages.
# The exact unchanged lockfile owns all registry checksums and Git revisions.
bash "$recipe_dir/sdk.sh" "$output" online cargo fetch --locked \
    --manifest-path /work/proton/gst-plugins-rs/Cargo.toml \
    --target i686-unknown-linux-gnu --target x86_64-unknown-linux-gnu
git -C "$output/proton/gst-plugins-rs" diff --exit-code HEAD -- Cargo.lock
epoch=$(python3 -c 'import json,sys;print(json.load(open(sys.argv[1]))["proton"]["source_date_epoch"])' \
    "$recipe_dir/source.json")
bash "$recipe_dir/sdk.sh" "$output" offline /bin/bash /work/build-inner.sh "$epoch" redist
mkdir -p -- "$output/build/redist/third-party-notices"
cp -a -- "$output/third-party-notices/." "$output/build/redist/third-party-notices/"
python3 "$recipe_dir/build-identity.py" --build-root "$output"
