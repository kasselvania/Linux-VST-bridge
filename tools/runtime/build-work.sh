#!/usr/bin/env bash
# Internal acquisition/build entry; build.sh owns storage, lock and cgroup limits.
set -euo pipefail
recipe_dir=$(cd -- "$(dirname -- "$0")" && pwd)
output=$1
source_dir=$output/proton
build_dir=$output/build
mkdir -- "$source_dir" "$build_dir" "$output/cargo" "$output/cache" "$output/tool"

read_recipe() {
    python3 -c 'import json,sys; x=json.load(open(sys.argv[1]));
for p in sys.argv[2:]: x=x[p]
print(x)' "$recipe_dir/source.json" "$@"
}
proton_commit=$(read_recipe proton commit)
wine_commit=$(read_recipe wine commit)
sdk=$(read_recipe sdk)
epoch=$(read_recipe proton source_date_epoch)
git -C "$source_dir" init
git -C "$source_dir" remote add origin "$(read_recipe proton repository)"
git -C "$source_dir" fetch --no-tags --depth=1 origin "$proton_commit"
git -C "$source_dir" checkout --detach FETCH_HEAD
[[ $(git -C "$source_dir" rev-parse 'HEAD^{tree}') == "$(read_recipe proton tree)" ]]

# The exact Proton Git tree owns all auxiliary gitlinks. Acquire its original
# Wine too, then replace its source checkout; no compiled Wine module is copied.
git -C "$source_dir" submodule update --init --recursive --depth=1 --jobs=3
git -C "$source_dir/wine" remote add lvb-comparison "$(read_recipe wine repository)"
git -C "$source_dir/wine" fetch --no-tags --depth=1 lvb-comparison "$wine_commit"
git -C "$source_dir/wine" checkout --detach FETCH_HEAD
python3 "$recipe_dir/prepare.py" --prepare-source "$source_dir/wine"
python3 "$recipe_dir/build-inputs.py" --proton-source "$source_dir"

# Acquire the image directly into the output tree, without Docker's global
# image cache. The acquisition helper is an exact upstream release artifact.
curl --fail --location \
    https://github.com/google/go-containerregistry/releases/download/v0.22.1/go-containerregistry_Linux_x86_64.tar.gz \
    --output "$output/tool/crane.tar.gz"
echo '0ab7a1d6932a213aed964ce97666c3077fe691c8606413674a8b3e0b9ec4cda0  crane.tar.gz' \
    | (cd -- "$output/tool" && sha256sum --check)
tar --no-same-owner -xzf "$output/tool/crane.tar.gz" -C "$output/tool" crane
"$output/tool/crane" export "$sdk" "$output/sdk-rootfs.tar"
mkdir -- "$output/sdk-rootfs"
tar --no-same-owner -xf "$output/sdk-rootfs.tar" -C "$output/sdk-rootfs"
mkdir -p -- "$output/sdk-rootfs/work/"{proton,build,cargo,cache}
touch -- "$output/sdk-rootfs/work/build-inner.sh"
cp -- "$recipe_dir/build-inner.sh" "$output/build-inner.sh"

# This is the entire Wine engine
# with its source-built dependencies, not make redist: browser/Mono packaging
# and production distribution remain separate decisions and later work.
bash "$recipe_dir/sdk.sh" "$output" offline /bin/bash /work/build-inner.sh "$epoch"
