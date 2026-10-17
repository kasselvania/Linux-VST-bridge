#!/usr/bin/env bash
# Reserved homelab source comparison. No install, publication or selection.
set -euo pipefail
if [[ $# != 1 ]]; then
    echo 'Usage: tools/runtime/build.sh NEW_STORAGE2_OUTPUT_DIRECTORY' >&2
    exit 2
fi
recipe_dir=$(cd -- "$(dirname -- "$0")" && pwd)
output=$(realpath -m -- "$1")
[[ "$output" == /mnt/Storage2/LinuxPluginBridge/?* ]] || {
    echo 'This homelab recipe requires an output under /mnt/Storage2/LinuxPluginBridge' >&2
    exit 1
}
[[ $(findmnt -n -o TARGET -T /mnt/Storage2) == /mnt/Storage2 ]] || {
    echo 'Storage2 is not mounted' >&2; exit 1;
}
exec 9>/mnt/Storage2/LinuxPluginBridge/heavy-build.lock
flock -n 9 || { echo 'The shared builder is busy' >&2; exit 1; }
mkdir -m 700 -- "$output"
systemd-run --user --scope --unit="lvb-runtime-build-$$" \
    -p CPUQuota=300% -p MemoryMax=6G -p MemorySwapMax=0 -p TasksMax=512 \
    /bin/bash "$recipe_dir/build-work.sh" "$output" > "$output/build.log" 2>&1
echo "Source Wine build complete: $output/build"
