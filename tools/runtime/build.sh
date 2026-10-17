#!/usr/bin/env bash
# Clean source comparison only. No install, publication or runtime selection.
set -euo pipefail

if [[ $# != 1 ]]; then
    echo 'Usage: tools/runtime/build.sh NEW_OUTPUT_DIRECTORY' >&2
    exit 2
fi
recipe_dir=$(cd -- "$(dirname -- "$0")" && pwd)
output=$(realpath -m -- "$1")
if [[ "$output" == /mnt/Storage2/* ]]; then
    [[ $(findmnt -n -o TARGET -T /mnt/Storage2) == /mnt/Storage2 ]] || {
        echo 'Storage2 is not mounted' >&2; exit 1;
    }
fi
mkdir -m 700 -- "$output"
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
mkdir -p -- "$output/sdk-rootfs/work"
cp -- "$recipe_dir/build-inner.sh" "$output/build-inner.sh"

# This is the entire Wine engine
# with its source-built dependencies, not make redist: browser/Mono packaging
# and production distribution remain separate decisions and later work.
if [[ "$output" == /mnt/Storage2/* ]]; then
    exec 9>/mnt/Storage2/LinuxPluginBridge/heavy-build.lock
    flock -n 9 || { echo 'The shared builder is busy' >&2; exit 1; }
fi
systemd-run --user --scope --unit="lvb-runtime-build-$$" \
    -p CPUQuota=300% -p MemoryMax=6G -p MemorySwapMax=0 -p TasksMax=512 \
    bwrap --unshare-user --uid 0 --gid 0 --unshare-pid --die-with-parent \
    --ro-bind "$output/sdk-rootfs" / --bind "$output" /work \
    --ro-bind /etc/resolv.conf /etc/resolv.conf --proc /proc --dev /dev --tmpfs /tmp \
    --setenv PATH /usr/lib/ccache:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin \
    --setenv CARGO_HOME /work/cargo --setenv XDG_CACHE_HOME /work/cache \
    --setenv CCACHE_DISABLE 1 --chdir /work/build \
    /bin/bash /work/build-inner.sh "$epoch" > "$output/build.log" 2>&1
echo "Source Wine build complete: $build_dir"
